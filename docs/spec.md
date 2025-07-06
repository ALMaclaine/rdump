Of course. Here is the complete, consolidated technical specification in a single file, incorporating all our decisions.

---

## **Technical Specification: `rdump` v1.0**

### **1. Guiding Principles & Design Philosophy**

*   **Zero-Configuration by Default:** The tool must be useful out-of-the-box with no setup. It will intelligently use common conventions like `.gitignore`.
*   **Performance is a Feature:** Every design decision will be weighed against its performance impact. We will prefer compile-time guarantees and parallel execution wherever possible.
*   **Clarity over Conciseness:** Error messages, query syntax, and output must be unambiguous, both for human users and AI agents.
*   **Deterministic Execution:** For a given filesystem state and query, the output must always be identical.

### **2. The Command-Line Interface (CLI)**

The CLI is the sole entry point for users. It is responsible for parsing all arguments and flags and passing them to the core engine.

**Command:** `rdump`

**Synopsis:** `rdump [FLAGS/OPTIONS] <QUERY_STRING>`

**Positional Arguments:**

*   **`<QUERY_STRING>`** (Required, String): A string encapsulating the query. Must be quoted by the shell to be treated as a single argument.

**Flags (Boolean Switches):**

*   `--no-ignore`: Disables all ignore file logic (`.gitignore`, `.ignore`, etc.). All files not explicitly excluded by the query are considered candidates.
*   `--hidden`: Forces the inclusion of hidden files and directories (those starting with a `.`), which are ignored by default.
*   `--line-numbers, -n`: Prepends line numbers to each line of the file content in the output. This affects `markdown` and `cat` formats. It has no effect on `json` or `paths` formats.
*   `--no-headers`: Strips all file path headers and metadata, effectively creating a concatenated stream of (optionally line-numbered) file contents. If used, it overrides the `markdown` and `json` formats, making the output equivalent to `cat`.
*   `--verbose, -v`: Prints detailed logging to `stderr`, including the parsed AST, directories being scanned, and reasons for file rejections.
*   `--help, -h`: Prints the help message and exits.
*   `--version, -V`: Prints the version number and exits.

**Options (Flags with Values):**

*   `--output <PATH>, -o <PATH>`: Redirects the formatted output to the specified file path. If not provided, output is written to `stdout`.
*   `--format <FORMAT>`: Specifies the output format.
    *   **Allowed values:** `markdown` (default), `json`, `cat`, `paths`.
    *   **Note:** If `--no-headers` is used, the effective output format will be `cat`, overriding this option.
*   `--root <DIR>`: Sets the root directory for the search. Defaults to the current working directory.
*   `--max-depth <N>`: Limits directory traversal to `N` levels deep. A depth of `0` searches only the root directory itself.
*   `--threads <N>`: Sets the number of worker threads. Defaults to the number of logical CPU cores.

---

### **3. The `rdump` Query Language (RQL)**

RQL is the core of the tool's expressiveness.

#### **3.1. Syntax and Grammar**

*   **Operators:** `&` (AND), `|` (OR), `!` (NOT).
*   **Grouping:** `( ... )` for explicit precedence.
*   **Precedence (Highest to Lowest):** `!`, then `&`, then `|`.
*   **Predicates:** `key:value` pairs.
    *   `key` is an alphanumeric identifier.
    *   `value` can be unquoted if it contains no special characters. If it contains spaces or operators, it **must** be enclosed in single `'...'` or double `"..."` quotes.

#### **3.2. Predicate Specification**

