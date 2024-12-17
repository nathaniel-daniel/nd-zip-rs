use anyhow::bail;
use anyhow::ensure;
use anyhow::Context;
use chardetng::EncodingDetector;
use std::fs::File;
use std::io::Write;
use std::path::Component as PathComponent;
use std::path::Path;
use std::path::PathBuf;
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

        if file.is_dir() {
            std::fs::create_dir_all(&out_path).with_context(|| {
                format!("failed to create directory \"{}\"", out_path.display())
            })?;
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
            out_file.flush()?;
            out_file.sync_all()?;
        } else {
            bail!("cannot extract entry that is not a file nor a dir");
        }
    }

    Ok(())
}
