//! Knull Programming Language - CLI Entry Point
//! The God Programming Language

#![allow(dead_code)]

mod ast;
mod c_codegen;
mod cli;
mod compiler;
mod comptime;
mod doc;
mod ffi;
mod gc;
mod incremental;
mod interpreter;
mod linear_check;
mod effects;
mod macros;
#[cfg(feature = "lsp")]
mod lsp;
mod lexer;
mod ownership;
mod parser;
mod pkg;
#[cfg(feature = "debugger")]
mod debugger;
mod type_system;
mod wasm_codegen;

#[cfg(feature = "llvm-backend")]
mod llvm_codegen;

use clap::{Parser, Subcommand};
use colored::Colorize;
use std::path::PathBuf;

const ASCII_ART: &str = r#"
.____/\ .______ .____     .___   .___
:   /  \:      \|    |___ |   |  |   |
|.  ___/|       ||    |   ||   |  |   |
|     \ |   |   ||    :   ||   |/\|   |/\
|      \|___|   ||        ||   /  \|   /  \
|___\  /    |___||. _____/ |______/|______/
     \/           :/
                  :
"#;

#[derive(Parser)]
#[command(name = "knull")]
#[command(version = "2.1.0")]
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
        /// Target architecture (e.g., wasm32, x86_64)
        #[arg(short, long, default_value = "native")]
        target: String,
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
    Test {
        /// Run benchmarks instead of tests
        #[arg(short, long)]
        bench: bool,
        /// Run property-based tests
        #[arg(short, long)]
        property: bool,
        /// Generate documentation
        #[arg(short, long)]
        doc: bool,
    },
    /// Start interactive REPL
    #[command(alias = "i")]
    Repl,
    /// Evaluate an inline expression or snippet
    #[command(alias = "e")]
    Eval {
        /// The Knull expression/snippet to evaluate
        expr: String,
    },
    /// Show version information
    #[command(alias = "v")]
    Version,
    /// Show help information
    #[command(name = "show-help")]
    ShowHelp,
    /// Start LSP server for IDE integration
    #[cfg(feature = "lsp")]
    Lsp {
        /// Port to listen on (default: 5007)
        #[arg(short, long, default_value = "5007")]
        port: u16,
        /// Initialize stdin/stdout for LSP protocol
        #[arg(long)]
        stdin: bool,
    },
    /// Start debugger
    #[cfg(feature = "debugger")]
    Debug {
        /// The .knull file to debug
        file: PathBuf,
        /// Breakpoint line numbers
        #[arg(short, long)]
        break_at: Option<Vec<u32>>,
    },
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
            target,
        }) => {
            if release {
                println!("{}", "Building in release mode...".bright_yellow());
                cli::build_release(&file, output.as_deref(), cli.verbose, &target)
            } else {
                cli::build_file(&file, output.as_deref(), cli.verbose, &target)
            }
        }
        Some(Commands::Asm { file, output }) => cli::generate_asm(&file, output.as_deref()),
        Some(Commands::Check { file }) => cli::check_file(&file),
        Some(Commands::Fmt { file }) => cli::format_file(&file),
        Some(Commands::New { name }) => cli::new_project(&name),
        Some(Commands::Add { package, version }) => {
            cli::add_dependency(&package, version.as_deref())
        }
        Some(Commands::Test { bench, property, doc }) => {
            if doc {
                println!("{}", "Generating documentation...".bright_yellow());
                let project_path = std::env::current_dir().unwrap_or_default();
                match crate::doc::generate_docs_for_project(&project_path) {
                    Ok(docs) => {
                        println!("{}", docs);
                        Ok(())
                    }
                    Err(e) => Err(e)
                }
            } else {
                cli::run_tests_with_options(bench, property)
            }
        }
        Some(Commands::Repl) => cli::start_repl(),
        Some(Commands::Eval { expr }) => cli::eval_expr(&expr, cli.verbose),
        Some(Commands::Version) => {
            show_version();
            Ok(())
        }
        Some(Commands::ShowHelp) | None => {
            show_help();
            Ok(())
        }
        #[cfg(feature = "lsp")]
        Some(Commands::Lsp { port, stdin }) => {
            if stdin {
                crate::lsp::run_stdio()?;
            } else {
                crate::lsp::run_server(port)?;
            }
        }
        #[cfg(feature = "debugger")]
        Some(Commands::Debug { file, break_at }) => {
            crate::debugger::start_debug_session(&file, break_at.unwrap_or_default())?;
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
    println!("{}", ASCII_ART.bright_purple());
    println!(
        "  {} {} — {}",
        "Knull".bright_green().bold(),
        "v2.1.0".bright_white(),
        "The God Programming Language".bright_yellow()
    );
    println!();
    #[cfg(feature = "llvm-backend")]
    println!("  Backend: LLVM (native compile)");
    #[cfg(not(feature = "llvm-backend"))]
    println!("  Backend: Interpreter + C codegen");
    println!("  Edition: 2024");
    println!();
    println!("{}", "WHAT'S NEW in v2.1.0:".bright_white().bold());
    println!("  + Real-time minifb GUI (gui_rect, gui_line, gui_circle, gui_circle_outline,");
    println!("    gui_rect_outline, gui_rgb, gui_set_title, gui_size)");
    println!("  + Working packages: json, http, crypto, sqlite");
    println!("  + Snake & Pong game examples");
    println!("  + SQLite TODO app example");
    println!("  + JSON / Crypto package demos");
}

