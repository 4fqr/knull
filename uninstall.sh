#!/bin/bash
# Knull Language Uninstaller
# Usage: curl -sSL https://raw.githubusercontent.com/4fqr/knull/master/uninstall.sh | bash

set -e

# Colors using printf for portability
red=$(printf '\033[0;31m')
green=$(printf '\033[0;32m')
yellow=$(printf '\033[1;33m')
blue=$(printf '\033[0;34m')
nc=$(printf '\033[0m')

# Print functions
print_header() {
    printf "%s\n" "${blue}"
    printf ".____/\ .______ .____     .___   .___\n"
    printf ":   /  \:      \|    |___ |   |  |   |\n"
    printf "|.  ___/|       ||    |   ||   |  |   |\n"
    printf "|     \ |   |   ||    :   ||   |/\|   |/\n"
    printf "|      \|___|   ||        ||   /  \|   /  \\n"
    printf "|___\  /    |___||. _____/ |______/|______/\n"
    printf "     \/           :/\n"
    printf "                  :\n"
    printf "%s\n" "${nc}"
    printf "%sUninstaller%s\n" "${yellow}" "${nc}"
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

# Find install directory
find_install_dir() {
    local paths="$HOME/.local/bin/knull $HOME/.knull/bin/knull /usr/local/bin/knull /usr/bin/knull"
    
    for path in $paths; do
        if [ -x "$path" ]; then
            dirname "$path"
            return 0
        fi
    done
    
    return 1
}

# Remove binary
remove_binary() {
    local install_dir
    install_dir=$(find_install_dir)
    
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
    
    local shell_rcs="$HOME/.bashrc $HOME/.zshrc $HOME/.profile $HOME/.bash_profile"
    
    for rc in $shell_rcs; do
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
    print_info "Removing cache..."
    
    rm -rf "$HOME/.knull/cache" 2>/dev/null || true
    rm -rf /tmp/knull-* 2>/dev/null || true
    
    print_success "Cache removed"
}

# Confirm uninstall
confirm_uninstall() {
    # Check if running non-interactively (piped from curl)
    if [ ! -t 0 ]; then
        # Auto-confirm when piped
        printf "\n"
        printf "Running non-interactively. Auto-confirming uninstall...\n"
        printf "\n"
        return 0
    fi
    
    printf "\n"
    printf "This will completely remove Knull from your system.\n"
    printf "\n"
    printf "Are you sure? [y/N] "
    read -r REPLY
    printf "\n"
    
    case "$REPLY" in
        [Yy]*)
            ;;
        *)
            printf "Uninstall cancelled.\n"
            exit 0
            ;;
    esac
}

# Print final message
print_footer() {
    printf "\n"
    printf "%s╔════════════════════════════════════════════════════════╗%s\n" "${green}" "${nc}"
    printf "%s║     Knull has been successfully uninstalled            ║%s\n" "${green}" "${nc}"
    printf "%s╚════════════════════════════════════════════════════════╝%s\n" "${green}" "${nc}"
    printf "\n"
    printf "To reinstall:\n"
    printf "  curl -sSL https://raw.githubusercontent.com/4fqr/knull/master/install.sh | bash\n"
    printf "\n"
    printf "Thanks for trying Knull!\n"
    printf "\n"
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
    if [ "$1" != "--force" ] && [ "$1" != "-f" ]; then
        confirm_uninstall
    fi
    
    # Remove everything
    remove_binary
    remove_stdlib
    remove_from_path
    remove_cache
    
    print_footer
}

main "$@"
