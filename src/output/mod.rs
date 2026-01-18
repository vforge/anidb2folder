use crate::rename::RenameResult;
use std::io::{self, Write};

/// Display dry run results in a formatted output
pub fn display_dry_run(result: &RenameResult, writer: &mut impl Write) -> io::Result<()> {
    writeln!(writer)?;
    writeln!(writer, "========================================")?;
    writeln!(writer, "              DRY RUN")?;
    writeln!(writer, "========================================")?;
    writeln!(writer)?;
    writeln!(writer, "Direction:  {}", result.direction.description())?;
    writeln!(writer, "Operations: {}", result.operations.len())?;
    writeln!(writer)?;

    if result.operations.is_empty() {
        writeln!(writer, "No directories to rename.")?;
        return Ok(());
    }

    writeln!(writer, "Planned changes:")?;
    writeln!(writer)?;

    for (i, op) in result.operations.iter().enumerate() {
        writeln!(writer, "  {}. [anidb-{}]", i + 1, op.anidb_id)?;
        writeln!(writer, "     From: {}", op.source_name)?;
        writeln!(writer, "     To:   {}", op.destination_name)?;

        if op.truncated {
            writeln!(writer, "     [!] Name truncated to fit filesystem limits")?;
        }

        writeln!(writer)?;
    }

    // Summary
    writeln!(writer, "----------------------------------------")?;
    writeln!(writer, "Summary:")?;
    writeln!(
        writer,
        "  {} directories would be renamed",
        result.operations.len()
    )?;

    let truncated_count = result.truncated_count();
    if truncated_count > 0 {
        writeln!(writer, "  {} names would be truncated", truncated_count)?;
    }

    writeln!(writer)?;
    writeln!(writer, "Run without --dry to apply these changes.")?;

    Ok(())
}

/// Display dry run results in a simple tab-separated format for scripting
pub fn display_dry_run_simple(result: &RenameResult, writer: &mut impl Write) -> io::Result<()> {
    for op in &result.operations {
        writeln!(
            writer,
            "{}\t{}\t{}",
            op.anidb_id, op.source_name, op.destination_name
        )?;
    }
    Ok(())
}

/// Display execution results (non-dry-run)
pub fn display_execution_result(result: &RenameResult, writer: &mut impl Write) -> io::Result<()> {
    writeln!(writer)?;
    writeln!(
        writer,
        "Successfully renamed {} directories.",
        result.operations.len()
    )?;

    let truncated_count = result.truncated_count();
    if truncated_count > 0 {
        writeln!(writer, "  {} names were truncated.", truncated_count)?;
    }

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::rename::{RenameDirection, RenameOperation};
    use std::path::PathBuf;

    fn create_test_result(dry_run: bool) -> RenameResult {
        let mut result = RenameResult::new(RenameDirection::AniDbToReadable, dry_run);

        result.add_operation(RenameOperation::new(
            PathBuf::from("/test/12345"),
            "Anime Title (2020) [anidb-12345]".to_string(),
            12345,
            false,
        ));

        result.add_operation(RenameOperation::new(
            PathBuf::from("/test/[X] 99"),
            "[X] Very Long Title That Was Truncated... [anidb-99]".to_string(),
            99,
            true,
        ));

        result
    }

    #[test]
    fn test_display_dry_run() {
        let result = create_test_result(true);
        let mut output = Vec::new();

        display_dry_run(&result, &mut output).unwrap();

        let output_str = String::from_utf8(output).unwrap();

        assert!(output_str.contains("DRY RUN"));
        assert!(output_str.contains("AniDB -> Human-readable"));
        assert!(output_str.contains("12345"));
        assert!(output_str.contains("Anime Title (2020) [anidb-12345]"));
        assert!(output_str.contains("truncated"));
        assert!(output_str.contains("2 directories would be renamed"));
        assert!(output_str.contains("1 names would be truncated"));
    }

    #[test]
    fn test_display_dry_run_empty() {
        let result = RenameResult::new(RenameDirection::AniDbToReadable, true);
        let mut output = Vec::new();

        display_dry_run(&result, &mut output).unwrap();

        let output_str = String::from_utf8(output).unwrap();

        assert!(output_str.contains("DRY RUN"));
        assert!(output_str.contains("No directories to rename"));
    }

    #[test]
    fn test_display_dry_run_simple() {
        let result = create_test_result(true);
        let mut output = Vec::new();

        display_dry_run_simple(&result, &mut output).unwrap();

        let output_str = String::from_utf8(output).unwrap();
        let lines: Vec<&str> = output_str.lines().collect();

        assert_eq!(lines.len(), 2);
        assert!(lines[0].contains("12345"));
        assert!(lines[0].contains("Anime Title"));
        assert!(lines[1].contains("99"));
    }

    #[test]
    fn test_display_execution_result() {
        let result = create_test_result(false);
        let mut output = Vec::new();

        display_execution_result(&result, &mut output).unwrap();

        let output_str = String::from_utf8(output).unwrap();

        assert!(output_str.contains("Successfully renamed 2 directories"));
        assert!(output_str.contains("1 names were truncated"));
    }
}
