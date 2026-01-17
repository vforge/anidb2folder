# 99 - GitHub Actions Release

## Summary

Set up CI/CD pipeline for automated building, testing, and releasing.

## Dependencies

None — this feature can be implemented at any time and is independent of other features.

## Description

This feature implements a GitHub Actions workflow for continuous integration and automated releases. The workflow handles:

- Running tests on every push and pull request
- Building binaries for multiple platforms
- Creating GitHub releases with artifacts
- Generating changelogs from commit history

## Requirements

### Functional Requirements

1. Run tests on every push to `main` and on pull requests
2. Build binaries for:
   - Linux x86_64 (GNU libc)
   - Linux x86_64 (musl - static)
   - Linux aarch64
   - macOS x86_64
   - macOS aarch64 (Apple Silicon)
3. Create GitHub release when a version tag is pushed (`v*.*.*`)
4. Generate changelog from commit history
5. Upload binaries as release assets
6. Run linting (clippy) and formatting checks

### Non-Functional Requirements

1. Use caching to speed up builds
2. Parallel builds for different platforms
3. Semantic versioning for releases
4. Include checksums for all binaries

## Implementation Guide

### Step 1: Create CI Workflow

```yaml
# .github/workflows/ci.yml
name: CI

on:
  push:
    branches: [main]
  pull_request:
    branches: [main]

env:
  CARGO_TERM_COLOR: always

jobs:
  test:
    name: Test
    runs-on: ubuntu-latest
    steps:
      - uses: actions/checkout@v4
      
      - name: Install Rust
        uses: dtolnay/rust-toolchain@stable
      
      - name: Cache cargo
        uses: Swatinem/rust-cache@v2
      
      - name: Run tests
        run: cargo test --all-features --verbose
      
      - name: Run doc tests
        run: cargo test --doc

  lint:
    name: Lint
    runs-on: ubuntu-latest
    steps:
      - uses: actions/checkout@v4
      
      - name: Install Rust
        uses: dtolnay/rust-toolchain@stable
        with:
          components: clippy, rustfmt
      
      - name: Cache cargo
        uses: Swatinem/rust-cache@v2
      
      - name: Check formatting
        run: cargo fmt --all -- --check
      
      - name: Run clippy
        run: cargo clippy --all-targets --all-features -- -D warnings

  build:
    name: Build (${{ matrix.target }})
    runs-on: ${{ matrix.os }}
    strategy:
      fail-fast: false
      matrix:
        include:
          - target: x86_64-unknown-linux-gnu
            os: ubuntu-latest
          - target: x86_64-unknown-linux-musl
            os: ubuntu-latest
          - target: aarch64-unknown-linux-gnu
            os: ubuntu-latest
          - target: x86_64-apple-darwin
            os: macos-latest
          - target: aarch64-apple-darwin
            os: macos-latest
    
    steps:
      - uses: actions/checkout@v4
      
      - name: Install Rust
        uses: dtolnay/rust-toolchain@stable
        with:
          targets: ${{ matrix.target }}
      
      - name: Install cross-compilation tools
        if: matrix.target == 'aarch64-unknown-linux-gnu'
        run: |
          sudo apt-get update
          sudo apt-get install -y gcc-aarch64-linux-gnu
      
      - name: Install musl tools
        if: matrix.target == 'x86_64-unknown-linux-musl'
        run: |
          sudo apt-get update
          sudo apt-get install -y musl-tools
      
      - name: Cache cargo
        uses: Swatinem/rust-cache@v2
        with:
          key: ${{ matrix.target }}
      
      - name: Build
        run: cargo build --release --target ${{ matrix.target }}
      
      - name: Upload artifact
        uses: actions/upload-artifact@v4
        with:
          name: anidb2folder-${{ matrix.target }}
          path: target/${{ matrix.target }}/release/anidb2folder
```

### Step 2: Create Release Workflow

