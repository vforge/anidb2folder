# 50 - UI/UX Cleanup

## Summary

Redesign the command-line output for a cleaner, more polished user experience with distinct verbose and normal modes.

## Dependencies

- **02-verbose-mode** — Current verbose implementation to be refined
- **All rename/revert features** — UI applies to all operations

## Description

This feature overhauls the CLI output to provide two distinct experiences:

1. **Normal mode (default):** Clean, colorful progress output with visual polish
2. **Verbose mode (`-v`):** Technical tracing output only, no decorative elements

Both modes should display a branded header on startup.

## Requirements

### Functional Requirements

#### 1. Branded Header (Both Modes)

Display an ASCII art header on startup:

```
   ___       _ ___  ___ ___  __      _    _
  / _ | ___ (_) _ \/ _ ) _ )/_/ ____| |__| |___ _ _
 / __ |/ _ \| / // / _  / _  / /___| / _| / / -_) '_|
/_/ |_/_//_/_/____/____/____/_/    |_\__|_\_\___|_|
                                            v0.1.0
```

Or simpler alternative:
```
╔═══════════════════════════════════════╗
║         anidb2folder v0.1.0           ║
╚═══════════════════════════════════════╝
```

#### 2. Normal Mode (Non-Verbose)

- **Colors** (when terminal supports it):
  - Green: Success messages, checkmarks
  - Yellow: Warnings
  - Red: Errors
  - Cyan: Progress indicators, info
  - Bold: Headers, important info

- **Progress indicators:**
  - Spinner or progress bar for long operations
  - `[1/10]` style counters
  - Clear step indicators

- **Clean output structure:**
  ```
  Scanning /path/to/anime...
  Found 50 directories

  Format: AniDB

  Renaming [████████████████████] 50/50

  ✓ Complete. 50 directories renamed.
  History saved to: anidb2folder-history-20260119-123456.json
  ```

- **No tracing/debug messages** — only user-facing progress

#### 3. Verbose Mode (`-v`)

- **Tracing output only:**
  - DEBUG, INFO, WARN, ERROR levels
  - Timestamps
  - Technical details (API calls, cache hits, etc.)

- **No decorative elements:**
  - No colors (or minimal)
  - No ASCII art header (or minimal text header)
  - No progress bars/spinners
  - Just structured log output

- **Example:**
  ```
  2026-01-19T12:00:00Z INFO  Loading environment
  2026-01-19T12:00:00Z DEBUG API config: client=myapp, version=1
  2026-01-19T12:00:00Z INFO  Scanning directory: /path/to/anime
  2026-01-19T12:00:00Z DEBUG Found entry: 12345
  2026-01-19T12:00:00Z INFO  Fetching metadata for anidb-12345
  2026-01-19T12:00:01Z DEBUG API response: 200 OK
  ...
  ```

#### 4. Terminal Detection

- Detect if stdout/stderr supports colors (isatty + TERM check)
- Respect `NO_COLOR` environment variable
- Respect `FORCE_COLOR` environment variable
- Fall back gracefully to plain text

#### 5. Dry Run Output

Both modes should clearly indicate dry run:

**Normal mode:**
```
╔═══════════════════════════════════════╗
║              DRY RUN                  ║
╚═══════════════════════════════════════╝

Would rename 50 directories:
  1. 12345 → Anime Title (2020) [anidb-12345]
  ...

No changes made. Run without --dry to apply.
```

**Verbose mode:**
```
2026-01-19T12:00:00Z INFO  DRY RUN MODE - no changes will be made
2026-01-19T12:00:00Z INFO  Would rename: 12345 -> Anime Title (2020) [anidb-12345]
...
```

### Non-Functional Requirements

1. Use `termcolor` or `colored` crate for cross-platform color support
2. Use `indicatif` crate for progress bars (optional)
3. Minimal performance overhead
4. Consistent styling across all commands

## Implementation Guide

### Step 1: Add Dependencies

```toml
# Cargo.toml
[dependencies]
termcolor = "1.4"       # Cross-platform terminal colors
atty = "0.2"            # Terminal detection
# OR use colored = "2.0" for simpler API
```

### Step 2: Create UI Module

