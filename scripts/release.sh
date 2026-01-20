#!/usr/bin/env bash
# Release script - bumps version and publishes
#
# Usage:
#   ./scripts/release.sh major    # 1.2.3 -> 2.0.0 (breaking changes)
#   ./scripts/release.sh minor    # 1.2.3 -> 1.3.0 (new features)
#   ./scripts/release.sh patch    # 1.2.3 -> 1.2.4 (bug fixes)
#   ./scripts/release.sh --dry    # Show what would happen

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
BUMP_TYPE=""

# Parse arguments
for arg in "$@"; do
    case $arg in
        --dry)
            DRY_RUN=true
            ;;
        major|minor|patch)
            BUMP_TYPE="$arg"
            ;;
        *)
            echo -e "${RED}Unknown argument: $arg${NC}"
            echo "Usage: $0 [major|minor|patch] [--dry]"
            exit 1
            ;;
    esac
done

if [ -z "$BUMP_TYPE" ] && ! $DRY_RUN; then
    echo -e "${RED}Error: Must specify bump type (major, minor, or patch)${NC}"
    echo ""
    echo "Usage: $0 <major|minor|patch> [--dry]"
    echo ""
    echo "  major  - Breaking changes (1.2.3 -> 2.0.0)"
    echo "  minor  - New features (1.2.3 -> 1.3.0)"
    echo "  patch  - Bug fixes (1.2.3 -> 1.2.4)"
    echo "  --dry  - Preview without making changes"
    exit 1
fi

info() { echo -e "${GREEN}[INFO]${NC} $1"; }
warn() { echo -e "${YELLOW}[WARN]${NC} $1"; }
error() { echo -e "${RED}[ERROR]${NC} $1"; exit 1; }
step() { echo -e "${BLUE}[STEP]${NC} $1"; }

# Get current version from Cargo.toml
get_version() {
    grep '^version = ' "$CARGO_TOML" | head -1 | sed 's/version = "\(.*\)"/\1/'
}

# Bump version based on type
bump_version() {
    local version=$1
    local type=$2

    local major minor patch
    IFS='.' read -r major minor patch <<< "$version"

    case $type in
        major)
            echo "$((major + 1)).0.0"
            ;;
        minor)
            echo "$major.$((minor + 1)).0"
            ;;
        patch)
            echo "$major.$minor.$((patch + 1))"
            ;;
    esac
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

# Show commits since last tag
show_changes() {
    local last_tag=$(git describe --tags --abbrev=0 2>/dev/null || echo "")

    if [ -z "$last_tag" ]; then
        echo "  (first release - all commits)"
        return
    fi

    echo "  Changes since $last_tag:"
    git log --pretty=format:"    %s" "$last_tag..HEAD" | head -20
    local count=$(git rev-list --count "$last_tag..HEAD")
    if [ "$count" -gt 20 ]; then
        echo "    ... and $((count - 20)) more commits"
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

    if $DRY_RUN && [ -z "$BUMP_TYPE" ]; then
        # Just show current state
        echo
        info "Current version: $current_version"
        echo
        echo "Bump options:"
        echo "  major -> $(bump_version "$current_version" major)"
        echo "  minor -> $(bump_version "$current_version" minor)"
        echo "  patch -> $(bump_version "$current_version" patch)"
        echo
        show_changes
        exit 0
    fi

    local new_version=$(bump_version "$current_version" "$BUMP_TYPE")
    local tag="v$new_version"

    echo
    info "Current version: $current_version"
    info "Bump type:       $BUMP_TYPE"
    info "New version:     $new_version"
    info "Git tag:         $tag"
    echo
    show_changes
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
        echo "  3. Commit: \"🔖 chore: release v$new_version\""
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
