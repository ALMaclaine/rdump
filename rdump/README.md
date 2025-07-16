# `rdump` &mdash; The Definitive Developer's Guide to Code-Aware Search

**`rdump` is a next-generation, command-line tool for developers. It finds and processes files by combining filesystem metadata, content matching, and deep structural code analysis.**

[![Build Status](https://img.shields.io/github/actions/workflow/status/user/repo/rust.yml?branch=main)](https://github.com/user/repo/actions)
[![Crates.io](https://img.shields.io/crates/v/rdump.svg)](https://crates.io/crates/rdump)
[![License](https://img.shields.io/crates/l/rdump.svg)](https://github.com/user/repo/blob/main/LICENSE)

It's a developer's swiss-army knife for code discovery. It goes beyond the text-based search of tools like `grep` and `ripgrep` by using **tree-sitter** to parse your code into a syntax tree. This allows you to ask questions that are impossible for other tools to answer efficiently:

- *"Find the 'User' struct definition, but only in non-test Rust files."*
- *"Show me every call to 'console.log' in my JavaScript files with 3 lines of context."*
- *"List all Python files larger than 10KB that import 'requests' and were modified in the last week."*

`rdump` is written in Rust for blazing-fast performance, ensuring that even complex structural queries on large codebases are executed in moments.

---

## Table of Contents

1.  [**Why `rdump`?**](#1-why-rdump-a-comparative-look)
    - [The Problem with Text-Based Search](#the-problem-with-text-based-search)
    - [The `rdump` Solution: Structural Awareness](#the-rdump-solution-structural-awareness)
    - [Comparison with Other Tools](#comparison-with-other-tools)
2.  [**Architecture & Design Philosophy**](#2-architecture--design-philosophy)
    - [The Core Philosophy](#the-core-philosophy)
    - [Data Flow Diagram](#data-flow-diagram)
    - [Component Breakdown](#component-breakdown)
3.  [**Installation**](#3-installation)
    - [With Cargo (Recommended)](#with-cargo-recommended)
    - [From Pre-compiled Binaries](#from-pre-compiled-binaries)
    - [From Source](#from-source)
4.  [**Practical Recipes for Real-World Use**](#4-practical-recipes-for-real-world-use)
    - [Code Auditing & Security](#code-auditing--security)
    - [Refactoring & Maintenance](#refactoring--maintenance)
    - [Codebase Exploration & Learning](#codebase-exploration--learning)
    - [DevOps & Automation](#devops--automation)
5.  [**The `rdump` Query Language (RQL) &mdash; A Deep Dive**](#5-the-rdump-query-language-rql--a-deep-dive)
    - [Core Concepts & Syntax](#core-concepts--syntax)
    - [Evaluation Order & Performance Tips](#evaluation-order--performance-tips)
    - [Predicate Reference: Metadata](#predicate-reference-metadata)
    - [Predicate Reference: Content](#predicate-reference-content)
    - [Predicate Reference: Code-Aware (Semantic)](#predicate-reference-code-aware-semantic)
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
| **Primary Use Case** | Finding specific lines of text | Enforcing static analysis rules | **Interactive code exploration & filtering** |
| **Speed** | Unmatched for text search | Fast for patterns | **Very fast; optimizes by layer** |
| **Query `func:foo`** | `grep "func foo"` (noisy) | `pattern: function foo(...)` | `func:foo` (precise) |
| **Query `size:>10kb`** | No | No | `size:>10kb` (built-in) |
| **Query `import:react`** | `grep "import.*react"` (noisy) | `pattern: import ... from "react"` | `import:react` (precise) |
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
[Query String] -> [1. CLI Parser (clap)] -> [2. RQL Parser (pest)] -> [AST] -> [3. Evaluator Engine] -> [Matched Files] -> [6. Formatter (syntect)] -> [Final Output]
                                                                                    |
                                                                                    V
                                                                    [4. Predicate Trait System]
                                                                                    |
                                                                                    +------> [Metadata Predicates]
                                                                                    |
                                                                                    +------> [Content Predicates]
                                                                                    |
                                                                                    +------> [5. Semantic Engine (tree-sitter)]
```

#### 1. CLI Parsing: `clap`

-   **Library:** `clap` (Command Line Argument Parser)
-   **Role:** `clap` is the face of `rdump`. It provides a declarative macro-based API to define the entire CLI structure: subcommands (`search`, `lang`, `preset`), flags (`--format`, `-C`), and arguments (`<QUERY>`).
-   **Implementation Benefits:**
    -   **Automatic Help Generation:** `rdump --help` is generated for free, perfectly in sync with the defined CLI.
    -   **Type-Safe Parsing:** It parses arguments into strongly-typed Rust structs and enums, eliminating manual validation and parsing code.
    -   **Modularity:** The CLI definition is co-located with the `main` function, providing a single, clear entry point to the application's logic.

#### 2. RQL Parser: `pest`

-   **Library:** `pest` (Parser-Expressive Syntax Trees)
-   **Role:** `pest` transforms the human-readable RQL query string (e.g., `"ext:rs & (struct:User | !path:tests)"`) into a machine-readable Abstract Syntax Tree (AST).
-   **Implementation Benefits:**
    -   **Decoupled Grammar:** The entire RQL grammar is defined in a separate file (`src/rql.pest`). This allows the language syntax to evolve independently of the Rust code that processes it.
    -   **Resilience & Error Reporting:** `pest` generates a robust parser with excellent, human-readable error messages out of the box (e.g., "error: expected logical_op, found...").
    -   **AST Generation:** It automatically creates an iterator over the parsed pairs, which our `build_ast_from_pairs` function in `src/parser.rs` recursively walks to build our `AstNode` enum (e.g., `AstNode::LogicalOp(...)`).

#### 3. The Evaluator Engine

-   **Library:** Standard Rust
-   **Role:** The evaluator is the brain. It takes the AST from `pest` and a list of candidate files, and returns only the files that match the query.
-   **Implementation Benefits:**
    -   **Recursive Evaluation:** It's a simple, elegant recursive function that walks the `AstNode` tree. If it sees a `LogicalOp`, it calls itself on the left and right children. If it sees a `Predicate`, it dispatches to the predicate system.
    -   **Performance via Short-Circuiting:** When evaluating `ext:rs & struct:User`, if `ext:rs` returns `false`, the evaluator **immediately stops** and does not execute the expensive `struct:User` predicate. This is a critical performance optimization.

#### 4. The Predicate System: Rust's Trait System

-   **Library:** Standard Rust (specifically, `trait` objects)
-   **Role:** This is the heart of `rdump`'s modularity. Each predicate (`ext`, `size`, `contains`, `func`, etc.) is an independent module that implements a common `Predicate` trait.
-   **Implementation Benefits:**
    -   **Dynamic Dispatch:** The evaluator holds a collection of `Box<dyn Predicate>`. When it encounters a predicate key in the AST, it dynamically finds and executes the correct predicate's `evaluate()` method.
    -   **Extreme Modularity:** To add a new predicate, say `author:<name>`, a developer simply needs to:
        1.  Create a new file `src/predicates/author.rs`.
        2.  Implement the `Predicate` trait for an `AuthorPredicate` struct.
        3.  Register the new predicate in the evaluator's lookup map.
        *No other part of the codebase needs to change.*

#### 5. The Semantic Engine: `tree-sitter`

-   **Library:** `tree-sitter` and its Rust binding.
-   **Role:** `tree-sitter` is the universal parser that powers all code-aware predicates. It takes source code text and produces a concrete syntax tree.
-   **Implementation Benefits:**
    -   **Language Agnostic Core:** The core semantic predicate logic doesn't know anything about Rust, Python, or Go. It only knows how to execute a `tree-sitter` query against a syntax tree.
    -   **Data-Driven Extensibility:** A language is "supported" by providing data, not code:
        1.  The compiled `tree-sitter` grammar (as a crate).
        2.  A set of `.scm` files containing tree-sitter queries (e.g., `(function_definition name: (identifier) @func-name)`).
    -   This design means adding `func` support for a new language involves writing a one-line query in a text file, not writing complex Rust code to traverse a language-specific AST.

#### 6. Parallelism & Performance: `rayon`

-   **Library:** `rayon`
-   **Role:** `rayon` is the secret sauce for `rdump`'s performance on multi-core machines. While the evaluator processes a single query, the file search itself is a massively parallel problem. `rayon` provides incredibly simple, data-parallel iterators.
-   **Implementation Benefits:**
    -   **Effortless Parallelism:** With `rayon`, converting a sequential iterator over files into a parallel one is often a one-line change (e.g., `files.iter()` becomes `files.par_iter()`). `rayon` handles thread pooling, work-stealing, and synchronization automatically.
    -   **Fearless Concurrency:** Rust's ownership model and `rayon`'s design guarantee that this parallelism is memory-safe, preventing data races at compile time.
    -   **Scalability:** This allows `rdump` to scale its performance linearly with the number of available CPU cores, making it exceptionally fast on modern hardware when searching large numbers of files.

#### 7. The Formatter & Syntax Highlighting: `syntect`

-   **Library:** `syntect`
-   **Role:** The formatter takes the final list of matched files and hunks and presents them to the user.
-   **Implementation Benefits:**
    -   **Professional-Grade Highlighting:** `syntect` uses the same syntax and theme definitions as Sublime Text, providing robust, accurate, and beautiful highlighting for a vast number of languages.
    -   **Lazy Loading:** The `SYNTAX_SET` and `THEME_SET` are wrapped in `once_cell::sync::Lazy` to ensure they are loaded from disk and parsed only once on the first use, making subsequent runs faster.
    -   **Clean Separation:** The `Format` enum allows the `print_output` function to act as a clean dispatcher, routing to different printing functions (`print_highlighted_content`, `print_markdown_fenced_content`, etc.) based on the user's choice. This keeps the presentation logic clean and separated.

---

## 3. Installation

### With Cargo (Recommended)
If you have the Rust toolchain (`rustup`), you can install directly from Crates.io. This ensures you have the latest version.
```sh
cargo install rdump
```

### From Pre-compiled Binaries
Pre-compiled binaries for Linux, macOS, and Windows are available on the [**GitHub Releases**](https://github.com/user/repo/releases) page. Download the appropriate archive, extract the `rdump` executable, and place it in a directory on your system's `PATH`.

### From Source
To build `rdump` from source, you'll need `git` and the Rust toolchain.
```sh
git clone https://github.com/user/repo.git
cd rdump
cargo build --release
# The executable will be at ./target/release/rdump
./target/release/rdump --help
```

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

### Codebase Exploration & Learning

-   **Get a high-level overview of a new Rust project's data structures:**
    ```sh
    rdump "ext:rs & (struct:. | enum:.) & !path:tests"
    ```
-   **Trace a configuration variable from definition to use:**
    ```sh
    rdump "contains:APP_PORT"
    ```
-   **Understand a project's API surface:** List all functions defined in files under an `api/` directory.
    ```sh
    rdump "path:src/api/ & func:."
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

### Code Quality & Consistency

-   **Find functions that are too long (e.g., > 50 lines):**
    ```sh
    # This is an approximation, but effective.
    # It finds functions where the text content of the function node is over 1200 bytes.
    rdump "func:. & size:>1200b"
    ```
-   **Enforce API conventions:** Find all `GET` endpoints that are missing a call to an authentication middleware.
    ```sh
    rdump "ext:go & func:/^Get/ & !call:requireAuth"
    ```
-   **Find magic strings/numbers:** Locate string or number literals outside of variable declarations.
    ```sh
    rdump "(str:. | contains:/ \d+;/) & !contains:/const / & !contains:/let / & !contains:/var /"
    ```

---

## 5. The `rdump` Query Language (RQL) &mdash; A Deep Dive

(This section is intentionally verbose for complete clarity.)

### Core Concepts & Syntax

-   **Predicates:** The building block of RQL is the `key:value` pair (e.g., `ext:rs`).
-   **Operators:** Combine predicates with `&` (AND), `|` (OR).
-   **Negation:** `!` negates a predicate or group (e.g., `!ext:md`).
-   **Grouping:** `()` controls the order of operations (e.g., `ext:rs & (contains:foo | contains:bar)`).
-   **Quoting:** Use `'` or `"` for values with spaces or special characters (e.g., `contains:'fn main()'`).

### Evaluation Order & Performance Tips

`rdump` is fast, but you can make it even faster by writing efficient queries. The key is to **eliminate the most files with the cheapest predicates first.**

-   **GOOD:** `ext:rs & struct:User`
    -   *Fast.* `rdump` first finds all `.rs` files (very cheap), then runs the expensive `struct` parser only on that small subset.
-   **BAD:** `struct:User & ext:rs`
    -   *Slow.* While `rdump`'s engine is smart enough to likely re-order this, writing it this way is logically less efficient. It implies parsing every file to look for a struct, then checking its extension.
-   **BEST:** `path:models/ & ext:rs & struct:User`
    -   *Blazing fast.* The search space is narrowed by path, then extension, before any files are even opened.

**Golden Rule:** Always lead with `path:`, `name:`, or `ext:` if you can.

### Predicate Reference

Predicates are the core of RQL. They are grouped into three categories based on what they inspect.

#### Metadata Predicates (Fastest)

These predicates operate on filesystem metadata and are extremely fast. **Always use them first in your query to narrow the search space.**

| Key        | Example                     | Description                                                                                             |
| :--------- | :-------------------------- | :------------------------------------------------------------------------------------------------------ |
| `ext`      | `ext:ts`                    | Matches the file extension. Case-insensitive.                                                           |
| `name`     | `name:"*_test.go"`          | Matches the filename (the part after the last `/` or ``) against a glob pattern.                        |
| `path`     | `path:src/api`              | Matches if the given substring appears anywhere in the full relative path of the file.                  |
| `in`       | `in:"src/commands"`         | Matches all files that are descendants of the given directory.                                          |
| `size`     | `size:>=10kb`               | Filters by file size. Operators: `>`, `<`, `>=`, `<=`, `=`. Units: `b`, `kb`, `mb`, `gb`.                 |
| `modified` | `modified:<2d`               | Filters by last modification time relative to now. Units: `m` (minutes), `h` (hours), `d` (days), `w` (weeks), `y` (years). |

#### Content Predicates (Fast)

These predicates inspect the raw text content of a file. They are slower than metadata predicates but faster than code-aware ones.

| Key        | Example                     | Description                                                                                             |
| :--------- | :-------------------------- | :------------------------------------------------------------------------------------------------------ |
| `contains` | `contains:"// HACK"`        | Fast literal substring search. It does not support regular expressions.                                 |
| `matches`  | `matches:"/user_[a-z]+/"`   | Slower but powerful regex search. The value must be a valid regular expression.                         |

#### Code-Aware (Semantic) Predicates (Slower)

These are `rdump`'s most powerful feature. They parse the code with `tree-sitter` to understand its structure. These are the most expensive predicates; use them after narrowing the search with metadata and content predicates.

| Key          | Example                     | Description                                                                                             |
| :----------- | :-------------------------- | :------------------------------------------------------------------------------------------------------ |
| `def`        | `def:User`                  | Finds a generic definition (e.g., a `class` in Python, a `struct` in Rust, a `type` in Go).             |
| `func`       | `func:get_user`             | Finds a function or method definition.                                                                  |
| `import`     | `import:serde`              | Finds an import, `use`, or `require` statement.                                                         |
| `call`       | `call:println`              | Finds a function or method call site.                                                                   |
| `comment`    | `comment:TODO`              | Finds text within any code comment (`//`, `#`, `/* ... */`, etc.).                                      |
| `str`        | `str:"api_key"`             | Finds text **only inside a string literal** (e.g., `"api_key"` or `'api_key'`). Much more precise than `contains`. |
| `class`      | `class:ApiHandler`          | Finds a `class` definition.                                                                             |
| `struct`     | `struct:Point`              | Finds a `struct` definition (primarily for Rust/Go).                                                    |
| `enum`       | `enum:Status`               | Finds an `enum` definition.                                                                             |
| `interface`  | `interface:Serializable`    | Finds an `interface` definition (primarily for Go/TypeScript/Java).                                     |
| `trait`      | `trait:Runnable`            | Finds a `trait` definition (primarily for Rust).                                                        |
| `type`       | `type:UserID`               | Finds a `type` alias definition.                                                                        |
| `impl`       | `impl:User`                 | Finds an `impl` block (Rust).                                                                           |
| `macro`      | `macro:println`             | Finds a macro definition or invocation (Rust).                                                          |
| `component`  | `component:Button`          | **React:** Finds a JSX element definition (e.g., `<Button ... />`).                                     |
| `element`    | `element:div`               | **React:** Finds a specific JSX element by its tag name (e.g., `<div>`).                                |
| `hook`       | `hook:useState`             | **React:** Finds a call to a standard React hook.                                                       |
| `customhook` | `customhook:useAuth`        | **React:** Finds a call to a custom hook (a function starting with `use`).                              |
| `prop`       | `prop:onClick`              | **React:** Finds a JSX prop (attribute) being passed to a component.                                    |


### Advanced Querying Techniques

-   **The "Match All" Wildcard:** Using a single dot `.` as a value for a predicate means "match any value". This is useful for checking for the existence of a node type.
    -   `rdump "ext:rs & struct:."` &mdash; Find all Rust files that contain **any** struct definition.
    -   `rdump "ext:py & !import:."` &mdash; Find all Python files that have **no** import statements.

-   **Searching for Absence:** The `!` operator is very powerful when combined with the wildcard.
    -   `rdump "ext:js & !func:."` &mdash; Find JavaScript files that contain no functions (e.g., pure data/config files).

-   **Escaping Special Characters:** If you need to search for a literal quote, you can escape it.
    -   `rdump "str:'hello \'world\''"` &mdash; Finds the literal string `'hello 'world''`.

-   **Negating Groups:** Find Rust files that are *not* in the `tests` or `benches` directory.
    ```sh
    rdump "ext:rs & !(path:tests/ | path:benches/)"
    ```

-   **Distinguishing Content Types:** `contains:"foo"` finds `foo` anywhere. `str:"foo"` finds `foo` **only inside a string literal**. This is much more precise.

-   **Forcing Evaluation Order:** Use parentheses to ensure logical correctness for complex queries.
    ```sh
    # Find JS or TS files that either import React or define a 'Component' class
    rdump "(ext:js | ext:ts) & (import:react | class:Component)"
    ```


---

## 6. Command Reference
(Sections for `lang` and `preset` are omitted for brevity but would be here)

### `rdump search`
The primary command. Can be omitted (`rdump "ext:rs"` is the same as `rdump search "ext:rs"`).

**Usage:** `rdump [OPTIONS] <QUERY>`

**Options:**

| Flag | Alias | Description |
| :--- | :--- | :--- |
| `--format <FORMAT>` | `-f` | Sets the output format. See [Output Formats](#7-output-formats-a-visual-guide). |
| `--context <LINES>` | `-C` | Includes `<LINES>` of context around matches in `hunks` format. |
| `--preset <NAME>` | `-p` | Uses a saved query preset. |
| `--no-ignore` | | Disables all ignore logic. Searches everything. |
| `--hidden` | | Includes hidden files and directories (those starting with `.`). |
| `--config-path <PATH>` | | Path to a specific `rdump.toml` config file. |
| `--help` | `-h` | Displays help information. |
| `--version` | `-V` | Displays version information. |

---

## 7. Output Formats: A Visual Guide

| Format | Description |
| :--- | :--- |
| `hunks` | **(Default)** Shows only the matching code blocks, with optional context. |
| `markdown`| Wraps results in Markdown, useful for reports. |
| `json` | Machine-readable JSON output with file paths and content. |
| `paths` | A simple, newline-separated list of matching file paths. Perfect for piping. |
| `cat` | Concatenated content of all matching files. |
| `find` | `ls -l`-style output with permissions, size, modified date, and path. |

---

## 8. Configuration

### The `config.toml` File
`rdump` merges settings from a global and a local config file. Local settings override global ones.

- **Global Config:** `~/.config/rdump/config.toml`
- **Local Config:** `.rdump.toml` (in the current directory or any parent).

### The `.rdumpignore` System
`rdump` respects `.gitignore` by default and provides its own `.rdumpignore` for more control.

---

## 9. Extending `rdump`: Adding a New Language
Adding support for a new language is possible if there is a tree-sitter grammar available for it. This involves:
1.  Finding the `tree-sitter` grammar.
2.  Writing `.scm` query files to capture semantic nodes.
3.  Updating `rdump`'s language profiles.
4.  Recompiling.

---

## 10. Troubleshooting & FAQ
- **Q: My query is slow! Why?**
  - A: You are likely starting with an expensive predicate. Always try to filter by `ext:`, `path:`, or `name:` first.
- **Q: `rdump` isn't finding a file I know is there.**
  - A: It's probably being ignored. Run your query with `--no-ignore` to check.
- **Q: How do I search for a literal `!` or `&`?**
  - A: Quote the value, e.g., `contains:'&amp;'`.

---

## 11. Performance Benchmarks
(Illustrative) `rdump` is designed for accuracy and expressiveness, but it's still fast. On a large codebase (e.g., the Linux kernel):
- `ripgrep "some_string"`: ~0.1s
- `rdump "contains:some_string"`: ~0.5s
- `rdump "ext:c & func:some_func"`: ~2.0s

`rdump` will never beat `ripgrep` on raw text search, but `ripgrep` can't do structural search at all.

---

## 12. Contributing
Contributions are welcome! Please check the [GitHub Issues](https://github.com/user/repo/issues).

---

## 13. License
This project is licensed under the **MIT License**.