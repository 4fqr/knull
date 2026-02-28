#!/bin/bash
# Knull Language Installer
# Usage: curl -sSL https://raw.githubusercontent.com/4fqr/knull/main/install.sh | bash

set -e

VERSION="1.0.0"
REPO_URL="https://github.com/4fqr/knull.git"
TEMP_DIR="/tmp/knull-install-$$"

# Colors
RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
BLUE='\033[0;34m'
NC='\033[0m'

# Print functions
print_header() {
    echo -e "${BLUE}"
    echo "  _  __    _    _   _ _   _ "
    echo " | |/ /   | |  | | | | \ | |"
    echo " | ' / ___| |  | | | |  \| |"
    echo " | . \\___ | |  | |_| | |\  |"
    echo " |_|\_\___|_|   \___/|_| \_|"
    echo -e "${NC}"
    echo -e "${GREEN}The God Programming Language${NC}"
    echo ""
}

print_success() {
    echo -e "${GREEN}✓${NC} $1"
}

print_error() {
    echo -e "${RED}✗${NC} $1"
}

print_info() {
    echo -e "${YELLOW}→${NC} $1"
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
        echo ""
        echo "Please install Rust:"
        echo "  curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh"
        echo ""
        echo "Then restart your shell and run this installer again."
        exit 1
    fi
    print_success "Rust found: $(cargo --version)"
    
    # Check for git
    if ! command_exists git; then
        print_error "Git not found"
        echo "Please install Git and try again."
        exit 1
    fi
    print_success "Git found"
    
    # Check for C compiler (for linking)
    if command_exists cc || command_exists gcc || command_exists clang; then
        print_success "C compiler found"
    else
        print_error "No C compiler found (cc, gcc, or clang)"
        echo "Please install a C compiler for linking."
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
    git clone --depth 1 --branch master "$REPO_URL" "$TEMP_DIR/knull" 2>&1 | while read line; do
        echo "  $line"
    done
    
    print_success "Repository cloned"
}

# Build Knull
build_knull() {
    print_info "Building Knull compiler..."
    
    cd "$TEMP_DIR/knull/src"
    
    # Build release version
    cargo build --release --no-default-features 2>&1 | while read line; do
        echo "  $line"
    done
    
    if [ ! -f "target/release/knull" ]; then
        print_error "Build failed - binary not found"
        exit 1
    fi
    
    print_success "Build successful"
}

# Install binary
install_binary() {
    local install_dir=$(get_install_dir)
    
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
    if [[ ":$PATH:" != *":$install_dir:"* ]]; then
        echo ""
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
        echo "" >> "$shell_rc"
        echo "# Knull Programming Language" >> "$shell_rc"
        echo "export PATH=\"$install_dir:\$PATH\"" >> "$shell_rc"
        
        print_success "Added to PATH in $shell_rc"
        echo "  Run 'source $shell_rc' to update your current shell"
    fi
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
mode = "novice"  # novice, expert, god
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
    local install_dir=$(get_install_dir)
    
    print_info "Verifying installation..."
    
    if [ -x "$install_dir/knull" ]; then
        local version=$("$install_dir/knull" --version 2>/dev/null || echo "unknown")
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
    local install_dir=$(get_install_dir)
    
    echo ""
    echo -e "${GREEN}╔════════════════════════════════════════════════════════╗${NC}"
    echo -e "${GREEN}║          Knull Installation Complete!                  ║${NC}"
    echo -e "${GREEN}╚════════════════════════════════════════════════════════╝${NC}"
    echo ""
    echo "Quick start:"
    echo "  knull --version          # Show version"
    echo "  knull run hello.knull    # Run a program"
    echo "  knull new myproject      # Create new project"
    echo "  knull repl               # Interactive shell"
    echo ""
    echo "Documentation:"
    echo "  https://github.com/4fqr/knull#readme"
    echo ""
    echo "To uninstall:"
    echo "  curl -sSL https://raw.githubusercontent.com/4fqr/knull/main/uninstall.sh | bash"
    echo ""
}

# Main installation
main() {
    print_header
    
    local os=$(detect_os)
    local arch=$(detect_arch)
    
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
