//! Progress output for user-facing status updates.
//!
//! This module provides progress output for internal functions (like rename_to_readable).
//! In verbose mode, output is suppressed since tracing handles everything.
//! In normal mode, output is shown with colors to give feedback during API calls etc.

use colored::Colorize;
use std::io::{self, IsTerminal, Write};

/// Progress reporter for user-facing output
pub struct Progress {
    writer: Box<dyn Write>,
    /// When true, all output is suppressed (verbose mode uses tracing instead)
    silent: bool,
    /// When true, output is colorized
    colors_enabled: bool,
}

/// Check if we should use colors in output
fn should_use_colors() -> bool {
    if std::env::var("NO_COLOR").is_ok() {
        return false;
    }
    if std::env::var("FORCE_COLOR").is_ok() {
        return true;
    }
    io::stderr().is_terminal()
}

impl Default for Progress {
    fn default() -> Self {
        Self::new()
    }
}

impl Progress {
    /// Create a new progress reporter writing to stderr
    pub fn new() -> Self {
        let colors_enabled = should_use_colors();
        Self {
            writer: Box::new(io::stderr()),
            silent: false,
            colors_enabled,
        }
    }

    /// Create a progress reporter that respects UI mode
    /// When verbose=true, output is suppressed (tracing handles it)
    pub fn new_with_ui(verbose: bool, colors_enabled: bool) -> Self {
        Self {
            writer: Box::new(io::stderr()),
            silent: verbose,
            colors_enabled,
        }
    }

    /// Create a progress reporter with a custom writer (for testing)
    #[cfg(test)]
    pub fn with_writer(writer: Box<dyn Write>) -> Self {
        Self {
            writer,
            silent: false,
            colors_enabled: false,
        }
    }

    /// Create a silent progress reporter (for testing or verbose mode)
    #[allow(dead_code)]
    pub fn silent() -> Self {
        Self {
            writer: Box::new(io::sink()),
            silent: true,
            colors_enabled: false,
        }
    }

    /// Report progress on a single rename
    pub fn rename_progress(&mut self, current: usize, total: usize, from: &str, to: &str) {
        if self.silent {
            return;
        }
        if self.colors_enabled {
            let counter = format!("[{}/{}]", current, total);
            let _ = writeln!(
                self.writer,
                "{} {} {} {}",
                counter.cyan(),
                from.dimmed(),
                "→".cyan(),
                to
            );
        } else {
            let _ = writeln!(self.writer, "[{}/{}] {} -> {}", current, total, from, to);
        }
    }

    /// Report fetching metadata from API
    pub fn fetch_start(&mut self, anidb_id: u32) {
        if self.silent {
            return;
        }
        if self.colors_enabled {
            let _ = write!(
                self.writer,
                "{}",
                format!("Fetching metadata for {}...", anidb_id).dimmed()
            );
        } else {
            let _ = write!(self.writer, "Fetching metadata for {}...", anidb_id);
        }
        let _ = self.writer.flush();
    }

    /// Report fetch complete (same line)
    pub fn fetch_complete(&mut self) {
        if self.silent {
            return;
        }
        if self.colors_enabled {
            let _ = writeln!(self.writer, " {}", "done".green());
        } else {
            let _ = writeln!(self.writer, " done");
        }
    }

    /// Report using cached data (silent - too noisy for normal output)
    pub fn using_cache(&mut self, _anidb_id: u32) {
        // Intentionally silent - cache usage is an implementation detail
        // that doesn't need to be shown to the user for every directory
    }

    /// Report that API would be called (dry run mode) - silent for cleaner output
    pub fn would_fetch(&mut self, _anidb_id: u32) {
        // Intentionally silent - too noisy for normal output
    }

    /// Report an error during operation (non-fatal)
    pub fn warn(&mut self, message: &str) {
        if self.silent {
            return;
        }
        if self.colors_enabled {
            let _ = writeln!(self.writer, "{} {}", "!".yellow().bold(), message.yellow());
        } else {
            let _ = writeln!(self.writer, "Warning: {}", message);
        }
    }

