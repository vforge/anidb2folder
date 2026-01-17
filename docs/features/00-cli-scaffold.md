# 00 - CLI Scaffold

## Summary

Set up the basic CLI application structure with argument parsing and entry point.

## Dependencies

None — this is the foundation feature.

## Description

This feature establishes the foundational CLI application structure for anidb2folder. It creates the basic Rust project with proper argument parsing, help text generation, and version information. The scaffold provides the entry point that all other features will build upon.

The CLI should support:

- A positional argument for the target directory path
- Optional flags for different operation modes (--dry, --verbose, --revert)
- Configuration options (--max-length, --cache-expiry)
- Standard --help and --version flags

## Requirements

### Functional Requirements

1. Accept a positional argument for the target directory path
2. Support the following command-line flags:
   - `--dry` / `-d` — Enable dry run mode
   - `--verbose` / `-v` — Enable verbose output
   - `--revert <file>` / `-r <file>` — Revert using history file
   - `--max-length <n>` / `-l <n>` — Set max directory name length (default: 255)
   - `--cache-expiry <days>` / `-c <days>` — Set cache expiration (default: 30)
   - `--help` / `-h` — Display help information
   - `--version` / `-V` — Display version information
3. Validate that the target path is provided (unless using --help or --version)
4. Display clear error messages for invalid arguments

### Non-Functional Requirements

1. Use `clap` crate for argument parsing (industry standard, derive macros)
2. Follow Rust best practices for project structure
3. Include proper Cargo.toml metadata
4. Set up logging infrastructure with `tracing` or `log` crate

## Implementation Guide

### Step 1: Initialize Rust Project

```bash
cargo new anidb2folder
cd anidb2folder
```

### Step 2: Add Dependencies to Cargo.toml

```toml
[package]
name = "anidb2folder"
version = "0.1.0"
edition = "2021"
description = "CLI tool for renaming anime directories between AniDB ID and human-readable formats"
authors = ["Your Name <email@example.com>"]
license = "MIT"
repository = "https://github.com/<owner>/anidb2folder"

[dependencies]
clap = { version = "4.4", features = ["derive"] }
anyhow = "1.0"
thiserror = "1.0"
tracing = "0.1"
tracing-subscriber = { version = "0.3", features = ["env-filter"] }
```

### Step 3: Create CLI Argument Structure

```rust
// src/cli.rs
use clap::Parser;
use std::path::PathBuf;

#[derive(Parser, Debug)]
#[command(name = "anidb2folder")]
#[command(author, version, about, long_about = None)]
#[command(about = "Rename anime directories between AniDB ID and human-readable formats")]
pub struct Args {
    /// Target directory containing anime subdirectories
    #[arg(required_unless_present = "revert")]
    pub target_dir: Option<PathBuf>,

    /// Simulate changes without modifying the filesystem
    #[arg(short, long)]
    pub dry: bool,

    /// Enable verbose output
    #[arg(short, long)]
    pub verbose: bool,

    /// Revert changes using a history file
    #[arg(short, long, value_name = "HISTORY_FILE")]
    pub revert: Option<PathBuf>,

    /// Maximum directory name length (default: 255)
    #[arg(short = 'l', long, default_value = "255")]
    pub max_length: usize,

    /// Cache expiration in days (default: 30)
    #[arg(short, long, default_value = "30")]
    pub cache_expiry: u32,
}
```

### Step 4: Create Main Entry Point

```rust
// src/main.rs
mod cli;

use anyhow::Result;
use clap::Parser;
use cli::Args;
use tracing::info;
use tracing_subscriber::EnvFilter;

fn main() -> Result<()> {
    // Parse CLI arguments
    let args = Args::parse();
    
    // Initialize logging
    let filter = if args.verbose {
        EnvFilter::new("debug")
    } else {
        EnvFilter::new("info")
    };
    
    tracing_subscriber::fmt()
        .with_env_filter(filter)
        .init();
    
    info!("anidb2folder starting...");
    
    // Route to appropriate operation
    if let Some(history_file) = &args.revert {
        info!("Revert mode: {:?}", history_file);
        // TODO: Implement revert (feature 42)
    } else if let Some(target_dir) = &args.target_dir {
        info!("Target directory: {:?}", target_dir);
        info!("Dry run: {}", args.dry);
        // TODO: Implement main operation (features 22, 23)
    }
    
    Ok(())
}
```

### Step 5: Project Structure

```
anidb2folder/
├── Cargo.toml
├── Cargo.lock
├── src/
│   ├── main.rs          # Entry point
│   ├── cli.rs           # CLI argument definitions
│   └── lib.rs           # Library root (for testing)
├── tests/
│   └── cli_tests.rs     # CLI integration tests
└── README.md
```

## Test Cases

### Unit Tests

1. **Argument parsing with all flags**
   - Verify all flags are correctly parsed
   - Verify default values are applied

2. **Help text generation**
   - Verify --help produces expected output
   - Verify all options are documented

3. **Version output**
   - Verify --version shows correct version

### Integration Tests

```rust
// tests/cli_tests.rs
use assert_cmd::Command;
use predicates::prelude::*;

#[test]
fn test_help_flag() {
    let mut cmd = Command::cargo_bin("anidb2folder").unwrap();
    cmd.arg("--help")
        .assert()
        .success()
        .stdout(predicate::str::contains("Rename anime directories"));
}

#[test]
fn test_version_flag() {
    let mut cmd = Command::cargo_bin("anidb2folder").unwrap();
    cmd.arg("--version")
        .assert()
        .success()
        .stdout(predicate::str::contains(env!("CARGO_PKG_VERSION")));
}

#[test]
fn test_missing_target_dir() {
    let mut cmd = Command::cargo_bin("anidb2folder").unwrap();
    cmd.assert()
        .failure()
        .stderr(predicate::str::contains("required"));
}

#[test]
fn test_dry_flag() {
    let mut cmd = Command::cargo_bin("anidb2folder").unwrap();
    cmd.args(["--dry", "/tmp/test"])
        .assert()
        .success();
}
```

## Notes

- The `clap` derive macros provide automatic help text and bash completion support
- Consider adding shell completion generation as a future enhancement
- The logging infrastructure should be consistent across all features
- Error handling uses `anyhow` for the application and `thiserror` for library code
