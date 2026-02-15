---
name: agent-cli
description: Patterns and guidance for building agent-friendly CLI subcommands in the kozmotic project.
invocation: Use when adding new CLI tools, subcommands, or agent-facing features to kozmotic.
---

# Agent-Friendly CLI Patterns for Kozmotic

## Output Envelope

Every command returns a consistent JSON envelope via `Output<T>`:

```rust
#[derive(Serialize, Deserialize)]
struct Output<T> {
    status: String,       // "success" or "error"
    data: T,              // command-specific payload
    metadata: Metadata,   // timestamp, tool name, version
}
```

Construct with `Output::success("tool_name", data)`. The `data` field is the only part that varies between commands — everything else is automatic.

For errors, use a parallel envelope:

```rust
let error_output = serde_json::json!({
    "status": "error",
    "error": {
        "code": "INVALID_INPUT",
        "message": "Name must not be empty"
    },
    "metadata": { /* same as success */ }
});
```

Use UPPER_SNAKE_CASE error codes that agents can match on (e.g., `FILE_NOT_FOUND`, `PERMISSION_DENIED`, `TIMEOUT`).

## Error Handling

Define domain errors with `thiserror` in each command module:

```rust
#[derive(Debug, thiserror::Error)]
enum ToolError {
    #[error("file not found: {0}")]
    FileNotFound(String),

    #[error("invalid input: {0}")]
    InvalidInput(String),
}
```

Map errors to semantic exit codes:

| Exit Code | Meaning |
|-----------|---------|
| 0 | Success |
| 1 | User/input error (bad args, missing file, validation failure) |
| 2 | System error (I/O failure, network timeout, unexpected crash) |

Return `ExitCode::from(code)` — never `process::exit()` which skips destructors.

## Adding a New Subcommand

Follow these steps exactly:

### 1. Add variant to `Commands` enum in `src/main.rs`

```rust
#[derive(Subcommand)]
enum Commands {
    /// Existing commands...

    /// Description of what the new tool does
    NewTool {
        /// Required argument description
        #[arg(short, long)]
        target: String,

        /// Optional argument with default
        #[arg(short, long, default_value = "10")]
        limit: u32,
    },
}
```

### 2. Handle in `main()` match

```rust
Some(Commands::NewTool { target, limit }) => {
    match run_new_tool(&target, limit) {
        Ok(data) => {
            match cli.format {
                OutputFormat::Json => {
                    let output = Output::success("new-tool", data);
                    println!("{}", serde_json::to_string_pretty(&output).unwrap());
                }
                OutputFormat::Human => {
                    println!("{}", format_human_readable(&data));
                }
            }
            ExitCode::SUCCESS
        }
        Err(e) => {
            let error_output = serde_json::json!({
                "status": "error",
                "error": { "code": e.code(), "message": e.to_string() }
            });
            eprintln!("{}", serde_json::to_string_pretty(&error_output).unwrap());
            ExitCode::from(e.exit_code())
        }
    }
}
```

### 3. Add integration tests in `tests/integration_test.rs`

```rust
#[test]
fn test_new_tool_json_output() {
    let mut cmd = Command::cargo_bin("kozmotic").unwrap();
    cmd.arg("new-tool")
        .arg("--target")
        .arg("test-value")
        .assert()
        .success()
        .stdout(predicate::str::contains("\"status\": \"success\""));
}

#[test]
fn test_new_tool_human_output() {
    let mut cmd = Command::cargo_bin("kozmotic").unwrap();
    cmd.arg("--format").arg("human")
        .arg("new-tool")
        .arg("--target")
        .arg("test-value")
        .assert()
        .success()
        .stdout(predicate::str::contains("status").not());
}

#[test]
fn test_new_tool_error_case() {
    let mut cmd = Command::cargo_bin("kozmotic").unwrap();
    cmd.arg("new-tool")
        .arg("--target")
        .arg("")  // trigger validation error
        .assert()
        .failure()
        .code(1);
}
```

