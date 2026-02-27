//! Knull Programming Language - CLI Entry Point

mod cli;
mod compiler;
mod lexer;
mod parser;

use clap::{Parser, Subcommand};
use std::path::PathBuf;

#[derive(Parser)]
#[command(name = "knull")]
#[command(version = "1.0.0")]
#[command(about = "The Knull Programming Language", long_about = None)]
struct Cli {
    #[command(subcommand)]
    command: Commands,

    #[arg(short, long, global = true)]
    verbose: bool,
}

#[derive(Subcommand)]
enum Commands {
    /// Run a Knull file
    Run {
        /// The .knull file to run
        file: PathBuf,
    },
    /// Compile a Knull file
    Build {
        /// The .knull file to compile
        file: PathBuf,
        /// Output file
        #[arg(short, long)]
        output: Option<PathBuf>,
    },
    /// Check syntax without building
    Check {
        /// The .knull file to check
        file: PathBuf,
    },
    /// Format a Knull file
    Fmt {
        /// The .knull file to format
        file: PathBuf,
    },
    /// Start REPL
    Repl,
}

fn main() {
    let cli = Cli::parse();

    let result = match cli.command {
        Commands::Run { file } => cli::run_file(&file),
        Commands::Build { file, output } => cli::build_file(&file, output.as_deref()),
        Commands::Check { file } => cli::check_file(&file),
        Commands::Fmt { file } => cli::format_file(&file),
        Commands::Repl => cli::start_repl(),
    };

    if let Err(e) = result {
        eprintln!("Error: {}", e);
        std::process::exit(1);
    }
}
