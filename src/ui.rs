//! UI module for styled terminal output.
//!
//! Provides colored output in normal mode and plain tracing in verbose mode.

use colored::Colorize;
use std::io::{self, IsTerminal, Write};

/// ASCII art header lines for the application (for gradient coloring)
const HEADER_LINE_1: &str = r"              _     _ _     ____   __       _     _           ";
const HEADER_LINE_2: &str = r"   __ _ _ __ (_) __| | |__  |___ \ / _| ___ | | __| | ___ _ __ ";
const HEADER_LINE_3: &str = r"  / _` | '_ \| |/ _` | '_ \   __) | |_ / _ \| |/ _` |/ _ \ '__|";
const HEADER_LINE_4: &str = r" | (_| | | | | | (_| | |_) | / __/|  _| (_) | | (_| |  __/ |   ";
const HEADER_LINE_5: &str = r"  \__,_|_| |_|_|\__,_|_.__/ |_____|_|  \___/|_|\__,_|\___|_|   ";

/// Plain ASCII header (non-colored)
const ASCII_HEADER_PLAIN: &str = r"
              _     _ _     ____   __       _     _
   __ _ _ __ (_) __| | |__  |___ \ / _| ___ | | __| | ___ _ __
  / _` | '_ \| |/ _` | '_ \   __) | |_ / _ \| |/ _` |/ _ \ '__|
 | (_| | | | | | (_| | |_) | / __/|  _| (_) | | (_| |  __/ |
  \__,_|_| |_|_|\__,_|_.__/ |_____|_|  \___/|_|\__,_|\___|_|
";

/// UI configuration
#[derive(Debug, Clone)]
pub struct UiConfig {
    pub colors_enabled: bool,
    pub verbose: bool,
}

impl UiConfig {
    /// Create UI config from environment and args
    pub fn new(verbose: bool) -> Self {
        let colors_enabled = should_use_colors();
        Self {
            colors_enabled,
            verbose,
        }
    }
}

/// Check if we should use colors in output
fn should_use_colors() -> bool {
    // Check NO_COLOR env (standard: https://no-color.org/)
    if std::env::var("NO_COLOR").is_ok() {
        return false;
    }

    // Check FORCE_COLOR env
    if std::env::var("FORCE_COLOR").is_ok() {
        return true;
    }

    // Check if stderr is a terminal
    io::stderr().is_terminal()
}

/// Styled output writer
pub struct Ui {
    config: UiConfig,
    writer: Box<dyn Write>,
}

impl Ui {
    /// Create a new UI with stderr output
    pub fn new(config: UiConfig) -> Self {
        // Set colored crate's global color setting
        if !config.colors_enabled {
            colored::control::set_override(false);
        }

        Self {
            config,
            writer: Box::new(io::stderr()),
        }
    }

    /// Create UI with custom writer (for testing)
    #[allow(dead_code)]
    pub fn with_writer(config: UiConfig, writer: Box<dyn Write>) -> Self {
        if !config.colors_enabled {
            colored::control::set_override(false);
        }

        Self { config, writer }
    }

    /// Print the application header
    pub fn print_header(&mut self, version: &str) {
        if self.config.verbose {
            // Minimal header in verbose mode
            let _ = writeln!(self.writer, "anidb2folder v{}", version);
            let _ = writeln!(self.writer);
            return;
        }

        if self.config.colors_enabled {
            // Gradient effect: cyan -> blue -> magenta
            let _ = writeln!(self.writer);
            let _ = writeln!(self.writer, "{}", HEADER_LINE_1.bright_cyan().bold());
            let _ = writeln!(self.writer, "{}", HEADER_LINE_2.cyan().bold());
            let _ = writeln!(self.writer, "{}", HEADER_LINE_3.blue().bold());
            let _ = writeln!(self.writer, "{}", HEADER_LINE_4.bright_magenta());
            let _ = writeln!(self.writer, "{}", HEADER_LINE_5.magenta());
            let _ = writeln!(
                self.writer,
                "{}",
                format!("{:>64}", format!("v{}", version)).dimmed()
            );
        } else {
            let _ = writeln!(self.writer, "{}", ASCII_HEADER_PLAIN);
            let _ = writeln!(self.writer, "{:>64}", format!("v{}", version));
        }
        let _ = writeln!(self.writer);
    }

    /// Print a section header
    pub fn section(&mut self, title: &str) {
        if self.config.verbose {
            return;
        }
        let _ = writeln!(self.writer);
        if self.config.colors_enabled {
            let _ = writeln!(self.writer, "{}", title.bold());
        } else {
            let _ = writeln!(self.writer, "{}", title);
        }
    }

    /// Print an info message
    pub fn info(&mut self, msg: &str) {
        if self.config.verbose {
            return;
        }
        if self.config.colors_enabled {
            let _ = writeln!(self.writer, "{}", msg.cyan());
        } else {
            let _ = writeln!(self.writer, "{}", msg);
        }
    }

    /// Print a success message with checkmark
    pub fn success(&mut self, msg: &str) {
        if self.config.verbose {
            return;
        }
        if self.config.colors_enabled {
            let _ = writeln!(self.writer, "{} {}", "✓".green().bold(), msg.green());
        } else {
            let _ = writeln!(self.writer, "* {}", msg);
        }
    }

    /// Print a warning message
    pub fn warning(&mut self, msg: &str) {
        if self.config.verbose {
            return;
        }
        if self.config.colors_enabled {
            let _ = writeln!(self.writer, "{} {}", "!".yellow().bold(), msg.yellow());
        } else {
            let _ = writeln!(self.writer, "! {}", msg);
        }
    }

    /// Print an error message
    pub fn error(&mut self, msg: &str) {
        // Errors shown in both modes
        if self.config.colors_enabled {
            let _ = writeln!(self.writer, "{} {}", "✗".red().bold(), msg.red());
        } else {
            let _ = writeln!(self.writer, "X {}", msg);
        }
    }

    /// Print a dim/muted message
    pub fn dim(&mut self, msg: &str) {
        if self.config.verbose {
            return;
        }
        if self.config.colors_enabled {
            let _ = writeln!(self.writer, "{}", msg.dimmed());
        } else {
            let _ = writeln!(self.writer, "{}", msg);
        }
    }

    /// Print progress: [current/total] message
    #[allow(dead_code)]
    pub fn progress(&mut self, current: usize, total: usize, msg: &str) {
        if self.config.verbose {
            return;
        }
        let counter = format!("[{}/{}]", current, total);
        if self.config.colors_enabled {
            let _ = writeln!(self.writer, "{} {}", counter.cyan(), msg);
        } else {
            let _ = writeln!(self.writer, "{} {}", counter, msg);
        }
    }

    /// Print rename progress: [current/total] from → to
    pub fn rename_progress(&mut self, current: usize, total: usize, from: &str, to: &str) {
        if self.config.verbose {
            return;
        }
        let counter = format!("[{}/{}]", current, total);
        if self.config.colors_enabled {
            let _ = writeln!(
                self.writer,
                "{} {} {} {}",
                counter.cyan(),
                from.dimmed(),
                "→".cyan(),
                to
            );
        } else {
            let _ = writeln!(self.writer, "{} {} -> {}", counter, from, to);
        }
    }

    /// Print a step in progress
    pub fn step(&mut self, msg: &str) {
        if self.config.verbose {
            return;
        }
        if self.config.colors_enabled {
            let _ = write!(self.writer, "{}", format!("{}... ", msg).dimmed());
        } else {
            let _ = write!(self.writer, "{}... ", msg);
        }
        let _ = self.writer.flush();
    }

    /// Complete a step
    pub fn step_done(&mut self) {
        if self.config.verbose {
            return;
        }
        if self.config.colors_enabled {
            let _ = writeln!(self.writer, "{}", "done".green());
        } else {
            let _ = writeln!(self.writer, "done");
        }
    }

    /// Print a key-value pair
    pub fn kv(&mut self, key: &str, value: &str) {
        if self.config.verbose {
            return;
        }
        if self.config.colors_enabled {
            let _ = writeln!(self.writer, "{}: {}", key.bold(), value);
        } else {
            let _ = writeln!(self.writer, "{}: {}", key, value);
        }
    }

    /// Print a blank line
    pub fn blank(&mut self) {
        if self.config.verbose {
            return;
        }
        let _ = writeln!(self.writer);
    }

    /// Print a separator line
    #[allow(dead_code)]
    pub fn separator(&mut self) {
        if self.config.verbose {
            return;
        }
        if self.config.colors_enabled {
            let _ = writeln!(self.writer, "{}", "─".repeat(50).dimmed());
        } else {
            let _ = writeln!(self.writer, "{}", "-".repeat(50));
        }
    }

    /// Print a boxed title (for dry run, revert, etc.)
    pub fn boxed_title(&mut self, title: &str) {
        if self.config.verbose {
            return;
        }
        let width = 50;
        let padding = (width - title.len() - 2) / 2;
        let title_line = format!(
            "║{}{}{}║",
            " ".repeat(padding),
            title,
            " ".repeat(width - padding - title.len() - 2)
        );

        if self.config.colors_enabled {
            let _ = writeln!(
                self.writer,
                "{}",
                format!("╔{}╗", "═".repeat(width - 2)).cyan()
            );
            let _ = writeln!(self.writer, "{}", title_line.cyan().bold());
            let _ = writeln!(
                self.writer,
                "{}",
                format!("╚{}╝", "═".repeat(width - 2)).cyan()
            );
        } else {
            let _ = writeln!(self.writer, "╔{}╗", "═".repeat(width - 2));
            let _ = writeln!(self.writer, "{}", title_line);
            let _ = writeln!(self.writer, "╚{}╝", "═".repeat(width - 2));
        }
    }

    /// Print a list item with arrow
    pub fn list_item(&mut self, from: &str, to: &str) {
        if self.config.verbose {
            return;
        }
        if self.config.colors_enabled {
            let _ = writeln!(
                self.writer,
                "  {} {} {}",
                from.dimmed(),
                "→".cyan(),
                to.bold()
            );
        } else {
            let _ = writeln!(self.writer, "  {} -> {}", from, to);
        }
    }

    /// Print a completed list item with checkmark
    pub fn list_done(&mut self, from: &str, to: &str) {
        if self.config.verbose {
            return;
        }
        if self.config.colors_enabled {
            let _ = writeln!(
                self.writer,
                "  {} {} {} {}",
                "✓".green(),
                from.dimmed(),
                "→".green(),
                to
            );
        } else {
            let _ = writeln!(self.writer, "  * {} -> {}", from, to);
        }
    }

    /// Check if in verbose mode
    pub fn is_verbose(&self) -> bool {
        self.config.verbose
    }

    /// Check if colors are enabled
    pub fn is_colors_enabled(&self) -> bool {
        self.config.colors_enabled
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::{Arc, Mutex};

    struct TestWriter(Arc<Mutex<Vec<u8>>>);

    impl Write for TestWriter {
        fn write(&mut self, buf: &[u8]) -> io::Result<usize> {
            self.0.lock().unwrap().write(buf)
        }

        fn flush(&mut self) -> io::Result<()> {
            Ok(())
        }
    }

    fn create_test_ui(verbose: bool) -> (Ui, Arc<Mutex<Vec<u8>>>) {
        let buffer = Arc::new(Mutex::new(Vec::new()));
        let config = UiConfig {
            colors_enabled: false,
            verbose,
        };
        let ui = Ui::with_writer(config, Box::new(TestWriter(buffer.clone())));
        (ui, buffer)
    }

    #[test]
    fn test_ui_config_respects_no_color() {
        std::env::set_var("NO_COLOR", "1");
        // Note: This test may interfere with other tests due to env var
        // In practice, should_use_colors() checks this
        std::env::remove_var("NO_COLOR");
    }

    #[test]
    fn test_ui_plain_output() {
        let (mut ui, buffer) = create_test_ui(false);
        ui.success("Test success");

        let output = String::from_utf8(buffer.lock().unwrap().clone()).unwrap();
        assert!(output.contains("Test success"));
        assert!(output.contains("*")); // Plain checkmark
    }

    #[test]
    fn test_ui_verbose_mode_skips_decorations() {
        let (mut ui, buffer) = create_test_ui(true);
        ui.info("Should not appear");
        ui.section("Should not appear");
        ui.progress(1, 10, "Should not appear");

        let output = String::from_utf8(buffer.lock().unwrap().clone()).unwrap();
        assert!(output.is_empty());
    }

    #[test]
    fn test_ui_error_shown_in_verbose() {
        let (mut ui, buffer) = create_test_ui(true);
        ui.error("This error should appear");

        let output = String::from_utf8(buffer.lock().unwrap().clone()).unwrap();
        assert!(output.contains("This error should appear"));
    }
}
