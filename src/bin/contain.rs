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
    Start {
        config: PathBuf,
        #[arg(short = 'c',
          value_names = ["KEY", "VALUE"],
          num_args = 2,
          action = clap::ArgAction::Append,
          help = "Override a configuration entry")]
        overrides: Vec<String>,
    },
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn Error + Send + Sync>> {
    let cli = Cli::parse();

    match cli.command {
        Commands::Start { config, overrides } => {
            let mut builder = config::Config::builder().add_source(config::File::from(config));

            for (key, value) in overrides
                .chunks_exact(2)
                .map(|p| (p[0].clone(), p[1].clone()))
            {
                builder = builder.set_override(key, value).expect("this is a bug");
            }

            let config: Config = builder
                .build()
                .unwrap()
                .try_deserialize()
                .expect("issue with config");
            run_vm(config).await?;
        }
    }

    Ok(())
}
