# anidb2folder

[![CI](https://github.com/vforge/anidb2folder/actions/workflows/ci.yml/badge.svg)](https://github.com/vforge/anidb2folder/actions/workflows/ci.yml)
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

### From source

```bash
cargo install --path .
```

### Build release binaries

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

## License

MIT License - see [LICENSE](LICENSE) for details.
