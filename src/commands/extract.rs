use anyhow::bail;
use anyhow::ensure;
use anyhow::Context;
use chardetng::EncodingDetector;
use std::fs::File;
use std::fs::FileTimes;
use std::io::Write;
use std::path::Component as PathComponent;
use std::path::Path;
use std::path::PathBuf;
use std::time::SystemTime;
use time::OffsetDateTime;
use zip::read::ZipFile;
use zip::ZipArchive;

#[derive(Debug, argh::FromArgs)]
#[argh(subcommand, name = "extract", description = "extract a zip file")]
pub struct Options {
    #[argh(positional)]
    pub input_file: PathBuf,

    #[argh(
        option,
        short = 'o',
        long = "out-path",
        description = "the path to decompress to"
    )]
    pub out_path: PathBuf,
}

pub fn exec(options: Options) -> anyhow::Result<()> {
    let input_file = File::open(&options.input_file)
        .with_context(|| format!("failed to open \"{}\"", options.input_file.display()))?;
    let mut archive = ZipArchive::new(input_file)?;

    let mut dir_times = Vec::new();
    for i in 0..archive.len() {
        let mut file = archive.by_index(i)?;
        let file_name_raw = file.name_raw();

        let mut encoding_detector = EncodingDetector::new();
        encoding_detector.feed(file_name_raw, true);
        let (encoding, is_likely_correct) = encoding_detector.guess_assess(None, true);

        ensure!(
            is_likely_correct,
            "failed to guess file name character encoding"
        );

        let (file_name, _encoding, malformed) = encoding.decode(file_name_raw);

        ensure!(!malformed, "file name \"{file_name}\" is malformed");

        let has_nul = file_name.contains('\0');
        ensure!(!has_nul, "file name has an interior NUL character");

        let file_path = Path::new(&*file_name);
        let mut depth: usize = 0;
        for component in file_path.components() {
            match component {
                PathComponent::Prefix(_) => {
                    bail!("file name contains a prefix");
                }
                PathComponent::RootDir => {
                    bail!("file name is absolute");
                }
                PathComponent::ParentDir => {
                    depth = depth
                        .checked_sub(1)
                        .context("file name attempts to go above root directory")?;
                }
                PathComponent::Normal(_) => {
                    depth = depth
                        .checked_add(1)
                        .context("file name exceeds maximum depth")?;
                }
                PathComponent::CurDir => {}
            }
        }

        let out_path = options.out_path.join(&*file_name);

        let times = get_zip_entry_file_times(&file)?;

        if file.is_dir() {
            std::fs::create_dir_all(&out_path).with_context(|| {
                format!("failed to create directory \"{}\"", out_path.display())
            })?;

            if let Some(times) = times {
                dir_times.push((out_path.clone(), times));
            }
        } else if file.is_file() {
            // Some bad ZIP files do not provide a dir entry before a file entry.
            if let Some(parent_dir) = out_path.parent() {
                std::fs::create_dir_all(parent_dir).with_context(|| {
                    format!("failed to create directory \"{}\"", out_path.display())
                })?;
            }

            let mut out_file = File::options()
                .write(true)
                .create_new(true)
                .open(&out_path)
                .with_context(|| format!("failed to open file at \"{}\"", out_path.display()))?;
            std::io::copy(&mut file, &mut out_file)?;

            if let Some((accessed, modified)) = times {
                let mut times = FileTimes::new();
                if let Some(accessed) = accessed {
                    times = times.set_accessed(accessed);
                }
                if let Some(modified) = modified {
                    times = times.set_modified(modified);
                }
                out_file.set_times(times)?;
            }

            out_file.flush()?;
            out_file.sync_all()?;
        } else {
            bail!("cannot extract entry that is not a file nor a dir");
        }
    }

    for (path, times) in dir_times {
        match times {
            (Some(accessed), Some(modified)) => {
                filetime::set_file_times(path, accessed.into(), modified.into())?;
            }
            (Some(accessed), None) => {
                filetime::set_file_atime(path, accessed.into())?;
            }
            (None, Some(modified)) => {
                filetime::set_file_mtime(path, modified.into())?;
            }
            (None, None) => {}
        }
    }

    Ok(())
}

/// Get the file times for a zip file.
///
/// # Returns
/// Returns a tuple of the accessed time, modified time, and create time.
fn get_zip_entry_file_times(
    file: &ZipFile<'_>,
) -> anyhow::Result<Option<(Option<SystemTime>, Option<SystemTime>)>> {
    match file.last_modified() {
        Some(last_modified) => {
            let last_modified = OffsetDateTime::try_from(last_modified)?;
            let last_modified = SystemTime::from(last_modified);

            Ok(Some((Some(last_modified), Some(last_modified))))
        }
        None => Ok(None),
    }
}
