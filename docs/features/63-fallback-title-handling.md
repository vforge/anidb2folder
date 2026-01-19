# 63 - Fallback Title Handling

## Summary

Use fallback titles when the primary main title is missing from AniDB responses.

## Dependencies

- **10-anidb-api-client** — Modifies the XML parsing logic

## Description

Some AniDB entries lack a proper `type="main"` title, causing the tool to fail with an `IncompleteData` error. This feature adds a fallback chain to find a usable title from other available title types.

### Current Behavior

The parser looks for titles in this order:
1. `type="main"` (any language)
2. `type="official"` with `xml:lang="x-jat"` (romaji) — only if no main title

If neither is found, the tool fails with:
```
AniDB returned incomplete data for anime ID XXXXX:
  Missing: main title
```

### Proposed Behavior

Extend the fallback chain:
1. `type="main"` (any language)
2. `type="official"` with `xml:lang="x-jat"` (romaji)
3. `type="official"` with `xml:lang="en"` (English)
4. `type="official"` (any language)
5. Any title (last resort)

Log a warning when using a fallback title so users know the source.

## Requirements

### Functional Requirements

1. Try all fallback options before returning `IncompleteData` error
2. Prefer titles in this priority order: main > official romaji > official English > official any > any
3. Log a warning when using non-primary fallback (level 3+)
4. Include the fallback type in verbose output

### Non-Functional Requirements

1. No additional API calls — use only data from the existing response
2. Minimal performance impact

## Implementation Guide

### Step 1: Update parse_anime_xml in api/client.rs

```rust
// Track multiple fallback options
let mut title_main: Option<String> = None;
let mut title_official_jat: Option<String> = None;
let mut title_official_en: Option<String> = None;
let mut title_official_any: Option<String> = None;
let mut title_any: Option<String> = None;

// In the title parsing loop:
if t_type == "main" {
    title_main = Some(text.clone());
} else if t_type == "official" {
    if t_lang == "x-jat" && title_official_jat.is_none() {
        title_official_jat = Some(text.clone());
    } else if t_lang == "en" && title_official_en.is_none() {
        title_official_en = Some(text.clone());
    } else if title_official_any.is_none() {
        title_official_any = Some(text.clone());
    }
}
// Always track any title as last resort
if title_any.is_none() {
    title_any = Some(text.clone());
}

// After parsing, resolve with fallback chain:
let (title_main, used_fallback) = if let Some(t) = title_main {
    (t, false)
} else if let Some(t) = title_official_jat {
    warn!("No main title found, using official romaji title");
    (t, true)
} else if let Some(t) = title_official_en {
    warn!("No main title found, using official English title");
    (t, true)
} else if let Some(t) = title_official_any {
    warn!("No main title found, using official title");
    (t, true)
} else if let Some(t) = title_any {
    warn!("No main title found, using fallback title");
    (t, true)
} else {
    return Err(ApiError::IncompleteData {
        anidb_id,
        field: "title (no titles found)".to_string(),
    });
};
```

### Step 2: Update AnimeInfo (optional)

Consider adding a field to indicate fallback was used:

```rust
pub struct AnimeInfo {
    pub anidb_id: u32,
    pub title_main: String,
    pub title_en: Option<String>,
    pub release_year: Option<u16>,
    pub used_fallback_title: bool,  // New field
}
```

### Step 3: Update tests

Add test cases for each fallback level.

## Test Cases

### Unit Tests

1. **test_fallback_to_official_romaji** — No main, has official x-jat
2. **test_fallback_to_official_english** — No main, no x-jat, has official en
3. **test_fallback_to_official_any** — No main, no x-jat, no en, has official ja
4. **test_fallback_to_any_title** — No main, no official, has synonym
5. **test_no_titles_still_errors** — Completely empty titles section
6. **test_main_title_preferred** — Has both main and official, uses main

### Test XML Examples

```xml
<!-- test_fallback_to_official_english -->
<anime id="99999">
  <titles>
    <title xml:lang="en" type="official">English Only Title</title>
  </titles>
</anime>

<!-- test_fallback_to_any_title -->
<anime id="99998">
  <titles>
    <title xml:lang="ja" type="synonym">Synonym Title</title>
  </titles>
</anime>
```

## Notes

- This is a robustness improvement, not a behavior change for normal cases
- Entries with proper main titles will behave exactly as before
- The warning logs help users understand when fallback is used
- Consider: should fallback titles be marked in the output directory name? (e.g., suffix with `[fallback]`) — probably not, keep it clean
