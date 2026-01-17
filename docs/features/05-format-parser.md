# 05 - Format Parser

## Summary

Parse directory names to identify and extract components from both AniDB and human-readable formats.

## Dependencies

- **01-directory-scanner** — Requires `DirectoryEntry` type for parsed results

## Description

This feature implements parsers for both directory naming formats used by anidb2folder. The parser examines directory names and determines which format they match, extracting structured data from each.

The two formats are:

1. **AniDB format:** `[<series>] <anidb_id>`
2. **Human-readable format:** `[<series>] <title_jp> ／ <title_en> (<year>) [anidb-<id>]`

The parser must accurately distinguish between formats and handle all valid variations, including optional components.

## Requirements

### Functional Requirements

1. Parse AniDB format directories:
   - Extract optional series tag
   - Extract AniDB ID
2. Parse human-readable format directories:
   - Extract optional series tag
   - Extract Japanese title
   - Extract optional English title (after `／`)
   - Extract optional release year
   - Extract AniDB ID from suffix
3. Return a structured result indicating:
   - Which format was detected
   - Extracted components
   - Parse errors for invalid formats
4. Handle edge cases:
   - Titles containing numbers
   - Titles containing parentheses
   - Missing optional components
   - Unicode characters in titles

### Non-Functional Requirements

1. Use `regex` crate for pattern matching
2. Comprehensive error messages for parse failures
3. Zero heap allocations for format detection (where possible)

## Implementation Guide

### Step 1: Add Dependencies

```toml
# Cargo.toml additions
[dependencies]
regex = "1.10"
once_cell = "1.19"  # For lazy static regex compilation
```

### Step 2: Define Format Types

```rust
// src/parser/types.rs
use thiserror::Error;

/// Detected directory format
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum DirectoryFormat {
    AniDb,
    HumanReadable,
}

/// Parsed AniDB format directory
#[derive(Debug, Clone)]
pub struct AniDbFormat {
    pub series_tag: Option<String>,
    pub anidb_id: u32,
    pub original_name: String,
}

/// Parsed human-readable format directory
#[derive(Debug, Clone)]
pub struct HumanReadableFormat {
    pub series_tag: Option<String>,
    pub title_jp: String,
    pub title_en: Option<String>,
    pub release_year: Option<u16>,
    pub anidb_id: u32,
    pub original_name: String,
}

/// Result of parsing a directory name
#[derive(Debug, Clone)]
pub enum ParsedDirectory {
    AniDb(AniDbFormat),
    HumanReadable(HumanReadableFormat),
}

impl ParsedDirectory {
    pub fn format(&self) -> DirectoryFormat {
        match self {
            ParsedDirectory::AniDb(_) => DirectoryFormat::AniDb,
            ParsedDirectory::HumanReadable(_) => DirectoryFormat::HumanReadable,
        }
    }
    
    pub fn anidb_id(&self) -> u32 {
        match self {
            ParsedDirectory::AniDb(f) => f.anidb_id,
            ParsedDirectory::HumanReadable(f) => f.anidb_id,
        }
    }
    
    pub fn series_tag(&self) -> Option<&str> {
        match self {
            ParsedDirectory::AniDb(f) => f.series_tag.as_deref(),
            ParsedDirectory::HumanReadable(f) => f.series_tag.as_deref(),
        }
    }
    
    pub fn original_name(&self) -> &str {
        match self {
            ParsedDirectory::AniDb(f) => &f.original_name,
            ParsedDirectory::HumanReadable(f) => &f.original_name,
        }
    }
}

#[derive(Error, Debug)]
pub enum ParseError {
    #[error("Directory name does not match any known format: {0}")]
    UnrecognizedFormat(String),
    
    #[error("Invalid AniDB ID: {0}")]
    InvalidAniDbId(String),
    
    #[error("Missing required component: {0}")]
    MissingComponent(String),
}
```

### Step 3: Implement Parsers

