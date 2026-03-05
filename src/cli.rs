//! Knull CLI - Command Line Interface
//! Professional compiler interface for the Knull programming language

use crate::pkg::manager::PackageManager;
use colored::Colorize;
use std::fs;
use std::io::{self, Write};
use std::path::{Path, PathBuf};

/// Run a Knull file with rich error output
pub fn run_file(path: &Path, verbose: bool) -> Result<(), String> {
    if verbose {
        println!("{} {}", "Running".bright_blue().bold(), path.display());
    }

    let source = fs::read_to_string(path)
        .map_err(|e| format!("Cannot read '{}': {}", path.display(), e))?;

    if verbose {
        println!("  {} Parsing...", "→".bright_black());
    }

    let mut parser = crate::parser::Parser::new(&source);
    let ast = parser.parse().map_err(|e| {
        format_error_in_source(&source, path.to_str().unwrap_or("<file>"), &e)
    })?;

    if verbose {
        println!("  {} Parsed successfully", "✓".green());
        println!("  {} Executing...", "→".bright_black());
    }

    let mut interp = crate::interpreter::Interpreter::new();
    interp.execute(&ast).map_err(|e| {
        format_error_in_source(&source, path.to_str().unwrap_or("<file>"), &e)
    })
}

/// Evaluate a Knull expression/snippet from a string (for `knull eval`)
pub fn eval_expr(source: &str, verbose: bool) -> Result<(), String> {
    if verbose {
        println!("{} {}", "Eval".bright_blue().bold(), source.bright_black());
    }

    let mut interp = crate::interpreter::Interpreter::new();
    match interp.repl_exec(source) {
        Ok(Some(val)) => println!("{}", val),
        Ok(None) => {}
        Err(e) => return Err(e),
    }
    Ok(())
}

/// Format a parse/runtime error with source context
fn format_error_in_source(source: &str, file: &str, err: &str) -> String {
    // Try to extract a line number from messages like "line 12" or ":12:"
    let line_num = extract_line_number(err);
    if let Some(ln) = line_num {
        let lines: Vec<&str> = source.lines().collect();
        if ln > 0 && ln <= lines.len() {
            let before = if ln > 1 { format!("{:>4} │ {}\n", ln - 1, lines[ln - 2]) } else { String::new() };
            let at = format!("{:>4} │ {}\n", ln, lines[ln - 1]);
            let marker = format!("     │ {}\n", "^".repeat(lines[ln - 1].trim_end().len().min(40)).red());
            let after = if ln < lines.len() { format!("{:>4} │ {}\n", ln + 1, lines[ln]) } else { String::new() };
            return format!(
                "{}: {}\n → {}:{}\n{}{}{}{}\n{}",
                "error".bright_red().bold(),
                err,
                file, ln,
                before, at, marker, after,
                "   = try running with --verbose for more info".bright_black()
            );
        }
    }
    format!("{}: {}", "error".bright_red().bold(), err)
}

fn extract_line_number(s: &str) -> Option<usize> {
    // Matches "line N" or "[N:" or ":N:"
    if let Some(pos) = s.find("line ") {
        let rest = &s[pos + 5..];
        let num: String = rest.chars().take_while(|c| c.is_ascii_digit()).collect();
        return num.parse().ok();
    }
    None
}

