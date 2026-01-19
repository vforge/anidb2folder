# Security Policy

## Reporting a Vulnerability

If you discover a security vulnerability, please report it by emailing the maintainer directly rather than opening a public issue.

Include:
- Description of the vulnerability
- Steps to reproduce
- Potential impact

You can expect a response within 7 days.

## Scope

This project interacts with:
- Local filesystem (renaming directories)
- AniDB HTTP API (fetching anime metadata)
- Local cache files

Security considerations:
- API credentials are stored in `.env` (not committed to git)
- Cache files are stored in `~/.cache/anidb2folder/`
- History files are written to the target directory
