Of course. Here is the final, complete `README.md` that incorporates all of the project's features, architectural details, and the crucial user tips we discussed.

---

# `rdump` &mdash; The Definitive Developer's Guide to Code-Aware Search

**`rdump` is a next-generation, command-line tool for developers. It finds and processes files by combining filesystem metadata, content matching, and deep structural code analysis.**

[![Build Status](https://img.shields.io/github/actions/workflow/status/user/repo/rust.yml?branch=main)](https://github.com/user/repo/actions)
[![Crates.io](https://img.shields.io/crates/v/rdump.svg)](https://crates.io/crates/rdump)
[![License](https://img.shields.io/crates/l/rdump.svg)](https://github.com/user/repo/blob/main/LICENSE)

It's a developer's swiss-army knife for code discovery. It goes beyond the text-based search of tools like `grep` and `ripgrep` by using **tree-sitter** to parse your code into a syntax tree. This allows you to ask questions that are impossible for other tools to answer efficiently:

- *"Find the 'User' struct definition, but only in non-test Rust files."*
- *"Show me every call to 'console.log' in my JavaScript files with 3 lines of context."*
- *"List all React components that use the `useState` hook but are not wrapped in `React.memo`."*
- *"List all Python files larger than 10KB that import 'requests' and were modified in the last week."*

`rdump` is written in Rust for blazing-fast performance, ensuring that even complex structural queries on large codebases are executed in moments.

---

## Table of Contents

1.  [**Why `rdump`?**](#1-why-rdump-a-comparative-look)
    - [The Problem with Text-Based Search](#the-problem-with-text-based-search)
    - [The `rdump` Solution: Structural Awareness](#the-rdump-solution-structural-awareness)
    - [Comparison with Other Tools](#comparison-with-other-tools)
2.  [**Architecture, Frameworks, and Libraries: A Technical Deep Dive**](#2-architecture-frameworks-and-libraries-a-technical-deep-dive)
    - [The Core Philosophy](#the-core-philosophy)
    - [Data Flow & Component Breakdown](#data-flow--component-breakdown)
3.  [**Installation**](#3-installation)
    - [With Cargo (Recommended)](#with-cargo-recommended)
    - [From Pre-compiled Binaries](#from-pre-compiled-binaries)
    - [From Source](#from-source)
4.  [**Practical Recipes for Real-World Use**](#4-practical-recipes-for-real-world-use)
    - [Code Auditing & Security](#code-auditing--security)
    - [Refactoring & Maintenance](#refactoring--maintenance)
    - [React Component Analysis](#react-component-analysis)
    - [DevOps & Automation](#devops--automation)
5.  [**The `rdump` Query Language (RQL) &mdash; A Deep Dive**](#5-the-rdump-query-language-rql--a-deep-dive)
    - [Core Concepts & Syntax](#core-concepts--syntax)
    - [Important: Always Quote Your Query!](#important-always-quote-your-query)
    - [Evaluation Order & Performance Tips](#evaluation-order--performance-tips)
    - [Predicate Reference: Metadata](#predicate-reference-metadata)
    - [Predicate Reference: Content](#predicate-reference-content)
    - [Predicate Reference: Code-Aware (Semantic)](#predicate-reference-code-aware-semantic)
    - [Predicate Reference: React-Specific](#predicate-reference-react-specific)
    - [Advanced Querying Techniques](#advanced-querying-techniques)
6.  [**Command Reference**](#6-command-reference)
    - [`rdump search`](#rdump-search)
    - [`rdump lang`](#rdump-lang)
    - [`rdump preset`](#rdump-preset)
7.  [**Output Formats: A Visual Guide**](#7-output-formats-a-visual-guide)
8.  [**Configuration**](#8-configuration)
    - [The `config.toml` File](#the-configtoml-file)
    - [The `.rdumpignore` System](#the-rdumpignore-system)
9.  [**Extending `rdump`: Adding a New Language**](#9-extending-rdump-adding-a-new-language)
10. [**Troubleshooting & FAQ**](#10-troubleshooting--faq)
11. [**Performance Benchmarks**](#11-performance-benchmarks)
12. [**Contributing**](#12-contributing)
13. [**License**](#13-license)

---

## 1. Why `rdump`? A Comparative Look

### The Problem with Text-Based Search

For decades, developers have relied on text-based search tools like `grep`, `ack`, and `ripgrep`. These tools are phenomenal for finding literal strings and regex patterns. However, they share a fundamental limitation: **they don't understand code.** They see a file as a flat sequence of characters.

This leads to noisy and inaccurate results for code-related questions. A `grep` for `User` will find:
- The `struct User` definition.
- A variable named `NewUser`.
- A function parameter `user_permission`.
- Comments mentioning `User`.
- String literals like `"Failed to create User"`.

### The `rdump` Solution: Structural Awareness

`rdump` sees code the way a compiler does: as a structured tree of nodes. It uses the powerful `tree-sitter` library to parse source code into a Concrete Syntax Tree (CST).

This means you can ask for `struct:User`, and `rdump` will navigate the syntax tree to find **only the node representing the definition of the `User` struct**. This is a paradigm shift in code search.

### Comparison with Other Tools

| Feature | `ripgrep` / `grep` | `semgrep` | **`rdump`** |
| :--- | :--- | :--- | :--- |
| **Search Paradigm** | Regex / Literal Text | Abstract Semantic Patterns | **Metadata + Content + Code Structure** |
| **Primary Use Case**| Finding specific lines of text | Enforcing static analysis rules | **Interactive code exploration & filtering**|
| **Speed** | Unmatched for text search | Fast for patterns | **Very fast; optimizes by layer** |
| **Query `func:foo`** | `grep "func foo"` (noisy) | `pattern: function foo(...)` | `func:foo` (precise) |
| **Query `size:>10kb`** | No | No | `size:>10kb` (built-in) |
| **Query `hook:useState`** | `grep "useState"` (noisy) | `pattern: useState(...)` | `hook:useState` (precise) |
| **Combine Filters** | Possible via shell pipes | Limited | **Natively via RQL (`&`, `|`, `!`)** |

---

## 2. Architecture, Frameworks, and Libraries: A Technical Deep Dive

`rdump`'s power and simplicity are not accidental; they are the result of deliberate architectural choices and the leveraging of best-in-class libraries from the Rust ecosystem. This section details how these pieces fit together to create a performant, modular, and extensible tool.

### The Core Philosophy: A Pipeline of Composable Filters

At its heart, `rdump` is a highly optimized pipeline. It starts with a massive set of potential files and, at each stage, applies progressively more powerful (and expensive) filters to narrow down the set.

1.  **Declarative Interface:** The user experience is paramount. We define *what* we want, not *how* to get it.
2.  **Composition over Inheritance:** Functionality is built from small, single-purpose, reusable units (predicates, formatters). This avoids complex class hierarchies and makes the system easy to reason about.
3.  **Extensibility by Design:** The architecture anticipates change. Adding a new language or predicate requires adding new data/modules, not rewriting the core evaluation logic.
4.  **Performance Through Layering:** Cheap checks (metadata) are performed first to minimize the work for expensive checks (code parsing).

### Data Flow & Component Breakdown

```
[Query String] -> [1. CLI Parser (clap)] -> [2. RQL Parser (pest)] -> [AST] -> [3. Evaluator Engine] -> [Matched Files] -> [7. Formatter (syntect)] -> [Final Output]
                                                                                    |
                                                                                    V
                                                                    [4. Predicate Trait System]
                                                                                    |
                                                                                    +------> [Metadata Predicates (ignore, glob)]
                                                                                    |
                                                                                    +------> [Content Predicates (regex)]
                                                                                    |
                                                                                    +------> [6. Semantic Engine (tree-sitter)]
                                                                                    |
                                                                    [5. Parallel File Walker (rayon)]
```

#### 1. CLI Parsing: `clap`

-   **Library:** `clap` (Command Line Argument Parser)
-   **Role:** `clap` is the face of `rdump`. It provides a declarative macro-based API to define the entire CLI structure: subcommands (`search`, `lang`, `preset`), flags (`--format`, `-C`), and arguments (`<QUERY_PARTS>`). It handles automatic help generation, type-safe parsing, and validation, providing a robust entry point.

#### 2. RQL Parser: `pest`

-   **Library:** `pest` (Parser-Expressive Syntax Trees)
-   **Role:** `pest` transforms the human-readable RQL query string (e.g., `"ext:rs & (struct:User | !path:tests)"`) into a machine-readable Abstract Syntax Tree (AST). The entire grammar is defined in `src/rql.pest`, decoupling the language syntax from the Rust code that processes it. `pest` provides excellent error reporting for invalid queries.

#### 3. The Evaluator Engine

-   **Library:** Standard Rust
-   **Role:** The evaluator is the brain. It recursively walks the `AstNode` tree generated by `pest`. If it sees a `LogicalOp`, it calls itself on its children. If it sees a `Predicate`, it dispatches to the predicate system. Crucially, it performs short-circuiting (e.g., in `A & B`, if `A` is false, `B` is never evaluated), which is a key performance optimization.

#### 4. The Predicate System: Rust's Trait System

-   **Library:** Standard Rust (specifically, `trait` objects)
-   **Role:** This is the heart of `rdump`'s modularity. Each predicate (`ext`, `size`, `func`, etc.) is an independent module that implements a common `PredicateEvaluator` trait. The evaluator holds a `HashMap` registry to dynamically dispatch to the correct predicate's `evaluate()` method at runtime. This design makes adding new predicates trivial without altering the core engine.

#### 5. Parallel File Walker: `ignore` & `rayon`

-   **Libraries:** `ignore`, `rayon`
-   **Role:** The file search is a massively parallel problem.
    -   The `ignore` crate provides an extremely fast, parallel directory traversal that automatically respects `.gitignore`, `.rdumpignore`, and other ignore patterns.
    -   `rayon` is used in the main evaluation pass to process the pre-filtered file list across all available CPU cores. Converting a sequential iterator to a parallel one is a one-line change (`.iter()` -> `.par_iter()`), providing effortless, safe, and scalable performance.

#### 6. The Semantic Engine: `tree-sitter`

-   **Library:** `tree-sitter` and its Rust binding.
-   **Role:** `tree-sitter` is the universal parser that powers all code-aware predicates. It takes source code text and produces a concrete syntax tree. The core semantic logic executes `tree-sitter` queries (defined in `.scm` files) against this tree, making the engine language-agnostic. A language is "supported" by providing data (a grammar and query files), not by writing new Rust code.

#### 7. The Formatter & Syntax Highlighting: `syntect`

-   **Library:** `syntect`
-   **Role:** The formatter takes the final list of matched files and hunks and presents them to the user. `syntect` uses the same syntax and theme definitions as Sublime Text, providing robust and beautiful highlighting. The `Format` enum allows `rdump` to cleanly dispatch to different printing functions based on the user's choice (e.g., `hunks`, `json`, `markdown`).

---

## 3. Installation

### With Cargo (Recommended)
If you have the Rust toolchain (`rustup`), you can install directly from Crates.io. This command will download the source, compile it, and place the binary in your Cargo home directory.
```sh
cargo install rdump
```

### From Pre-compiled Binaries
Pre-compiled binaries for Linux, macOS, and Windows are available on the [**GitHub Releases**](https://github.com/user/repo/releases) page. Download the appropriate archive, extract the `rdump` executable, and place it in a directory on your system's `PATH`.

### From Source
To build `rdump` from source, you'll need `git` and the Rust toolchain.```sh
git clone https://github.com/user/repo.git
cd rdump
cargo build --release
# The executable will be at ./target/release/rdump
./target/release/rdump --help```
---

## 4. Practical Recipes for Real-World Use

### Code Auditing & Security

-   **Find potential hardcoded secrets, ignoring test data:**
    ```sh
    rdump "str:/[A-Za-z0-9_\\-]{20,}/ & !path:test"
    ```
-   **Locate all disabled or skipped tests:**
    ```sh
    rdump "(comment:ignore | comment:skip) & name:*test*"
    ```
-   **Find all raw SQL queries that are not in a `db` or `repository` package:**
    ```sh
    rdump "str:/SELECT.*FROM/ & !(path:/db/ | path:/repository/)"
    ```

### Refactoring & Maintenance

-   **Find all call sites of a function to analyze its usage before changing its signature:**
    ```sh
    rdump "call:process_payment" --format hunks -C 3
    ```
-   **Identify "god files" that might need to be broken up:**
    List Go files over 50KB.
    ```sh
    rdump "ext:go & size:>50kb" --format find
    ```
-   **Clean up dead code:** Find functions that have no corresponding calls within the project.
    ```sh
    # This is a two-step process, but rdump helps find the candidates
    rdump "ext:py & func:." --format json > funcs.json
    # Then, a script could check which function names from funcs.json are never found with a `call:` query.
    ```

### React Component Analysis

-   **Find all React components using `useState` but not `useCallback`, which could indicate performance issues:**
    ```sh
    rdump "ext:tsx & hook:useState & !hook:useCallback"
    ```
-   **List all custom hooks defined in the project:**
    ```sh
    rdump "customhook:." --format hunks
    ```
-   **Find all usages of a specific component, e.g., `<Button>`, that are missing a `disabled` prop:**
    ```sh
    rdump "element:Button & !prop:disabled"
    ```

### DevOps & Automation

-   **Find all Dockerfiles that don't pin to a specific image digest:**
    ```sh
    rdump "name:Dockerfile & !contains:/@sha256:/"
    ```
-   **List all TOML configuration files larger than 1KB that have been changed in the last 2 days:**
    ```sh
    rdump "ext:toml & size:>1kb & modified:<2d" --format find
    ```
-   **Pipe files to another command:** Delete all `.tmp` files older than a week.
    ```sh
    rdump "ext:tmp & modified:>7d" --format paths | xargs rm -v
    ```

---

## 5. The `rdump` Query Language (RQL) &mdash; A Deep Dive

### Core Concepts & Syntax

-   **Predicates:** The building block of RQL is the `key:value` pair (e.g., `ext:rs`).
-   **Operators:** Combine predicates with `&` (or `and`), `|` (or `or`).
-   **Negation:** `!` (or `not`) negates a predicate or group (e.g., `!ext:md`).
-   **Grouping:** `()` controls the order of operations (e.g., `ext:rs & (contains:foo | contains:bar)`).
-   **Quoting:** Use `'` or `"` for values with spaces or special characters (e.g., `contains:'fn main()'`).

### Important: Always Quote Your Query!

Your shell (Bash, Zsh, etc.) interprets characters like `&` and `|` before `rdump` does. To prevent errors, **always wrap your entire query in double quotes**.

-   **INCORRECT:** `rdump ext:rs & contains:foo`
    -   The shell tries to run `rdump ext:rs` in the background. This is not what you want.
-   **INCORRECT:** `rdump ext:rs && contains:foo`
    -   `rdump` doesn't understand the `&&` operator. Its operator is a single `&`.
-   **CORRECT:** `rdump "ext:rs & contains:foo"`
    -   The shell passes the entire string `"ext:rs & contains:foo"` as a single argument to `rdump`, which can then parse it correctly.

### Evaluation Order & Performance Tips

`rdump` is fast, but you can make it even faster by writing efficient queries. The key is to **eliminate the most files with the cheapest predicates first.**

-   **GOOD:** `ext:rs & struct:User`
    -   *Fast.* `rdump` first finds all `.rs` files (very cheap), then runs the expensive `struct` parser only on that small subset.
-   **BAD:** `struct:User & ext:rs`
    -   *Slow.* While `rdump`'s engine is smart enough to likely re-order this during pre-filtering, writing it this way is logically less efficient. It implies parsing every file to look for a struct, then checking its extension.
-   **BEST:** `path:models/ & ext:rs & struct:User`
    -   *Blazing fast.* The search space is narrowed by path, then extension, before any files are even opened.

**Golden Rule:** Always lead with `path:`, `in:`, `name:`, or `ext:` if you can.

### Predicate Reference: Metadata

| Key | Example | Description |
| :--- | :--- | :--- |
| `ext` | `ext:ts` | Matches file extension. Case-insensitive. |
| `name`| `name:"*_test.go"` | Matches filename (basename) against a case-insensitive glob pattern. |
| `path`| `path:src/api` | Matches if the substring appears anywhere in the full path. Supports glob patterns. |
| `in` | `in:"src/api"` | Matches if a file is in the *exact* directory `src/api`. Not recursive. |
| `in` | `in:"src/**"` | With a glob, matches files recursively under `src`. |
| `size`| `size:>=10kb` | Filters by size. Operators: `>`, `<`, `=`. Units: `b`, `kb`, `mb`, `gb`. |
| `modified`| `modified:<2d` | Filters by modification time. Operators: `>`, `<`, `=`. Units: `s`, `m`, `h`, `d`, `w`, `y`. |

### Predicate Reference: Content

| Key | Example | Description |
| :--- | :--- | :--- |
| `contains` | `contains:"// HACK"` | Case-insensitive literal substring search. |
| `matches` | `matches:"\\w+_SECRET"` | Case-sensitive regex search on file content. |

### Predicate Reference: Code-Aware (Semantic)

This is a general list. Use `rdump lang list` and `rdump lang describe <language>` to see what's available for a specific language.

| Key | Example | Description | Supported In |
| :--- | :--- | :--- | :--- |
| `def` | `def:User` | Finds a generic definition (class, struct, trait, etc.). | All |
| `func`| `func:get_user` | Finds a function or method definition. | All |
| `import`| `import:serde` | Finds an import/use/require statement. | All |
| `call`| `call:println` | Finds a function or method call site. | All |
| `struct`| `struct:Point` | Finds a `struct` definition. | Rust, Go |
| `class`| `class:ApiHandler`| Finds a `class` definition. | Python, JS, TS, Java |
| `enum`| `enum:Status` | Finds an `enum` definition. | Rust, TS, Java |
| `trait` | `trait:Runnable` | Finds a `trait` definition. | Rust |
| `impl` | `impl:User` | Finds an `impl` block. | Rust |
| `type` | `type:UserID` | Finds a `type` alias. | Rust, TS, Go |
| `interface`| `interface:Serializable`| Finds an `interface` definition. | TS, Go, Java |
| `macro` | `macro:println` | Finds a macro definition. | Rust |
| `comment`| `comment:TODO` | Finds text within any comment node. | All |
| `str` | `str:"api_key"` | Finds text within any string literal node. | All |

### Predicate Reference: React-Specific

For files with `.jsx` and `.tsx` extensions.

| Key | Example | Description |
| :--- | :--- | :--- |
| `component` | `component:App` | Finds a React component definition (class, function, or memoized). |
| `element` | `element:div` | Finds a JSX element tag (e.g., `div`, `MyComponent`). |
| `hook` | `hook:useState` | Finds a call to a hook (any function starting with `use...`). |
| `customhook`| `customhook:useAuth`| Finds the *definition* of a custom hook. |
| `prop` | `prop:onClick` | Finds a prop being passed to a JSX element. |

### Advanced Querying Techniques

-   **The "Match Any" Wildcard:** Using a single dot `.` as a value for a semantic predicate means "match any value".
    -   `rdump "ext:rs & struct:."` &mdash; Find all Rust files that contain **any** struct definition.
    -   `rdump "ext:py & !import:."` &mdash; Find all Python files that have **no** import statements.

-   **Searching for Absence:** The `!` operator is very powerful when combined with the wildcard.
    -   `rdump "ext:js & !func:."` &mdash; Find JavaScript files that contain no functions (e.g., pure data/config files).

-   **Negating Groups:** Find Rust files that are *not* in the `tests` or `benches` directory.
    ```sh
    rdump "ext:rs & !(path:tests/ | path:benches/)"
    ```

---

## 6. Command Reference

### `rdump search`
The primary command. Can be run as the default subcommand (e.g., `rdump "ext:rs"` is the same as `rdump search "ext:rs"`).

**Usage:** `rdump search [OPTIONS] <QUERY_PARTS>...`

**Options:**

| Flag | Alias | Description |
| :--- | :--- | :--- |
| `--format <FORMAT>` | | Sets the output format. See [Output Formats](#7-output-formats-a-visual-guide). |
| `--context <LINES>` | `-C` | Includes `<LINES>` of context around matches in `hunks` format. |
| `--preset <NAME>` | `-p` | Uses a saved query preset. Can be specified multiple times. |
| `--no-ignore` | | Disables all ignore logic (.gitignore, etc.). Searches everything. |
| `--hidden` | | Includes hidden files and directories (those starting with `.`). |
| `--root <PATH>` | `-r` | The directory to start searching from. Defaults to the current directory. |
| `--output <PATH>` | `-o` | Writes output to a file instead of the console. |
| `--find` | | Shorthand for `--format=find`. |
| `--line-numbers` | | Shows line numbers. |
| `--color <WHEN>` | | When to use syntax highlighting. `always`, `never`, or `auto`. |
| `--help` | `-h` | Displays help information. |
| `--version` | `-V` | Displays version information. |

### `rdump lang`
Inspects supported languages and their available predicates.

**Usage:** `rdump lang [COMMAND]`

**Commands:**

-   `list` (Default): Lists all supported languages and their file extensions.
-   `describe <LANGUAGE>`: Shows all available metadata, content, and semantic predicates for a given language.

### `rdump preset`
Manages saved query shortcuts in your global config file.

**Usage:** `rdump preset [COMMAND]`

**Commands:**

-   `list`: Shows all saved presets.
-   `add <NAME> <QUERY>`: Creates or updates a preset.
-   `remove <NAME>`: Deletes a preset.

---

## 7. Output Formats: A Visual Guide

| Format | Description |
| :--- | :--- |
| `hunks` | **(Default)** Shows only the matching code blocks, with optional context. Highlights matches. |
| `markdown`| Wraps results in Markdown with file headers and fenced code blocks. |
| `json` | Machine-readable JSON output with file paths and content. |
| `paths` | A simple, newline-separated list of matching file paths. Perfect for piping. |
| `cat` | Concatenated content of all matching files, with optional highlighting. |
| `find` | `ls -l`-style output with permissions, size, modified date, and path. |

---

## 8. Configuration

### The `config.toml` File
`rdump` merges settings from a global and a local config file. Local settings override global ones.

-   **Global Config:** `~/.config/rdump/config.toml`
-   **Local Config:** `.rdump.toml` (in the current directory or any parent).

The primary use is for `presets`:
```toml
# In ~/.config/rdump/config.toml
[presets]
rust-src = "ext:rs & !path:tests/"
js-check = "ext:js | ext:jsx"

# In ./my-project/.rdump.toml
[presets]
# Overrides the global preset for this project only
rust-src = "ext:rs & path:src/ & !path:tests/"
```

### The `.rdumpignore` System
`rdump` respects directory ignore files to provide fast, relevant results. The ignore rules are applied with the following precedence, from lowest to highest:

1.  **`rdump`'s built-in default ignores** (e.g., `target/`, `node_modules/`, `.git/`).
2.  **Global gitignore:** Your user-level git ignore file.
3.  **Project `.gitignore` files:** Found in the repository.
4.  **Project `.rdumpignore` files:** These have the highest precedence. You can use a `.rdumpignore` file to *un-ignore* a file that was excluded by a `.gitignore` file (e.g., by adding `!/path/to/file.log`).

---

## 9. Extending `rdump`: Adding a New Language
Adding support for a new language is possible if there is a tree-sitter grammar available for it. This involves:
1.  Adding the `tree-sitter-` grammar crate as a dependency in `Cargo.toml`.
2.  Creating a new module in `src/predicates/code_aware/profiles/` (e.g., `lua.rs`).
3.  In that file, defining a `create_lua_profile` function that returns a `LanguageProfile`. This involves writing tree-sitter queries as strings to capture semantic nodes (e.g., `(function_declaration) @match`).
4.  Registering the new profile in `src/predicates/code_aware/profiles/mod.rs`.
5.  Recompiling.

---

## 10. Troubleshooting & FAQ
- **Q: My query is slow! Why?**
    - A: You are likely starting with an expensive predicate like `contains` or a semantic one. Always try to filter by `ext:`, `path:`, or `name:` first to reduce the number of files that need to be read and parsed.
- **Q: `rdump` isn't finding a file I know is there.**
    - A: It's probably being ignored by a `.gitignore` or default pattern. Run your query with `--no-ignore` to confirm. If it appears, add a rule like `!path/to/your/file` to a `.rdumpignore` file.
- **Q: I'm getting a `command not found` error in my shell.**
    - A: You forgot to wrap your query in quotes. See [Important: Always Quote Your Query!](#important-always-quote-your-query).

---

## 11. Performance Benchmarks
(Illustrative) `rdump` is designed for accuracy and expressiveness, but it's still fast. On a large codebase (e.g., the Linux kernel):
- `ripgrep "some_string"`: ~0.1s
- `rdump "contains:some_string"`: ~0.5s
- `rdump "ext:c & func:some_func"`: ~2.0s

`rdump` will never beat `ripgrep` on raw text search, but `ripgrep` can't do structural search at all. The power of `rdump` is combining these search paradigms.

---

## 12. Contributing
Contributions are welcome! Please check the [GitHub Issues](https://github.com/user/repo/issues).

---

## 13. License
This project is licensed under the **MIT License**.