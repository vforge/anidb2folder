use crate::api::AnimeInfo;

/// Configuration for name building
#[derive(Debug, Clone)]
pub struct NameBuilderConfig {
    pub max_length: usize,
}

impl Default for NameBuilderConfig {
    fn default() -> Self {
        Self { max_length: 255 }
    }
}

/// Result of building a name
#[derive(Debug, Clone)]
pub struct NameBuildResult {
    pub name: String,
    pub truncated: bool,
}

/// Build a human-readable directory name from anime info
pub fn build_human_readable_name(
    series_tag: Option<&str>,
    info: &AnimeInfo,
    config: &NameBuilderConfig,
) -> NameBuildResult {
    let mut parts: Vec<String> = Vec::new();

    // Series tag
    if let Some(tag) = series_tag {
        parts.push(format!("[{}]", tag));
    }

    // Titles - use fullwidth slash separator if different
    let title_part = build_title_part(&info.title_main, info.title_en.as_deref());
    parts.push(title_part);

    // Year
    if let Some(year) = info.release_year {
        parts.push(format!("({})", year));
    }

    // AniDB ID suffix (always required)
    parts.push(format!("[anidb-{}]", info.anidb_id));

    // Join and sanitize
    let raw_name = parts.join(" ");
    let sanitized = sanitize_filename(&raw_name);

    // Truncate if needed
    if sanitized.len() > config.max_length {
        let truncated_name = truncate_name(series_tag, info, config.max_length);

        NameBuildResult {
            name: truncated_name,
            truncated: true,
        }
    } else {
        NameBuildResult {
            name: sanitized,
            truncated: false,
        }
    }
}

/// Build the title part of the name
fn build_title_part(title_main: &str, title_en: Option<&str>) -> String {
    match title_en {
        Some(en) if en != title_main && !en.is_empty() => {
            // Use fullwidth slash as separator (／)
            format!("{} ／ {}", title_main, en)
        }
        _ => title_main.to_string(),
    }
}

/// Sanitize filename by replacing invalid characters
/// This is a basic implementation - feature 30 will provide more complete sanitization
fn sanitize_filename(name: &str) -> String {
    let invalid_chars = ['/', '\\', ':', '*', '?', '"', '<', '>', '|'];

    name.chars()
        .map(|c| {
            if invalid_chars.contains(&c) {
                '_'
            } else {
                c
            }
        })
        .collect()
}

/// Truncate name to fit within max length while preserving required parts
/// This is a basic implementation - feature 31 will provide smarter truncation
fn truncate_name(series_tag: Option<&str>, info: &AnimeInfo, max_length: usize) -> String {
    // Required suffix: [anidb-ID]
    let suffix = format!("[anidb-{}]", info.anidb_id);
    let suffix_len = suffix.len();

    // Optional prefix: [series_tag]
    let prefix = series_tag.map(|t| format!("[{}] ", t)).unwrap_or_default();
    let prefix_len = prefix.len();

    // Optional year: (YYYY)
    let year_part = info
        .release_year
        .map(|y| format!(" ({})", y))
        .unwrap_or_default();
    let year_len = year_part.len();

    // Calculate available space for title
    // Format: [prefix] title [year] [suffix]
    // Need at least 1 space before suffix
    let fixed_len = prefix_len + year_len + 1 + suffix_len;

    if fixed_len >= max_length {
        // Can't even fit the fixed parts, just use minimal format
        return format!("{}... {}", &info.title_main[..3.min(info.title_main.len())], suffix);
    }

    let available_for_title = max_length - fixed_len;

    // Use only main title when truncating (drop English title)
    let title = sanitize_filename(&info.title_main);

    let truncated_title = if title.len() > available_for_title {
        // Truncate with ellipsis
        let truncate_at = available_for_title.saturating_sub(3);
        format!("{}...", &title[..truncate_at.min(title.len())])
    } else {
        title
    };

    format!("{}{}{} {}", prefix, truncated_title, year_part, suffix)
}

