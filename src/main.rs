mod commands;

use clap::Parser;

const LONG_VERSION: &str = concat!(env!("CARGO_PKG_VERSION"), " ", "(", env!("GIT_REV"), ")");

#[derive(Debug, clap::Parser)]
#[command(about = "A tool to interact with zip files", long_about=None, long_version = LONG_VERSION)]
struct Options {
    #[command(subcommand)]
    subcommand: Subcommand,
}

#[derive(Debug, clap::Subcommand)]
enum Subcommand {
    Extract(self::commands::extract::Options),
    Info(self::commands::info::Options),
    GenerateCompletions(self::commands::generate_completions::Options),
}

fn main() -> anyhow::Result<()> {
    let options = Options::parse();

    match options.subcommand {
        Subcommand::Extract(options) => self::commands::extract::exec(options)?,
        Subcommand::Info(options) => self::commands::info::exec(options)?,
        Subcommand::GenerateCompletions(options) => {
            self::commands::generate_completions::exec(options)?
        }
    }

    Ok(())
}
