# Features Documentation Index

This document describes the structure and organization of feature documentation for anidb2folder.

---

## Overview

Each feature is documented as a separate markdown file in the `docs/features/` directory. Features are designed to be **atomic, self-contained units** that can be implemented and tested independently (respecting their dependencies).

---

## File Naming Convention

```
docs/features/<index>-<feature-name>.md
```

### Index Ranges

| Range | Category | Description |
|-------|----------|-------------|
| `00-04` | Foundation | Core infrastructure, CLI, logging, errors |
| `05-09` | Parsing | Format detection and validation |
| `10-19` | Data Layer | API integration, caching, data handling |
| `20-29` | Core Logic | Renaming operations |
| `30-39` | Safety | Sanitization, truncation |
| `40-49` | Operations | Dry run, history, revert |
| `50-59` | UI/UX | User interface and experience improvements |
| `60-69` | Enhancements | Safety and quality-of-life improvements |
| `99` | Independent | Features with no strict implementation order |

---

## Feature Document Structure

Each feature file follows this template:

```markdown
# Feature Title

## Summary
One-line description of what this feature does.

## Dependencies
List of feature indices this feature depends on.

## Description
Detailed explanation of the feature's purpose and scope.

## Requirements
Specific requirements and acceptance criteria.

## Implementation Guide
Suggested approach to implementing this feature.

## Test Cases
Key scenarios that must be tested.

## Notes
Additional considerations, edge cases, or future enhancements.
```

---

## Feature List

### Foundation (00-04)

| Index | Feature | Dependencies | Description |
|-------|---------|--------------|-------------|
| [00](features/00-cli-scaffold.md) | CLI Scaffold | None | Basic CLI application structure |
| [01](features/01-directory-scanner.md) | Directory Scanner | 00 | Scan and list subdirectories |
| [02](features/02-verbose-mode.md) | Verbose Mode | 00 | Detailed logging output |
| [03](features/03-error-handling.md) | Error Handling | 00 | Standardized error codes and messages |

### Parsing (05-09)

| Index | Feature | Dependencies | Description |
|-------|---------|--------------|-------------|
| [05](features/05-format-parser.md) | Format Parser | 01 | Parse and identify directory formats |
| [06](features/06-format-validator.md) | Format Validator | 05 | Validate all directories match one format |

### Data Layer (10-19)

| Index | Feature | Dependencies | Description |
|-------|---------|--------------|-------------|
| [10](features/10-anidb-api-client.md) | AniDB API Client | 00, 03 | Fetch anime data from AniDB |
| [11](features/11-local-cache.md) | Local Cache | 10 | Cache API responses locally |

### Core Logic (20-29)

| Index | Feature | Dependencies | Description |
|-------|---------|--------------|-------------|
| [20](features/20-rename-to-readable.md) | Rename to Readable | 06, 11 | Convert AniDB format to human-readable |
| [21](features/21-rename-to-anidb.md) | Rename to AniDB | 06 | Convert human-readable to AniDB format |

### Safety (30-39)

| Index | Feature | Dependencies | Description |
|-------|---------|--------------|-------------|
| [30](features/30-character-sanitizer.md) | Character Sanitizer | 05 | Replace invalid filesystem characters |
| [31](features/31-name-truncation.md) | Name Truncation | 30 | Handle directory name length limits |

### Operations (40-49)

| Index | Feature | Dependencies | Description |
|-------|---------|--------------|-------------|
| [40](features/40-dry-run-mode.md) | Dry Run Mode | 20, 21 | Preview changes without execution |
| [41](features/41-history-tracking.md) | History Tracking | 20, 21 | Log all changes to JSON history file |
| [42](features/42-revert-operation.md) | Revert Operation | 41 | Restore directories from history file |

### Enhancements (60-69)

| Index | Feature | Dependencies | Description |
|-------|---------|--------------|-------------|
| [60](features/60-revert-safety-validation.md) | Revert Safety Validation | 42 | Validate target directory before revert operation |
| [61](features/61-cache-management.md) | Cache Management CLI | 11 | CLI commands for cache info, clear, prune |
| 62 | Use Direction Descriptions | 20, 21 | Use RenameDirection::description() instead of hardcoded strings in main.rs |
| [63](features/63-fallback-title-handling.md) | Fallback Title Handling | 10 | Use fallback titles when main title is missing |
| [64](features/64-web-fallback-on-ban.md) | Web Fallback on Ban | 10 | Scrape web pages when API is rate-limited/banned |

