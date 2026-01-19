use clap::Parser;
use std::path::PathBuf;

#[derive(Parser, Debug)]
#[command(name = "anidb2folder")]
#[command(author, version, about, long_about = None)]
#[command(about = "Rename anime directories between AniDB ID and human-readable formats")]
pub struct Args {
    /// Target directory containing anime subdirectories
    #[arg(required_unless_present_any = ["revert", "cache_info", "cache_clear", "cache_prune"])]
    pub target_dir: Option<PathBuf>,

    /// Simulate changes without modifying the filesystem
    #[arg(short, long)]
    pub dry: bool,

    /// Increase verbosity (-v, -vv, -vvv)
    #[arg(short, long, action = clap::ArgAction::Count)]
    pub verbose: u8,

    /// Revert changes using a history file
    #[arg(short, long, value_name = "HISTORY_FILE")]
    pub revert: Option<PathBuf>,

    /// Maximum directory name length
    #[arg(short = 'l', long, default_value = "255")]
    pub max_length: usize,

    /// Cache expiration in days
    #[arg(short, long, default_value = "30")]
    pub cache_expiry: u32,

    /// Show cache information for a directory
    #[arg(long, value_name = "DIR")]
    pub cache_info: Option<PathBuf>,

    /// Clear all cached entries for a directory
    #[arg(long, value_name = "DIR")]
    pub cache_clear: Option<PathBuf>,

    /// Remove expired cache entries for a directory
    #[arg(long, value_name = "DIR")]
    pub cache_prune: Option<PathBuf>,
}
