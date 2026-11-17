#!/bin/sh
# Set Fractique up on macOS or Linux with one command: ./setup.sh [--skip-tests]
# It installs Rust when missing (rustup, into your home folder, no sudo), builds the command line tool and runs the
# tests, and ends with the command that renders your first image. Safe to run again.
set -eu
cd "$(dirname "$0")"

say() { printf '[setup] %s\n' "$1"; }

if ! command -v cc >/dev/null 2>&1; then
    say "No C compiler found: Rust needs one to link programs. Install it, then run this again:"
    case "$(uname -s)" in
        Darwin) say "  xcode-select --install" ;;
        *)
            say "  Debian, Ubuntu:  sudo apt install build-essential"
            say "  Fedora:          sudo dnf install gcc"
            say "  Arch:            sudo pacman -S base-devel"
            ;;
    esac
    exit 1
fi

if ! command -v cargo >/dev/null 2>&1 && [ -x "$HOME/.cargo/bin/cargo" ]; then
    PATH="$HOME/.cargo/bin:$PATH"
    export PATH
fi
if ! command -v cargo >/dev/null 2>&1; then
    say "Installing Rust (rustup)..."
    if command -v curl >/dev/null 2>&1; then
        curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh -s -- -y --profile minimal --default-toolchain none
    elif command -v wget >/dev/null 2>&1; then
        wget -qO- https://sh.rustup.rs | sh -s -- -y --profile minimal --default-toolchain none
    else
        say "Neither curl nor wget is installed: install one of them, then run this again."
        exit 1
    fi
    PATH="$HOME/.cargo/bin:$PATH"
    export PATH
fi

# The first cargo command also installs the Rust version pinned in rust-toolchain.toml.
say "Building (the first build takes a minute or two)..."
cargo build --release --locked

if [ "${1:-}" != "--skip-tests" ]; then
    say "Running the tests..."
    cargo test --release --locked --quiet
fi

say "Done. Render your first image:"
say "  ./target/release/fractique render --preset aurora-spiral -o aurora.png"
say "More: ./target/release/fractique --help, and docs/usage.md"
