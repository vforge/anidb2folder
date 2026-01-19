#!/usr/bin/env bash
# Build release binaries for all supported platforms using cargo-zigbuild
#
# Prerequisites:
#   - zig: brew install zig (or your package manager)
#   - cargo-zigbuild: cargo install cargo-zigbuild
#   - Rust targets are added automatically by this script
#
# Usage:
#   ./scripts/build-release.sh          # Build all targets
#   ./scripts/build-release.sh linux    # Build only Linux targets
#   ./scripts/build-release.sh macos    # Build only macOS targets

set -e

SCRIPT_DIR="$(cd "$(dirname "$0")" && pwd)"
PROJECT_DIR="$(cd "$SCRIPT_DIR/.." && pwd)"
DIST_DIR="$PROJECT_DIR/dist"
BINARY_NAME="anidb2folder"

# Target definitions as "target:suffix" pairs (bash 3.x compatible)
# Note: Windows is excluded due to cross-compilation issues with the ring crate.
# Use GitHub Actions or a Windows machine for Windows builds.
ALL_TARGETS=(
    "x86_64-unknown-linux-gnu:linux-x64"
    "aarch64-unknown-linux-gnu:linux-arm64"
    "x86_64-apple-darwin:macos-x64"
    "aarch64-apple-darwin:macos-arm64"
)

# Group targets by platform
LINUX_TARGETS=("x86_64-unknown-linux-gnu:linux-x64" "aarch64-unknown-linux-gnu:linux-arm64")
MACOS_TARGETS=("x86_64-apple-darwin:macos-x64" "aarch64-apple-darwin:macos-arm64")

# Colors for output
RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
NC='\033[0m' # No Color

info() {
    echo -e "${GREEN}[INFO]${NC} $1"
}

warn() {
    echo -e "${YELLOW}[WARN]${NC} $1"
}

error() {
    echo -e "${RED}[ERROR]${NC} $1"
    exit 1
}

check_prerequisites() {
    info "Checking prerequisites..."

    if ! command -v zig &> /dev/null; then
        error "zig not found. Install with: brew install zig"
    fi

    if ! command -v cargo-zigbuild &> /dev/null; then
        error "cargo-zigbuild not found. Install with: cargo install cargo-zigbuild"
    fi

    info "Prerequisites OK"
}

ensure_target() {
    local target=$1
    if ! rustup target list --installed | grep -q "^$target\$"; then
        info "Adding Rust target: $target"
        rustup target add "$target"
    fi
}

build_target() {
    local target_pair=$1
    local target="${target_pair%%:*}"
    local suffix="${target_pair##*:}"

    info "Building for $target..."
    ensure_target "$target"

    # Use regular cargo for macOS targets (zigbuild has framework issues on macOS)
    # Use zigbuild for Linux cross-compilation
    if [[ "$target" == *"apple-darwin"* ]]; then
        cargo build --release --target "$target"
    else
        cargo zigbuild --release --target "$target"
    fi

    # Determine source binary path and output name
    local src_binary="$PROJECT_DIR/target/$target/release/$BINARY_NAME"
    local dest_binary="$DIST_DIR/$BINARY_NAME-$suffix"

    # Windows binaries have .exe extension
    if [[ "$target" == *"windows"* ]]; then
        src_binary="${src_binary}.exe"
        dest_binary="${dest_binary}.exe"
    fi

    if [[ -f "$src_binary" ]]; then
        cp "$src_binary" "$dest_binary"
        info "Created: $dest_binary"
    else
        error "Build succeeded but binary not found: $src_binary"
    fi
}

select_targets() {
    local filter=$1
    case "$filter" in
        linux)
            echo "${LINUX_TARGETS[@]}"
            ;;
        macos)
            echo "${MACOS_TARGETS[@]}"
            ;;
        ""|all)
            echo "${ALL_TARGETS[@]}"
            ;;
        *)
            error "Unknown platform filter: $filter. Use: linux, macos, or all"
            ;;
    esac
}

main() {
    local filter=${1:-all}

    cd "$PROJECT_DIR"
    check_prerequisites

    # Create dist directory
    mkdir -p "$DIST_DIR"

    # Get targets to build
    local targets
    read -ra targets <<< "$(select_targets "$filter")"

    info "Building ${#targets[@]} target(s)..."
    echo

    local failed=0
    for target in "${targets[@]}"; do
        if ! build_target "$target"; then
            warn "Failed to build: $target"
            ((failed++))
        fi
        echo
    done

    # Summary
    echo "========================================="
    info "Build complete!"
    echo
    echo "Binaries in $DIST_DIR:"
    ls -lh "$DIST_DIR/" 2>/dev/null || echo "(empty)"
    echo

    if [[ $failed -gt 0 ]]; then
        warn "$failed target(s) failed to build"
        exit 1
    fi

    info "Verify with: file $DIST_DIR/*"
}

main "$@"
