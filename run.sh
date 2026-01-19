#!/bin/bash
set -e

SCRIPT_DIR="$(cd "$(dirname "$0")" && pwd)"

setup_test_data() {
    "$SCRIPT_DIR/scripts/setup-test-data.sh"
}

case "${1:-help}" in
    build)
        cargo build --release
        ;;
    release)
        shift
        "$SCRIPT_DIR/scripts/build-release.sh" "$@"
        ;;
    test)
        setup_test_data
        cargo test
        ;;
    check)
        setup_test_data
        cargo fmt --check
        cargo clippy
        cargo test
        ;;
    fmt)
        cargo fmt
        ;;
    run)
        shift
        cargo run -- "$@"
        ;;
    help|*)
        cat <<EOF
Usage: ./run.sh <command>

Commands:
  build    Build release binary
  release  Build cross-platform binaries (linux, macos)
  test     Run all tests (sets up test-data first)
  check    Run fmt check, clippy, and tests
  fmt      Format code
  run      Run with arguments (e.g., ./run.sh run --help)

Verbosity levels:
  (none)  Only warnings and errors
  -v      Info messages
  -vv     Debug messages
  -vvv    Trace messages

Examples:
  ./run.sh run -v ./test-data
  ./run.sh run -vv ./test-data
EOF
        ;;
esac