```rust
// src/ui/mod.rs
mod colors;
mod header;
mod progress;

pub use colors::*;
pub use header::*;
pub use progress::*;

pub struct UiConfig {
    pub colors_enabled: bool,
    pub verbose: bool,
}

impl UiConfig {
    pub fn from_env(verbose: bool) -> Self {
        let colors_enabled = should_use_colors();
        Self { colors_enabled, verbose }
    }
}

fn should_use_colors() -> bool {
    // Check NO_COLOR env
    if std::env::var("NO_COLOR").is_ok() {
        return false;
    }

    // Check FORCE_COLOR env
    if std::env::var("FORCE_COLOR").is_ok() {
        return true;
    }

    // Check if terminal
    atty::is(atty::Stream::Stdout)
}
```

### Step 3: Refactor Progress Module

```rust
// src/ui/progress.rs
use termcolor::{Color, ColorChoice, ColorSpec, StandardStream, WriteColor};

pub struct ProgressOutput {
    stream: StandardStream,
    colors_enabled: bool,
}

impl ProgressOutput {
    pub fn new(colors_enabled: bool) -> Self {
        let choice = if colors_enabled {
            ColorChoice::Auto
        } else {
            ColorChoice::Never
        };

        Self {
            stream: StandardStream::stderr(choice),
            colors_enabled,
        }
    }

    pub fn success(&mut self, msg: &str) {
        if self.colors_enabled {
            self.stream.set_color(ColorSpec::new().set_fg(Some(Color::Green))).ok();
        }
        write!(self.stream, "✓ ").ok();
        self.stream.reset().ok();
        writeln!(self.stream, "{}", msg).ok();
    }

    pub fn warning(&mut self, msg: &str) {
        if self.colors_enabled {
            self.stream.set_color(ColorSpec::new().set_fg(Some(Color::Yellow))).ok();
        }
        write!(self.stream, "⚠ ").ok();
        self.stream.reset().ok();
        writeln!(self.stream, "{}", msg).ok();
    }

    // ... more methods
}
```

### Step 4: Create Header Module

```rust
// src/ui/header.rs

const ASCII_HEADER: &str = r#"
   ___       _ ___  ___ ___  __      _    _
  / _ | ___ (_) _ \/ _ ) _ )/_/ ____| |__| |___ _ _
 / __ |/ _ \| / // / _  / _  / /___| / _| / / -_) '_|
/_/ |_/_//_/_/____/____/____/_/    |_\__|_\_\___|_|
"#;

pub fn print_header(version: &str, colors_enabled: bool) {
    if colors_enabled {
        // Print with colors
    } else {
        println!("{}", ASCII_HEADER);
        println!("{:>50}", format!("v{}", version));
    }
    println!();
}

pub fn print_simple_header(version: &str) {
    println!("anidb2folder v{}", version);
    println!();
}
```

### Step 5: Update Main

```rust
// src/main.rs
use ui::{UiConfig, print_header, ProgressOutput};

fn main() {
    let args = Args::parse();

    let ui_config = UiConfig::from_env(args.verbose);

    // Show header (skip in verbose mode or show minimal)
    if !args.verbose {
        print_header(env!("CARGO_PKG_VERSION"), ui_config.colors_enabled);
    }

    // Initialize logging OR progress based on mode
    if args.verbose {
        logging::init(true);  // Tracing output
    } else {
        // Use ProgressOutput for user-facing messages
    }

    // ... rest of app
}
```

## Test Cases

### Manual Testing

1. **Normal mode colors:**
   ```bash
   anidb2folder /path/to/anime --dry
   # Should show colored output with progress
   ```

2. **Verbose mode:**
   ```bash
   anidb2folder /path/to/anime --dry -v
   # Should show only tracing output
   ```

3. **No color mode:**
   ```bash
   NO_COLOR=1 anidb2folder /path/to/anime --dry
   # Should show plain text without colors
   ```

4. **Piped output:**
   ```bash
   anidb2folder /path/to/anime --dry | cat
   # Should detect non-TTY and disable colors
   ```

### Unit Tests

```rust
#[test]
fn test_color_detection_no_color_env() {
    std::env::set_var("NO_COLOR", "1");
    assert!(!should_use_colors());
}

#[test]
fn test_color_detection_force_color() {
    std::env::set_var("FORCE_COLOR", "1");
    assert!(should_use_colors());
}
```

## Notes

- Consider `indicatif` crate for progress bars if spinner/bar is desired
- ASCII art should fit within 80-column terminals
- Test on Windows, macOS, and Linux terminals
- Consider `console` crate as alternative to `termcolor`
- May want `--quiet` flag in future to suppress all output except errors
