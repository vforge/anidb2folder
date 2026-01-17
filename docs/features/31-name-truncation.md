# 31 - Name Truncation

## Summary

Handle directory names that exceed filesystem length limits by intelligently truncating while preserving critical information.

## Dependencies

- **30-character-sanitizer** — Truncation is applied after sanitization

## Description

This feature implements intelligent name truncation for human-readable directory names that exceed the filesystem's maximum path component length (typically 255 bytes on most systems).

The truncation algorithm prioritizes preserving:

1. **Always preserved:** Series tag `[series]`, release year `(YYYY)`, AniDB suffix `[anidb-<id>]`
2. **Truncated first:** English title (if present)
3. **Truncated second:** Japanese title (if English exhausted)

Truncated portions are indicated with an ellipsis `…` character.

## Requirements

### Functional Requirements

1. Accept a configurable maximum length (default: 255)
2. Never truncate or remove:
   - Series tag (if present)
   - Release year (if present)
   - AniDB ID suffix
3. Truncation order:
   1. English title (add ellipsis after truncation)
   2. Japanese title (add ellipsis after truncation)
4. Result must not end with space or punctuation
5. Return both the truncated name and a flag indicating truncation occurred
6. Log warnings for truncated names

### Non-Functional Requirements

1. UTF-8 aware (count bytes, not characters, for filesystem limits)
2. Preserve word boundaries where possible
3. Minimum readable title length (at least first word if possible)

## Implementation Guide

### Step 1: Define Truncation Types

```rust
// src/truncator/mod.rs
use tracing::warn;

/// Result of truncation operation
#[derive(Debug)]
pub struct TruncationResult {
    pub name: String,
    pub truncated: bool,
    pub original_length: usize,
    pub final_length: usize,
}

/// Configuration for truncation
#[derive(Debug, Clone)]
pub struct TruncationConfig {
    /// Maximum byte length for the name
    pub max_length: usize,
    /// Minimum characters to keep for each title
    pub min_title_chars: usize,
}

impl Default for TruncationConfig {
    fn default() -> Self {
        Self {
            max_length: 255,
            min_title_chars: 10,
        }
    }
}
```

### Step 2: Implement Truncation Logic

