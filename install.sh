#!/bin/bash
# Knull Language Installer
# Usage: curl -sSL https://raw.githubusercontent.com/4fqr/knull/master/install.sh | bash

set -e

VERSION="1.0.0"
REPO_URL="https://github.com/4fqr/knull.git"
TEMP_DIR="/tmp/knull-install-$$"

# Colors using printf for portability
red=$(printf '\033[0;31m')
green=$(printf '\033[0;32m')
yellow=$(printf '\033[1;33m')
blue=$(printf '\033[0;34m')
nc=$(printf '\033[0m')

# Print functions
print_header() {
    printf "%s\n" "${blue}"
    printf "  _  __    _    _   _ _   _ \n"
    printf " | |/ /   | |  | | | | \\ | |\n"
    printf " | ' / ___| |  | | | |  \\| |\n"
    printf " | . \\___ | |  | |_| | |\\  |\n"
    printf " |_|\\_\\___|_|   \\___/|_| \\_|\n"
    printf "%s\n" "${nc}"
    printf "%sThe God Programming Language%s\n" "${green}" "${nc}"
    printf "\n"
}

print_success() {
    printf "%s✓%s %s\n" "${green}" "${nc}" "$1"
}

print_error() {
    printf "%s✗%s %s\n" "${red}" "${nc}" "$1"
}

print_info() {
    printf "%s→%s %s\n" "${yellow}" "${nc}" "$1"
}

# Detect OS
detect_os() {
    case "$(uname -s)" in
        Linux*)     echo "linux";;
        Darwin*)    echo "macos";;
        MINGW*|MSYS*|CYGWIN*) echo "windows";;
        *)          echo "unknown";;
    esac
}

# Detect architecture
detect_arch() {
    case "$(uname -m)" in
        x86_64|amd64)  echo "x86_64";;
        aarch64|arm64) echo "arm64";;
        i386|i686)     echo "x86";;
        *)             echo "unknown";;
    esac
}

# Check if command exists
command_exists() {
    command -v "$1" >/dev/null 2>&1
}

# Check dependencies
check_deps() {
    print_info "Checking dependencies..."
    
    # Check for Rust
    if ! command_exists cargo; then
        print_error "Rust not found"
        printf "\nPlease install Rust:\n"
        printf "  curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh\n"
        printf "\nThen restart your shell and run this installer again.\n"
        exit 1
    fi
    print_success "Rust found: $(cargo --version)"
    
    # Check for git
    if ! command_exists git; then
        print_error "Git not found"
        printf "Please install Git and try again.\n"
        exit 1
    fi
    print_success "Git found"
    
    # Check for C compiler (for linking)
    if command_exists cc || command_exists gcc || command_exists clang; then
        print_success "C compiler found"
    else
        print_error "No C compiler found (cc, gcc, or clang)"
        printf "Please install a C compiler for linking.\n"
        exit 1
    fi
}

# Get install directory
get_install_dir() {
    if [ -n "$PREFIX" ]; then
        echo "$PREFIX/bin"
    elif [ -w "/usr/local/bin" ]; then
        echo "/usr/local/bin"
    elif [ -d "$HOME/.local/bin" ]; then
        echo "$HOME/.local/bin"
    else
        echo "$HOME/.knull/bin"
    fi
}

# Clone repository
clone_repo() {
    print_info "Cloning Knull repository..."
    
    if [ -d "$TEMP_DIR" ]; then
        rm -rf "$TEMP_DIR"
    fi
    
    mkdir -p "$TEMP_DIR"
    git clone --depth 1 --branch master "$REPO_URL" "$TEMP_DIR/knull" 2>&1
    
    print_success "Repository cloned"
}

# Build Knull
build_knull() {
    print_info "Building Knull compiler..."
    
    cd "$TEMP_DIR/knull/src"
    
    # Build release version
    cargo build --release --no-default-features 2>&1
    
    if [ ! -f "target/release/knull" ]; then
        print_error "Build failed - binary not found"
        exit 1
    fi
    
    print_success "Build successful"
}