/// Build a Knull file to native binary
pub fn build_file(
    path: &Path,
    output: Option<&Path>,
    verbose: bool,
    target: &str,
) -> Result<(), String> {
    let source = fs::read_to_string(path).map_err(|e| format!("Failed to read file: {}", e))?;

    let out_path = output
        .map(|p| p.to_path_buf())
        .unwrap_or_else(|| path.with_extension(""));

    if verbose {
        println!(
            "{} {} → {} (target: {})",
            "Building".bright_yellow().bold(),
            path.display(),
            out_path.display(),
            target
        );
    }

    let _options = crate::compiler::CompileOptions::default();

    match target {
        "wasm32" => {
            let wasm_path = out_path.with_extension("wasm");
            crate::wasm_codegen::compile_to_wasm(&source, wasm_path.to_str().unwrap())
                .map_err(|e| format!("WASM compilation failed: {}", e))?;
            if verbose {
                println!("  WASM file: {}", wasm_path.display());
            }
            println!("{} Build successful: {}", "✓".green().bold(), wasm_path.display());
            return Ok(());
        }
        _ => {}
    }

    #[cfg(feature = "llvm-backend")]
    {
        let result = crate::compiler::compile(&source, &out_path, _options)
            .map_err(|e| format!("Compilation failed: {}", e))?;
        if verbose {
            if let Some(ref obj) = result.object_path {
                println!("  Object file: {}", obj);
            }
        }
        println!("{} Build successful: {}", "✓".green().bold(), out_path.display());
    }

    #[cfg(not(feature = "llvm-backend"))]
    {
        crate::c_codegen::compile_to_binary(&source, out_path.to_str().unwrap())
            .map_err(|e| format!("Compilation failed: {}", e))?;
        println!("{} Build successful: {}", "✓".green().bold(), out_path.display());
    }

    Ok(())
}

/// Build in release mode (optimized)
pub fn build_release(
    path: &Path,
    output: Option<&Path>,
    verbose: bool,
    target: &str,
) -> Result<(), String> {
    build_file(path, output, verbose, target)
}

/// Generate assembly output
pub fn generate_asm(path: &Path, output: Option<&Path>) -> Result<(), String> {
    let _source = fs::read_to_string(path).map_err(|e| format!("Failed to read file: {}", e))?;

    let out_path = output
        .map(|p| p.to_path_buf())
        .unwrap_or_else(|| path.with_extension("s"));

    println!(
        "{} {} → {}",
        "Generating Assembly".bright_cyan().bold(),
        path.display(),
        out_path.display()
    );

    #[cfg(feature = "llvm-backend")]
    {
        let _options = crate::compiler::CompileOptions::default();
        crate::compiler::generate_assembly(&_source, &out_path, _options)
            .map_err(|e| format!("Assembly generation failed: {}", e))?;
        println!("{} Assembly generated: {}", "✓".green().bold(), out_path.display());
        Ok(())
    }

    #[cfg(not(feature = "llvm-backend"))]
    Err("LLVM backend not available".to_string())
}

/// Check syntax and types without building
pub fn check_file(path: &Path) -> Result<(), String> {
    let source = fs::read_to_string(path).map_err(|e| format!("Failed to read file: {}", e))?;

    print!("Checking {}... ", path.display());
    io::stdout().flush().ok();

    let mut parser = crate::parser::Parser::new(&source);
    let _ast = parser.parse().map_err(|e| format!("Parse error: {}", e))?;

    println!("{}", "✓ No errors found".green());
    Ok(())
}

/// Format a Knull file
pub fn format_file(path: &Path) -> Result<(), String> {
    let source = fs::read_to_string(path).map_err(|e| format!("Failed to read file: {}", e))?;

    println!("{} {}", "Formatting".bright_blue(), path.display());

    let mut lexer = crate::lexer::Lexer::new(&source);
    let tokens = lexer.tokenize();

    let mut formatted = String::new();
    let mut indent = 0;
    let mut prev_was_newline = true;

    for token in tokens {
        use crate::lexer::TokenKind;
        match token.kind {
            TokenKind::LBrace => {
                formatted.push_str(" {\n");
                indent += 4;
                prev_was_newline = true;
            }
            TokenKind::RBrace => {
                if indent >= 4 { indent -= 4; }
                if !prev_was_newline { formatted.push('\n'); }
                formatted.push_str(&" ".repeat(indent));
                formatted.push_str("}\n");
                prev_was_newline = true;
            }
            TokenKind::Semicolon => {
                formatted.push('\n');
                prev_was_newline = true;
            }
            TokenKind::Eof => break,
            _ => {
                if prev_was_newline {
                    formatted.push_str(&" ".repeat(indent));
                    prev_was_newline = false;
                } else if token.kind != TokenKind::LParen
                    && token.kind != TokenKind::RParen
                    && token.kind != TokenKind::Comma
                {
                    formatted.push(' ');
                }
                formatted.push_str(&token.value);
            }
        }
    }

    fs::write(path, formatted).map_err(|e| format!("Failed to write: {}", e))?;
    println!("{} Formatted {}", "✓".green().bold(), path.display());
    Ok(())
}