| Key (`key:value`) | Value Type | Matching Logic |
| :--- | :--- | :--- |
| **`ext`** | `string` | **Exact Match.** Matches the file extension *without* the leading dot. Case-insensitive. `ext:rs` matches `file.rs` and `file.RS`. |
| **`name`** | `glob` | **Glob Pattern Match.** Matches against the file's base name (e.g., `foo.txt`). Uses standard glob syntax (`*`, `?`, `[]`). `name:"*_test.rs"` |
| **`path`** | `string` | **Substring Match.** Matches if the value appears anywhere in the file's full canonical path. `path:src/components` |
| **`path_exact`** | `string` | **Exact Path Match.** Matches if the value is identical to the file's full canonical path. |
| **`size`** | `size_qualifier` | **Numeric Comparison.** `value` is `[>\|<][number][kb\|mb\|gb]`. No space between operator and number. `size:>100kb` |
| **`modified`** | `time_qualifier` | **Timestamp Comparison.** `value` is `[>\|<][number][s\|m\|h\|d\|w]`. `modified:<2d` matches files modified in the last 48 hours. |
| **`contains` / `c`** | `string` | **Literal Substring Search.** Case-sensitive. Reads the file content and returns true if the exact substring is found. `c:'fn main()'` |
| **`matches` / `m`** | `regex` | **Regular Expression Search.** The `value` is a regular expression. The tool will use the Rust `regex` crate. `m:'/struct \w+/'` |

---

### **4. Core Architecture & Execution Flow**

1.  **Initialization:** `clap` parses all CLI arguments. The `root` directory is canonicalized. The `rayon` thread pool is configured.

2.  **Query Parsing:** The `<QUERY_STRING>` is fed into the **Parser Module** (e.g., `pest`), which transforms it into an **Abstract Syntax Tree (AST)**. On failure, the program exits with a user-friendly syntax error.

3.  **Candidate Discovery:** A parallel directory walker (`jwalk`) discovers all files, respecting ignore-file logic (unless `--no-ignore`) and hidden file rules (unless `--hidden`).

4.  **Concurrent Evaluation Pipeline:** The stream of discovered files is piped into a `rayon` parallel iterator. For each file on a worker thread:
    a.  An internal **`FileContext`** struct is created, lazily loading path, then metadata, and finally content only when absolutely required.
    b.  The `FileContext` is evaluated against the AST using a **short-circuiting strategy**. Cheap metadata checks are performed before expensive content checks. A file is only read from disk if all preceding metadata predicates in an `&` chain pass.
    c.  If the AST evaluates to `true`, the `FileContext` is sent to the main thread for aggregation.

5.  **Result Aggregation & Sorting:** The main thread collects all matching `FileContext` objects. The final list is sorted alphabetically by path to ensure deterministic output.

6.  **Output Rendering:** The sorted list is passed to the **Formatter Module**, whose behavior is modified by the output flags:
    a.  **Internal Pre-processing:** If `--line-numbers` is active, the `content` string within each matching `FileContext` is transformed line-by-line before formatting.
    b.  **Format Logic:**
    *   **If `--no-headers` is specified:** The formatter ignores `--format`. It simply iterates through results and prints the (potentially line-numbered) content of each file.
    *   **If `--no-headers` is NOT specified:** The formatter uses `--format`:
    *   **`markdown`:** Prints a separator, metadata header, and the (potentially line-numbered) content for each file.
    *   **`json`:** The `content` field in the JSON will **always be the original, unmodified file content**. Line numbers are not injected to preserve data integrity.
    *   **`cat`:** Prints the concatenated (and potentially line-numbered) content.
    *   **`paths`:** Prints only the list of matching file paths.

---

### **5. Output Schemas**

#### **Markdown Format (Example with `--line-numbers`)**
```markdown
---
File: src/main.rs
Size: 78 B
Modified: 2023-10-29T10:30:00Z
---
    1 | use std::io;
    2 | 
    3 | fn main() {
    4 |     println!("Hello, rdump!");
    5 | }
```

#### **`cat` / `--no-headers` Format (Example with `--line-numbers`)**
*Assuming `file1.txt` ("a\nb") and `file2.txt` ("c\nd") matched:*
```
    1 | a
    2 | b
    1 | c
    2 | d
```

#### **JSON Format (Schema)**
*The output is a single JSON array `[...]`. Each object conforms to this schema:*
```json
{
  "path": "string",             // Canonical path to the file
  "size_bytes": "integer",          // File size in bytes
  "modified_iso8601": "string", // ISO 8601 formatted UTC timestamp
  "content": "string"             // The raw, original content of the file. NO line numbers.
}
```
