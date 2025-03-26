mod client;
mod commands;

use anyhow::Result;
use clap::{Parser, CommandFactory};
use clap_complete::{generate, Generator, Shell};
use std::io;
use commands::Commands;
use client::Client;

#[derive(Parser)]
#[command(author, version, about, long_about = None)]
struct Cli {
    /// Generate shell completion script
    #[arg(long = "generate", value_enum)]
    generator: Option<Shell>,

    #[command(subcommand)]
    command: Option<Commands>,
}

#[tokio::main]
async fn main() -> Result<()> {
    let cli = Cli::parse();

    if let Some(generator) = cli.generator {
        generate_completion(generator, &mut io::stdout());
        return Ok(());
    }

    let command = cli.command.ok_or_else(|| {
        anyhow::anyhow!("A subcommand is required unless using --generate")
    })?;

    let client = Client::new();

    match command {
        Commands::Volume { command } => {
            commands::volume::handle_volume_command(&client, command).await?
        }
        Commands::Scan { command } => {
            commands::scan::handle_scan_command(&client, command).await?
        }
        Commands::Ui => {
            commands::ui::handle_ui_command(&client).await?
        }
    }

    Ok(())
}

fn generate_completion<G: Generator>(gen: G, buf: &mut dyn io::Write) {
    generate(gen, &mut Cli::command(), "rsf", buf);
}