```yaml
# .github/workflows/release.yml
name: Release

on:
  push:
    tags:
      - 'v*.*.*'

permissions:
  contents: write

env:
  CARGO_TERM_COLOR: always

jobs:
  build-release:
    name: Build Release (${{ matrix.target }})
    runs-on: ${{ matrix.os }}
    strategy:
      fail-fast: false
      matrix:
        include:
          - target: x86_64-unknown-linux-gnu
            os: ubuntu-latest
            artifact: anidb2folder-linux-x86_64
          - target: x86_64-unknown-linux-musl
            os: ubuntu-latest
            artifact: anidb2folder-linux-x86_64-musl
          - target: aarch64-unknown-linux-gnu
            os: ubuntu-latest
            artifact: anidb2folder-linux-aarch64
          - target: x86_64-apple-darwin
            os: macos-latest
            artifact: anidb2folder-macos-x86_64
          - target: aarch64-apple-darwin
            os: macos-latest
            artifact: anidb2folder-macos-aarch64
    
    steps:
      - uses: actions/checkout@v4
      
      - name: Install Rust
        uses: dtolnay/rust-toolchain@stable
        with:
          targets: ${{ matrix.target }}
      
      - name: Install cross-compilation tools
        if: matrix.target == 'aarch64-unknown-linux-gnu'
        run: |
          sudo apt-get update
          sudo apt-get install -y gcc-aarch64-linux-gnu
          echo "CARGO_TARGET_AARCH64_UNKNOWN_LINUX_GNU_LINKER=aarch64-linux-gnu-gcc" >> $GITHUB_ENV
      
      - name: Install musl tools
        if: matrix.target == 'x86_64-unknown-linux-musl'
        run: |
          sudo apt-get update
          sudo apt-get install -y musl-tools
      
      - name: Build
        run: cargo build --release --target ${{ matrix.target }}
      
      - name: Create archive
        run: |
          cd target/${{ matrix.target }}/release
          if [[ "${{ matrix.os }}" == "windows-latest" ]]; then
            7z a ../../../${{ matrix.artifact }}.zip anidb2folder.exe
          else
            tar -czvf ../../../${{ matrix.artifact }}.tar.gz anidb2folder
          fi
      
      - name: Generate checksum
        run: |
          if [[ "${{ matrix.os }}" == "macos-latest" ]]; then
            shasum -a 256 ${{ matrix.artifact }}.tar.gz > ${{ matrix.artifact }}.tar.gz.sha256
          else
            sha256sum ${{ matrix.artifact }}.tar.gz > ${{ matrix.artifact }}.tar.gz.sha256
          fi
      
      - name: Upload artifact
        uses: actions/upload-artifact@v4
        with:
          name: ${{ matrix.artifact }}
          path: |
            ${{ matrix.artifact }}.tar.gz
            ${{ matrix.artifact }}.tar.gz.sha256

  create-release:
    name: Create Release
    needs: build-release
    runs-on: ubuntu-latest
    steps:
      - uses: actions/checkout@v4
        with:
          fetch-depth: 0  # For changelog generation
      
      - name: Download all artifacts
        uses: actions/download-artifact@v4
        with:
          path: artifacts
      
      - name: Generate changelog
        id: changelog
        run: |
          # Get previous tag
          PREV_TAG=$(git describe --tags --abbrev=0 HEAD^ 2>/dev/null || echo "")
          
          if [ -z "$PREV_TAG" ]; then
            # First release
            COMMITS=$(git log --pretty=format:"- %s (%h)" --no-merges)
          else
            COMMITS=$(git log ${PREV_TAG}..HEAD --pretty=format:"- %s (%h)" --no-merges)
          fi
          
          # Create changelog file
          echo "## What's Changed" > CHANGELOG.md
          echo "" >> CHANGELOG.md
          echo "$COMMITS" >> CHANGELOG.md
          echo "" >> CHANGELOG.md
          echo "## Checksums" >> CHANGELOG.md
          echo "" >> CHANGELOG.md
          echo '```' >> CHANGELOG.md
          cat artifacts/*/anidb2folder-*.sha256 >> CHANGELOG.md
          echo '```' >> CHANGELOG.md
      
      - name: Create release
        uses: softprops/action-gh-release@v1
        with:
          body_path: CHANGELOG.md
          files: |
            artifacts/*/anidb2folder-*.tar.gz
            artifacts/*/anidb2folder-*.tar.gz.sha256
          draft: false
          prerelease: ${{ contains(github.ref, '-rc') || contains(github.ref, '-beta') || contains(github.ref, '-alpha') }}
        env:
          GITHUB_TOKEN: ${{ secrets.GITHUB_TOKEN }}
```

### Step 3: Cargo.toml Metadata

```toml
[package]
name = "anidb2folder"
version = "0.1.0"
edition = "2021"
authors = ["Your Name <email@example.com>"]
description = "CLI tool for renaming anime directories between AniDB ID and human-readable formats"
license = "MIT"
repository = "https://github.com/username/anidb2folder"
readme = "README.md"
keywords = ["anime", "anidb", "rename", "cli"]
categories = ["command-line-utilities", "filesystem"]

[package.metadata.release]
# Configuration for cargo-release if used
pre-release-commit-message = "chore: release {{version}}"
tag-message = "{{version}}"
tag-prefix = "v"
```

### Step 4: Release Process Documentation

Create a `RELEASING.md` file:

```markdown
# Release Process

## Versioning

This project follows [Semantic Versioning](https://semver.org/):

- **MAJOR** (x.0.0): Breaking changes to CLI interface or file formats
- **MINOR** (0.x.0): New features, backward-compatible
- **PATCH** (0.0.x): Bug fixes, backward-compatible

## Creating a Release

1. **Update version in Cargo.toml**

   ```bash
   # Edit Cargo.toml and update version
   vim Cargo.toml
   ```

2. **Update CHANGELOG.md** (optional but recommended)

   ```bash
   # Add release notes
   vim CHANGELOG.md
   ```

3. **Commit version bump**

   ```bash
   git add Cargo.toml Cargo.lock CHANGELOG.md
   git commit -m "chore: release v0.2.0"
   ```

4. **Create and push tag**

   ```bash
   git tag v0.2.0
   git push origin main
   git push origin v0.2.0
   ```

5. **Wait for GitHub Actions**
   
   The release workflow will automatically:
   - Build binaries for all platforms
   - Generate changelog from commits
   - Create GitHub release with artifacts

## Pre-releases

For pre-releases (alpha, beta, release candidates):

```bash
git tag v0.2.0-beta.1
git push origin v0.2.0-beta.1
```

These will be marked as pre-releases on GitHub.
```

### Step 5: Badge for README

Add to README.md:

```markdown
[![CI](https://github.com/username/anidb2folder/actions/workflows/ci.yml/badge.svg)](https://github.com/username/anidb2folder/actions/workflows/ci.yml)
[![Release](https://img.shields.io/github/v/release/username/anidb2folder)](https://github.com/username/anidb2folder/releases/latest)
```

## Test Cases

### Workflow Tests

1. **CI workflow triggers correctly**
   - Push to main triggers CI
   - Pull request triggers CI
   - Tests and linting run successfully

2. **Release workflow triggers correctly**
   - Pushing `v*.*.*` tag triggers release
   - All platform builds succeed
   - Release is created with correct artifacts

3. **Artifact integrity**
   - Checksums match downloaded files
   - Binaries are executable
   - Static builds work on target platforms

## Directory Structure

```
.github/
├── workflows/
│   ├── ci.yml
│   └── release.yml
├── ISSUE_TEMPLATE/
│   ├── bug_report.md
│   └── feature_request.md
└── PULL_REQUEST_TEMPLATE.md
```

## Notes

- The musl build creates a fully static binary for maximum Linux compatibility
- GitHub Actions provides free macOS runners for open source projects
- Consider adding Windows builds if there's demand
- Use `cargo-release` locally for streamlined release process
- Artifacts are retained for 90 days by default
- Consider adding code coverage reporting (e.g., codecov.io)
