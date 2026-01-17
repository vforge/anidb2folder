# 40 - Dry Run Mode

## Summary

Preview rename operations without making any changes to the filesystem.

## Dependencies

- **20-rename-to-readable** — Uses the same preparation logic
- **21-rename-to-anidb** — Uses the same preparation logic

## Description

This feature implements the dry run mode that allows users to preview what changes would be made before executing them. The dry run simulates the entire rename process, including API calls and name generation, but skips the actual filesystem rename operations.

Dry run mode is essential for:

- Verifying the tool will work correctly before execution
- Reviewing name transformations before committing
- Debugging issues without risking data
- Building user confidence in the tool

## Requirements

### Functional Requirements

1. Enable with `--dry` or `-d` command-line flag
2. Execute all preparation steps (parsing, validation, API fetching)
3. Skip only the filesystem rename operations
4. Display planned changes clearly to stdout
5. Indicate what would be truncated or modified
6. Exit with success if dry run would succeed

### Non-Functional Requirements

1. Output should be clear and human-readable
2. Support piping output for scripting
3. Same exit codes as actual execution would produce

## Implementation Guide

### Step 1: Output Formatting

```rust
// src/output/mod.rs
use crate::rename::{RenameDirection, RenameOperation, RenameResult};
use std::io::{self, Write};

/// Format and display dry run results
pub fn display_dry_run_results(result: &RenameResult, writer: &mut impl Write) -> io::Result<()> {
    let direction_str = match result.direction {
        RenameDirection::AniDbToReadable => "AniDB → Human-readable",
        RenameDirection::ReadableToAniDb => "Human-readable → AniDB",
    };
    
    writeln!(writer, "\n╔══════════════════════════════════════════════════════════════╗")?;
    writeln!(writer, "║                         DRY RUN                              ║")?;
    writeln!(writer, "╠══════════════════════════════════════════════════════════════╣")?;
    writeln!(writer, "║  Direction: {:<47} ║", direction_str)?;
    writeln!(writer, "║  Operations: {:<46} ║", result.operations.len())?;
    writeln!(writer, "╚══════════════════════════════════════════════════════════════╝")?;
    writeln!(writer)?;
    
    if result.operations.is_empty() {
        writeln!(writer, "No directories to rename.")?;
        return Ok(());
    }
    
    writeln!(writer, "Planned changes:\n")?;
    
    for (i, op) in result.operations.iter().enumerate() {
        display_operation(i + 1, op, writer)?;
    }
    
    // Summary
    let truncated_count = result.operations.iter().filter(|op| op.truncated).count();
    
    writeln!(writer)?;
    writeln!(writer, "─────────────────────────────────────────────────────────────────")?;
    writeln!(writer, "Summary:")?;
    writeln!(writer, "  Total: {} directories would be renamed", result.operations.len())?;
    
    if truncated_count > 0 {
        writeln!(writer, "  ⚠ {} names would be truncated", truncated_count)?;
    }
    
    writeln!(writer)?;
    writeln!(writer, "Run without --dry to apply these changes.")?;
    
    Ok(())
}

fn display_operation(index: usize, op: &RenameOperation, writer: &mut impl Write) -> io::Result<()> {
    writeln!(writer, "  {}. [anidb-{}]", index, op.anidb_id)?;
    writeln!(writer, "     From: {}", op.source_name)?;
    writeln!(writer, "     To:   {}", op.destination_name)?;
    
    if op.truncated {
        writeln!(writer, "     ⚠ Name will be truncated")?;
    }
    
    writeln!(writer)?;
    
    Ok(())
}

/// Display simple machine-readable output for scripting
pub fn display_dry_run_simple(result: &RenameResult, writer: &mut impl Write) -> io::Result<()> {
    for op in &result.operations {
        writeln!(
            writer,
            "{}\t{}\t{}",
            op.anidb_id,
            op.source_name,
            op.destination_name
        )?;
    }
    Ok(())
}
```

### Step 2: Update Main for Dry Run

```rust
// src/main.rs (dry run integration)
use crate::output::{display_dry_run_results, display_dry_run_simple};

fn main() -> Result<()> {
    let args = Args::parse();
    // ... setup ...
    
    let validation = validate_directories(&entries)?;
    
    let result = match validation.format {
        DirectoryFormat::AniDb => {
            let options = RenameToReadableOptions {
                max_length: args.max_length,
                dry_run: args.dry,  // This controls execution
            };
            rename_to_readable(target_dir, &validation, &mut cached_client, &options)?
        }
        DirectoryFormat::HumanReadable => {
            let options = RenameToAniDbOptions {
                dry_run: args.dry,
            };
            rename_to_anidb(target_dir, &validation, &options)?
        }
    };
    
    // Display results
    if args.dry {
        display_dry_run_results(&result, &mut std::io::stdout())?;
    } else {
        // Normal output for actual execution
        info!("Successfully renamed {} directories", result.operations.len());
        
        // Write history file (feature 41)
        // write_history(&result, target_dir)?;
    }
    
    Ok(())
}
```

### Step 3: Enhanced Dry Run Output