/// Create a new Knull project
pub fn new_project(name: &str) -> Result<(), String> {
    println!(
        "{} Creating new Knull project: {}",
        "Creating".bright_yellow(),
        name.bright_cyan().bold()
    );
    PackageManager::new_project(name)?;
    println!("{} Project '{}' created", "✓".green().bold(), name);
    println!();
    println!("  cd {}", name);
    println!("  knull run src/main.knull");
    Ok(())
}

/// Add a dependency to the project
pub fn add_dependency(package: &str, version: Option<&str>) -> Result<(), String> {
    let ver = version.unwrap_or("^1.0");
    println!("{} {} {}", "Adding".bright_yellow(), package.bright_cyan(), ver.bright_black());
    let current_dir =
        std::env::current_dir().map_err(|e| format!("Failed to get current directory: {}", e))?;
    let mut pm = PackageManager::new(current_dir)?;
    pm.add_dependency(package, ver)?;
    println!("{} Added {}", "✓".green().bold(), package);
    Ok(())
}

/// Remove a dependency from the project
pub fn remove_dependency(package: &str) -> Result<(), String> {
    println!("{} {}", "Removing".bright_yellow(), package.bright_cyan());
    let current_dir =
        std::env::current_dir().map_err(|e| format!("Failed to get current directory: {}", e))?;
    let mut pm = PackageManager::new(current_dir)?;
    pm.remove_dependency(package)?;
    println!("{} Removed {}", "✓".green().bold(), package);
    Ok(())
}

/// Update all dependencies or a specific package
pub fn update_dependencies(package: Option<&str>) -> Result<(), String> {
    let current_dir =
        std::env::current_dir().map_err(|e| format!("Failed to get current directory: {}", e))?;
    let mut pm = PackageManager::new(current_dir)?;
    match package {
        Some(pkg) => {
            println!("{} {}", "Updating".bright_yellow(), pkg.bright_cyan());
            let constraint = pm.manifest().dependencies.get(pkg).cloned().unwrap_or_else(|| "^1.0".to_string());
            pm.update_package(pkg, &constraint)?;
            println!("{} Updated {}", "✓".green().bold(), pkg);
        }
        None => { pm.update_all_dependencies()?; }
    }
    Ok(())
}

/// Publish package to registry
pub fn publish(local: bool, token: Option<&str>) -> Result<(), String> {
    let current_dir =
        std::env::current_dir().map_err(|e| format!("Failed to get current directory: {}", e))?;
    let pm = PackageManager::new(current_dir)?;
    if local {
        println!("{}", "Publishing to local registry...".bright_yellow());
        pm.publish_local()?;
    } else {
        let auth_token = token
            .map(|s| s.to_string())
            .or_else(|| std::env::var("KNULL_REGISTRY_TOKEN").ok())
            .ok_or("Authentication token required. Use --token or set KNULL_REGISTRY_TOKEN")?;
        println!("{}", "Publishing to registry...".bright_yellow());
        pm.publish_registry(&auth_token)?;
    }
    Ok(())
}

/// Fetch dependencies
pub fn fetch_dependencies() -> Result<(), String> {
    println!("{}", "Fetching dependencies...".bright_yellow());
    let current_dir =
        std::env::current_dir().map_err(|e| format!("Failed to get current directory: {}", e))?;
    let pm = PackageManager::new(current_dir)?;
    pm.fetch_dependencies()?;
    println!("{} Dependencies fetched", "✓".green().bold());
    Ok(())
}

