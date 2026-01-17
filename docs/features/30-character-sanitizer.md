# 30 - Character Sanitizer

## Summary

Replace characters that are invalid for directory names with safe Unicode alternatives.

## Dependencies

- **05-format-parser** — Requires understanding of name formats to sanitize correctly

## Description

This feature implements filename sanitization to replace characters that are invalid on various filesystems with visually similar Unicode alternatives. This ensures directory names are valid on all supported platforms.

Different operating systems have different restrictions:

- **Windows:** Cannot use `/ \ : * ? " < > |`
- **macOS/Linux:** Cannot use `/` and null character
- **All:** Should avoid leading/trailing spaces and periods

The sanitizer uses fullwidth Unicode characters as replacements since they look similar to the originals but are valid in filenames.

## Requirements

### Functional Requirements

1. Replace invalid characters with Unicode equivalents:
   - `/` → `／` (U+FF0F Fullwidth Solidus)
   - `\` → `＼` (U+FF3C Fullwidth Reverse Solidus)
   - `:` → `：` (U+FF1A Fullwidth Colon)
   - `*` → `＊` (U+FF0A Fullwidth Asterisk)
   - `?` → `？` (U+FF1F Fullwidth Question Mark)
   - `"` → `＂` (U+FF02 Fullwidth Quotation Mark)
   - `<` → `＜` (U+FF1C Fullwidth Less-Than Sign)
   - `>` → `＞` (U+FF1E Fullwidth Greater-Than Sign)
   - `|` → `｜` (U+FF5C Fullwidth Vertical Line)
2. Remove or replace control characters
3. Trim leading/trailing whitespace
4. Replace multiple consecutive spaces with single space
5. Handle null characters
6. Preserve all other Unicode characters

### Non-Functional Requirements

1. Zero-copy where possible (only allocate when changes needed)
2. Compile-time lookup table for replacements
3. Cross-platform compatibility

## Implementation Guide

### Step 1: Define Replacement Mappings

```rust
// src/sanitizer/mod.rs
use std::borrow::Cow;

/// Character replacement mappings for filesystem safety
const REPLACEMENTS: &[(char, char)] = &[
    ('/', '／'),   // U+FF0F Fullwidth Solidus
    ('\\', '＼'),  // U+FF3C Fullwidth Reverse Solidus
    (':', '：'),   // U+FF1A Fullwidth Colon
    ('*', '＊'),   // U+FF0A Fullwidth Asterisk
    ('?', '？'),   // U+FF1F Fullwidth Question Mark
    ('"', '＂'),   // U+FF02 Fullwidth Quotation Mark
    ('<', '＜'),   // U+FF1C Fullwidth Less-Than Sign
    ('>', '＞'),   // U+FF1E Fullwidth Greater-Than Sign
    ('|', '｜'),   // U+FF5C Fullwidth Vertical Line
];

/// Characters that should be removed entirely
const REMOVE_CHARS: &[char] = &[
    '\0',          // Null character
    '\x01', '\x02', '\x03', '\x04', '\x05', '\x06', '\x07',
    '\x08', '\x09', '\x0A', '\x0B', '\x0C', '\x0D', '\x0E', '\x0F',
    '\x10', '\x11', '\x12', '\x13', '\x14', '\x15', '\x16', '\x17',
    '\x18', '\x19', '\x1A', '\x1B', '\x1C', '\x1D', '\x1E', '\x1F',
];
```

### Step 2: Implement Sanitizer

