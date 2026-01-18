#!/bin/bash
set -e

# luxctl installer
# Usage: curl -fsSL https://raw.githubusercontent.com/thearyanahmed/luxctl/master/install.sh | bash

VERSION="${LUXCTL_VERSION:-}"
CARGO_BIN="${CARGO_HOME:-$HOME/.cargo}/bin"

info() {
    printf "\033[0;34m==>\033[0m %s\n" "$1"
}

success() {
    printf "\033[0;32m==>\033[0m %s\n" "$1"
}

error() {
    printf "\033[0;31merror:\033[0m %s\n" "$1" >&2
    exit 1
}

check_cmd() {
    command -v "$1" >/dev/null 2>&1
}

install_rustup() {
    info "Rust not found. Installing via rustup..."
    curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh -s -- -y --no-modify-path

    # source cargo env for current session
    if [ -f "$CARGO_BIN/../env" ]; then
        . "$CARGO_BIN/../env"
    fi

    if ! check_cmd cargo; then
        export PATH="$CARGO_BIN:$PATH"
    fi
}

main() {
    echo ""
    echo "  luxctl - projectlighthouse.io"
    echo ""

    # check for cargo
    if ! check_cmd cargo; then
        install_rustup
    fi

    # verify cargo is available
    if ! check_cmd cargo; then
        error "cargo not found after installation. Please install Rust manually: https://rustup.rs"
    fi

    info "Installing luxctl..."

    if [ -n "$VERSION" ]; then
        cargo install luxctl --version "$VERSION"
    else
        cargo install luxctl
    fi

    # verify installation
    if check_cmd luxctl; then
        INSTALLED_VERSION=$(luxctl --version 2>/dev/null | grep -oE '[0-9]+\.[0-9]+\.[0-9]+' | head -1 || echo "unknown")
        success "luxctl $INSTALLED_VERSION installed successfully!"
    else
        # might need to add to PATH
        if [ -f "$CARGO_BIN/luxctl" ]; then
            success "luxctl installed to $CARGO_BIN/luxctl"
            echo ""
            echo "Add cargo bin to your PATH if not already:"
            echo "  export PATH=\"\$HOME/.cargo/bin:\$PATH\""
        else
            error "Installation failed"
        fi
    fi

    echo ""
    echo "Get started:"
    echo "  luxctl auth --token <YOUR_TOKEN>"
    echo "  luxctl doctor"
    echo ""
}

main