/// Build an AniDB format directory name
pub fn build_anidb_name(series_tag: Option<&str>, anidb_id: u32) -> String {
    match series_tag {
        Some(tag) => format!("[{}] {}", tag, anidb_id),
        None => anidb_id.to_string(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn create_test_info(
        id: u32,
        title_main: &str,
        title_en: Option<&str>,
        year: Option<u16>,
    ) -> AnimeInfo {
        AnimeInfo {
            anidb_id: id,
            title_main: title_main.to_string(),
            title_en: title_en.map(|s| s.to_string()),
            release_year: year,
        }
    }

    #[test]
    fn test_build_name_full() {
        let info = create_test_info(1, "Cowboy Bebop", Some("Cowboy Bebop"), Some(1998));

        let result = build_human_readable_name(Some("AS0"), &info, &NameBuilderConfig::default());

        // Same title shouldn't be duplicated
        assert_eq!(result.name, "[AS0] Cowboy Bebop (1998) [anidb-1]");
        assert!(!result.truncated);
    }

    #[test]
    fn test_build_name_different_titles() {
        let info = create_test_info(1, "Kauboi Bibappu", Some("Cowboy Bebop"), Some(1998));

        let result = build_human_readable_name(Some("AS0"), &info, &NameBuilderConfig::default());

        assert_eq!(
            result.name,
            "[AS0] Kauboi Bibappu ／ Cowboy Bebop (1998) [anidb-1]"
        );
        assert!(!result.truncated);
    }

    #[test]
    fn test_build_name_no_series() {
        let info = create_test_info(12345, "Naruto", None, Some(2002));

        let result = build_human_readable_name(None, &info, &NameBuilderConfig::default());

        assert_eq!(result.name, "Naruto (2002) [anidb-12345]");
    }

    #[test]
    fn test_build_name_no_year() {
        let info = create_test_info(999, "Unknown Anime", None, None);

        let result = build_human_readable_name(None, &info, &NameBuilderConfig::default());

        assert_eq!(result.name, "Unknown Anime [anidb-999]");
    }

    #[test]
    fn test_build_name_same_titles_not_duplicated() {
        let info = create_test_info(69, "One Piece", Some("One Piece"), Some(1999));

        let result = build_human_readable_name(None, &info, &NameBuilderConfig::default());

        // Should not include duplicate title
        assert_eq!(result.name, "One Piece (1999) [anidb-69]");
    }

    #[test]
    fn test_build_name_with_special_chars() {
        let info = create_test_info(123, "Title: With/Special*Chars?", None, Some(2020));

        let result = build_human_readable_name(None, &info, &NameBuilderConfig::default());

        // Special chars should be sanitized
        assert!(!result.name.contains('/'));
        assert!(!result.name.contains(':'));
        assert!(!result.name.contains('*'));
        assert!(!result.name.contains('?'));
        assert!(result.name.contains("Title_ With_Special_Chars_"));
    }

    #[test]
    fn test_build_name_truncation() {
        let long_title = "A".repeat(300);
        let info = create_test_info(1, &long_title, None, Some(2020));

        let config = NameBuilderConfig { max_length: 100 };
        let result = build_human_readable_name(None, &info, &config);

        assert!(result.truncated);
        assert!(result.name.len() <= 100);
        assert!(result.name.contains("..."));
        assert!(result.name.ends_with("[anidb-1]"));
    }

    #[test]
    fn test_sanitize_filename() {
        let input = "Test: File/Name*With?Invalid<Chars>And|More\"Stuff";
        let result = sanitize_filename(input);

        assert_eq!(
            result,
            "Test_ File_Name_With_Invalid_Chars_And_More_Stuff"
        );
    }

    #[test]
    fn test_build_anidb_name_with_series() {
        let result = build_anidb_name(Some("AS0"), 12345);
        assert_eq!(result, "[AS0] 12345");
    }

    #[test]
    fn test_build_anidb_name_without_series() {
        let result = build_anidb_name(None, 12345);
        assert_eq!(result, "12345");
    }
}