```rust
// src/sanitizer/mod.rs (continued)

/// Check if a character needs replacement
fn get_replacement(c: char) -> Option<char> {
    REPLACEMENTS.iter()
        .find(|(from, _)| *from == c)
        .map(|(_, to)| *to)
}

/// Check if a character should be removed
fn should_remove(c: char) -> bool {
    REMOVE_CHARS.contains(&c)
}

/// Check if the string needs any sanitization
fn needs_sanitization(s: &str) -> bool {
    s.chars().any(|c| {
        get_replacement(c).is_some() || should_remove(c)
    }) || s.starts_with(' ') 
      || s.ends_with(' ') 
      || s.contains("  ")
}

/// Sanitize a filename by replacing invalid characters
/// 
/// Returns a `Cow<str>` to avoid allocation if no changes needed.
pub fn sanitize_filename(input: &str) -> Cow<str> {
    if !needs_sanitization(input) {
        return Cow::Borrowed(input);
    }
    
    let mut result = String::with_capacity(input.len());
    let mut last_was_space = true; // Treat start as after space to trim leading
    
    for c in input.chars() {
        // Skip characters to remove
        if should_remove(c) {
            continue;
        }
        
        // Handle spaces (collapse multiple, trim leading)
        if c == ' ' {
            if !last_was_space {
                result.push(' ');
                last_was_space = true;
            }
            continue;
        }
        
        last_was_space = false;
        
        // Replace or keep character
        if let Some(replacement) = get_replacement(c) {
            result.push(replacement);
        } else {
            result.push(c);
        }
    }
    
    // Trim trailing space
    if result.ends_with(' ') {
        result.pop();
    }
    
    Cow::Owned(result)
}

/// Sanitize with additional platform-specific rules
pub fn sanitize_for_platform(input: &str, platform: Platform) -> Cow<str> {
    let base_sanitized = sanitize_filename(input);
    
    match platform {
        Platform::Windows => sanitize_windows_specific(&base_sanitized),
        Platform::Unix => base_sanitized,
    }
}

#[derive(Debug, Clone, Copy)]
pub enum Platform {
    Windows,
    Unix,
}

fn sanitize_windows_specific(input: &str) -> Cow<str> {
    // Windows also doesn't allow names ending with period or space
    // (already handled by base sanitizer for spaces)
    
    let trimmed = input.trim_end_matches('.');
    
    if trimmed.len() != input.len() {
        Cow::Owned(trimmed.to_string())
    } else {
        Cow::Borrowed(input)
    }
}

/// Check if a sanitized name is valid for the filesystem
pub fn is_valid_filename(name: &str) -> bool {
    if name.is_empty() {
        return false;
    }
    
    // Check for any invalid characters remaining
    for c in name.chars() {
        if should_remove(c) || get_replacement(c).is_some() {
            return false;
        }
    }
    
    // Check for leading/trailing spaces
    if name.starts_with(' ') || name.ends_with(' ') {
        return false;
    }
    
    true
}
```

### Step 3: Add Module to Library

```rust
// src/lib.rs
pub mod sanitizer;

pub use sanitizer::{sanitize_filename, sanitize_for_platform, is_valid_filename, Platform};
```

## Test Cases

### Unit Tests