## Stdout / Stderr Discipline

- **stdout** — structured data only (JSON envelope or human-readable output). This is what agents parse.
- **stderr** — diagnostics, progress indicators, warnings, debug logs. Agents ignore this.

Never mix diagnostics into stdout. Use `eprintln!()` for anything that isn't the command's result.

## No Interactive Prompts

Agent-driven CLIs must never block on stdin. Follow these rules:

- Never call `stdin().read_line()` or any interactive prompt.
- For destructive operations, require an explicit `--yes` / `-y` flag to skip confirmation. Without `--yes`, print what *would* happen and exit with code 0 (treat it as a dry-run).
- Accept data via stdin pipes (e.g., `cat file | kozmotic tool --input -`), but never prompt for it.
- All required values must be expressible as flags or arguments.

## Global Flags

These flags apply across all subcommands. Add them to the `Cli` struct:

| Flag | Short | Purpose |
|------|-------|---------|
| `--format` | `-f` | Output format: `json` (default) or `human` |
| `--quiet` | `-q` | Suppress all stderr output |
| `--verbose` | `-v` | Increase stderr verbosity (stackable: `-vvv`) |
| `--timeout` | `-t` | Operation timeout in seconds |
| `--dry-run` | | Show what would happen without doing it |
| `--no-color` | | Disable ANSI colors in human output |

Implement only the flags needed for the current command set. Don't add flags preemptively — add them when a command needs them.

## Idempotency

Commands should produce deterministic output:

- Sort object keys and array elements in a stable order (alphabetical or by primary key).
- Use deterministic timestamps when possible, or accept `--timestamp` override for testing.
- Operations should be safe to retry — creating something that already exists should return success with the existing data, not an error.

## Pagination

For commands that return lists or large datasets:

```rust
#[arg(long, default_value = "100")]
limit: u32,

#[arg(long, default_value = "0")]
offset: u32,
```

Include pagination metadata in the response:

```json
{
  "status": "success",
  "data": { "items": [...], "total": 250 },
  "metadata": {
    "pagination": { "limit": 100, "offset": 0, "next_offset": 100 }
  }
}
```

Use cursor-based pagination when offset-based is impractical (e.g., streaming sources).

## Dry-Run

For any command that modifies state (files, network, databases):

- Accept `--dry-run` flag.
- When active, compute and return the *same envelope* as a real run but with `"dry_run": true` in metadata and no side effects.
- This lets agents preview operations before committing.

## Testing Pattern

All tests use `assert_cmd` + `predicates` against the compiled binary:

```rust
use assert_cmd::Command;
use predicates::prelude::*;
```

Every new command needs at minimum:

1. **JSON output test** — verify `"status": "success"` and expected data fields
2. **Human output test** — verify human-readable text appears, JSON envelope does not
3. **Error case test** — verify failure exit code and error envelope
4. **Edge cases** — missing args (clap handles this), empty inputs, boundary values

Run tests with `cargo test`. Run a single test with `cargo test <test_name>`.

## Flag Conventions

- Use **kebab-case** for flag names: `--output-dir`, not `--output_dir`.
- Provide both **long and short** forms for frequently-used flags: `--name` / `-n`.
- Support **env var overrides** with `KOZMOTIC_` prefix: `KOZMOTIC_FORMAT=json`.

In clap, declare env var support:

```rust
#[arg(short, long, env = "KOZMOTIC_FORMAT", default_value = "json")]
format: OutputFormat,
```

## Checklist for New Commands

Before submitting a new subcommand, verify:

- [ ] Added `Commands` variant with doc comment
- [ ] Handled in `main()` match with both JSON and human format paths
- [ ] Errors return structured error envelope on stderr with correct exit code
- [ ] No interactive prompts — all inputs via flags/args
- [ ] Integration tests cover JSON output, human output, and error cases
- [ ] `cargo fmt`, `cargo clippy -- -D warnings`, and `cargo test` all pass
