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
  build   Build release binary
  test    Run all tests (sets up test-data first)
  check   Run fmt check, clippy, and tests
  fmt     Format code
  run     Run with arguments (e.g., ./run.sh run --help)
EOF
        ;;
esac
