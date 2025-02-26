use clap::{Parser, Subcommand};
use std::{error::Error, path::PathBuf};

use contain::{config::Config, run::run_vm};

#[derive(Parser)]
#[command(version, about, long_about = None)]
#[command(propagate_version = true)]
struct Cli {
    #[command(subcommand)]
    command: Commands,
}

#[derive(Subcommand)]
enum Commands {
    Start { config: PathBuf },
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn Error + Send + Sync>> {

    let cli = Cli::parse();

    match cli.command {
        Commands::Start { config } => {
            let config = Config::try_from(config)?;
            run_vm(config).await?;
        },
    }

    Ok(())
}
