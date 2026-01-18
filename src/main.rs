mod api;
mod cache;
mod cli;
mod error;
mod logging;
mod output;
mod parser;
mod progress;
mod rename;
mod scanner;
mod validator;

use api::config_from_env;
use clap::Parser;
use cli::Args;
use error::AppError;
use output::{display_dry_run, display_execution_result};
use parser::DirectoryFormat;
use progress::Progress;
use rename::{build_anidb_name, rename_to_readable, RenameDirection, RenameOperation, RenameOptions, RenameResult};
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
    let mut progress = Progress::new();

    if let Some(history_file) = &args.revert {
        info!("Revert mode: {:?}", history_file);
        // TODO: Implement revert (feature 42)
    } else if let Some(target_dir) = &args.target_dir {
        // Step 1: Scan directory
        progress.scan_start(target_dir);
        let entries = scan_directory(target_dir)?;
        progress.scan_complete(entries.len());

        info!("Found {} subdirectories", entries.len());
        for entry in &entries {
            debug!("  {}", entry.name);
        }

        // Step 2: Validate format
        progress.validate_start();
        let validation = validate_directories(&entries)?;
        let format_name = match validation.format {
            DirectoryFormat::AniDb => "AniDB",
            DirectoryFormat::HumanReadable => "Human-readable",
        };
        progress.validate_complete(format_name);

        info!("All directories are in {:?} format", validation.format);

        // Step 3: Perform rename based on current format
        let result = match validation.format {
            DirectoryFormat::AniDb => {
                // AniDB -> Human-readable: requires API for metadata
                let api_config = config_from_env();

                if !api_config.is_configured() && !args.dry {
                    info!("API not configured, will use cached data if available");
                }

                let options = RenameOptions {
                    max_length: args.max_length,
                    dry_run: args.dry,
                    cache_expiry_days: args.cache_expiry,
                };

                progress.rename_start(
                    validation.directories.len(),
                    "AniDB -> Human-readable",
                );

                rename_to_readable(target_dir, &validation, &api_config, &options, &mut progress)?
            }
            DirectoryFormat::HumanReadable => {
                // Human-readable -> AniDB: no API needed
                let mut result = RenameResult::new(RenameDirection::ReadableToAniDb, args.dry);
                let total = validation.directories.len();

                progress.rename_start(total, "Human-readable -> AniDB");

                for (i, parsed) in validation.directories.iter().enumerate() {
                    let destination_name = build_anidb_name(
                        parsed.series_tag(),
                        parsed.anidb_id(),
                    );

                    let source_path = target_dir.join(parsed.original_name());

                    let op = RenameOperation::new(
                        source_path.clone(),
                        destination_name.clone(),
                        parsed.anidb_id(),
                        false,
                    );

                    // Check destination doesn't exist
                    if op.destination_path.exists() && !args.dry {
                        return Err(AppError::RenameError {
                            from: op.source_name.clone(),
                            to: op.destination_name.clone(),
                            source: std::io::Error::new(
                                std::io::ErrorKind::AlreadyExists,
                                "Destination already exists",
                            ),
                        });
                    }

                    progress.rename_progress(i + 1, total, &op.source_name, &op.destination_name);

                    // Execute rename if not dry run
                    if !args.dry {
                        std::fs::rename(&op.source_path, &op.destination_path)
                            .map_err(|e| AppError::RenameError {
                                from: op.source_name.clone(),
                                to: op.destination_name.clone(),
                                source: e,
                            })?;

                        info!("Renamed: {} -> {}", op.source_name, op.destination_name);
                    }

                    result.add_operation(op);
                }

                result
            }
        };

        progress.rename_complete(result.operations.len(), args.dry);

        // Display detailed results
        if args.dry {
            display_dry_run(&result, &mut std::io::stdout())
                .map_err(|e| AppError::Other(format!("Failed to display output: {}", e)))?;
        } else {
            display_execution_result(&result, &mut std::io::stdout())
                .map_err(|e| AppError::Other(format!("Failed to display output: {}", e)))?;
        }
    }

    Ok(())
}