```rust
// src/truncator/mod.rs (continued)

const ELLIPSIS: &str = "…";

/// Truncate a human-readable directory name to fit within limits
/// 
/// Priority for preservation (never truncated):
/// 1. Series tag [series]
/// 2. Release year (YYYY)
/// 3. AniDB suffix [anidb-<id>]
/// 
/// Truncation order (truncated first):
/// 1. English title
/// 2. Japanese title
pub fn truncate_name(
    series_tag: Option<&str>,
    title_jp: &str,
    title_en: Option<&str>,
    release_year: Option<u16>,
    anidb_id: u32,
    max_length: usize,
) -> String {
    // Build the fixed parts (never truncated)
    let series_part = series_tag.map(|t| format!("[{}] ", t)).unwrap_or_default();
    let year_part = release_year.map(|y| format!(" ({})", y)).unwrap_or_default();
    let anidb_part = format!(" [anidb-{}]", anidb_id);
    
    // Calculate space for titles
    let fixed_length = series_part.len() + year_part.len() + anidb_part.len();
    
    if fixed_length >= max_length {
        // Shouldn't happen in practice, but handle gracefully
        warn!("Fixed parts exceed max length for anidb-{}", anidb_id);
        return format!("{}{}", series_part.trim(), anidb_part);
    }
    
    let available_for_titles = max_length - fixed_length;
    
    // Build title part
    let title_part = build_title_part(title_jp, title_en, available_for_titles);
    
    // Assemble final name
    let name = format!("{}{}{}{}", series_part, title_part, year_part, anidb_part);
    
    // Final safety check - shouldn't be needed but be safe
    if name.len() > max_length {
        truncate_final_safety(&name, max_length)
    } else {
        name
    }
}

fn build_title_part(title_jp: &str, title_en: Option<&str>, max_bytes: usize) -> String {
    match title_en {
        Some(en) if en != title_jp => {
            // Full format: "JP ／ EN"
            let separator = " ／ ";
            let full_title = format!("{}{}{}", title_jp, separator, en);
            
            if full_title.len() <= max_bytes {
                return full_title;
            }
            
            // Try truncating English title first
            let jp_with_sep = format!("{}{}", title_jp, separator);
            if jp_with_sep.len() + 10 <= max_bytes {
                // Enough room for some English text
                let en_space = max_bytes - jp_with_sep.len() - ELLIPSIS.len();
                let truncated_en = truncate_string_smart(en, en_space);
                return format!("{}{}{}{}", title_jp, separator, truncated_en, ELLIPSIS);
            }
            
            // Not enough room for English, just use Japanese
            if title_jp.len() <= max_bytes {
                return title_jp.to_string();
            }
            
            // Truncate Japanese
            let truncated_jp = truncate_string_smart(title_jp, max_bytes - ELLIPSIS.len());
            format!("{}{}", truncated_jp, ELLIPSIS)
        }
        _ => {
            // Only Japanese title
            if title_jp.len() <= max_bytes {
                title_jp.to_string()
            } else {
                let truncated = truncate_string_smart(title_jp, max_bytes - ELLIPSIS.len());
                format!("{}{}", truncated, ELLIPSIS)
            }
        }
    }
}

/// Truncate a string to fit within byte limit, preserving word boundaries
fn truncate_string_smart(s: &str, max_bytes: usize) -> String {
    if s.len() <= max_bytes {
        return s.to_string();
    }
    
    // Find the last character boundary before max_bytes
    let mut last_valid = 0;
    let mut last_word_boundary = 0;
    
    for (i, c) in s.char_indices() {
        let next_pos = i + c.len_utf8();
        
        if next_pos > max_bytes {
            break;
        }
        
        last_valid = next_pos;
        
        // Track word boundaries
        if c.is_whitespace() || c == '-' {
            last_word_boundary = i;
        }
    }
    
    // Prefer word boundary if it preserves reasonable length
    let cut_point = if last_word_boundary > last_valid / 2 {
        last_word_boundary
    } else {
        last_valid
    };
    
    s[..cut_point].trim_end().to_string()
}

/// Final safety truncation - just cut at byte boundary
fn truncate_final_safety(s: &str, max_bytes: usize) -> String {
    if s.len() <= max_bytes {
        return s.to_string();
    }
    
    // Find valid UTF-8 boundary
    let mut end = max_bytes;
    while end > 0 && !s.is_char_boundary(end) {
        end -= 1;
    }
    
    s[..end].to_string()
}

/// Check if a name would need truncation
pub fn would_truncate(
    series_tag: Option<&str>,
    title_jp: &str,
    title_en: Option<&str>,
    release_year: Option<u16>,
    anidb_id: u32,
    max_length: usize,
) -> bool {
    let full_name = build_full_name(series_tag, title_jp, title_en, release_year, anidb_id);
    full_name.len() > max_length
}

fn build_full_name(
    series_tag: Option<&str>,
    title_jp: &str,
    title_en: Option<&str>,
    release_year: Option<u16>,
    anidb_id: u32,
) -> String {
    let mut parts = Vec::new();
    
    if let Some(tag) = series_tag {
        parts.push(format!("[{}]", tag));
    }
    
    match title_en {
        Some(en) if en != title_jp => {
            parts.push(format!("{} ／ {}", title_jp, en));
        }
        _ => {
            parts.push(title_jp.to_string());
        }
    }
    
    if let Some(year) = release_year {
        parts.push(format!("({})", year));
    }
    
    parts.push(format!("[anidb-{}]", anidb_id));
    
    parts.join(" ")
}
```

### Step 3: Add Module to Library

```rust
// src/lib.rs
pub mod truncator;

pub use truncator::{truncate_name, would_truncate, TruncationConfig};
```

## Test Cases

### Unit Tests