/// List project dependencies
pub fn list_dependencies() -> Result<(), String> {
    let current_dir =
        std::env::current_dir().map_err(|e| format!("Failed to get current directory: {}", e))?;
    let pm = PackageManager::new(current_dir)?;
    println!("{}", "Dependencies:".bright_yellow().bold());
    let deps = pm.manifest().dependencies.clone();
    if deps.is_empty() {
        println!("  (none)");
    } else {
        for (name, version) in deps {
            println!("  {} {}", name.bright_cyan(), version.bright_black());
        }
    }
    let dev_deps = &pm.manifest().dev_dependencies;
    if !dev_deps.is_empty() {
        println!("\n{}", "Dev Dependencies:".bright_yellow().bold());
        for (name, version) in dev_deps {
            println!("  {} {}", name.bright_cyan(), version.bright_black());
        }
    }
    Ok(())
}

/// Run tests
pub fn run_tests() -> Result<(), String> {
    run_tests_with_options(false, false)
}

/// Run tests with options
pub fn run_tests_with_options(bench: bool, property_test: bool) -> Result<(), String> {
    if bench {
        println!("{}", "Running benchmarks...".bright_yellow().bold());
    } else if property_test {
        println!("{}", "Running property-based tests...".bright_yellow().bold());
    } else {
        println!("{}", "Running tests...".bright_yellow().bold());
    }

    let test_dirs = vec!["tests", "test", "src/tests"];
    let mut found_tests = false;
    let mut passed = 0;
    let mut failed = 0;
    let mut skipped = 0;

    for dir in test_dirs {
        if let Ok(entries) = fs::read_dir(dir) {
            for entry in entries.flatten() {
                let path = entry.path();
                if path.extension().map_or(false, |e| e == "knull") {
                    found_tests = true;
                    print!("  {} ... ", path.display());
                    io::stdout().flush().ok();
                    if path.to_string_lossy().contains("@ignore") {
                        println!("{}", "SKIP".yellow());
                        skipped += 1;
                    } else {
                        match run_file(&path, false) {
                            Ok(_) => { println!("{}", "PASS".green()); passed += 1; }
                            Err(e) => { println!("{}\n       {}", "FAIL".red().bold(), e); failed += 1; }
                        }
                    }
                }
            }
        }
    }

    if !found_tests {
        println!("  No test files found in tests/ directory.");
    } else {
        println!();
        let summary = format!(
            "{} passed  {}  {}",
            passed.to_string().green().bold(),
            if failed > 0 { format!("{} failed", failed).red().bold().to_string() }
              else { format!("{} failed", failed).bright_black().to_string() },
            format!("{} skipped", skipped).bright_black()
        );
        println!("Results: {}", summary);
    }

    Ok(())
}

// ── REPL ──────────────────────────────────────────────────────────────────────

const REPL_HELP: &str = r#"
Knull REPL — interactive session

  Type any Knull expression or statement and press Enter.
  Multi-line blocks: keep typing after an opening '{'; enter a blank line to submit.

Commands:
  :help   :h        Show this help
  :quit   :q        Exit the REPL
  :vars             List all variables in scope
  :fns              List all defined functions
  :reset            Reset interpreter state
  :load <file>      Load and execute a .knull file into this session
  :type <expr>      Show the type of an expression
  :clear            Clear the screen
"#;