fn show_help() {
    println!("{}", ASCII_ART.bright_purple());
    println!(
        "  {} {} — {}",
        "Knull".bright_green().bold(),
        "v2.1.0".bright_white(),
        "The God Programming Language".bright_yellow()
    );
    println!();
    println!("{}", "USAGE:".bright_white().bold());
    println!("  knull [OPTIONS] <COMMAND>");
    println!();
    println!("{}", "CORE:".bright_white().bold());
    println!("  {}  Run a .knull file",                   "run   <file>      ".bright_cyan());
    println!("  {}  Evaluate an inline snippet",           "eval  <expr>      ".bright_cyan());
    println!("  {}  Start interactive REPL",               "repl              ".bright_cyan());
    println!("  {}  Check syntax/types",                   "check <file>      ".bright_cyan());
    println!("  {}  Format a .knull file",                 "fmt   <file>      ".bright_cyan());
    println!();
    println!("{}", "BUILD:".bright_white().bold());
    println!("  {}  Compile to binary",                    "build <file>      ".bright_cyan());
    println!("  {}  Emit assembly",                        "asm   <file>      ".bright_cyan());
    println!();
    println!("{}", "PROJECT:".bright_white().bold());
    println!("  {}  Create a new project",                 "new   <name>      ".bright_cyan());
    println!("  {}  Add a dependency",                     "add   <pkg>       ".bright_cyan());
    println!("  {}  Run test suite",                       "test              ".bright_cyan());
    println!();
    println!("{}", "OPTIONS:".bright_white().bold());
    println!("  -v, --verbose   Verbose output");
    println!("  -h, --help      Print help");
    println!("  -V, --version   Print version");
    println!();
    println!("{}", "STDLIB HIGHLIGHTS:".bright_white().bold());
    println!("  {}  Windowed GUI, drawing primitives",     "gui_*             ".bright_yellow());
    println!("  {}  SQLite database (rusqlite backed)",    "db_*              ".bright_yellow());
    println!("  {}  HTTP client (GET/POST/PUT/DELETE)",    "http_*            ".bright_yellow());
    println!("  {}  SHA-256, MD5, base64, random",        "sha256  base64_*  ".bright_yellow());
    println!("  {}  Parse/stringify JSON",                 "json_parse  json_stringify".bright_yellow());
    println!("  {}  Spawned threads, channels",            "spawn   chan      ".bright_yellow());
    println!("  {}  FFI / syscall / fork / exec",          "ffi_*   syscall   ".bright_yellow());
    println!("  {}  Image load/save/resize/pixel",         "img_*             ".bright_yellow());
    println!();
    println!("{}", "PACKAGES:".bright_white().bold());
    println!("  {}  JSON helpers (parse/stringify/merge/has)", "import \"packages/json/src/lib.knull\"   ".bright_green());
    println!("  {}  HTTP helpers (get_json/post_json)",    "import \"packages/http/src/lib.knull\"   ".bright_green());
    println!("  {}  Crypto (hash/hmac/base64/token)",      "import \"packages/crypto/src/lib.knull\" ".bright_green());
    println!("  {}  SQLite ORM helpers",                   "import \"packages/sqlite/src/lib.knull\" ".bright_green());
    println!();
    println!("{}", "EXAMPLES:".bright_white().bold());
    println!("  knull run hello.knull");
    println!("  knull eval 'println(42 * 2)'");
    println!("  knull repl");
    println!("  knull build --release main.knull -o myapp");
    println!("  knull run examples/games/snake.knull");
    println!("  knull run examples/games/pong.knull");
    println!("  knull run examples/db/todo_app.knull");
    println!("  knull run examples/json_demo.knull");
    println!();
    println!("{}", "https://github.com/4fqr/knull".bright_black());
}
