#!/bin/bash
# Recreates test-data directories for manual testing

set -e

SCRIPT_DIR="$(cd "$(dirname "$0")" && pwd)"
PROJECT_DIR="$(dirname "$SCRIPT_DIR")"
TEST_DATA_DIR="$PROJECT_DIR/test-data"

rm -rf "$TEST_DATA_DIR"
mkdir -p "$TEST_DATA_DIR"

# AniDB format directories from anidbs_50.txt
IDS=(
10914 18297 14728 5235 18818 12337 12107 346 147 14437
4414 3431 11782 18249 10181 5007 10392 15530 18722 8364
15359 15327 18708 13698 18086 8307 10663 13518 14442 13363
6238 18862 15110 8809 18547 18892 10272 959 17075 9734
16848 1045 11272 12494 4270 15215 2881 6201 17222 11725
)

for id in "${IDS[@]}"; do
  mkdir -p "$TEST_DATA_DIR/$id"
done

echo "Test data created in $TEST_DATA_DIR with ${#IDS[@]} directories"
