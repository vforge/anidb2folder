#!/usr/bin/env bash
# Release script - bumps major version and publishes
#
# Usage:
#   ./scripts/release.sh         # Bump major version and release
#   ./scripts/release.sh --dry   # Show what would happen without making changes

set -e

SCRIPT_DIR="$(cd "$(dirname "$0")" && pwd)"
PROJECT_DIR="$(cd "$SCRIPT_DIR/.." && pwd)"
CARGO_TOML="$PROJECT_DIR/Cargo.toml"

# Colors
RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
BLUE='\033[0;34m'
NC='\033[0m'

DRY_RUN=false
if [[ "$1" == "--dry" ]]; then
    DRY_RUN=true
fi

info() { echo -e "${GREEN}[INFO]${NC} $1"; }
warn() { echo -e "${YELLOW}[WARN]${NC} $1"; }
error() { echo -e "${RED}[ERROR]${NC} $1"; exit 1; }
step() { echo -e "${BLUE}[STEP]${NC} $1"; }

# Get current version from Cargo.toml
get_version() {
    grep '^version = ' "$CARGO_TOML" | head -1 | sed 's/version = "\(.*\)"/\1/'
}

# Bump major version: X.0.0 -> (X+1).0.0
bump_major() {
    local version=$1
    local major="${version%%.*}"
    echo "$((major + 1)).0.0"
}

# Update version in Cargo.toml
set_version() {
    local new_version=$1
    if [[ "$OSTYPE" == "darwin"* ]]; then
        sed -i '' "s/^version = \".*\"/version = \"$new_version\"/" "$CARGO_TOML"
    else
        sed -i "s/^version = \".*\"/version = \"$new_version\"/" "$CARGO_TOML"
    fi
}

main() {
    cd "$PROJECT_DIR"

    # Pre-flight checks
    step "Running pre-flight checks..."

    # Check for uncommitted changes
    if [[ -n $(git status --porcelain) ]]; then
        error "Working directory not clean. Commit or stash changes first."
    fi

    # Check we're on master
    local branch=$(git branch --show-current)
    if [[ "$branch" != "master" ]]; then
        error "Not on master branch (current: $branch)"
    fi

    # Check remote is up to date
    git fetch origin master --quiet
    local local_rev=$(git rev-parse HEAD)
    local remote_rev=$(git rev-parse origin/master)
    if [[ "$local_rev" != "$remote_rev" ]]; then
        warn "Local and remote are out of sync. Push or pull first."
        error "Local: $local_rev, Remote: $remote_rev"
    fi

    # Get versions
    local current_version=$(get_version)
    local new_version=$(bump_major "$current_version")
    local tag="v$new_version"

    echo
    info "Current version: $current_version"
    info "New version:     $new_version"
    info "Git tag:         $tag"
    echo

    # Check tag doesn't exist
    if git tag -l | grep -q "^$tag\$"; then
        error "Tag $tag already exists"
    fi

    if $DRY_RUN; then
        warn "Dry run - no changes made"
        echo
        echo "Would execute:"
        echo "  1. Update Cargo.toml version to $new_version"
        echo "  2. Run cargo check to update Cargo.lock"
        echo "  3. Commit: \"chore: release v$new_version\""
        echo "  4. Create tag: $tag"
        echo "  5. Push commit and tag to origin"
        echo "  6. GitHub Actions builds and publishes release"
        exit 0
    fi

    # Confirm
    echo -n "Proceed with release? [y/N] "
    read -r confirm
    if [[ "$confirm" != "y" && "$confirm" != "Y" ]]; then
        info "Aborted"
        exit 0
    fi

    echo

    # Update version
    step "Updating Cargo.toml to $new_version..."
    set_version "$new_version"

    # Update Cargo.lock
    step "Updating Cargo.lock..."
    cargo check --quiet

    # Commit
    step "Creating commit..."
    git add Cargo.toml Cargo.lock
    git commit -m "🔖 chore: release v$new_version"

    # Tag
    step "Creating tag $tag..."
    git tag -a "$tag" -m "Release $new_version"

    # Push
    step "Pushing to origin..."
    git push origin master
    git push origin "$tag"

    echo
    info "Release $new_version published!"
    echo
    echo "GitHub Actions will now build and create the release."
    echo "Monitor at: https://github.com/vforge/anidb2folder/actions"
    echo "Release at: https://github.com/vforge/anidb2folder/releases/tag/$tag"
}

main "$@"