```rust
// src/parser/mod.rs
mod types;

pub use types::*;

use once_cell::sync::Lazy;
use regex::Regex;

// AniDB format: [<series>] <anidb_id>
// Examples: "12345", "[AS0] 12345", "[My Series] 67890"
static ANIDB_REGEX: Lazy<Regex> = Lazy::new(|| {
    Regex::new(r"^(?:\[([^\]]+)\]\s*)?(\d+)$").unwrap()
});

// Human-readable format: [<series>] <title_jp> ／ <title_en> (<year>) [anidb-<id>]
// The unicode slash ／ (U+FF0F) separates JP and EN titles
// Examples:
//   "Naruto (2002) [anidb-12345]"
//   "[AS0] Cowboy Bebop ／ Cowboy Bebop (1998) [anidb-1]"
//   "[Series] Title [anidb-123]"
static HUMAN_READABLE_REGEX: Lazy<Regex> = Lazy::new(|| {
    Regex::new(
        r"^(?:\[([^\]]+)\]\s*)?(.*?)\s*(?:\((\d{4})\))?\s*\[anidb-(\d+)\]$"
    ).unwrap()
});

// Regex to split JP/EN titles on unicode slash
static TITLE_SPLIT_REGEX: Lazy<Regex> = Lazy::new(|| {
    Regex::new(r"\s*／\s*").unwrap()
});

/// Parse a directory name and return structured data
pub fn parse_directory_name(name: &str) -> Result<ParsedDirectory, ParseError> {
    // Try human-readable format first (more specific pattern)
    if let Some(parsed) = try_parse_human_readable(name) {
        return Ok(ParsedDirectory::HumanReadable(parsed));
    }
    
    // Try AniDB format
    if let Some(parsed) = try_parse_anidb(name) {
        return Ok(ParsedDirectory::AniDb(parsed));
    }
    
    Err(ParseError::UnrecognizedFormat(name.to_string()))
}

fn try_parse_anidb(name: &str) -> Option<AniDbFormat> {
    let captures = ANIDB_REGEX.captures(name)?;
    
    let series_tag = captures.get(1).map(|m| m.as_str().to_string());
    let anidb_id: u32 = captures.get(2)?.as_str().parse().ok()?;
    
    Some(AniDbFormat {
        series_tag,
        anidb_id,
        original_name: name.to_string(),
    })
}

fn try_parse_human_readable(name: &str) -> Option<HumanReadableFormat> {
    let captures = HUMAN_READABLE_REGEX.captures(name)?;
    
    let series_tag = captures.get(1).map(|m| m.as_str().to_string());
    let titles_part = captures.get(2)?.as_str().trim();
    let release_year: Option<u16> = captures.get(3)
        .and_then(|m| m.as_str().parse().ok());
    let anidb_id: u32 = captures.get(4)?.as_str().parse().ok()?;
    
    // Split titles on unicode slash
    let (title_jp, title_en) = split_titles(titles_part);
    
    // Must have at least a Japanese title
    if title_jp.is_empty() {
        return None;
    }
    
    Some(HumanReadableFormat {
        series_tag,
        title_jp,
        title_en,
        release_year,
        anidb_id,
        original_name: name.to_string(),
    })
}

fn split_titles(titles: &str) -> (String, Option<String>) {
    let parts: Vec<&str> = TITLE_SPLIT_REGEX.split(titles).collect();
    
    match parts.len() {
        0 => (String::new(), None),
        1 => (parts[0].trim().to_string(), None),
        _ => {
            let jp = parts[0].trim().to_string();
            let en = parts[1].trim().to_string();
            
            // If titles are identical, treat as single title
            if jp == en {
                (jp, None)
            } else {
                (jp, Some(en))
            }
        }
    }
}

/// Quick check if a name looks like AniDB format (for validation)
pub fn is_anidb_format(name: &str) -> bool {
    ANIDB_REGEX.is_match(name)
}

/// Quick check if a name looks like human-readable format (for validation)
pub fn is_human_readable_format(name: &str) -> bool {
    HUMAN_READABLE_REGEX.is_match(name)
}

/// Detect the format without full parsing
pub fn detect_format(name: &str) -> Option<DirectoryFormat> {
    if is_human_readable_format(name) {
        Some(DirectoryFormat::HumanReadable)
    } else if is_anidb_format(name) {
        Some(DirectoryFormat::AniDb)
    } else {
        None
    }
}
```

### Step 4: Add to Library

```rust
// src/lib.rs
pub mod parser;
pub mod scanner;

pub use parser::{
    parse_directory_name, detect_format, is_anidb_format, is_human_readable_format,
    AniDbFormat, HumanReadableFormat, ParsedDirectory, DirectoryFormat, ParseError,
};
```

## Test Cases

### Unit Tests

