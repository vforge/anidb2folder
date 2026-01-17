mod cli;

use anyhow::Result;
use clap::Parser;
use cli::Args;
use tracing::info;
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
        info!("Target directory: {:?}", target_dir);
        info!("Dry run: {}", args.dry);
        // TODO: Implement main operation (features 20, 21)
    }

    Ok(())
}
