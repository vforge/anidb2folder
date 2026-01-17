mod cli;
mod scanner;

use anyhow::{Context, Result};
use clap::Parser;
use cli::Args;
use scanner::scan_directory;
use tracing::{debug, info};
use tracing_subscriber::EnvFilter;

fn main() -> Result<()> {
    let args = Args::parse();

    let filter = if args.verbose {
        EnvFilter::new("debug")
    } else {
        EnvFilter::new("info")
    };

    tracing_subscriber::fmt().with_env_filter(filter).init();

    if let Some(history_file) = &args.revert {
        info!("Revert mode: {:?}", history_file);
        // TODO: Implement revert (feature 42)
    } else if let Some(target_dir) = &args.target_dir {
        let entries = scan_directory(target_dir).context("Failed to scan target directory")?;

        info!("Found {} subdirectories", entries.len());

        for entry in &entries {
            debug!("  {}", entry.name);
        }

        // TODO: Implement main operation (features 20, 21)
    }

    Ok(())
}
