#!/usr/bin/env sh
set -eu

PREFIX=${PREFIX:-"$HOME/.local"}
BINDIR=${BINDIR:-"$PREFIX/bin"}
INSTALL_DEPS=1
UNINSTALL=0
TARGET=target/release/termeprompter

say() {
    printf '%s\n' "$*"
}

usage() {
    cat <<'USAGE'
Usage: ./install.sh [options]

Build and install termeprompter.

Options:
  --prefix PATH   Install under PATH instead of ~/.local
  --bindir PATH   Install binary into PATH instead of PREFIX/bin
  --no-deps       Do not install Rust tooling if cargo is missing
  --uninstall     Remove the installed binary
  -h, --help      Show this help

Examples:
  ./install.sh
  ./install.sh --prefix /usr/local
  ./install.sh --no-deps
USAGE
}

parse_args() {
    while [ "$#" -gt 0 ]; do
        case "$1" in
            --prefix)
                [ "$#" -ge 2 ] || {
                    echo "error: --prefix needs a path" >&2
                    exit 2
                }
                PREFIX=$2
                BINDIR=$PREFIX/bin
                shift 2
                ;;
            --bindir)
                [ "$#" -ge 2 ] || {
                    echo "error: --bindir needs a path" >&2
                    exit 2
                }
                BINDIR=$2
                shift 2
                ;;
            --no-deps)
                INSTALL_DEPS=0
                shift
                ;;
            --uninstall)
                UNINSTALL=1
                shift
                ;;
            -h | --help)
                usage
                exit 0
                ;;
            *)
                echo "error: unknown option: $1" >&2
                usage >&2
                exit 2
                ;;
        esac
    done
}

need_cmd() {
    command -v "$1" >/dev/null 2>&1
}

run_as_root() {
    if [ "$(id -u)" -eq 0 ]; then
        "$@"
    elif need_cmd sudo; then
        sudo "$@"
    else
        echo "error: need root privileges for: $*" >&2
        echo "hint: rerun with sudo or install under --prefix \"\$HOME/.local\"" >&2
        exit 1
    fi
}

run_file_op() {
    if "$@"; then
        return
    fi

    echo "retrying with elevated privileges: $*" >&2
    run_as_root "$@"
}

detect_pkg_manager() {
    if need_cmd pacman; then
        echo pacman
    elif need_cmd apt-get; then
        echo apt
    else
        echo unknown
    fi
}

install_rust_tooling() {
    if need_cmd cargo; then
        return
    fi

    if [ "$INSTALL_DEPS" -eq 0 ]; then
        echo "error: cargo is not installed" >&2
        echo "hint: install Rust tooling, or rerun without --no-deps on Arch/Ubuntu" >&2
        exit 1
    fi

    case "$(detect_pkg_manager)" in
        pacman)
            say "Installing Rust tooling with pacman..."
            run_as_root pacman -Sy --needed rust
            ;;
        apt)
            say "Installing Rust tooling with apt..."
            run_as_root apt-get update
            run_as_root apt-get install -y cargo rustc ca-certificates
            ;;
        *)
            echo "error: could not find pacman or apt-get, and cargo is missing" >&2
            echo "hint: install Rust from your package manager, then rerun ./install.sh --no-deps" >&2
            exit 1
            ;;
    esac
}

build_binary() {
    say "Building termeprompter..."
    cargo build --release --locked
}

install_binary() {
    say "Installing to $BINDIR/termeprompter..."
    run_file_op mkdir -p "$BINDIR"
    run_file_op install -m 0755 "$TARGET" "$BINDIR/termeprompter"
    say "Installed termeprompter."
    say "Run: termeprompter --demo"
}

uninstall_binary() {
    if [ ! -e "$BINDIR/termeprompter" ]; then
        say "Nothing to remove at $BINDIR/termeprompter"
        return
    fi

    run_file_op rm -f "$BINDIR/termeprompter"
    say "Removed $BINDIR/termeprompter"
}

main() {
    parse_args "$@"

    if [ "$UNINSTALL" -eq 1 ]; then
        uninstall_binary
        exit 0
    fi

    install_rust_tooling
    build_binary
    install_binary
}

main "$@"
