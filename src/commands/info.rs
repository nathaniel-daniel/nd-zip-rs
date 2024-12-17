use anyhow::Context;
use std::fs::File;
use std::path::PathBuf;
use zip::HasZipMetadata;
use zip::ZipArchive;

#[derive(Debug, argh::FromArgs)]
#[argh(subcommand, name = "info", description = "get info about a zip file")]
pub struct Options {
    #[argh(positional)]
    pub input_file: PathBuf,
}

pub fn exec(options: Options) -> anyhow::Result<()> {
    let input_file = File::open(&options.input_file)
        .with_context(|| format!("failed to open \"{}\"", options.input_file.display()))?;
    let mut archive = ZipArchive::new(input_file)?;

    for i in 0..archive.len() {
        let file = archive.by_index(i)?;

        println!("File Entry {i}");
        // TODO: Should we use chardetng here?
        println!("  Name: {}", file.name());
        println!("  Is UTF8?: {}", file.get_metadata().is_utf8);
        println!();
    }

    Ok(())
}