```rust
#[cfg(test)]
mod tests {
    use super::*;
    
    // ============ Basic Replacements ============
    
    #[test]
    fn test_replace_forward_slash() {
        let result = sanitize_filename("Title/Subtitle");
        assert_eq!(result, "Title／Subtitle");
    }
    
    #[test]
    fn test_replace_backslash() {
        let result = sanitize_filename("Path\\Name");
        assert_eq!(result, "Path＼Name");
    }
    
    #[test]
    fn test_replace_colon() {
        let result = sanitize_filename("Title: Subtitle");
        assert_eq!(result, "Title： Subtitle");
    }
    
    #[test]
    fn test_replace_asterisk() {
        let result = sanitize_filename("Rating: *****");
        assert_eq!(result, "Rating： ＊＊＊＊＊");
    }
    
    #[test]
    fn test_replace_question_mark() {
        let result = sanitize_filename("What?");
        assert_eq!(result, "What？");
    }
    
    #[test]
    fn test_replace_quotes() {
        let result = sanitize_filename("\"Title\"");
        assert_eq!(result, "＂Title＂");
    }
    
    #[test]
    fn test_replace_angle_brackets() {
        let result = sanitize_filename("<Title>");
        assert_eq!(result, "＜Title＞");
    }
    
    #[test]
    fn test_replace_pipe() {
        let result = sanitize_filename("A|B");
        assert_eq!(result, "A｜B");
    }
    
    // ============ Multiple Replacements ============
    
    #[test]
    fn test_multiple_replacements() {
        let result = sanitize_filename("Title: Part 1/2 <Special>");
        assert_eq!(result, "Title： Part 1／2 ＜Special＞");
    }
    
    // ============ No Changes Needed ============
    
    #[test]
    fn test_no_changes_needed() {
        let input = "Normal Title (2020) [anidb-12345]";
        let result = sanitize_filename(input);
        
        // Should return borrowed reference (no allocation)
        assert!(matches!(result, Cow::Borrowed(_)));
        assert_eq!(result, input);
    }
    
    #[test]
    fn test_unicode_preserved() {
        let input = "日本語タイトル";
        let result = sanitize_filename(input);
        assert_eq!(result, input);
    }
    
    // ============ Whitespace Handling ============
    
    #[test]
    fn test_trim_leading_spaces() {
        let result = sanitize_filename("  Title");
        assert_eq!(result, "Title");
    }
    
    #[test]
    fn test_trim_trailing_spaces() {
        let result = sanitize_filename("Title  ");
        assert_eq!(result, "Title");
    }
    
    #[test]
    fn test_collapse_multiple_spaces() {
        let result = sanitize_filename("Title   With    Spaces");
        assert_eq!(result, "Title With Spaces");
    }
    
    // ============ Control Characters ============
    
    #[test]
    fn test_remove_null_character() {
        let result = sanitize_filename("Title\0Name");
        assert_eq!(result, "TitleName");
    }
    
    #[test]
    fn test_remove_control_characters() {
        let result = sanitize_filename("Title\x01\x02\x03Name");
        assert_eq!(result, "TitleName");
    }
    
    // ============ Validation ============
    
    #[test]
    fn test_is_valid_clean_name() {
        assert!(is_valid_filename("Normal Title (2020)"));
    }
    
    #[test]
    fn test_is_invalid_with_slash() {
        assert!(!is_valid_filename("Title/Name"));
    }
    
    #[test]
    fn test_is_invalid_empty() {
        assert!(!is_valid_filename(""));
    }
    
    // ============ Platform Specific ============
    
    #[test]
    fn test_windows_trailing_period() {
        let result = sanitize_for_platform("Title.", Platform::Windows);
        assert_eq!(result, "Title");
    }
    
    #[test]
    fn test_windows_multiple_trailing_periods() {
        let result = sanitize_for_platform("Title...", Platform::Windows);
        assert_eq!(result, "Title");
    }
    
    #[test]
    fn test_unix_allows_trailing_period() {
        let result = sanitize_for_platform("Title.", Platform::Unix);
        assert_eq!(result, "Title.");
    }
    
    // ============ Edge Cases ============
    
    #[test]
    fn test_only_spaces() {
        let result = sanitize_filename("     ");
        assert_eq!(result, "");
    }
    
    #[test]
    fn test_only_invalid_chars() {
        let result = sanitize_filename("/\\:*?\"<>|");
        // All replaced with fullwidth equivalents
        assert!(!result.contains('/'));
        assert!(!result.is_empty());
    }
    
    #[test]
    fn test_mixed_unicode_and_invalid() {
        let result = sanitize_filename("アニメ: Title/日本");
        assert_eq!(result, "アニメ： Title／日本");
    }
}
```

### Performance Test

```rust
#[test]
fn test_no_allocation_when_clean() {
    let input = "Clean Title Without Issues";
    let result = sanitize_filename(input);
    
    // Verify it's a borrowed reference (no heap allocation)
    match result {
        Cow::Borrowed(s) => assert_eq!(s, input),
        Cow::Owned(_) => panic!("Unexpected allocation"),
    }
}
```

## Notes

- The `Cow<str>` return type avoids allocation when no changes are needed
- Fullwidth Unicode characters are visually similar to ASCII originals
- The sanitizer is called during name building (feature 22), not on raw directory names
- Consider adding a reverse mapping function to convert sanitized names back
- Windows has additional reserved names (CON, PRN, etc.) — could add those checks later
