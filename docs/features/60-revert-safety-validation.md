# 60 - Revert Safety Validation

## Summary

Validate target directory before revert operation to prevent accidental reverts on wrong directories.

## Dependencies

- **42-revert-operation** — Requires revert functionality to be implemented

## Description

This feature adds safety validation to the revert flow. When a user runs `--revert`, the tool should clearly indicate which directory will be affected and optionally validate that the user-provided target directory matches the history file's target directory.

The existing `validate_for_revert` function in `history/reader.rs` provides the core validation logic but is not yet integrated into the revert flow.

## Requirements

### Functional Requirements

1. When `--revert` is used without `target_dir`:
   - Display which directory will be affected (from history file)
   - Show clear warning before proceeding

2. When `--revert` is used with `target_dir`:
   - Validate that `target_dir` matches history file's `target_directory`
   - Fail with clear error if directories don't match

3. Display the target directory path prominently in revert output

### Non-Functional Requirements

1. Clear, unambiguous error messages
2. No silent failures or unexpected directory operations

## Implementation Guide

### Step 1: Update main.rs revert flow

```rust
// In the revert branch of run()
if let Some(history_file) = &args.revert {
    // Read history first to get target directory
    let history = read_history(history_file)
        .map_err(|e| AppError::Other(format!("Failed to read history: {}", e)))?;

    // Display target directory prominently
    ui.kv("Target directory", &history.target_directory.display().to_string());

    // If user provided target_dir, validate it matches
    if let Some(target_dir) = &args.target_dir {
        validate_for_revert(&history, target_dir)
            .map_err(|e| AppError::Other(format!("Directory mismatch: {}", e)))?;
        ui.success("Target directory verified");
    } else {
        // Warn user about which directory will be affected
        ui.warning(&format!(
            "Will revert changes in: {}",
            history.target_directory.display()
        ));
    }

    // Continue with revert...
}
```

### Step 2: Remove dead code annotation

Remove `#[allow(dead_code)]` from `validate_for_revert` in `history/reader.rs`.

### Step 3: Export validate_for_revert

Ensure `validate_for_revert` is exported from the history module.

## Test Cases

### Unit Tests

1. Test validation passes when directories match
2. Test validation fails when directories don't match
3. Test revert proceeds correctly after validation

### Integration Tests

1. Test `--revert history.json /correct/path` succeeds
2. Test `--revert history.json /wrong/path` fails with clear error
3. Test `--revert history.json` (no target) shows warning with correct path

## Notes

- This is a safety feature to prevent user error
- The validation is already implemented in `validate_for_revert`, just needs integration
- Consider adding `--force` flag in future to skip validation if needed
