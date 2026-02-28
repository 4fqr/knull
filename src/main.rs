//! Knull Programming Language - CLI Entry Point
//! The God Programming Language

#![allow(dead_code)]

mod ast;
mod cli;
mod compiler;
mod interpreter;
mod lexer;
mod ownership;
mod parser;
mod pkg;
mod type_system;

use clap::{Parser, Subcommand};
use colored::Colorize;
use std::path::PathBuf;

const ASCII_ART: &str = r#"
  _  __    _    _   _ _   _ 
 | |/ /   | |  | | | | \ | |
 | ' / ___| |  | | | |  \| |
 | . \___ | |  | |_| | |\  |
 |_|\_\___|_|   \___/|_| \_|
"#;

#[derive(Parser)]
#[command(name = "knull")]
#[command(version = "1.0.0")]
#[command(about = "The Knull Programming Language", long_about = None)]
struct Cli {
    #[command(subcommand)]
    command: Option<Commands>,

    #[arg(short, long, global = true, help = "Verbose output")]
    verbose: bool,
}

#[derive(Subcommand)]
enum Commands {
    /// Run a Knull file
    #[command(alias = "r")]
    Run {
        /// The .knull file to run
        file: PathBuf,
    },
    /// Compile a Knull file to binary
    #[command(alias = "b")]
    Build {
        /// The .knull file to compile
        file: PathBuf,
        /// Output file path
        #[arg(short, long)]
        output: Option<PathBuf>,
        /// Release mode (optimized)
        #[arg(short, long)]
        release: bool,
    },
    /// Generate assembly output
    #[command(alias = "a")]
    Asm {
        /// The .knull file to generate assembly from
        file: PathBuf,
        /// Output assembly file
        #[arg(short, long)]
        output: Option<PathBuf>,
    },
    /// Check syntax without building
    #[command(alias = "c")]
    Check {
        /// The .knull file to check
        file: PathBuf,
    },
    /// Format a Knull file
    #[command(alias = "f")]
    Fmt {
        /// The .knull file to format
        file: PathBuf,
    },
    /// Create a new Knull project
    #[command(alias = "n")]
    New {
        /// Project name
        name: String,
    },
    /// Add a dependency to the project
    #[command(alias = "A")]
    Add {
        /// Package name
        package: String,
        /// Package version
        version: Option<String>,
    },
    /// Run tests
    #[command(alias = "t")]
    Test,
    /// Start interactive REPL
    #[command(alias = "i")]
    Repl,
    /// Show version information
    #[command(alias = "v")]
    Version,
    /// Show help information
    #[command(alias = "h")]
    Help,
}

fn main() {
    // Enable colored output
    colored::control::set_override(true);

    let cli = Cli::parse();

    let result = match cli.command {
        Some(Commands::Run { file }) => cli::run_file(&file, cli.verbose),
        Some(Commands::Build {
            file,
            output,
            release,
        }) => {
            if release {
                println!("{}", "Building in release mode...".bright_yellow());
                cli::build_release(&file, output.as_deref(), cli.verbose)
            } else {
                cli::build_file(&file, output.as_deref(), cli.verbose)
            }
        }
        Some(Commands::Asm { file, output }) => cli::generate_asm(&file, output.as_deref()),
        Some(Commands::Check { file }) => cli::check_file(&file),
        Some(Commands::Fmt { file }) => cli::format_file(&file),
        Some(Commands::New { name }) => cli::new_project(&name),
        Some(Commands::Add { package, version }) => {
            cli::add_dependency(&package, version.as_deref())
        }
        Some(Commands::Test) => cli::run_tests(),
        Some(Commands::Repl) => cli::start_repl(),
        Some(Commands::Version) => {
            show_version();
            Ok(())
        }
        Some(Commands::Help) | None => {
            show_help();
            Ok(())
        }
    };

    match result {
        Ok(_) => {}
        Err(e) => {
            eprintln!("{} {}", "Error:".bright_red().bold(), e);
            std::process::exit(1);
        }
    }
}

fn show_version() {
    println!("{}", ASCII_ART.bright_cyan());
    println!(
        "{} {}",
        "Knull".bright_green().bold(),
        "1.0.0".bright_white()
    );
    println!("{}", "The God Programming Language".bright_yellow());
}

fn show_help() {
    println!("{}", ASCII_ART.bright_cyan());
    println!(
        "{} {} - {}",
        "Knull".bright_green().bold(),
        "1.0.0".bright_white(),
        "The God Programming Language".bright_yellow()
    );
    println!();
    println!("{}", "USAGE:".bright_white().bold());
    println!("  knull [OPTIONS] <COMMAND>");
    println!();
    println!("{}", "COMMANDS:".bright_white().bold());
    println!("  run <file>      Run a Knull file");
    println!("  build <file>    Compile a Knull file to binary");
    println!("  check <file>    Check syntax without building");
    println!("  fmt <file>      Format a Knull file");
    println!("  new <name>      Create a new Knull project");
    println!("  add <pkg>       Add a dependency to the project");
    println!("  test            Run tests");
    println!("  repl            Start interactive REPL");
    println!("  version         Show version information");
    println!("  help            Show this help message");
    println!();
    println!("{}", "OPTIONS:".bright_white().bold());
    println!("  -v, --verbose   Verbose output");
    println!("  -h, --help      Print help");
    println!("  -V, --version   Print version");
    println!();
    println!("{}", "EXAMPLES:".bright_white().bold());
    println!("  knull run hello.knull           # Run a program");
    println!("  knull new myproject             # Create new project");
    println!("  knull build --release main.knull # Build optimized binary");
    println!();
    println!("Documentation: https://github.com/4fqr/knull");
}
