# AGENTS.md

Instructions for AI coding assistants working on this project.

## Project Overview

**anidb2folder** is a Rust CLI tool that renames anime directories between two formats:

- **AniDB format:** `[series] 12345`
- **Human-readable format:** `[series] Title ／ English Title (2024) [anidb-12345]`

## ⚠️ Required Reading

**Before making ANY changes, you MUST read:**

1. [docs/high-level-overview.md](docs/high-level-overview.md) — Project architecture and design decisions
2. [docs/features.md](docs/features.md) — Feature index, dependencies, and implementation order
3. The specific feature file in `docs/features/` for the task at hand

**Do not assume knowledge about this project. The documentation is the source of truth.**

## Documentation Structure

```
docs/
├── high-level-overview.md    # Architecture, formats, error handling
├── features.md               # Feature index, status, and dependency graph
└── features/                 # Individual feature specifications
```

See [docs/features.md](docs/features.md) for the complete feature list and implementation status.

## Do's ✅

- **Read the feature documentation** before implementing any feature
- **Follow the implementation order** defined in `docs/features.md`
- **Respect feature dependencies** — don't implement features before their dependencies
- **Use the error handling patterns** defined in `03-error-handling.md`
- **Use the logging patterns** defined in `02-verbose-mode.md`
- **Write tests** for all public functions
- **Use the character sanitization** from feature 30 when constructing directory names
- **Update `docs/features.md` status tracking** when completing features
- **Preserve the `[series]` tag** in all rename operations
- **Use Unicode lookalikes** for invalid filesystem characters (see feature 30)

## Don'ts ❌

- **Don't skip reading documentation** — assumptions lead to incorrect implementations
- **Don't ignore feature dependencies** — the order exists for safety reasons
- **Don't make API calls without rate limiting** — AniDB will ban the client
- **Don't perform filesystem operations without dry-run support**
- **Don't truncate names arbitrarily** — follow the truncation rules in feature 31
- **Don't hardcode paths** — use the provided path arguments
- **Don't assume directory format** — always validate first (feature 06)
- **Don't mix concerns** — keep features modular as documented

## Tech Stack

| Component | Choice |
|-----------|--------|
| Language | Rust |
| CLI parsing | `clap` v4.4+ (derive macros) |
| Logging | `tracing` + `tracing-subscriber` |
| HTTP client | `reqwest` (blocking, rustls-tls) |
| XML parsing | `quick-xml` |
| Serialization | `serde` + `serde_json` |
| Error handling | `thiserror` (library) / `anyhow` (application) |

## Key Conventions

### Directory Formats

```
AniDB:          [series] 12345
Human-readable: [series] Title ／ English Title (2024) [anidb-12345]
```

- The `[series]` tag is **optional** and must be preserved if present
- Use fullwidth solidus `／` (U+FF0F) as title separator, not regular `/`
- The AniDB ID suffix `[anidb-XXXXX]` is **required** in human-readable format

### Error Exit Codes

| Code | Meaning |
|------|---------|
| 0 | Success |
| 1 | General error |
| 2 | Invalid arguments |
| 3 | Directory not found |
| 4 | Mixed formats |
| 5 | Unrecognized format |
| 6 | API error |
| 7 | Permission error |
| 8 | History error |
| 9 | Rename error |
| 10 | Cache error |

### File Naming

- Feature files: `XX-feature-name.md` (XX = index number)
- History files: `.anidb2folder-YYYYMMDD-HHMMSS.history.json`
- Cache location: `~/.cache/anidb2folder/`

## Testing

When implementing features:

1. Unit test all parsing and formatting logic
2. Use mock HTTP responses for API tests
3. Use temporary directories for filesystem tests
4. Test both success and error paths
5. Test edge cases documented in each feature file

## Development

Use `./run.sh <command>` for common tasks. See [README.md](README.md) for available commands.

## Common Tasks

### Implementing a New Feature

1. Read the feature's documentation in `docs/features/`
2. Check dependencies are implemented
3. Follow the structure and requirements in the doc
4. Write tests covering the requirements
5. Update status in `docs/features.md`

### Debugging

1. Enable verbose mode: `--verbose` or `-v`
2. Check history files in target directory
3. Inspect cache at `~/.cache/anidb2folder/`

### Adding a Dependency

1. Update `Cargo.toml`
2. Document why in commit message
3. Prefer well-maintained, minimal crates

## Commit Guidelines

**Only commit after all tests pass.**

### Format

```
<emoji> <type>: <description>
```

Single line, max 72 characters. Be specific and meaningful.

### Commit Types & Emojis

| Emoji | Type | Use for |
|-------|------|---------|
| ✨ | `feat` | New feature |
| 🐛 | `fix` | Bug fix |
| 📝 | `docs` | Documentation only |
| ♻️ | `refactor` | Code restructure (no behavior change) |
| 🧪 | `test` | Adding or updating tests |
| 🔧 | `chore` | Build, config, dependencies |
| 🚀 | `perf` | Performance improvement |
| 🎨 | `style` | Formatting, whitespace |

### Examples

```
✨ feat: implement format parser for AniDB directories
🐛 fix: handle missing English title in API response
📝 docs: add edge cases to format-parser spec
🧪 test: add unit tests for character sanitizer
♻️ refactor: extract rate limiter into separate module
🔧 chore: add reqwest dependency for HTTP client
```

### Rules

1. **Run tests first** — `cargo test` must pass before committing
2. **One logical change per commit** — don't mix features with fixes
3. **Reference feature index** — mention `[feat-XX]` when relevant
4. **No WIP commits** — every commit should be complete and working

## Questions to Ask

If requirements are unclear, ask about:

- Which directory format to expect as input
- Whether to use `--dry-run` for testing
- Specific anime IDs for testing
- Expected behavior for edge cases
