use std::fs;
use std::path::{Path, PathBuf};
use thiserror::Error;
use tracing::{debug, trace};

#[derive(Error, Debug)]
pub enum ScannerError {
    #[error("Path does not exist: {0}")]
    PathNotFound(PathBuf),

    #[error("Path is not a directory: {0}")]
    NotADirectory(PathBuf),

    #[error("Permission denied: {0}")]
    PermissionDenied(PathBuf),

    #[error("Failed to read directory: {0}")]
    IoError(#[from] std::io::Error),
}

#[derive(Debug, Clone)]
pub struct DirectoryEntry {
    pub name: String,
    pub path: PathBuf,
}

impl DirectoryEntry {
    pub fn new(name: String, path: PathBuf) -> Self {
        Self { name, path }
    }
}

pub fn scan_directory(target: &Path) -> Result<Vec<DirectoryEntry>, ScannerError> {
    debug!(path = ?target, "Scanning directory");

    if !target.exists() {
        return Err(ScannerError::PathNotFound(target.to_path_buf()));
    }

    if !target.is_dir() {
        return Err(ScannerError::NotADirectory(target.to_path_buf()));
    }

    let mut entries = Vec::new();

    let read_dir = fs::read_dir(target).map_err(|e| {
        if e.kind() == std::io::ErrorKind::PermissionDenied {
            ScannerError::PermissionDenied(target.to_path_buf())
        } else {
            ScannerError::IoError(e)
        }
    })?;

    for entry in read_dir {
        let entry = entry?;
        let path = entry.path();

        trace!(entry = ?path, "Examining entry");

        if !path.is_dir() {
            trace!(path = ?path, "Skipping non-directory");
            continue;
        }

        let name = match path.file_name() {
            Some(n) => n.to_string_lossy().to_string(),
            None => continue,
        };

        if name.starts_with('.') {
            trace!(name = %name, "Skipping hidden directory");
            continue;
        }

        debug!(name = %name, "Found subdirectory");
        entries.push(DirectoryEntry::new(name, path));
    }

    entries.sort_by(|a, b| a.name.cmp(&b.name));

    debug!(count = entries.len(), "Scan complete");

    Ok(entries)
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::tempdir;

    #[test]
    fn test_scan_empty_directory() {
        let dir = tempdir().unwrap();
        let result = scan_directory(dir.path()).unwrap();
        assert!(result.is_empty());
    }

    #[test]
    fn test_scan_with_subdirectories() {
        let dir = tempdir().unwrap();
        fs::create_dir(dir.path().join("subdir1")).unwrap();
        fs::create_dir(dir.path().join("subdir2")).unwrap();

        let result = scan_directory(dir.path()).unwrap();

        assert_eq!(result.len(), 2);
        assert_eq!(result[0].name, "subdir1");
        assert_eq!(result[1].name, "subdir2");
    }

    #[test]
    fn test_ignores_files() {
        let dir = tempdir().unwrap();
        fs::create_dir(dir.path().join("subdir")).unwrap();
        fs::write(dir.path().join("file.txt"), "content").unwrap();

        let result = scan_directory(dir.path()).unwrap();

        assert_eq!(result.len(), 1);
        assert_eq!(result[0].name, "subdir");
    }

    #[test]
    fn test_ignores_hidden_directories() {
        let dir = tempdir().unwrap();
        fs::create_dir(dir.path().join(".hidden")).unwrap();
        fs::create_dir(dir.path().join("visible")).unwrap();

        let result = scan_directory(dir.path()).unwrap();

        assert_eq!(result.len(), 1);
        assert_eq!(result[0].name, "visible");
    }

    #[test]
    fn test_path_not_found() {
        let result = scan_directory(Path::new("/nonexistent/path"));
        assert!(matches!(result, Err(ScannerError::PathNotFound(_))));
    }

    #[test]
    fn test_not_a_directory() {
        let dir = tempdir().unwrap();
        let file_path = dir.path().join("file.txt");
        fs::write(&file_path, "content").unwrap();

        let result = scan_directory(&file_path);
        assert!(matches!(result, Err(ScannerError::NotADirectory(_))));
    }

    #[test]
    fn test_alphabetical_sorting() {
        let dir = tempdir().unwrap();
        fs::create_dir(dir.path().join("zebra")).unwrap();
        fs::create_dir(dir.path().join("alpha")).unwrap();
        fs::create_dir(dir.path().join("beta")).unwrap();

        let result = scan_directory(dir.path()).unwrap();

        assert_eq!(result[0].name, "alpha");
        assert_eq!(result[1].name, "beta");
        assert_eq!(result[2].name, "zebra");
    }
}