```rust
// src/output/mod.rs (additional output formats)

/// Display with color support (for terminals)
pub fn display_dry_run_colored(result: &RenameResult, writer: &mut impl Write) -> io::Result<()> {
    use termcolor::{Color, ColorChoice, ColorSpec, StandardStream, WriteColor};
    
    let mut stdout = StandardStream::stdout(ColorChoice::Auto);
    
    // Header
    stdout.set_color(ColorSpec::new().set_fg(Some(Color::Cyan)).set_bold(true))?;
    writeln!(stdout, "\n═══════════════════════════════════════════════════")?;
    writeln!(stdout, "                     DRY RUN")?;
    writeln!(stdout, "═══════════════════════════════════════════════════")?;
    stdout.reset()?;
    
    let direction_str = match result.direction {
        RenameDirection::AniDbToReadable => "AniDB → Human-readable",
        RenameDirection::ReadableToAniDb => "Human-readable → AniDB",
    };
    
    writeln!(stdout, "Direction: {}", direction_str)?;
    writeln!(stdout, "Operations: {}\n", result.operations.len())?;
    
    for (i, op) in result.operations.iter().enumerate() {
        writeln!(stdout, "{}. [anidb-{}]", i + 1, op.anidb_id)?;
        
        stdout.set_color(ColorSpec::new().set_fg(Some(Color::Red)))?;
        writeln!(stdout, "   - {}", op.source_name)?;
        
        stdout.set_color(ColorSpec::new().set_fg(Some(Color::Green)))?;
        writeln!(stdout, "   + {}", op.destination_name)?;
        
        stdout.reset()?;
        
        if op.truncated {
            stdout.set_color(ColorSpec::new().set_fg(Some(Color::Yellow)))?;
            writeln!(stdout, "   ⚠ Truncated")?;
            stdout.reset()?;
        }
        
        writeln!(stdout)?;
    }
    
    Ok(())
}
```

## Test Cases

### Unit Tests

```rust
#[cfg(test)]
mod tests {
    use super::*;
    use std::path::PathBuf;
    
    fn create_test_result() -> RenameResult {
        RenameResult {
            direction: RenameDirection::AniDbToReadable,
            operations: vec![
                RenameOperation {
                    source_path: PathBuf::from("/test/12345"),
                    source_name: "12345".to_string(),
                    destination_path: PathBuf::from("/test/Anime Title (2020) [anidb-12345]"),
                    destination_name: "Anime Title (2020) [anidb-12345]".to_string(),
                    anidb_id: 12345,
                    reason: "Converted".to_string(),
                    truncated: false,
                },
                RenameOperation {
                    source_path: PathBuf::from("/test/[X] 99"),
                    source_name: "[X] 99".to_string(),
                    destination_path: PathBuf::from("/test/[X] Very Long... [anidb-99]"),
                    destination_name: "[X] Very Long... [anidb-99]".to_string(),
                    anidb_id: 99,
                    reason: "Converted".to_string(),
                    truncated: true,
                },
            ],
        }
    }
    
    #[test]
    fn test_display_dry_run() {
        let result = create_test_result();
        let mut output = Vec::new();
        
        display_dry_run_results(&result, &mut output).unwrap();
        
        let output_str = String::from_utf8(output).unwrap();
        
        assert!(output_str.contains("DRY RUN"));
        assert!(output_str.contains("12345"));
        assert!(output_str.contains("Anime Title (2020) [anidb-12345]"));
        assert!(output_str.contains("truncated"));
    }
    
    #[test]
    fn test_display_simple_format() {
        let result = create_test_result();
        let mut output = Vec::new();
        
        display_dry_run_simple(&result, &mut output).unwrap();
        
        let output_str = String::from_utf8(output).unwrap();
        let lines: Vec<&str> = output_str.lines().collect();
        
        assert_eq!(lines.len(), 2);
        assert!(lines[0].contains("12345"));
        assert!(lines[1].contains("99"));
    }
    
    #[test]
    fn test_display_empty_result() {
        let result = RenameResult {
            direction: RenameDirection::AniDbToReadable,
            operations: vec![],
        };
        
        let mut output = Vec::new();
        display_dry_run_results(&result, &mut output).unwrap();
        
        let output_str = String::from_utf8(output).unwrap();
        assert!(output_str.contains("No directories"));
    }
}
```

### Integration Tests

```rust
// tests/dry_run_tests.rs
use assert_cmd::Command;
use predicates::prelude::*;
use std::fs;
use tempfile::tempdir;

#[test]
fn test_dry_run_cli() {
    let dir = tempdir().unwrap();
    fs::create_dir(dir.path().join("12345")).unwrap();
    
    let mut cmd = Command::cargo_bin("anidb2folder").unwrap();
    cmd.arg("--dry")
        .arg(dir.path())
        .assert()
        .success()
        .stdout(predicate::str::contains("DRY RUN"));
}

#[test]
fn test_dry_run_no_changes() {
    let dir = tempdir().unwrap();
    let original_name = "12345";
    fs::create_dir(dir.path().join(original_name)).unwrap();
    
    let mut cmd = Command::cargo_bin("anidb2folder").unwrap();
    cmd.arg("--dry")
        .arg(dir.path())
        .assert()
        .success();
    
    // Verify directory unchanged
    assert!(dir.path().join(original_name).exists());
}

#[test]
fn test_dry_run_shows_truncation_warning() {
    // Test with mock data that would cause truncation
}
```

## Notes

- Dry run executes API calls to show accurate previews — consider adding a `--offline` mode
- The simple output format (tab-separated) is designed for piping to other tools
- Color output uses `termcolor` which respects `NO_COLOR` environment variable
- Dry run should produce the same exit codes that actual execution would
- Consider adding a `--json` flag for structured dry run output
