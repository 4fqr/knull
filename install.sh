#!/bin/bash
# Knull Installer - Universal Installer Script
# Usage: curl -sSL https://raw.githubusercontent.com/4fqr/knull/main/install.sh | sh

set -e

VERSION="1.0.0"
INSTALL_DIR="/usr/local/bin"
TEMP_DIR="/tmp/knull-install"

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

# Download and install
install() {
    local os=$1
    local arch=$2
    local url="https://github.com/4fqr/knull/releases/download/v${VERSION}/knull-${os}-${arch}"
    
    echo "Detected: ${os} (${arch})"
    echo "Downloading from: ${url}"
    
    # Create temp directory
    mkdir -p ${TEMP_DIR}
    
    # Download binary
    echo "Downloading..."
    if command -v curl &> /dev/null; then
        curl -sSL -o "${TEMP_DIR}/knull" "${url}" || {
            echo -e "${YELLOW}Binary not found. Building from source...${NC}"
            return 1
        }
    elif command -v wget &> /dev/null; then
        wget -q -O "${TEMP_DIR}/knull" "${url}" || {
            echo -e "${YELLOW}Binary not found. Building from source...${NC}"
            return 1
        }
    else
        echo -e "${RED}Error: curl or wget required${NC}"
        return 1
    fi
    
    # Make executable
    chmod +x "${TEMP_DIR}/knull"
    
    # Install
    if [ -w "${INSTALL_DIR}" ]; then
        mv "${TEMP_DIR}/knull" "${INSTALL_DIR}/knull"
        echo -e "${GREEN}Installed to ${INSTALL_DIR}/knull${NC}"
    else
        echo -e "${YELLOW}Warning: ${INSTALL_DIR} not writable, using ~/.local/bin${NC}"
        mkdir -p "${HOME}/.local/bin"
        mv "${TEMP_DIR}/knull" "${HOME}/.local/bin/knull"
        echo -e "Add ~/.local/bin to your PATH"
    fi
    
    # Cleanup
    rm -rf ${TEMP_DIR}
    
    echo -e "${GREEN}Installation complete!${NC}"
    echo ""
    echo "Run 'knull --version' to verify"
}

# Build from source (fallback)
build_from_source() {
    echo "Building from source..."
    
    if ! command -v cargo &> /dev/null; then
        echo -e "${RED}Error: Rust not installed. Install from https://rustup.rs${NC}"
        exit 1
    fi
    
    # Clone or use existing
    if [ ! -d "/tmp/knull" ]; then
        git clone --depth 1 https://github.com/4fqr/knull.git /tmp/knull 2>/dev/null || {
            echo "Using existing source..."
        }
    fi
    
    cd /tmp/knull/src
    cargo build --release --no-default-features
    
    # Install
    if [ -w "${INSTALL_DIR}" ]; then
        cp target/release/knull "${INSTALL_DIR}/knull"
    else
        mkdir -p "${HOME}/.local/bin"
        cp target/release/knull "${HOME}/.local/bin/knull"
    fi
    
    echo -e "${GREEN}Built and installed successfully!${NC}"
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
    
    # Try download first, fall back to build
    install $os $arch || build_from_source
    
    # Verify
    if command -v knull &> /dev/null; then
        knull --version || echo "Run 'knull --version' to verify"
    fi
}

main
