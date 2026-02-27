#!/bin/bash
# Knull Installer - Build from Source
# Usage: curl -sSL https://raw.githubusercontent.com/4fqr/knull/main/install.sh | sh

set -e

VERSION="1.0.0"
INSTALL_DIR="/usr/local/bin"
REPO_URL="https://github.com/4fqr/knull.git"

# Colors
RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
NC='\033[0m'

echo -e "${GREEN}Installing Knull ${VERSION}...${NC}"

# Detect OS
detect_os() {
    case "$(uname -s)" in
        Linux*)     echo "linux";;
        Darwin*)    echo "macos";;
        *)          echo "unsupported";;
    esac
}

# Detect architecture
detect_arch() {
    case "$(uname -m)" in
        x86_64)    echo "x86_64";;
        aarch64)    echo "arm64";;
        arm64)      echo "arm64";;
        *)          echo "unsupported";;
    esac
}

# Check for Rust
check_rust() {
    # Check common cargo locations
    local cargo_paths=("cargo" "$HOME/.cargo/bin/cargo" "/usr/bin/cargo" "$HOME/.cargo/bin/cargo")
    
    for path in "${cargo_paths}"; do
        if command -v "$path" &> /dev/null; then
            echo "Rust found: $($path --version 2>/dev/null || echo 'cargo found')"
            return 0
        fi
    done
    
    # Also check RUSTUP_HOME
    if [ -n "$RUSTUP_HOME" ] && [ -x "$RUSTUP_HOME/bin/cargo" ]; then
        echo "Rust found at $RUSTUP_HOME/bin/cargo"
        return 0
    fi
    
    echo -e "${RED}Error: Rust not found${NC}"
    echo "Please install Rust from https://rustup.rs"
    echo "Then run this installer again."
    exit 1
}

# Build from source
build_from_source() {
    local temp_dir="/tmp/knull-build"
    
    echo -e "${YELLOW}Building from source...${NC}"
    
    # Clone repository
    if [ -d "${temp_dir}" ]; then
        rm -rf "${temp_dir}"
    fi
    
    echo "Cloning Knull..."
    git clone --depth 1 "${REPO_URL}" "${temp_dir}"
    
    # Build
    echo "Building compiler..."
    cd "${temp_dir}/src"
    cargo build --release --no-default-features
    
    # Install
    echo "Installing..."
    if [ -w "${INSTALL_DIR}" ]; then
        cp target/release/knull "${INSTALL_DIR}/knull"
    else
        mkdir -p "${HOME}/.local/bin"
        cp target/release/knull "${HOME}/.local/bin/knull"
        echo "Added to \${HOME}/.local/bin - add to PATH if needed"
    fi
    
    # Cleanup
    rm -rf "${temp_dir}"
    
    echo -e "${GREEN}Installation complete!${NC}"
}

# Main
main() {
    local os=$(detect_os)
    local arch=$(detect_arch)
    
    if [ "$os" = "unsupported" ]; then
        echo -e "${RED}Unsupported OS${NC}"
        exit 1
    fi
    
    if [ "$arch" = "unsupported" ]; then
        echo -e "${RED}Unsupported architecture${NC}"
        exit 1
    fi
    
    echo "Detected: ${os} (${arch})"
    
    # Check for Rust
    check_rust
    
    # Build and install
    build_from_source
    
    # Verify
    if command -v knull &> /dev/null; then
        echo ""
        knull --version
    else
        echo ""
        echo "Run 'knull --version' to verify installation"
    fi
}

main