```rust
#[cfg(test)]
mod tests {
    use super::*;
    
    // ============ No Truncation Needed ============
    
    #[test]
    fn test_no_truncation_short_name() {
        let result = truncate_name(
            Some("AS0"),
            "Short Title",
            Some("Short Title EN"),
            Some(2020),
            12345,
            255,
        );
        
        assert_eq!(result, "[AS0] Short Title ／ Short Title EN (2020) [anidb-12345]");
    }
    
    // ============ English Title Truncation ============
    
    #[test]
    fn test_truncate_english_title() {
        let long_en = "A".repeat(200);
        let result = truncate_name(
            None,
            "Short JP",
            Some(&long_en),
            Some(2020),
            1,
            100,
        );
        
        // Should contain ellipsis
        assert!(result.contains(ELLIPSIS));
        // Should preserve JP title
        assert!(result.contains("Short JP"));
        // Should preserve year and ID
        assert!(result.contains("(2020)"));
        assert!(result.contains("[anidb-1]"));
        // Should be within limit
        assert!(result.len() <= 100);
    }
    
    // ============ Japanese Title Truncation ============
    
    #[test]
    fn test_truncate_japanese_title() {
        let long_jp = "あ".repeat(100); // Long Japanese string
        let result = truncate_name(
            None,
            &long_jp,
            None,
            Some(2020),
            1,
            100,
        );
        
        assert!(result.contains(ELLIPSIS));
        assert!(result.contains("(2020)"));
        assert!(result.contains("[anidb-1]"));
        assert!(result.len() <= 100);
    }
    
    // ============ Both Titles Truncated ============
    
    #[test]
    fn test_truncate_both_titles() {
        let long_jp = "A".repeat(100);
        let long_en = "B".repeat(100);
        
        let result = truncate_name(
            Some("Series"),
            &long_jp,
            Some(&long_en),
            Some(2020),
            12345,
            100,
        );
        
        // Should preserve critical parts
        assert!(result.contains("[Series]"));
        assert!(result.contains("[anidb-12345]"));
        assert!(result.len() <= 100);
    }
    
    // ============ Series Tag Preserved ============
    
    #[test]
    fn test_series_tag_always_preserved() {
        let long_jp = "A".repeat(200);
        
        let result = truncate_name(
            Some("My Important Series Tag"),
            &long_jp,
            None,
            None,
            999,
            100,
        );
        
        assert!(result.contains("[My Important Series Tag]"));
        assert!(result.contains("[anidb-999]"));
    }
    
    // ============ Year Preserved ============
    
    #[test]
    fn test_year_always_preserved() {
        let long_jp = "A".repeat(200);
        
        let result = truncate_name(
            None,
            &long_jp,
            None,
            Some(1999),
            1,
            100,
        );
        
        assert!(result.contains("(1999)"));
        assert!(result.contains("[anidb-1]"));
    }
    
    // ============ Word Boundary Preservation ============
    
    #[test]
    fn test_word_boundary_truncation() {
        let result = truncate_string_smart(
            "The Quick Brown Fox Jumps",
            15,
        );
        
        // Should cut at word boundary
        assert!(!result.ends_with(' '));
        assert!(result.len() <= 15);
    }
    
    // ============ UTF-8 Safety ============
    
    #[test]
    fn test_utf8_boundary_safe() {
        let jp = "日本語タイトル長い名前";
        
        let result = truncate_string_smart(jp, 15);
        
        // Should be valid UTF-8
        assert!(result.is_char_boundary(result.len()));
        // Should not panic or corrupt
        String::from(&result); // Would panic if invalid
    }
    
    // ============ Edge Cases ============
    
    #[test]
    fn test_minimum_output() {
        // Even with very short limit, should produce valid output
        let result = truncate_name(
            Some("X"),
            "Title",
            None,
            None,
            1,
            30,
        );
        
        // Should at least have series tag and anidb id
        assert!(result.contains("[X]"));
        assert!(result.contains("[anidb-1]"));
    }
    
    #[test]
    fn test_would_truncate_true() {
        let long = "A".repeat(300);
        assert!(would_truncate(None, &long, None, None, 1, 255));
    }
    
    #[test]
    fn test_would_truncate_false() {
        assert!(!would_truncate(None, "Short", None, None, 1, 255));
    }
    
    // ============ Same Titles ============
    
    #[test]
    fn test_same_titles_no_duplicate() {
        // When JP and EN are same, should not repeat
        let result = truncate_name(
            None,
            "Same Title",
            Some("Same Title"),
            Some(2020),
            1,
            255,
        );
        
        // Should NOT have the title twice
        assert_eq!(result.matches("Same Title").count(), 1);
    }
}
```

### Integration with Rename

```rust
// Test that truncation integrates correctly with renaming
#[test]
fn test_truncation_in_rename_flow() {
    // This would be tested in rename tests with long anime titles
}
```

## Notes

- The 255-byte limit is based on most filesystem's maximum filename length
- Some filesystems use characters, not bytes — consider making this configurable
- The ellipsis `…` is a single Unicode character (3 bytes UTF-8)
- Word boundary detection helps maintain readability
- Consider adding a configuration option for minimum title length
- Log truncation warnings so users are aware of shortened names
