# Anidb2folder - High Level Overview

## General Description

- This document is a high-level description of small utility tool
- This is a cli tool that can be executed on linux machine, and should be build locally and on github actions as well
- this tool works on files in the local filesystem on subdirectories of single directory; it basically renames the directories; it does not touch next level of files/directories inside those directories and it does not touch any files inside those directory
  - the execution will look like this: `anidb2folder /path/to/directory/with/anidb/folders/`
- there needs to be a dry mode as well as history file to track changes
  - dry mode will simulate the changes and print them to stdout without making any changes to the filesystem: `anidb2folder --dry /path/to/directory/with/anidb/folders/`
  - the history file will be created in the target directory where the tool is executed; it will log all the changes made by the tool
- history file should be in json format with clear structure and a datetime (in the filename) when the changes were made - `<anidb2folder-history-YYYYMMDD-HHMMSS.json>`
- the format of the JSON should have as follows:
  - source directory/file path, destination directory, reason and reason of change
- history json file should allow to revert changes made by the tool COMPLETELY; changes done by the revert should also be logged in the history file as `<original filename>-revert-<YYYYMMDD-HHMMSS of revert>.json`
  - the revert command should look like this: `anidb2folder --revert /path/to/directory/with/anidb/folders/<anidb2folder-history-YYYYMMDD-HHMMSS.json>`
- the tool should have a verbose mode to log all the changes made
- the language used to build this tool is <Rust/Go/Python/?> (choose one)
- there should be a robust testing, with a lot of edge cases covered
- this tool should be built by github actions, added as a release to gihub, with each version, version should also include automated changelog based on the commit history

## Functional Description

- there are two distinct ways this tool affects directories when executed with a directory path (details follow):
  1. Renaming directories from AniDB ID to a more human-readable format.
  2. Renaming directories back from the human-readable format to the original AniDB ID format.
  3. Basically this is switching between two formats for all sub-directories of the given directory path - first execution renames from AniDB ID to human-readable format, second execution renames back from human-readable format to AniDB ID format.
- The tool should be able to identify directories named with AniDB IDs and rename them to a format that includes the anime title and release year (details follow)
- ALL subdirecotries of the given directory path should be looked at first and they should be in either of the two formats described below
  - if format of ony of the subdirectories is not recognized, the tool should immediately exit with an error message indicating which directories were not recognized; all directories should remain unchanged;; log should indicate that mixed formats were found and list the directories in each format
  - if not ALL subdirectories are in the same format (i.e. some are in AniDB format and some are in Human-Readable format), the tool should immediately exit with an error message indicating which directories were not recognized; all directories should remain unchanged; log should indicate that mixed formats were found and list the directories in each format

### AniDB format

- `[<series>] <anidb_id>`
  - `series` is optional string that will contain any characters except `]`, it will be enclosed in square brackets and will be PRESERVED during renaming
  - `anidb_id` is a numeric identifier corresponding to the anime in the AniDB database
  - examples: `[series] 12345`, `67890`, `[My Series] 54321`, `[AS0] 98765`, `12345`

### Human-Readable format

- `[<series>] <anime_title_jp> / <anime_title_en> (<release_year>) [anidb-<anidb_id>]`
  - `series` is the same optional string from the AniDB format, it will be PRESERVED during renaming
  - `anime_title_jp` is the Japanese title of the anime, fetched from AniDB, writren in romaji
  - `/` is unicode forward slash character used as a separator between Japanese and English titles that is compatible with filesystems, but looks like a slash
    - if there is no English title available, only the Japanese title will be used without the slash
  - `anime_title_en` is the English title of the anime, fetched from AniDB
    - if there is no English title available, this part (including the preceding slash) will be omitted
    - if the english title is the same as the Japanese title, this part (including the preceding slash) will be omitted
  - `release_year` is the year the anime was released, fetched from AniDB, enclosed in parentheses
    - if the release year is not available, this part (including the parentheses) will be omitted
  - `anidb_id` is the same numeric identifier from the AniDB format,
  - examples: `Naruto (2002) [anidb-12345]`, `[One Piece] One Piece (1999) [anidb-67890]`, `[FMA] Fullmetal Alchemist (2003) [anidb-54321]`, `[AS0] Cowboyu Bebopu / Cowboy Bebop (1998) [anidb-98765]`
- additional notes:
  - if the romaji or english title contains characters that are invalid for directory names on the target filesystem (e.g., `/`, `\`, `:`, `*`, `?`, `"`, `<`, `>`, `|` on Windows), those characters should be replaced with a similar unicode character that is valid for directory names (e.g., replace `/` with `／`, `\` with `＼`, `:` with `：`, `*` with `＊`, `?` with `？`, `"` with `＂`, `<` with `＜`, `>` with `＞`, `|` with `｜` on Windows)
  - if the resulting directory name exceeds typical filesystem limits (e.g., 255 characters on many filesystems), it should be truncated in a way that preserves as much information as possible, prioritizing the anime titles and release year over the series tag
    - for example, if truncation is necessary, the english title should be shortened first (with ellipsis replacing removed part), followed by shortening the japanese title; while ensuring that the series tag, anidb_id and release year remain intact in ALL cases, they CANNOT be truncated or removed EVER
    - this truncation logic should be clearly documented and consistently applied
    - if truncation occurs, the tool should log a warning indicating which directories were truncated and what the original and truncated names are (and this should be included in the history file as well)
    - if truncation occurs, the tool should ensure that the truncated name does not end with a space or punctuation character
    - the length limit for directory names should be configurable via an optional command-line argument, defaulting to 255 characters

### Where to source data from

- the tool should fetch anime titles and release years from AniDB using their public API
- the tool should handle API rate limiting gracefully, implementing retries with exponential backoff as needed
- the tool should cache fetched data locally to minimize API calls for subsequent executions; the cache should have a configurable expiration time (e.g., 24 hours, 30 days, 1 year)
- cache could be stored in two a simple JSON file in the target directory where the tool is executed or in a dedicated cache directory in the user's home directory
- if the cache file is corrupted or cannot be read, the tool should ignore it and fetch fresh data from the AniDB API, rebuilding the cache as needed
- cache should be a simple JSON with anidb, japanese title, english title, release year and last fetched datetime

## Tests

- there should be unit tests for all functions, especially those that handle string parsing, renaming logic, and API interactions
- there should be integration tests that simulate the entire renaming process, including dry runs and actual renaming
- there should be tests for edge cases, such as:
  - directories with missing or malformed AniDB IDs
  - directories with special characters in titles
  - directories that exceed typical filesystem name length limits
  - handling of API rate limiting and failures
  - cache expiration and corruption scenarios
- tests should be automated and run as part of the CI/CD pipeline on github actions
- test data should be included in the repository to facilitate testing without relying on live API calls
- mocking should be used for API calls to ensure tests are reliable and do not depend on external services
- tests should cover the revert functionality, ensuring that directories can be restored to their original names accurately
- tests should verify the integrity and correctness of the history JSON files generated during renaming and reverting operations