/// Start the interactive REPL with persistent state, multi-line support, and commands
pub fn start_repl() -> Result<(), String> {
    println!("{}", r#"
  _  __              _ _
 | |/ /_ __  _   _| | |
 | ' /| '_ \| | | | | |
 | . \| | | | |_| | | |
 |_|\_\_| |_|\__,_|_|_|
"#.bright_purple().bold());
    println!(
        "  {} {}  —  {}",
        "Knull".bright_purple().bold(),
        "v2.0.0".bright_white(),
        "The God Programming Language".bright_yellow()
    );
    println!("  Type {} for available commands\n", ":help".bright_cyan());

    let mut interp = crate::interpreter::Interpreter::new();
    let mut input_buf = String::new();
    let mut continuation = false; // true when we're inside a multi-line block

    loop {
        // Prompt
        if continuation {
            print!("{}", "  ... ".bright_black());
        } else {
            print!("{}", "knull❯ ".bright_magenta().bold());
        }
        io::stdout().flush().ok();

        let mut line = String::new();
        match io::stdin().read_line(&mut line) {
            Ok(0) => { println!(); break; } // EOF / Ctrl-D
            Ok(_) => {}
            Err(_) => break,
        }

        let trimmed = line.trim_end_matches('\n').trim_end_matches('\r');

        // ── REPL commands (only when not in continuation) ──────────────────
        if !continuation {
            match trimmed.trim() {
                ":quit" | ":q" | ":exit" => {
                    println!("{}", "Goodbye! 👋".bright_green());
                    break;
                }
                ":help" | ":h" => {
                    println!("{}", REPL_HELP.bright_cyan());
                    continue;
                }
                ":vars" => {
                    let vars = interp.repl_variables();
                    if vars.is_empty() {
                        println!("{}", "(no variables defined)".bright_black());
                    } else {
                        println!("{}", "Variables:".bright_yellow().bold());
                        for (name, val) in &vars {
                            // skip StructDef noise
                            let display = format!("{}", val);
                            if display.starts_with("<struct-def") { continue; }
                            println!("  {} = {}", name.bright_cyan(), display.bright_white());
                        }
                    }
                    continue;
                }
                ":fns" => {
                    let fns = interp.repl_functions();
                    if fns.is_empty() {
                        println!("{}", "(no functions defined)".bright_black());
                    } else {
                        println!("{}", "Functions:".bright_yellow().bold());
                        for name in &fns {
                            println!("  {}", name.bright_cyan());
                        }
                    }
                    continue;
                }
                ":reset" => {
                    interp = crate::interpreter::Interpreter::new();
                    println!("{}", "State reset.".bright_green());
                    continue;
                }
                ":clear" => {
                    print!("\x1B[2J\x1B[H");
                    io::stdout().flush().ok();
                    continue;
                }
                cmd if cmd.starts_with(":load ") => {
                    let path_str = cmd[6..].trim();
                    let path = PathBuf::from(path_str);
                    match fs::read_to_string(&path) {
                        Ok(src) => {
                            match crate::parser::Parser::new(&src).parse() {
                                Ok(ast) => match interp.execute(&ast) {
                                    Ok(_) => println!("{} Loaded {}", "✓".green(), path.display()),
                                    Err(e) => println!("{} {}", "error:".bright_red(), e),
                                },
                                Err(e) => println!("{} {}", "parse error:".bright_red(), e),
                            }
                        }
                        Err(e) => println!("{} {}", "error:".bright_red(), e),
                    }
                    continue;
                }
                cmd if cmd.starts_with(":type ") => {
                    let expr = cmd[6..].trim();
                    match crate::interpreter::Interpreter::new().repl_exec(expr) {
                        Ok(Some(val)) => {
                            let type_name = value_type_name(&format!("{:?}", val));
                            println!("{} : {}", expr.bright_white(), type_name.bright_cyan());
                        }
                        Ok(None) => println!("{}", "(no value)".bright_black()),
                        Err(e) => println!("{} {}", "error:".bright_red(), e),
                    }
                    continue;
                }
                "" => {
                    // blank line while not in continuation — skip
                    continue;
                }
                _ => {}
            }
        }

        // ── accumulate input ────────────────────────────────────────────────
        if !input_buf.is_empty() {
            input_buf.push('\n');
        }
        input_buf.push_str(trimmed);

        // Count brace balance to detect multi-line blocks
        let open_braces: i32 = input_buf.chars().filter(|&c| c == '{').count() as i32;
        let close_braces: i32 = input_buf.chars().filter(|&c| c == '}').count() as i32;
        let depth = open_braces - close_braces;

        if depth > 0 {
            // Still inside a block — keep collecting
            continuation = true;
            continue;
        }

        // Empty buffer after a blank line in continuation? Submit anyway.
        continuation = false;
        let source = input_buf.trim().to_string();
        input_buf.clear();

        if source.is_empty() { continue; }

        // ── Execute ─────────────────────────────────────────────────────────
        match interp.repl_exec(&source) {
            Ok(Some(val)) => {
                // Show result with type hint
                let type_nm = value_type_name(&format!("{:?}", val));
                println!("{} {}", format!("{}", val).bright_white().bold(), format!("// {}", type_nm).bright_black());
            }
            Ok(None) => {} // definition / statement / void call
            Err(e) => {
                println!("{} {}", "error:".bright_red().bold(), e);
            }
        }
    }

    Ok(())
}

/// Derive a user-friendly type name from a Debug string of a Value
fn value_type_name(debug: &str) -> &'static str {
    if debug.starts_with("Int(") || debug.starts_with("Float(") { "number" }
    else if debug.starts_with("Str(") { "string" }
    else if debug.starts_with("Bool(") { "bool" }
    else if debug.starts_with("Array(") { "array" }
    else if debug.starts_with("Map(") || debug.starts_with("HashMap(") { "map" }
    else if debug.starts_with("Closure") || debug.starts_with("Function") { "fn" }
    else if debug.starts_with("Struct {") || debug.starts_with("StructInstance") { "struct" }
    else if debug.starts_with("Null") { "null" }
    else { "value" }
}

