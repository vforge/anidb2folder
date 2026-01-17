mod api;
mod cli;
mod error;
mod logging;
mod parser;
mod scanner;
mod validator;

use clap::Parser;
use cli::Args;
use error::AppError;
use scanner::scan_directory;
use tracing::{debug, error, info};
use validator::validate_directories;

fn main() {
    // Load .env file if present (silently ignore if not found)
    let _ = dotenvy::dotenv();

    let args = Args::parse();

    logging::init(args.verbose);

    debug!("Environment loaded, checking API configuration");

    if let Err(e) = run(args) {
        error!("{}", e);
        eprintln!("\nError: {}", e.detailed_message());
        std::process::exit(e.exit_code().into());
    }
}

fn run(args: Args) -> Result<(), AppError> {
    if let Some(history_file) = &args.revert {
        info!("Revert mode: {:?}", history_file);
        // TODO: Implement revert (feature 42)
    } else if let Some(target_dir) = &args.target_dir {
        let entries = scan_directory(target_dir)?;

        info!("Found {} subdirectories", entries.len());

        for entry in &entries {
            debug!("  {}", entry.name);
        }

        // Validate all directories are in same format
        let validation = validate_directories(&entries)?;

        info!(
            "All directories are in {:?} format",
            validation.format
        );

        // TODO: Implement main operation (features 20, 21)
    }

    Ok(())
}
