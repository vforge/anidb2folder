#!/bin/bash
# Recreates test-data directories for manual testing

set -e

SCRIPT_DIR="$(cd "$(dirname "$0")" && pwd)"
PROJECT_DIR="$(dirname "$SCRIPT_DIR")"
TEST_DATA_DIR="$PROJECT_DIR/test-data"

rm -rf "$TEST_DATA_DIR"
mkdir -p "$TEST_DATA_DIR"

# AniDB format directories
mkdir -p "$TEST_DATA_DIR/[AS0] 12345"
mkdir -p "$TEST_DATA_DIR/67890"

echo "Test data created in $TEST_DATA_DIR"
