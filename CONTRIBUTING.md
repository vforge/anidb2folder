# Contributing

Thanks for your interest in contributing to anidb2folder!

## Getting Started

1. Fork and clone the repository
2. Install Rust (stable)
3. Set up git hooks: `git config core.hooksPath .githooks`
4. Copy `.env.example` to `.env` and add your AniDB API credentials

## Development

```bash
./run.sh check    # Run fmt, clippy, and tests
./run.sh fmt      # Format code
./run.sh test     # Run tests only
```

## Before Submitting

- Run `./run.sh check` (or let the pre-commit hook do it)
- Add tests for new functionality
- Update documentation if needed

## Commit Messages

Use the format: `<emoji> <type>: <description>`

| Emoji | Type | Use for |
|-------|------|---------|
| ✨ | feat | New feature |
| 🐛 | fix | Bug fix |
| 📝 | docs | Documentation |
| ♻️ | refactor | Code restructure |
| 🧪 | test | Tests |
| 🔧 | chore | Build, config |

## Code Style

- Run `cargo fmt` before committing
- No clippy warnings (`cargo clippy -- -D warnings`)
- Write tests for public functions

## Questions?

Open an issue for questions or discussion.
