# anidb2folder

[![CI](https://github.com/vforge/anidb2folder/actions/workflows/ci.yml/badge.svg)](https://github.com/vforge/anidb2folder/actions/workflows/ci.yml)
[![Release](https://img.shields.io/github/v/release/vforge/anidb2folder)](https://github.com/vforge/anidb2folder/releases/latest)
[![License: MIT](https://img.shields.io/badge/License-MIT-blue.svg)](LICENSE)

CLI tool for renaming anime directories between AniDB ID and human-readable formats.

## Formats

**AniDB format** (input):
```
[series] 12345
12345
```

**Human-readable format** (output):
```
[series] Title ／ English Title (2024) [anidb-12345]
Title (2024) [anidb-12345]
```

## Installation

### Download binary

Download the latest release for your platform from [Releases](https://github.com/vforge/anidb2folder/releases/latest):

| Platform | Binary |
|----------|--------|
| Linux x64 | `anidb2folder-linux-x64` |
| Linux ARM64 | `anidb2folder-linux-arm64` |
| macOS Intel | `anidb2folder-macos-x64` |
| macOS Apple Silicon | `anidb2folder-macos-arm64` |
| Windows x64 | `anidb2folder-windows-x64.exe` |

```bash
# Example: macOS Apple Silicon
curl -LO https://github.com/vforge/anidb2folder/releases/latest/download/anidb2folder-macos-arm64
chmod +x anidb2folder-macos-arm64
mv anidb2folder-macos-arm64 /usr/local/bin/anidb2folder
```

### From source

```bash
cargo install --path .
```

### Build locally

```bash
# Requires: zig, cargo-zigbuild
./run.sh release
```

Binaries are output to `dist/` for Linux (x64, ARM64) and macOS (x64, ARM64).

## Configuration

Register an API client at [AniDB](https://anidb.net/perl-bin/animedb.pl?show=client), then create a `.env` file:

```bash
cp .env.example .env
# Edit .env with your client credentials
```

## Usage

```bash
# Preview changes (dry run)
anidb2folder --dry /path/to/anime

# Rename directories
anidb2folder /path/to/anime

# Revert using history file
anidb2folder --revert .anidb2folder-20240115-143052.history.json /path/to/anime

# Verbose output
anidb2folder -v /path/to/anime    # Info
anidb2folder -vv /path/to/anime   # Debug
anidb2folder -vvv /path/to/anime  # Trace
```

### Options

| Flag | Description |
|------|-------------|
| `-d, --dry` | Simulate changes without modifying filesystem |
| `-v, --verbose` | Increase verbosity (repeat for more) |
| `-r, --revert <FILE>` | Revert changes using history file |
| `-l, --max-length <N>` | Maximum directory name length (default: 255) |
| `-c, --cache-expiry <DAYS>` | Cache expiration in days (default: 30) |
| `--cache-info <DIR>` | Show cache information |
| `--cache-clear <DIR>` | Clear cached entries |
| `--cache-prune <DIR>` | Remove expired cache entries |

## Development

```bash
./run.sh build    # Build release binary
./run.sh test     # Run tests
./run.sh check    # Run fmt check, clippy, and tests
./run.sh fmt      # Format code
./run.sh run      # Run with arguments
./run.sh release  # Build cross-platform binaries
./run.sh publish  # Bump version and publish release
```

### Test data

Create sample directories for manual testing:

```bash
./scripts/setup-test-data.sh
./run.sh run --dry ./test-data
```

### Git hooks

Enable pre-commit checks (fmt, clippy, tests):

```bash
git config core.hooksPath .githooks
```

### Publishing a release

Releases follow [semantic versioning](https://semver.org/):

```bash
./run.sh publish patch  # Bug fixes (1.2.3 -> 1.2.4)
./run.sh publish minor  # New features (1.2.3 -> 1.3.0)
./run.sh publish major  # Breaking changes (1.2.3 -> 2.0.0)
./run.sh publish --dry  # Preview without changes
```

This bumps the version in `Cargo.toml`, commits, tags, and pushes to GitHub. GitHub Actions then builds binaries for all platforms and creates the release with an auto-generated changelog.

## License

MIT License - see [LICENSE](LICENSE) for details.
