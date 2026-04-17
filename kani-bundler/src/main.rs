//! `kani-bundler` — project manager for kani-engine games.
//!
//! Subcommands:
//! - `new <name>`                    scaffold a new project
//! - `run [--project <dir>]`         launch in developer mode
//! - `check [--project <dir>]`       validate scripts and asset references
//! - `bundle [--project <dir>] [--target <triple>]`
//!                                   build a distributable release

mod cmd;
mod config;

use std::path::PathBuf;

use clap::{Parser, Subcommand};

#[derive(Parser)]
#[command(name = "kani-bundler", about = "Project manager for kani-engine games")]
struct Cli {
    #[command(subcommand)]
    command: Commands,
}

#[derive(Subcommand)]
enum Commands {
    /// Create a new project scaffold.
    New {
        /// Name of the project (also the directory to create).
        name: String,
    },

    /// Run the game in developer mode (hot-reload, filesystem assets).
    Run {
        /// Path to the project directory (default: current directory).
        #[arg(long, default_value = ".")]
        project: PathBuf,
    },

    /// Check scripts for syntax errors and missing asset references.
    Check {
        /// Path to the project directory (default: current directory).
        #[arg(long, default_value = ".")]
        project: PathBuf,
    },

    /// Bundle the project into a distributable `.pak` + binary.
    Bundle {
        /// Path to the project directory (default: current directory).
        #[arg(long, default_value = ".")]
        project: PathBuf,

        /// Override the Rust target triple (default: from kani.toml or host).
        #[arg(long)]
        target: Option<String>,
    },
}

fn main() -> anyhow::Result<()> {
    let cli = Cli::parse();

    match cli.command {
        Commands::New { name } => cmd::new::run(&name),
        Commands::Run { project } => cmd::run::run(&project),
        Commands::Check { project } => cmd::check::run(&project),
        Commands::Bundle { project, target } => cmd::bundle::run(&project, target.as_deref()),
    }
}
