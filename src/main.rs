mod api;
mod cache;
mod cli;
mod error;
mod logging;
mod output;
mod parser;
mod rename;
mod scanner;
mod validator;

use clap::Parser;
use cli::Args;
use error::AppError;
use output::display_dry_run;
use parser::DirectoryFormat;
use rename::{RenameDirection, RenameOperation, RenameResult};
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

        // Determine rename direction based on current format
        let direction = match validation.format {
            DirectoryFormat::AniDb => RenameDirection::AniDbToReadable,
            DirectoryFormat::HumanReadable => RenameDirection::ReadableToAniDb,
        };

        // Build rename operations
        let mut result = RenameResult::new(direction, args.dry);

        for parsed in &validation.directories {
            // For now, create placeholder operations showing what would happen
            // Full implementation will come with features 20/21
            let destination_name = match direction {
                RenameDirection::AniDbToReadable => {
                    // TODO: Fetch from API and build human-readable name (feature 20)
                    format!(
                        "{}Title [anidb-{}]",
                        parsed
                            .series_tag()
                            .map(|s| format!("[{}] ", s))
                            .unwrap_or_default(),
                        parsed.anidb_id()
                    )
                }
                RenameDirection::ReadableToAniDb => {
                    // Build AniDB format name (feature 21)
                    format!(
                        "{}{}",
                        parsed
                            .series_tag()
                            .map(|s| format!("[{}] ", s))
                            .unwrap_or_default(),
                        parsed.anidb_id()
                    )
                }
            };

            let source_path = target_dir.join(parsed.original_name());

            result.add_operation(RenameOperation::new(
                source_path,
                destination_name,
                parsed.anidb_id(),
                false, // Truncation check will come with feature 31
            ));
        }

        // Display results
        if args.dry {
            display_dry_run(&result, &mut std::io::stdout())
                .map_err(|e| AppError::Other(format!("Failed to display output: {}", e)))?;
        } else {
            // TODO: Execute actual renames (features 20, 21)
            info!(
                "Would rename {} directories (use --dry to preview)",
                result.len()
            );
        }
    }

    Ok(())
}