    /// Report history file written
    pub fn history_written(&mut self, path: &std::path::Path) {
        if self.silent {
            return;
        }
        if self.colors_enabled {
            let _ = writeln!(
                self.writer,
                "{}",
                format!("History saved to: {}", path.display()).dimmed()
            );
        } else {
            let _ = writeln!(self.writer, "History saved to: {}", path.display());
        }
    }

    /// Report starting a revert operation
    pub fn revert_start(&mut self, total: usize, from_timestamp: &str) {
        if self.silent {
            return;
        }
        let _ = writeln!(self.writer);
        if self.colors_enabled {
            let _ = writeln!(
                self.writer,
                "{}",
                format!(
                    "Reverting {} directories from history ({})",
                    total, from_timestamp
                )
                .bold()
            );
        } else {
            let _ = writeln!(
                self.writer,
                "Reverting {} directories from history ({})",
                total, from_timestamp
            );
        }
    }

    /// Report progress on a single revert
    pub fn revert_progress(&mut self, current: usize, total: usize, from: &str, to: &str) {
        if self.silent {
            return;
        }
        if self.colors_enabled {
            let counter = format!("[{}/{}]", current, total);
            let _ = writeln!(
                self.writer,
                "{} {} {} {}",
                counter.cyan(),
                from.dimmed(),
                "→".cyan(),
                to
            );
        } else {
            let _ = writeln!(self.writer, "[{}/{}] {} -> {}", current, total, from, to);
        }
    }

    /// Report revert complete
    pub fn revert_complete(&mut self, count: usize, dry_run: bool) {
        if self.silent {
            return;
        }
        let _ = writeln!(self.writer);
        if dry_run {
            if self.colors_enabled {
                let _ = writeln!(
                    self.writer,
                    "{}",
                    format!("Dry run complete. {} directories would be reverted.", count).dimmed()
                );
            } else {
                let _ = writeln!(
                    self.writer,
                    "Dry run complete. {} directories would be reverted.",
                    count
                );
            }
        } else if self.colors_enabled {
            let _ = writeln!(
                self.writer,
                "{} {}",
                "✓".green().bold(),
                format!("{} directories restored", count).green()
            );
        } else {
            let _ = writeln!(
                self.writer,
                "Revert complete. {} directories restored.",
                count
            );
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn create_test_progress() -> (Progress, std::sync::Arc<std::sync::Mutex<Vec<u8>>>) {
        let buffer = std::sync::Arc::new(std::sync::Mutex::new(Vec::new()));
        let writer = TestWriter(buffer.clone());
        let progress = Progress::with_writer(Box::new(writer));
        (progress, buffer)
    }

    struct TestWriter(std::sync::Arc<std::sync::Mutex<Vec<u8>>>);

    impl Write for TestWriter {
        fn write(&mut self, buf: &[u8]) -> io::Result<usize> {
            self.0.lock().unwrap().write(buf)
        }

        fn flush(&mut self) -> io::Result<()> {
            Ok(())
        }
    }

    #[test]
    fn test_rename_progress() {
        let (mut progress, buffer) = create_test_progress();

        progress.rename_progress(1, 3, "12345", "Anime Title [anidb-12345]");
        progress.rename_progress(2, 3, "67890", "Another Anime [anidb-67890]");

        let output = String::from_utf8(buffer.lock().unwrap().clone()).unwrap();
        assert!(output.contains("[1/3]"));
        assert!(output.contains("[2/3]"));
    }

    #[test]
    fn test_fetch_output() {
        let (mut progress, buffer) = create_test_progress();

        progress.fetch_start(12345);
        progress.fetch_complete();

        let output = String::from_utf8(buffer.lock().unwrap().clone()).unwrap();
        assert!(output.contains("Fetching metadata for 12345"));
        assert!(output.contains("done"));
    }

    #[test]
    fn test_cache_output_is_silent() {
        let (mut progress, buffer) = create_test_progress();

        progress.using_cache(12345);

        // Cache messages are now silent to reduce noise
        let output = String::from_utf8(buffer.lock().unwrap().clone()).unwrap();
        assert!(output.is_empty());
    }
}
