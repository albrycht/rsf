pub mod scan;
pub mod volume;
pub mod ui;

use clap::Subcommand;

#[derive(Subcommand)]
pub enum Commands {
    /// Volume-related commands
    Volume {
        #[command(subcommand)]
        command: volume::VolumeCommands,
    },
    /// Scan-related commands
    Scan {
        #[command(subcommand)]
        command: scan::ScanCommands,
    },
    /// Interactive UI mode
    Ui,
} 