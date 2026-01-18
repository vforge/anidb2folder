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

/// Character replacement mappings for filesystem safety
/// Uses fullwidth Unicode characters that look similar to ASCII originals
const REPLACEMENTS: &[(char, char)] = &[
    ('/', '／'),  // U+FF0F Fullwidth Solidus
    ('\\', '＼'), // U+FF3C Fullwidth Reverse Solidus
    (':', '：'),  // U+FF1A Fullwidth Colon
    ('*', '＊'),  // U+FF0A Fullwidth Asterisk
    ('?', '？'),  // U+FF1F Fullwidth Question Mark
    ('"', '＂'),  // U+FF02 Fullwidth Quotation Mark
    ('<', '＜'),  // U+FF1C Fullwidth Less-Than Sign
    ('>', '＞'),  // U+FF1E Fullwidth Greater-Than Sign
    ('|', '｜'),  // U+FF5C Fullwidth Vertical Line
    ('`', '\''),  // Backtick to single quote
];

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

    // Titles - use fullwidth slash separator if different and EN not contained in JP
    let title_part = build_title_part(&info.title_main, info.title_en.as_deref());
    parts.push(title_part);

    // Year - only add if not already present in titles
    if let Some(year) = info.release_year {
        let year_str = year.to_string();
        let title_contains_year = info.title_main.contains(&year_str)
            || info
                .title_en
                .as_ref()
                .map(|en| en.contains(&year_str))
                .unwrap_or(false);

        if !title_contains_year {
            parts.push(format!("({})", year));
        }
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
/// Skips EN title if:
/// - It's the same as main title
/// - It's empty
/// - It's contained within the main title (e.g., JP: "Vakhiin/Vakhii", EN: "Vakhii")
fn build_title_part(title_main: &str, title_en: Option<&str>) -> String {
    match title_en {
        Some(en) if !en.is_empty() && en != title_main && !title_main.contains(en) => {
            // Use fullwidth slash as separator (／)
            format!("{} ／ {}", title_main, en)
        }
        _ => title_main.to_string(),
    }
}

/// Sanitize filename by replacing invalid characters with fullwidth Unicode equivalents
pub fn sanitize_filename(name: &str) -> String {
    let mut result = String::with_capacity(name.len());
    let mut last_was_space = true; // Treat start as after space to trim leading

    for c in name.chars() {
        // Skip control characters (ASCII 0-31)
        if c.is_ascii_control() {
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

        // Replace invalid characters with fullwidth equivalents
        if let Some(&(_, replacement)) = REPLACEMENTS.iter().find(|&&(from, _)| from == c) {
            result.push(replacement);
        } else {
            result.push(c);
        }
    }

    // Trim trailing space
    if result.ends_with(' ') {
        result.pop();
    }

    result
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

    // ============ Basic Name Building ============

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

    // ============ EN Title Contained in JP Title ============

    #[test]
    fn test_en_title_contained_in_jp_uses_only_jp() {
        // JP title contains EN title (e.g., "Vakhiin/Vakhii" contains "Vakhii")
        let info = create_test_info(123, "Vakhiin/Vakhii", Some("Vakhii"), Some(2020));

        let result = build_human_readable_name(None, &info, &NameBuilderConfig::default());

        // Should use only JP title since EN is contained within it
        assert_eq!(result.name, "Vakhiin／Vakhii (2020) [anidb-123]");
        assert!(!result.name.contains(" ／ Vakhii")); // No separate EN title
    }

    #[test]
    fn test_en_title_substring_of_jp_uses_only_jp() {
        let info = create_test_info(456, "Mobile Suit Gundam", Some("Gundam"), Some(1979));

        let result = build_human_readable_name(None, &info, &NameBuilderConfig::default());

        // EN "Gundam" is substring of JP "Mobile Suit Gundam"
        assert_eq!(result.name, "Mobile Suit Gundam (1979) [anidb-456]");
    }

    #[test]
    fn test_jp_title_not_containing_en_shows_both() {
        let info = create_test_info(789, "Shingeki no Kyojin", Some("Attack on Titan"), Some(2013));

        let result = build_human_readable_name(None, &info, &NameBuilderConfig::default());

        // EN is not contained in JP, so both should appear
        assert_eq!(
            result.name,
            "Shingeki no Kyojin ／ Attack on Titan (2013) [anidb-789]"
        );
    }

    // ============ Year Already in Title ============

    #[test]
    fn test_year_in_main_title_not_duplicated() {
        let info = create_test_info(100, "Anime 2020", None, Some(2020));

        let result = build_human_readable_name(None, &info, &NameBuilderConfig::default());

        // Year is already in title, should not add (2020) suffix
        assert_eq!(result.name, "Anime 2020 [anidb-100]");
        assert!(!result.name.contains("(2020)"));
    }

    #[test]
    fn test_year_in_en_title_not_duplicated() {
        let info = create_test_info(101, "Anime Movie", Some("Anime Movie 2021"), Some(2021));

        let result = build_human_readable_name(None, &info, &NameBuilderConfig::default());

        // Year is in EN title, should not add (2021) suffix
        assert!(!result.name.contains("(2021)"));
    }

    #[test]
    fn test_different_year_in_title_still_adds_correct_year() {
        // Title has "2019" but release year is 2020
        let info = create_test_info(102, "Anime 2019 Remaster", None, Some(2020));

        let result = build_human_readable_name(None, &info, &NameBuilderConfig::default());

        // 2019 != 2020, so year should be added
        assert!(result.name.contains("(2020)"));
        assert_eq!(
            result.name,
            "Anime 2019 Remaster (2020) [anidb-102]"
        );
    }

    #[test]
    fn test_year_not_in_title_adds_year() {
        let info = create_test_info(103, "Normal Anime", None, Some(2023));

        let result = build_human_readable_name(None, &info, &NameBuilderConfig::default());

        // No year in title, should add (2023)
        assert_eq!(result.name, "Normal Anime (2023) [anidb-103]");
    }

    // ============ Character Sanitization - Fullwidth Replacements ============

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

    #[test]
    fn test_replace_backtick_with_single_quote() {
        let result = sanitize_filename("It`s a test");
        assert_eq!(result, "It's a test");
    }

    #[test]
    fn test_multiple_backticks() {
        let result = sanitize_filename("`Hello` `World`");
        assert_eq!(result, "'Hello' 'World'");
    }

    // ============ Multiple Replacements ============

    #[test]
    fn test_multiple_replacements() {
        let result = sanitize_filename("Title: Part 1/2 <Special>");
        assert_eq!(result, "Title： Part 1／2 ＜Special＞");
    }

    #[test]
    fn test_all_invalid_chars_replaced() {
        let input = "/\\:*?\"<>|`";
        let result = sanitize_filename(input);
        assert_eq!(result, "／＼：＊？＂＜＞｜'");
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

    #[test]
    fn test_only_spaces() {
        let result = sanitize_filename("     ");
        assert_eq!(result, "");
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

    #[test]
    fn test_remove_tab_and_newline() {
        let result = sanitize_filename("Title\tWith\nNewline");
        assert_eq!(result, "TitleWithNewline");
    }

    // ============ Unicode Preservation ============

    #[test]
    fn test_unicode_preserved() {
        let input = "日本語タイトル";
        let result = sanitize_filename(input);
        assert_eq!(result, input);
    }

    #[test]
    fn test_mixed_unicode_and_invalid() {
        let result = sanitize_filename("アニメ: Title/日本");
        assert_eq!(result, "アニメ： Title／日本");
    }

    // ============ No Changes Needed ============

    #[test]
    fn test_no_changes_needed() {
        let input = "Normal Title (2020) [anidb-12345]";
        let result = sanitize_filename(input);
        assert_eq!(result, input);
    }

    // ============ Full Name Building with Sanitization ============

    #[test]
    fn test_build_name_with_special_chars() {
        let info = create_test_info(123, "Title: With/Special*Chars?", None, Some(2020));

        let result = build_human_readable_name(None, &info, &NameBuilderConfig::default());

        // Special chars should be replaced with fullwidth
        assert!(result.name.contains("Title："));
        assert!(result.name.contains("／"));
        assert!(result.name.contains("＊"));
        assert!(result.name.contains("？"));
        assert_eq!(
            result.name,
            "Title： With／Special＊Chars？ (2020) [anidb-123]"
        );
    }

    #[test]
    fn test_build_name_with_backticks() {
        let info = create_test_info(200, "It`s My Life", None, Some(2022));

        let result = build_human_readable_name(None, &info, &NameBuilderConfig::default());

        assert_eq!(result.name, "It's My Life (2022) [anidb-200]");
    }

    // ============ Truncation ============

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

    // ============ AniDB Name Building ============

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
