use anyhow::Context;
use anyhow::bail;
use anyhow::ensure;
use chardetng::EncodingDetector;
use chardetng::Iso2022JpDetection;
use chardetng::Utf8Detection;
use clap::Parser;
use std::borrow::Cow;
use std::fs::File;
use std::fs::FileTimes as StdFileTimes;
use std::io::Read;
use std::io::Write;
use std::path::Component as PathComponent;
use std::path::Path;
use std::path::PathBuf;
use std::time::SystemTime;
use time::OffsetDateTime;
use time::PrimitiveDateTime;
use zip::ZipArchive;
use zip::read::ZipFile;

#[derive(Debug, Parser)]
#[command(about = "Extract a zip file")]
pub struct Options {
    pub input_file: PathBuf,

    #[arg(short = 'o', long = "out-path", help = "The path to decompress to")]
    pub out_path: PathBuf,

    #[arg(short = 'v', long = "verbose", help = "Increase command verbosity")]
    pub verbose: bool,
}

struct FileTimes {
    accessed: Option<SystemTime>,
    modified: Option<SystemTime>,
    created: Option<SystemTime>,
}

impl FileTimes {
    fn has_time(&self) -> bool {
        self.accessed.is_some() || self.modified.is_some() || self.created.is_some()
    }
}

impl From<FileTimes> for StdFileTimes {
    fn from(times: FileTimes) -> Self {
        let mut std_times = StdFileTimes::new();
        if let Some(accessed) = times.accessed {
            std_times = std_times.set_accessed(accessed);
        }
        if let Some(modified) = times.modified {
            std_times = std_times.set_modified(modified);
        }

        #[cfg(windows)]
        if let Some(created) = times.created {
            use std::os::windows::fs::FileTimesExt;
            std_times = std_times.set_created(created);
        }

        #[cfg(target_vendor = "apple")]
        if let Some(created) = times.created {
            use std::os::darwin::fs::FileTimesExt;
            std_times = std_times.set_created(created);
        }

        std_times
    }
}

/// Get the file times for a zip file.
fn get_zip_entry_file_times<R>(file: &ZipFile<'_, R>) -> anyhow::Result<FileTimes>
where
    R: Read,
{
    // I have no idea how to do this properly.
    // I think nobody else does too.
    // Zip files have a modern time format and a legacy one.
    // (modern format handling is TODO).
    // Lots of zip files only use the legacy format.
    // This format is an encoded date time, with no timezone.
    // As a result, lots of software can't agree how to handle it.
    // 7Zip, WinRar, the Windows 11 file extractor, and this impl all give different answers,
    // even apparently giving different UTC offsets between different files.
    // I have no idea what I'm doing something wrong, if I'm even doing anything wrong, or if everyone else is doing something wrong.
    // This is best effort anyways, and usually within a day of the "real?" value.

    // TODO: Read extra fields
    // dbg!(file.extra_data_fields().count());

    match file.last_modified() {
        Some(last_modified) => {
            let last_modified = PrimitiveDateTime::try_from(last_modified)?.assume_utc();
            let last_modified = SystemTime::from(last_modified);

            Ok(FileTimes {
                accessed: Some(last_modified),
                modified: Some(last_modified),
                created: Some(last_modified),
            })
        }
        None => Ok(FileTimes {
            accessed: None,
            modified: None,
            created: None,
        }),
    }
}

fn get_zip_entry_file_name<'a, R>(file: &'a ZipFile<R>) -> anyhow::Result<Cow<'a, str>>
where
    R: Read,
{
    let file_name_raw = file.name_raw();

    let mut encoding_detector = EncodingDetector::new(Iso2022JpDetection::Deny);
    let is_last = true;
    encoding_detector.feed(file_name_raw, is_last);
    let encoding = encoding_detector.guess(None, Utf8Detection::Allow);

    let (file_name, _encoding, malformed) = encoding.decode(file_name_raw);

    ensure!(!malformed, "File name \"{file_name}\" is malformed");

    let has_nul = file_name.contains('\0');
    ensure!(!has_nul, "File name has an interior NUL character");

    let file_path = Path::new(&*file_name);
    let mut depth: usize = 0;
    for component in file_path.components() {
        match component {
            PathComponent::Prefix(_) => {
                bail!("File name contains a prefix");
            }
            PathComponent::RootDir => {
                bail!("File name is absolute");
            }
            PathComponent::ParentDir => {
                depth = depth
                    .checked_sub(1)
                    .context("File name attempts to go above root directory")?;
            }
            PathComponent::Normal(_) => {
                depth = depth
                    .checked_add(1)
                    .context("File name exceeds maximum depth")?;
            }
            PathComponent::CurDir => {}
        }
    }

    Ok(file_name)
}

pub fn exec(options: Options) -> anyhow::Result<()> {
    let input_file = File::open(&options.input_file)
        .with_context(|| format!("Failed to open \"{}\"", options.input_file.display()))?;
    let mut archive = ZipArchive::new(input_file)?;

    let mut dir_times = Vec::new();
    for i in 0..archive.len() {
        let mut file = archive.by_index(i)?;

        let file_name = get_zip_entry_file_name(&file)?;

        let out_path = options.out_path.join(&*file_name);

        let times = get_zip_entry_file_times(&file)?;

        if options.verbose {
            println!("{file_name}");

            if let Some(accessed) = times.accessed {
                println!("  Accessed: {}", OffsetDateTime::from(accessed));
            }

            if let Some(modified) = times.modified {
                println!("  Modified: {}", OffsetDateTime::from(modified));
            }

            if let Some(created) = times.created {
                println!("  Created: {}", OffsetDateTime::from(created));
            }
        }

        if file.is_dir() {
            std::fs::create_dir_all(&out_path).with_context(|| {
                format!("Failed to create directory \"{}\"", out_path.display())
            })?;

            if times.has_time() {
                dir_times.push((out_path.clone(), times));
            }
        } else if file.is_file() {
            // Some bad ZIP files do not provide a dir entry before a file entry.
            if let Some(parent_dir) = out_path.parent() {
                std::fs::create_dir_all(parent_dir).with_context(|| {
                    format!("Failed to create directory \"{}\"", out_path.display())
                })?;
            }

            let mut out_file = File::options()
                .write(true)
                .create_new(true)
                .open(&out_path)
                .with_context(|| format!("Failed to open file at \"{}\"", out_path.display()))?;
            std::io::copy(&mut file, &mut out_file)?;

            if times.has_time() {
                out_file.set_times(times.into())?;
            }

            out_file.flush()?;
            out_file.sync_all()?;
        } else {
            bail!("Cannot extract entry that is not a file nor a dir");
        }
    }

    for (path, times) in dir_times.into_iter().rev() {
        std::fs::set_times(path, times.into())?;
    }

    Ok(())
}
