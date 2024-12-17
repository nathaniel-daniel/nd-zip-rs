mod commands;

#[derive(Debug, argh::FromArgs)]
#[argh(description = "A tool to interact with zip files")]
struct Options {
    #[argh(subcommand)]
    subcommand: Subcommand,
}

#[derive(Debug, argh::FromArgs)]
#[argh(subcommand)]
enum Subcommand {
    Extract(self::commands::extract::Options),
}

fn main() -> anyhow::Result<()> {
    let options: Options = argh::from_env();

    match options.subcommand {
        Subcommand::Extract(options) => self::commands::extract::exec(options)?,
    }

    Ok(())
}
