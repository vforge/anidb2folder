mod api;
mod cache;
mod cli;
mod error;
mod history;
mod logging;
mod output;
mod parser;
mod progress;
mod rename;
mod revert;
mod scanner;
mod ui;
mod validator;

use api::config_from_env;
use clap::Parser;
use cli::Args;
use error::AppError;
use history::write_history;
use parser::DirectoryFormat;
use progress::Progress;
use rename::{
    build_anidb_name, rename_to_readable, RenameDirection, RenameOperation, RenameOptions,
    RenameResult,
};
use revert::{revert_from_history, RevertOptions};
use scanner::scan_directory;
use tracing::{debug, error, info};
use ui::{Ui, UiConfig};
use validator::validate_directories;

fn main() {
    // Load .env file if present (silently ignore if not found)
    let _ = dotenvy::dotenv();

    let args = Args::parse();

    // Convert verbose count to bool for UI/Progress
    let is_verbose = args.verbose > 0;

    // Initialize logging (only in verbose mode do we show tracing output)
    logging::init(args.verbose);

    // Create UI
    let ui_config = UiConfig::new(is_verbose);
    let mut ui = Ui::new(ui_config);

    // Show header
    ui.print_header(env!("CARGO_PKG_VERSION"));

    debug!("Environment loaded, checking API configuration");

    if let Err(e) = run(args, &mut ui) {
        error!("{}", e);
        ui.error(&e.detailed_message());
        std::process::exit(e.exit_code().into());
    }
}

fn run(args: Args, ui: &mut Ui) -> Result<(), AppError> {
    // Create progress for internal use (for functions that need it)
    let mut progress = Progress::new_with_ui(ui.is_verbose(), ui.is_colors_enabled());

    if let Some(history_file) = &args.revert {
        info!("Revert mode: {:?}", history_file);

        ui.info(&format!("Loading history from: {}", history_file.display()));

        let options = RevertOptions {
            dry_run: args.dry,
        };

        let result = revert_from_history(history_file, &options, &mut progress)
            .map_err(|e| AppError::Other(format!("Revert failed: {}", e)))?;

        // Display results
        display_revert_result(ui, &result);
    } else if let Some(target_dir) = &args.target_dir {
        // Step 1: Scan directory
        ui.step(&format!("Scanning {}", target_dir.display()));
        let entries = scan_directory(target_dir)?;
        ui.step_done();
        ui.kv("Found", &format!("{} directories", entries.len()));

        info!("Found {} subdirectories", entries.len());
        for entry in &entries {
            debug!("  {}", entry.name);
        }

        // Step 2: Validate format
        ui.step("Validating format");
        let validation = validate_directories(&entries)?;
        ui.step_done();

        let format_name = match validation.format {
            DirectoryFormat::AniDb => "AniDB",
            DirectoryFormat::HumanReadable => "Human-readable",
        };
        ui.kv("Format", format_name);

        info!("All directories are in {:?} format", validation.format);

        // Step 3: Perform rename based on current format
        ui.blank();

        let direction_str = match validation.format {
            DirectoryFormat::AniDb => "AniDB → Human-readable",
            DirectoryFormat::HumanReadable => "Human-readable → AniDB",
        };

        if args.dry {
            ui.boxed_title("DRY RUN");
        }

        ui.section(&format!("Renaming ({})", direction_str));
        ui.blank();

        let result = match validation.format {
            DirectoryFormat::AniDb => {
                // AniDB -> Human-readable: requires API for metadata
                let api_config = config_from_env();

                if !api_config.is_configured() && !args.dry {
                    ui.warning("API not configured, using cached data if available");
                    info!("API not configured, will use cached data if available");
                }

                let options = RenameOptions {
                    max_length: args.max_length,
                    dry_run: args.dry,
                    cache_expiry_days: args.cache_expiry,
                };

                rename_to_readable(target_dir, &validation, &api_config, &options, &mut progress)?
            }
            DirectoryFormat::HumanReadable => {
                // Human-readable -> AniDB: no API needed
                let mut result = RenameResult::new(RenameDirection::ReadableToAniDb, args.dry);
                let total = validation.directories.len();

                for (i, parsed) in validation.directories.iter().enumerate() {
                    let destination_name =
                        build_anidb_name(parsed.series_tag(), parsed.anidb_id());

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

                    ui.progress(i + 1, total, &format!("{} → {}", op.source_name, op.destination_name));

                    // Execute rename if not dry run
                    if !args.dry {
                        std::fs::rename(&op.source_path, &op.destination_path).map_err(|e| {
                            AppError::RenameError {
                                from: op.source_name.clone(),
                                to: op.destination_name.clone(),
                                source: e,
                            }
                        })?;

                        info!("Renamed: {} -> {}", op.source_name, op.destination_name);
                    }

                    result.add_operation(op);
                }

                result
            }
        };

        // Summary
        ui.blank();

        if args.dry {
            ui.dim(&format!(
                "{} directories would be renamed. Run without --dry to apply.",
                result.operations.len()
            ));
        } else {
            ui.success(&format!("{} directories renamed", result.operations.len()));

            // Write history file
            if !result.is_empty() {
                match write_history(&result, target_dir) {
                    Ok(history_path) => {
                        ui.dim(&format!("History: {}", history_path.display()));
                    }
                    Err(e) => {
                        ui.warning(&format!("Failed to write history: {}", e));
                    }
                }
            }
        }

        ui.blank();
    }

    Ok(())
}

fn display_revert_result(ui: &mut Ui, result: &revert::RevertResult) {
    ui.blank();

    if result.dry_run {
        ui.boxed_title("REVERT DRY RUN");
        ui.blank();
        ui.kv("History file", &result.original_history.display().to_string());
        ui.blank();
        ui.info(&format!(
            "Would revert {} directories:",
            result.operations.len()
        ));
        ui.blank();

        for op in &result.operations {
            ui.list_item(&op.current_name, &op.revert_name);
        }

        ui.blank();
        ui.dim("Run without --dry to apply these reverts.");
    } else {
        ui.boxed_title("REVERT COMPLETE");
        ui.blank();
        ui.success(&format!(
            "{} directories restored",
            result.operations.len()
        ));
        ui.blank();

        for op in &result.operations {
            ui.list_done(&op.current_name, &op.revert_name);
        }

        if let Some(ref history_path) = result.revert_history_path {
            ui.blank();
            ui.dim(&format!("Revert history: {}", history_path.display()));
        }
    }

    ui.blank();
}
