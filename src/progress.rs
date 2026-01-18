//! Progress output for user-facing status updates.
//!
//! This module provides progress output that is shown regardless of verbose mode.
//! Verbose mode adds detailed tracing on top of this base progress output.

use std::io::{self, Write};

/// Progress reporter for user-facing output
pub struct Progress {
    writer: Box<dyn Write>,
}

impl Default for Progress {
    fn default() -> Self {
        Self::new()
    }
}

impl Progress {
    /// Create a new progress reporter writing to stderr
    pub fn new() -> Self {
        Self {
            writer: Box::new(io::stderr()),
        }
    }

    /// Create a progress reporter with a custom writer (for testing)
    #[cfg(test)]
    pub fn with_writer(writer: Box<dyn Write>) -> Self {
        Self { writer }
    }

    /// Report starting a scan
    pub fn scan_start(&mut self, path: &std::path::Path) {
        let _ = writeln!(self.writer, "Scanning {}...", path.display());
    }

    /// Report scan complete
    pub fn scan_complete(&mut self, count: usize) {
        let _ = writeln!(self.writer, "Found {} directories", count);
    }

    /// Report starting validation
    pub fn validate_start(&mut self) {
        let _ = writeln!(self.writer, "Validating directory formats...");
    }

    /// Report validation complete
    pub fn validate_complete(&mut self, format: &str) {
        let _ = writeln!(self.writer, "Format: {}", format);
    }

    /// Report starting rename operation
    pub fn rename_start(&mut self, total: usize, direction: &str) {
        let _ = writeln!(self.writer);
        let _ = writeln!(self.writer, "Renaming {} directories ({})", total, direction);
    }

    /// Report progress on a single rename
    pub fn rename_progress(&mut self, current: usize, total: usize, from: &str, to: &str) {
        let _ = writeln!(
            self.writer,
            "[{}/{}] {} -> {}",
            current, total, from, to
        );
    }

    /// Report fetching metadata from API
    pub fn fetch_start(&mut self, anidb_id: u32) {
        let _ = write!(self.writer, "Fetching metadata for {}...", anidb_id);
        let _ = self.writer.flush();
    }

    /// Report fetch complete (same line)
    pub fn fetch_complete(&mut self) {
        let _ = writeln!(self.writer, " done");
    }

    /// Report using cached data
    pub fn using_cache(&mut self, anidb_id: u32) {
        let _ = writeln!(self.writer, "Using cached data for {}", anidb_id);
    }

    /// Report that API would be called (dry run mode)
    pub fn would_fetch(&mut self, anidb_id: u32) {
        let _ = writeln!(self.writer, "Would fetch metadata for {} (dry run)", anidb_id);
    }

    /// Report rename complete
    pub fn rename_complete(&mut self, success_count: usize, dry_run: bool) {
        let _ = writeln!(self.writer);
        if dry_run {
            let _ = writeln!(
                self.writer,
                "Dry run complete. {} directories would be renamed.",
                success_count
            );
        } else {
            let _ = writeln!(
                self.writer,
                "Complete. {} directories renamed.",
                success_count
            );
        }
    }

    /// Report an error during operation (non-fatal)
    pub fn warn(&mut self, message: &str) {
        let _ = writeln!(self.writer, "Warning: {}", message);
    }

    /// Report history file written
    pub fn history_written(&mut self, path: &std::path::Path) {
        let _ = writeln!(self.writer, "History saved to: {}", path.display());
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
    fn test_scan_output() {
        let (mut progress, buffer) = create_test_progress();
        let path = std::path::Path::new("/test/path");

        progress.scan_start(path);
        progress.scan_complete(5);

        let output = String::from_utf8(buffer.lock().unwrap().clone()).unwrap();
        assert!(output.contains("Scanning /test/path"));
        assert!(output.contains("Found 5 directories"));
    }

    #[test]
    fn test_validate_output() {
        let (mut progress, buffer) = create_test_progress();

        progress.validate_start();
        progress.validate_complete("AniDB");

        let output = String::from_utf8(buffer.lock().unwrap().clone()).unwrap();
        assert!(output.contains("Validating"));
        assert!(output.contains("Format: AniDB"));
    }

    #[test]
    fn test_rename_progress() {
        let (mut progress, buffer) = create_test_progress();

        progress.rename_start(3, "AniDB -> Human-readable");
        progress.rename_progress(1, 3, "12345", "Anime Title [anidb-12345]");
        progress.rename_progress(2, 3, "67890", "Another Anime [anidb-67890]");
        progress.rename_complete(2, false);

        let output = String::from_utf8(buffer.lock().unwrap().clone()).unwrap();
        assert!(output.contains("Renaming 3 directories"));
        assert!(output.contains("[1/3]"));
        assert!(output.contains("[2/3]"));
        assert!(output.contains("Complete. 2 directories renamed"));
    }

    #[test]
    fn test_dry_run_complete() {
        let (mut progress, buffer) = create_test_progress();

        progress.rename_complete(5, true);

        let output = String::from_utf8(buffer.lock().unwrap().clone()).unwrap();
        assert!(output.contains("Dry run complete"));
        assert!(output.contains("5 directories would be renamed"));
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
    fn test_cache_output() {
        let (mut progress, buffer) = create_test_progress();

        progress.using_cache(12345);

        let output = String::from_utf8(buffer.lock().unwrap().clone()).unwrap();
        assert!(output.contains("Using cached data for 12345"));
    }
}
