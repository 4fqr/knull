#!/bin/bash
# Knull Language Uninstaller
# Usage: curl -sSL https://raw.githubusercontent.com/4fqr/knull/main/uninstall.sh | bash

set -e

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
    echo -e "${YELLOW}Uninstaller${NC}"
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

# Find install directory
find_install_dir() {
    local paths=(
        "$HOME/.local/bin/knull"
        "$HOME/.knull/bin/knull"
        "/usr/local/bin/knull"
        "/usr/bin/knull"
    )
    
    for path in "${paths[@]}"; do
        if [ -x "$path" ]; then
            dirname "$path"
            return 0
        fi
    done
    
    return 1
}

# Remove binary
remove_binary() {
    local install_dir=$(find_install_dir)
    
    if [ -n "$install_dir" ]; then
        print_info "Removing binary from $install_dir..."
        
        rm -f "$install_dir/knull"
        rm -f "$install_dir/kn"
        
        print_success "Binary removed"
    else
        print_error "Knull binary not found"
    fi
}

# Remove stdlib
remove_stdlib() {
    local stdlib_dir="$HOME/.knull"
    
    if [ -d "$stdlib_dir" ]; then
        print_info "Removing standard library..."
        rm -rf "$stdlib_dir"
        print_success "Standard library removed"
    fi
}

# Remove from PATH
remove_from_path() {
    print_info "Cleaning up PATH..."
    
    local shell_rcs=(
        "$HOME/.bashrc"
        "$HOME/.zshrc"
        "$HOME/.profile"
        "$HOME/.bash_profile"
    )
    
    for rc in "${shell_rcs[@]}"; do
        if [ -f "$rc" ]; then
            # Remove Knull-related lines
            sed -i '/# Knull Programming Language/d' "$rc" 2>/dev/null || true
            sed -i '/export PATH=.*knull/d' "$rc" 2>/dev/null || true
        fi
    done
    
    print_success "PATH cleaned"
}

# Remove cache
remove_cache() {
    local cache_dirs=(
        "$HOME/.knull/cache"
        "/tmp/knull-*"
    )
    
    print_info "Removing cache..."
    
    for dir in "${cache_dirs[@]}"; do
        rm -rf $dir 2>/dev/null || true
    done
    
    print_success "Cache removed"
}

# Confirm uninstall
confirm_uninstall() {
    echo ""
    echo "This will completely remove Knull from your system."
    echo ""
    read -p "Are you sure? [y/N] " -n 1 -r
    echo ""
    
    if [[ ! $REPLY =~ ^[Yy]$ ]]; then
        echo "Uninstall cancelled."
        exit 0
    fi
}

# Print final message
print_footer() {
    echo ""
    echo -e "${GREEN}╔════════════════════════════════════════════════════════╗${NC}"
    echo -e "${GREEN}║     Knull has been successfully uninstalled            ║${NC}"
    echo -e "${GREEN}╚════════════════════════════════════════════════════════╝${NC}"
    echo ""
    echo "To reinstall:"
    echo "  curl -sSL https://raw.githubusercontent.com/4fqr/knull/main/install.sh | bash"
    echo ""
    echo "Thanks for trying Knull!"
    echo ""
}

# Main
main() {
    print_header
    
    # Check if installed
    if ! find_install_dir >/dev/null 2>&1; then
        print_error "Knull is not installed"
        exit 1
    fi
    
    # Confirm
    confirm_uninstall
    
    # Remove everything
    remove_binary
    remove_stdlib
    remove_from_path
    remove_cache
    
    print_footer
}

# Handle force uninstall
if [ "$1" = "--force" ] || [ "$1" = "-f" ]; then
    main
else
    main
fi
