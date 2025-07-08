# Gemini Notes: rdump Project

This file contains notes and observations about the `rdump` project to aid in development.

## Project Overview

- **Purpose**: `rdump` appears to be a command-line file search and query tool.
- **Primary Language**: The core logic is written in **Rust**.
- **Query Language**: It uses a custom query language, likely defined in `rdump/src/rql.pest` and parsed by `rdump/src/parser.rs`.
- **Language-Aware Features**: The presence of `rdump/src/predicates/code_aware/` and language-specific tests (`tests/go_search.rs`, `tests/java_search.rs`, etc.) suggests it has code-aware search capabilities.
- **Configuration**: Configuration is likely managed through `.rdump.toml`.

## Key Files & Directories

- `rdump/src/main.rs`: The main entry point for the application.
- `rdump/src/parser.rs`: Handles parsing the rdump query language.
- `rdump/src/evaluator.rs`: Evaluates the parsed query against the filesystem.
- `rdump/src/predicates/`: Contains the logic for different search filters (e.g., `name`, `path`, `size`, `contains`).
- `tests/`: Contains integration and CLI tests.
- `insane_test_bed/`: A directory with a wide variety of files used for testing `rdump`'s capabilities.

## Development Notes

- **Build**: The project is built using `cargo build` from the `rdump` directory.
- **Run**: The application can be run with `cargo run` from the `rdump` directory.
- **Test**: Tests are executed with `cargo test` from the `rdump` directory.
- **Formatting**: Assumed to follow standard Rust formatting (`rustfmt`).

## Development Rules

1.  **Always Add Tests**: For any bug fix or feature addition, a corresponding test must be added to verify the change and prevent regressions.
2.  **Log All Changes**: All modifications, including bug fixes and new features, must be logged in the `GEMINI_EDITS.log` file. The log entry should be dated and clearly describe the change.
