# Anidb2folder - High-Level Overview

A CLI utility tool for renaming anime directories between AniDB ID format and human-readable format.

---

## Table of Contents

1. [Introduction](#introduction)
2. [Features](#features)
3. [Requirements](#requirements)
4. [Installation](#installation)
5. [Usage](#usage)
6. [Directory Naming Formats](#directory-naming-formats)
7. [Data Source & Caching](#data-source--caching)
8. [History & Revert System](#history--revert-system)
9. [Configuration](#configuration)
10. [Error Handling](#error-handling)
11. [Testing](#testing)
12. [Build & Release](#build--release)
13. [Roadmap](#roadmap)
14. [Contributing](#contributing)
15. [License](#license)

---

## Introduction

### Purpose

Anidb2folder is a small CLI utility that renames subdirectories within a given directory path. It toggles between two naming conventions:

- **AniDB ID format** — compact identifiers for internal organization
- **Human-readable format** — descriptive names with anime titles, release years, and metadata

### Scope

- Operates **only** on immediate subdirectories of the specified path
- Does **not** modify files or nested directories within those subdirectories
- Runs on Linux (and macOS) systems
- Builds locally and via GitHub Actions

### Technology Stack

- **Language:** Rust *(recommended for performance, safety, and cross-platform support)*
- **Build System:** Cargo
- **CI/CD:** GitHub Actions

---

## Features

| Feature | Description |
|---------|-------------|
| Bidirectional renaming | Toggle between AniDB ID and human-readable formats |
| Dry run mode | Preview changes without modifying the filesystem |
| Verbose mode | Detailed logging of all operations |
| History tracking | JSON-based audit trail of all changes |
| Revert functionality | Restore directories to previous state using history files |
| Local caching | Minimize API calls with configurable cache expiration |
| Rate limiting | Graceful handling of AniDB API limits with exponential backoff |
| Filesystem safety | Unicode replacements for invalid characters, name truncation |

---

## Requirements

### System Requirements

- Linux or macOS operating system
- Filesystem supporting Unicode directory names
- Internet connection for AniDB API access (initial fetch only)

### Dependencies

- Rust toolchain (for building from source)
- Network access to AniDB public API

---

## Installation

### From GitHub Releases

```bash
# Download the latest release binary for your platform
curl -LO https://github.com/<owner>/anidb2folder/releases/latest/download/anidb2folder-<platform>
chmod +x anidb2folder-<platform>
mv anidb2folder-<platform> /usr/local/bin/anidb2folder
```

### Building from Source

```bash
git clone https://github.com/<owner>/anidb2folder.git
cd anidb2folder
cargo build --release
./target/release/anidb2folder --help
```

---

## Usage

### Basic Commands

```bash
# Rename directories (toggles between formats)
anidb2folder /path/to/directory/with/anidb/folders/

# Dry run - preview changes without modifying filesystem
anidb2folder --dry /path/to/directory/with/anidb/folders/

# Verbose mode - detailed logging (use -v, -vv, or -vvv for more detail)
anidb2folder -v /path/to/directory/with/anidb/folders/

# Revert changes using a history file
anidb2folder --revert /path/to/directory/<anidb2folder-history-YYYYMMDD-HHMMSS.json>
```

### Command-Line Options

| Option | Short | Description |
|--------|-------|-------------|
| `--dry` | `-d` | Simulate changes without modifying filesystem |
| `--verbose` | `-v` | Increase verbosity (-v info, -vv debug, -vvv trace) |
| `--revert <file>` | `-r` | Revert changes using specified history file |
| `--max-length <n>` | `-l` | Maximum directory name length (default: 255) |
| `--cache-expiry <days>` | `-c` | Cache expiration in days (default: 30) |
| `--help` | `-h` | Display help information |
| `--version` | `-V` | Display version information |

---

## Directory Naming Formats

### AniDB ID Format

```
[<series>] <anidb_id>
```

| Component | Required | Description |
|-----------|----------|-------------|
| `series` | No | Optional tag in square brackets (any characters except `]`) |
| `anidb_id` | Yes | Numeric AniDB identifier |

**Examples:**
- `12345`
- `67890`
- `[series] 12345`
- `[My Series] 54321`
- `[AS0] 98765`

### Human-Readable Format

```
[<series>] <anime_title_jp> ／ <anime_title_en> (<release_year>) [anidb-<anidb_id>]
```

| Component | Required | Description |
|-----------|----------|-------------|
| `series` | No | Preserved from AniDB format if present |
| `anime_title_jp` | Yes | Japanese title in romaji (fetched from AniDB) |
| `／` | Conditional | Unicode slash separator (only if English title differs) |
| `anime_title_en` | No | English title (omitted if same as Japanese or unavailable) |
| `release_year` | No | Year in parentheses (omitted if unavailable) |
| `anidb_id` | Yes | Original AniDB identifier |

**Examples:**
- `Naruto (2002) [anidb-12345]`
- `[One Piece] One Piece (1999) [anidb-67890]`
- `[FMA] Fullmetal Alchemist (2003) [anidb-54321]`
- `[AS0] Cowboyu Bebopu ／ Cowboy Bebop (1998) [anidb-98765]`

### Character Replacement Rules

Invalid filesystem characters are replaced with Unicode equivalents:

| Invalid | Replacement | Unicode Name |
|---------|-------------|--------------|
| `/` | `／` | Fullwidth Solidus |
| `\` | `＼` | Fullwidth Reverse Solidus |
| `:` | `：` | Fullwidth Colon |
| `*` | `＊` | Fullwidth Asterisk |
| `?` | `？` | Fullwidth Question Mark |
| `"` | `＂` | Fullwidth Quotation Mark |
| `<` | `＜` | Fullwidth Less-Than Sign |
| `>` | `＞` | Fullwidth Greater-Than Sign |
| `|` | `｜` | Fullwidth Vertical Line |

### Name Length Truncation

When directory names exceed the filesystem limit (default: 255 characters):

1. **Priority preservation** (never truncated):
   - Series tag `[series]`
   - Release year `(YYYY)`
   - AniDB identifier `[anidb-XXXXX]`

2. **Truncation order**:
   1. English title shortened first (with ellipsis `…`)
   2. Japanese title shortened if still needed

3. **Rules**:
   - Truncated names must not end with spaces or punctuation
   - All truncations are logged with warnings
   - Original and truncated names recorded in history file

---

## Data Source & Caching

### AniDB API Integration

- Fetches anime titles (Japanese/English) and release years
- Implements rate limiting with exponential backoff
- Graceful handling of API failures and timeouts

### Cache System

**Storage locations** (in order of preference):
1. Target directory: `.anidb2folder-cache.json`
2. User home directory: `~/.cache/anidb2folder/cache.json`

**Cache structure:**
```json
{
  "entries": {
    "<anidb_id>": {
      "anidb_id": 12345,
      "title_jp": "Romaji Title",
      "title_en": "English Title",
      "release_year": 2002,
      "fetched_at": "2026-01-15T10:30:00Z"
    }
  },
  "version": "1.0"
}
```

**Cache behavior:**
- Configurable expiration (default: 30 days)
- Corrupted cache is ignored and rebuilt automatically
- Cache is shared across executions in the same directory

---

## History & Revert System

### History File Format

**Filename:** `anidb2folder-history-YYYYMMDD-HHMMSS.json`

**Location:** Target directory where the tool is executed

**Structure:**
```json
{
  "version": "1.0",
  "executed_at": "2026-01-15T10:30:00Z",
  "operation": "rename",
  "direction": "anidb_to_readable",
  "target_directory": "/path/to/directory",
  "changes": [
    {
      "source": "[AS0] 98765",
      "destination": "[AS0] Cowboyu Bebopu ／ Cowboy Bebop (1998) [anidb-98765]",
      "reason": "Converted from AniDB format to human-readable format",
      "truncated": false
    }
  ]
}
```

### Revert Functionality

**Revert command:**
```bash
anidb2folder --revert /path/to/directory/anidb2folder-history-YYYYMMDD-HHMMSS.json
```

**Revert history filename:** `<original-filename>-revert-YYYYMMDD-HHMMSS.json`

**Guarantees:**
- Complete restoration to previous state
- Revert operations are also logged
- Supports chained reverts (revert of a revert)

---

## Configuration

### Environment Variables

| Variable | Description | Default |
|----------|-------------|---------|
| `ANIDB2FOLDER_CACHE_DIR` | Custom cache directory | `~/.cache/anidb2folder` |
| `ANIDB2FOLDER_MAX_LENGTH` | Max directory name length | `255` |
| `ANIDB2FOLDER_CACHE_EXPIRY` | Cache expiration in days | `30` |
| `ANIDB2FOLDER_LOG_LEVEL` | Logging verbosity | `info` |

### Configuration File (Optional)

Location: `~/.config/anidb2folder/config.toml`

```toml
[general]
max_length = 255
verbose = false

[cache]
expiry_days = 30
directory = "~/.cache/anidb2folder"

[api]
retry_attempts = 3
retry_delay_ms = 1000
```

---

## Error Handling

### Exit Codes

| Code | Description |
|------|-------------|
| `0` | Success |
| `1` | General error |
| `2` | Invalid arguments |
| `3` | Directory not found |
| `4` | Mixed format directories detected |
| `5` | Unrecognized directory format |
| `6` | API error (after retries exhausted) |
| `7` | Filesystem permission error |
| `8` | History file not found or corrupted |

### Format Validation

Before any renaming operation:
1. All subdirectories are scanned and validated
2. Each must match **either** AniDB format **or** human-readable format
3. **All** directories must be in the **same** format

**On validation failure:**
- Tool exits immediately with appropriate error code
- No changes are made to the filesystem
- Error message lists unrecognized or mixed-format directories

---

## Testing

### Test Categories

| Category | Description |
|----------|-------------|
| Unit tests | String parsing, renaming logic, format validation |
| Integration tests | Full renaming workflow, dry runs, actual operations |
| Edge case tests | Special characters, truncation, malformed IDs |
| API tests | Rate limiting, failures, retries |
| Cache tests | Expiration, corruption, rebuilding |
| Revert tests | Restoration accuracy, history file integrity |

### Test Infrastructure

- **Mocking:** API calls mocked for reliable, offline testing
- **Test data:** Included in repository (`tests/fixtures/`)
- **CI integration:** All tests run automatically on GitHub Actions
- **Coverage:** Minimum 80% code coverage required

### Running Tests

```bash
# Run all tests
cargo test

# Run with verbose output
cargo test -- --nocapture

# Run specific test category
cargo test unit::
cargo test integration::
```

---

## Build & Release

### GitHub Actions Workflow

**Triggers:**
- Push to `main` branch
- Pull request to `main` branch
- Version tag push (`v*.*.*`)

**Build matrix:**
- Linux x86_64
- Linux aarch64
- macOS x86_64
- macOS aarch64 (Apple Silicon)

### Release Process

1. Create version tag: `git tag v1.0.0`
2. Push tag: `git push origin v1.0.0`
3. GitHub Actions automatically:
   - Builds binaries for all platforms
   - Generates changelog from commit history
   - Creates GitHub Release with artifacts
   - Publishes release notes

### Versioning

Follows [Semantic Versioning](https://semver.org/):
- **MAJOR:** Breaking changes to CLI interface or file formats
- **MINOR:** New features, backward-compatible
- **PATCH:** Bug fixes, backward-compatible

---

## Roadmap

### Planned Features

- [ ] Windows support
- [ ] Configuration file support
- [ ] Interactive mode with confirmation prompts
- [ ] Support for additional anime databases (MyAnimeList, Kitsu)
- [ ] GUI wrapper application
- [ ] Batch processing from file list
- [ ] Custom naming format templates

### Known Limitations

- Currently Linux/macOS only
- Requires internet for initial AniDB fetch
- Single-threaded API requests (due to rate limiting)

---

## Contributing

### Development Setup

```bash
git clone https://github.com/<owner>/anidb2folder.git
cd anidb2folder
cargo build
cargo test
```

### Contribution Guidelines

1. Fork the repository
2. Create a feature branch (`git checkout -b feature/my-feature`)
3. Write tests for new functionality
4. Ensure all tests pass (`cargo test`)
5. Submit a pull request

### Code Style

- Follow Rust standard formatting (`cargo fmt`)
- Run linter before committing (`cargo clippy`)
- Document public APIs with doc comments

---

## License

*[Specify license here — MIT, Apache 2.0, GPL, etc.]*

---

## Appendix

### Glossary

| Term | Definition |
|------|------------|
| AniDB | Anime Database — community-driven anime information database |
| AniDB ID | Unique numeric identifier for an anime entry in AniDB |
| Romaji | Japanese text written in Latin alphabet |
| Dry run | Simulation mode that shows changes without executing them |

### Related Resources

- [AniDB Website](https://anidb.net/)
- [AniDB API Documentation](https://wiki.anidb.net/API)
- [Rust Programming Language](https://www.rust-lang.org/)