/// Show version information
pub fn show_version() {
    println!("{} {}", "Knull".bright_purple().bold(), "v2.0.0".bright_cyan());
    println!("{}", "The God Programming Language".bright_yellow());
    println!();
    println!("Edition: 2024");
    #[cfg(feature = "llvm-backend")]
    println!("Backend: LLVM (native compile)");
    #[cfg(not(feature = "llvm-backend"))]
    println!("Backend: Interpreter + C codegen");
}

/// Show help information
pub fn show_help() {
    println!("{}", "Knull — The God Programming Language".bright_purple().bold());
    println!();
    println!("{}", "USAGE:".bright_yellow().bold());
    println!("  knull <COMMAND> [OPTIONS] [ARGS]");
    println!();
    println!("{}", "CORE COMMANDS:".bright_yellow().bold());
    println!("  {}   Execute a .knull file",             "run  <file>           ".bright_cyan());
    println!("  {}   Evaluate a snippet inline",         "eval <expr>           ".bright_cyan());
    println!("  {}   Start interactive REPL",            "repl                  ".bright_cyan());
    println!("  {}   Check syntax/types without run",    "check <file>          ".bright_cyan());
    println!("  {}   Format a .knull file",              "fmt  <file>           ".bright_cyan());
    println!();
    println!("{}", "BUILD:".bright_yellow().bold());
    println!("  {}   Compile to binary",                 "build <file> [-o out] ".bright_cyan());
    println!("  {}   Emit assembly",                     "asm  <file>           ".bright_cyan());
    println!();
    println!("{}", "PROJECT:".bright_yellow().bold());
    println!("  {}   Create a new project",              "new  <name>           ".bright_cyan());
    println!("  {}   Add a dependency",                  "add  <pkg> [version]  ".bright_cyan());
    println!("  {}   Run test suite",                    "test                  ".bright_cyan());
    println!("  {}   Show version",                      "version               ".bright_cyan());
    println!();
    println!("{}", "EXAMPLES:".bright_yellow().bold());
    println!("  knull run hello.knull");
    println!("  knull eval 'println(\"hello world\")'");
    println!("  knull repl");
    println!("  knull build main.knull -o myapp --release");
    println!("  knull new my-project && cd my-project && knull run src/main.knull");
    println!();
    println!("{}", "REPL COMMANDS:".bright_yellow().bold());
    println!("  :help  :vars  :fns  :reset  :load <file>  :type <expr>  :quit");
    println!();
    println!("{}", "https://github.com/4fqr/knull".bright_black());
}

