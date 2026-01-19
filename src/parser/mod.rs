mod types;

pub use types::*;

use once_cell::sync::Lazy;
use regex::Regex;

// AniDB format: [<series>] <anidb_id>
// Examples: "12345", "[AS0] 12345", "[My Series] 67890"
static ANIDB_REGEX: Lazy<Regex> = Lazy::new(|| Regex::new(r"^(?:\[([^\]]+)\]\s*)?(\d+)$").unwrap());

// Human-readable format: [<series>] <title_jp> ／ <title_en> (<year>) [anidb-<id>]
// The unicode slash ／ (U+FF0F) separates JP and EN titles
static HUMAN_READABLE_REGEX: Lazy<Regex> = Lazy::new(|| {
    Regex::new(r"^(?:\[([^\]]+)\]\s*)?(.*?)\s*(?:\((\d{4})\))?\s*\[anidb-(\d+)\]$").unwrap()
});

// Regex to split JP/EN titles on unicode slash
static TITLE_SPLIT_REGEX: Lazy<Regex> = Lazy::new(|| Regex::new(r"\s*／\s*").unwrap());

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
    let release_year: Option<u16> = captures.get(3).and_then(|m| m.as_str().parse().ok());
    let anidb_id: u32 = captures.get(4)?.as_str().parse().ok()?;

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
        let result =
            parse_directory_name("[AS0] Cowboyu Bebopu ／ Cowboy Bebop (1998) [anidb-1]").unwrap();

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
        let result = parse_directory_name("Steins;Gate (Anime) (2011) [anidb-7729]").unwrap();

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

    // ============ Helper Method Tests ============

    #[test]
    fn test_parsed_directory_methods() {
        let anidb = parse_directory_name("[S1] 123").unwrap();
        assert_eq!(anidb.format(), DirectoryFormat::AniDb);
        assert_eq!(anidb.anidb_id(), 123);
        assert_eq!(anidb.series_tag(), Some("S1"));
        assert_eq!(anidb.original_name(), "[S1] 123");

        let human = parse_directory_name("Test (2020) [anidb-456]").unwrap();
        assert_eq!(human.format(), DirectoryFormat::HumanReadable);
        assert_eq!(human.anidb_id(), 456);
        assert!(human.series_tag().is_none());
    }
}
