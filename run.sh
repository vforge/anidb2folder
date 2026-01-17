#!/bin/bash
set -e

case "${1:-help}" in
    build)
        cargo build --release
        ;;
    test)
        cargo test
        ;;
    check)
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
  test    Run all tests
  check   Run fmt check, clippy, and tests
  fmt     Format code
  run     Run with arguments (e.g., ./run.sh run --help)
EOF
        ;;
esac
