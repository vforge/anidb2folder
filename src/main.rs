mod cli;
mod logging;
mod scanner;

use anyhow::{Context, Result};
use clap::Parser;
use cli::Args;
use scanner::scan_directory;
use tracing::{debug, info};

fn main() -> Result<()> {
    let args = Args::parse();

    logging::init(args.verbose);

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