### Independent (99)

| Index | Feature | Dependencies | Description |
|-------|---------|--------------|-------------|
| [99](features/99-github-actions-release.md) | GitHub Actions Release | None | CI/CD pipeline and automated releases |

---

## Dependency Graph

```
                                    ┌─────────────────┐
                                    │ 99-github-actions│
                                    └─────────────────┘
                                           (independent)

┌────────────┐
│ 00-cli     │
└──┬──┬──┬───┘
   │  │  │
   │  │  └────────────────────────────────┐
   │  │                                   │
   ▼  ▼                                   ▼
┌────────────┐  ┌────────────┐     ┌────────────┐
│ 01-scanner │  │ 02-verbose │     │ 03-errors  │
└─────┬──────┘  └────────────┘     └──────┬─────┘
      │                                   │
      ▼                                   │
┌────────────┐                            │
│ 05-parser  │────────────────────┐       │
└─────┬──────┘                    │       │
      │                           ▼       │
      ▼                     ┌────────────┐│
┌────────────┐              │ 30-sanitize││
│ 06-validate│              └─────┬──────┘│
└─────┬──────┘                    │       │
      │                           ▼       │
      │                     ┌────────────┐│
      │                     │ 31-truncate││
      │                     └────────────┘│
      │                                   │
      │         ┌────────────┐            │
      │         │ 10-api     │◄───────────┘
      │         └─────┬──────┘
      │               │
      │               ▼
      │         ┌────────────┐
      │         │ 11-cache   │
      │         └─────┬──────┘
      │               │
      ▼               ▼
┌────────────┐  ┌────────────┐
│ 21-to-anid │  │ 20-to-read │
└─────┬──────┘  └─────┬──────┘
      │               │
      └───────┬───────┘
              │
      ┌───────┴───────┐
      ▼               ▼
┌────────────┐  ┌────────────┐
│ 40-dry-run │  │ 41-history │
└────────────┘  └─────┬──────┘
                      │
                      ▼
                ┌────────────┐
                │ 42-revert  │
                └────────────┘
```

---

## Implementation Order

For a complete implementation, follow this order:

1. **Phase 1 - Foundation**
   - 00-cli-scaffold
   - 01-directory-scanner
   - 02-verbose-mode
   - 03-error-handling

2. **Phase 2 - Parsing & Safety**
   - 05-format-parser
   - 06-format-validator
   - 30-character-sanitizer
   - 31-name-truncation

3. **Phase 3 - Data Layer**
   - 10-anidb-api-client
   - 11-local-cache

4. **Phase 4 - Core Operations**
   - 20-rename-to-readable
   - 21-rename-to-anidb
   - 40-dry-run-mode

5. **Phase 5 - History & Revert**
   - 41-history-tracking
   - 42-revert-operation

6. **Anytime**
   - 99-github-actions-release (can be set up early for CI)

---

## Status Tracking

Use this section to track implementation progress:

| Feature | Status | Notes |
|---------|--------|-------|
| 00-cli-scaffold | ✅ Complete | |
| 01-directory-scanner | ✅ Complete | |
| 02-verbose-mode | ✅ Complete | |
| 03-error-handling | ✅ Complete | |
| 05-format-parser | ✅ Complete | |
| 06-format-validator | ✅ Complete | |
| 10-anidb-api-client | ✅ Complete | |
| 11-local-cache | ✅ Complete | |
| 20-rename-to-readable | ✅ Complete | |
| 21-rename-to-anidb | ✅ Complete | Implemented in main.rs |
| 30-character-sanitizer | ✅ Complete | Inline in name_builder.rs |
| 31-name-truncation | ✅ Complete | Inline in name_builder.rs |
| 40-dry-run-mode | ✅ Complete | |
| 41-history-tracking | ✅ Complete | |
| 42-revert-operation | ✅ Complete | |
| 60-revert-safety | ✅ Complete | |
| 61-cache-management | ✅ Complete | |
| 62-direction-descriptions | ✅ Complete | |
| 63-fallback-title-handling | ⬜ Not Started | |
| 64-web-fallback-on-ban | ⬜ Not Started | |
| 99-github-actions-release | ⬜ Not Started | |

**Legend:** ⬜ Not Started | 🟡 In Progress | ✅ Complete | ❌ Blocked