```rust
#[cfg(test)]
mod tests {
    use super::*;
    
    // ============ AniDB Format Tests ============
    
    #[test]
    fn test_parse_anidb_simple() {
        let result = parse_directory_name("12345").unwrap();
        match result {
            ParsedDirectory::AniDb(f) => {
                assert_eq!(f.anidb_id, 12345);
                assert!(f.series_tag.is_none());
            }
            _ => panic!("Expected AniDB format"),
        }
    }
    
    #[test]
    fn test_parse_anidb_with_series() {
        let result = parse_directory_name("[AS0] 12345").unwrap();
        match result {
            ParsedDirectory::AniDb(f) => {
                assert_eq!(f.anidb_id, 12345);
                assert_eq!(f.series_tag, Some("AS0".to_string()));
            }
            _ => panic!("Expected AniDB format"),
        }
    }
    
    #[test]
    fn test_parse_anidb_with_long_series() {
        let result = parse_directory_name("[My Favorite Series] 67890").unwrap();
        match result {
            ParsedDirectory::AniDb(f) => {
                assert_eq!(f.anidb_id, 67890);
                assert_eq!(f.series_tag, Some("My Favorite Series".to_string()));
            }
            _ => panic!("Expected AniDB format"),
        }
    }
    
    // ============ Human-Readable Format Tests ============
    
    #[test]
    fn test_parse_human_readable_full() {
        let result = parse_directory_name(
            "[AS0] Cowboyu Bebopu ／ Cowboy Bebop (1998) [anidb-1]"
        ).unwrap();
        
        match result {
            ParsedDirectory::HumanReadable(f) => {
                assert_eq!(f.series_tag, Some("AS0".to_string()));
                assert_eq!(f.title_jp, "Cowboyu Bebopu");
                assert_eq!(f.title_en, Some("Cowboy Bebop".to_string()));
                assert_eq!(f.release_year, Some(1998));
                assert_eq!(f.anidb_id, 1);
            }
            _ => panic!("Expected human-readable format"),
        }
    }
    
    #[test]
    fn test_parse_human_readable_no_series() {
        let result = parse_directory_name("Naruto (2002) [anidb-12345]").unwrap();
        
        match result {
            ParsedDirectory::HumanReadable(f) => {
                assert!(f.series_tag.is_none());
                assert_eq!(f.title_jp, "Naruto");
                assert!(f.title_en.is_none());
                assert_eq!(f.release_year, Some(2002));
                assert_eq!(f.anidb_id, 12345);
            }
            _ => panic!("Expected human-readable format"),
        }
    }
    
    #[test]
    fn test_parse_human_readable_no_year() {
        let result = parse_directory_name("[FMA] Fullmetal Alchemist [anidb-54321]").unwrap();
        
        match result {
            ParsedDirectory::HumanReadable(f) => {
                assert_eq!(f.series_tag, Some("FMA".to_string()));
                assert_eq!(f.title_jp, "Fullmetal Alchemist");
                assert!(f.release_year.is_none());
                assert_eq!(f.anidb_id, 54321);
            }
            _ => panic!("Expected human-readable format"),
        }
    }
    
    #[test]
    fn test_parse_human_readable_same_titles() {
        // When JP and EN titles are the same, EN should be None
        let result = parse_directory_name("One Piece ／ One Piece (1999) [anidb-69]").unwrap();
        
        match result {
            ParsedDirectory::HumanReadable(f) => {
                assert_eq!(f.title_jp, "One Piece");
                assert!(f.title_en.is_none()); // Duplicates collapsed
                assert_eq!(f.release_year, Some(1999));
            }
            _ => panic!("Expected human-readable format"),
        }
    }
    
    // ============ Edge Cases ============
    
    #[test]
    fn test_parse_unrecognized() {
        let result = parse_directory_name("Random Folder Name");
        assert!(matches!(result, Err(ParseError::UnrecognizedFormat(_))));
    }
    
    #[test]
    fn test_parse_title_with_parentheses() {
        let result = parse_directory_name(
            "Steins;Gate (Anime) (2011) [anidb-7729]"
        ).unwrap();
        
        match result {
            ParsedDirectory::HumanReadable(f) => {
                assert_eq!(f.title_jp, "Steins;Gate (Anime)");
                assert_eq!(f.release_year, Some(2011));
            }
            _ => panic!("Expected human-readable format"),
        }
    }
    
    #[test]
    fn test_parse_title_with_numbers() {
        let result = parse_directory_name("86 (2021) [anidb-15587]").unwrap();
        
        match result {
            ParsedDirectory::HumanReadable(f) => {
                assert_eq!(f.title_jp, "86");
                assert_eq!(f.release_year, Some(2021));
            }
            _ => panic!("Expected human-readable format"),
        }
    }
    
    // ============ Format Detection Tests ============
    
    #[test]
    fn test_detect_anidb_format() {
        assert_eq!(detect_format("12345"), Some(DirectoryFormat::AniDb));
        assert_eq!(detect_format("[X] 99"), Some(DirectoryFormat::AniDb));
    }
    
    #[test]
    fn test_detect_human_readable_format() {
        assert_eq!(
            detect_format("Title (2020) [anidb-1]"), 
            Some(DirectoryFormat::HumanReadable)
        );
    }
    
    #[test]
    fn test_detect_unknown_format() {
        assert_eq!(detect_format("Unknown Directory"), None);
    }
}
```

## Notes

- The unicode slash `／` (U+FF0F) is used because regular `/` is invalid in directory names
- Regex patterns are compiled once using `once_cell::Lazy` for performance
- Human-readable format is checked first because it's more specific (AniDB format could match numbers in titles)
- Edge cases with titles containing years in parentheses need careful regex handling
- Consider adding support for alternative separators in the future
