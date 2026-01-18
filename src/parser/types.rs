use thiserror::Error;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DirectoryFormat {
    AniDb,
    HumanReadable,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AniDbFormat {
    pub series_tag: Option<String>,
    pub anidb_id: u32,
    pub original_name: String,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct HumanReadableFormat {
    pub series_tag: Option<String>,
    pub title_jp: String,
    pub title_en: Option<String>,
    pub release_year: Option<u16>,
    pub anidb_id: u32,
    pub original_name: String,
}

#[derive(Debug, Clone, PartialEq, Eq)]
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

#[derive(Error, Debug, PartialEq, Eq)]
pub enum ParseError {
    #[error("Directory name does not match any known format: {0}")]
    UnrecognizedFormat(String),
}