# Install binary
install_binary() {
    local install_dir
    install_dir=$(get_install_dir)
    
    print_info "Installing to $install_dir..."
    
    # Create directory if needed
    mkdir -p "$install_dir"
    
    # Copy binary
    cp "$TEMP_DIR/knull/src/target/release/knull" "$install_dir/knull"
    chmod +x "$install_dir/knull"
    
    # Create symlinks for common names
    if [ ! -e "$install_dir/kn" ]; then
        ln -sf "$install_dir/knull" "$install_dir/kn" 2>/dev/null || true
    fi
    
    print_success "Binary installed"
    
    # Check if in PATH
    case ":$PATH:" in
        *":$install_dir:"*)
            # Already in PATH
            ;;
        *)
            printf "\n"
            print_info "Adding to PATH..."
            
            # Detect shell
            local shell_rc=""
            if [ -n "$ZSH_VERSION" ]; then
                shell_rc="$HOME/.zshrc"
            elif [ -n "$BASH_VERSION" ]; then
                shell_rc="$HOME/.bashrc"
            else
                shell_rc="$HOME/.profile"
            fi
            
            # Add to shell rc
            printf "\n# Knull Programming Language\n" >> "$shell_rc"
            printf "export PATH=\"%s:\$PATH\"\n" "$install_dir" >> "$shell_rc"
            
            print_success "Added to PATH in $shell_rc"
            printf "  Run 'source %s' to update your current shell\n" "$shell_rc"
            ;;
    esac
}

# Install stdlib
install_stdlib() {
    local stdlib_dir="$HOME/.knull/stdlib"
    
    print_info "Installing standard library..."
    
    mkdir -p "$stdlib_dir"
    
    # Copy runtime files
    if [ -d "$TEMP_DIR/knull/runtime" ]; then
        cp -r "$TEMP_DIR/knull/runtime"/* "$stdlib_dir/" 2>/dev/null || true
    fi
    
    # Copy std files
    if [ -d "$TEMP_DIR/knull/src/std" ]; then
        cp -r "$TEMP_DIR/knull/src/std" "$stdlib_dir/" 2>/dev/null || true
    fi
    
    print_success "Standard library installed"
}

# Create config
create_config() {
    local config_dir="$HOME/.knull"
    
    print_info "Creating configuration..."
    
    mkdir -p "$config_dir"
    
    cat > "$config_dir/config.toml" << 'EOF'
# Knull Configuration

[compiler]
mode = "novice"
opt_level = 2
target = "x86_64-linux-gnu"

[paths]
stdlib = "~/.knull/stdlib"
cache = "~/.knull/cache"

[tools]
linker = "cc"
assembler = "nasm"
EOF
    
    print_success "Configuration created"
}

# Run tests
run_tests() {
    print_info "Running tests..."
    
    cd "$TEMP_DIR/knull/src"
    
    if cargo test --no-default-features 2>&1 | grep -q "test result: ok"; then
        print_success "Tests passed"
    else
        print_error "Some tests failed (continuing anyway)"
    fi
}

# Verify installation
verify_install() {
    local install_dir
    install_dir=$(get_install_dir)
    
    print_info "Verifying installation..."
    
    if [ -x "$install_dir/knull" ]; then
        local version
        version=$("$install_dir/knull" --version 2>/dev/null || echo "unknown")
        print_success "Knull installed: $version"
    else
        print_error "Installation verification failed"
        exit 1
    fi
}

# Cleanup
cleanup() {
    if [ -d "$TEMP_DIR" ]; then
        rm -rf "$TEMP_DIR"
    fi
}

# Print final message
print_footer() {
    local install_dir
    install_dir=$(get_install_dir)
    
    printf "\n"
    printf "%s╔════════════════════════════════════════════════════════╗%s\n" "${green}" "${nc}"
    printf "%s║          Knull Installation Complete!                  ║%s\n" "${green}" "${nc}"
    printf "%s╚════════════════════════════════════════════════════════╝%s\n" "${green}" "${nc}"
    printf "\n"
    printf "Quick start:\n"
    printf "  knull --version          # Show version\n"
    printf "  knull run hello.knull    # Run a program\n"
    printf "  knull new myproject      # Create new project\n"
    printf "  knull repl               # Interactive shell\n"
    printf "\n"
    printf "Documentation:\n"
    printf "  https://github.com/4fqr/knull#readme\n"
    printf "\n"
    printf "To uninstall:\n"
    printf "  curl -sSL https://raw.githubusercontent.com/4fqr/knull/master/uninstall.sh | bash\n"
    printf "\n"
}

# Main installation
main() {
    print_header
    
    local os
    os=$(detect_os)
    local arch
    arch=$(detect_arch)
    
    print_info "Detected: $os ($arch)"
    
    if [ "$os" = "unknown" ]; then
        print_error "Unsupported operating system"
        exit 1
    fi
    
    if [ "$arch" = "unknown" ]; then
        print_error "Unsupported architecture"
        exit 1
    fi
    
    # Run installation steps
    check_deps
    clone_repo
    build_knull
    run_tests
    install_binary
    install_stdlib
    create_config
    verify_install
    cleanup
    
    print_footer
}

# Handle interrupts
trap cleanup EXIT INT TERM

# Run main
main
