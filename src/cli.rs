use clap::Parser;
use std::path::PathBuf;

#[derive(Parser, Debug)]
#[command(name = "anidb2folder")]
#[command(author, version, about, long_about = None)]
#[command(about = "Rename anime directories between AniDB ID and human-readable formats")]
pub struct Args {
    /// Target directory containing anime subdirectories
    #[arg(required_unless_present = "revert")]
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
}
