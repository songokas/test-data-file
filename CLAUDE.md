# CLAUDE.md

This file provides guidance to Claude Code (claude.ai/code) when working with code in this repository.

## What this is

`test-data-file` is a Rust proc-macro crate (single crate, `proc-macro = true`). It provides the `#[test_data_file(path = "...")]` attribute macro, which loads test data from a file (csv, json, yaml, ron, toml, sql, or a space-separated `list` format) and calls the decorated test function once per row/entry. All macro logic lives in `src/lib.rs` — there is no other source file.

## Commands

```sh
cargo fmt --check   # formatting check (CI enforces this)
cargo clippy        # lint (CI enforces this)
cargo test          # run all tests
cargo test --test all_types           # run one integration test file
cargo test test_test_me_with_yaml     # run a single test by name
cargo insta review                    # review/accept snapshot changes (tests/insta.rs uses insta)
```

CI (`.github/workflows/linux.yml`) runs exactly `cargo fmt --check`, `cargo clippy`, `cargo test` on push/PR to `main` — match this locally before considering work done.

## Architecture

The macro is implemented entirely in `src/lib.rs` via `impl_test_data_file`, which generates code at compile time:

1. **Argument parsing** (`TestFileDataAttributes::parse`): reads `path` (required, checked to exist on disk *at macro-expansion time*) and `kind` (optional). If `kind` is omitted, it's inferred from the file extension against `SUPPORTED_KINDS = ["csv", "json", "yaml", "ron", "toml", "list", "sql"]`. Files with no/unrecognized extension require `kind` explicitly.
2. **Function transformation**: the original function is renamed to `_<original_name>` (its `#[test]`/`#[should_panic]`/`#[tokio::*]` attributes are stripped), and a new function named `<original_name>` is generated that keeps the original attributes. This is what lets `cargo test` and IDE test runners discover the wrapper transparently.
3. **Body generation per `kind`**, branching in `impl_test_data_file`:
   - `csv`: builds an internal `_Data` struct (fields = the original function's parameters/types) and deserializes each row with `csv::ReaderBuilder`.
   - `list`: no serde struct — reads lines with `BufRead`, skips the header line, splits each subsequent line on spaces, and parses each field with `FromStr`.
   - `sql`: parses `INSERT INTO ... VALUES ...;` statements at runtime via `sqlparser`, groups rows by table name into `serde_json::Value` maps, auto-detects a single "root" table (the one table with no outbound `<other_table>_id` column), and embeds any child table's matching rows (joined on `<root_table>_id` = root's `id`) under a key named after the child table — one match embeds as an object, multiple matches embed as an array. `id`/`<parent>_id` join columns are stripped before the final `_Data` deserialization. If the decorated function has exactly one parameter and the root table's name matches it, the row is wrapped under that parameter name (mirrors how the JSON nested example wraps entries under `"user"`); otherwise the row's columns map directly to the function's parameters. When there is no single root table because none of the tables reference another (more than one root candidate), the macro switches to "multi-table" mode: the first non-child table in file order (tracked via `table_order`) is the root and the test runs once per root row; each table's row is deserialized into its own struct embedded as a field of `_Data` keyed by table name (so the function's parameter names must match the table names). Non-root tables pair with root rows purely by position and may have any row count — a position with no row leaves the key absent (so that parameter must be `Option<_>` and deserializes to `None`); non-root rows past the last root row are ignored. Requires the consumer to add `sqlparser` (and `serde_json`, for the intermediate `Value` representation) as dev-dependencies.
   - everything else (`json`, `yaml`, `ron`, `toml`): also builds a `_Data` struct, plus an untagged `Collection` enum (`Index(Vec<_Data>)` or `Map(HashMap<String, _Data>)`) so both top-level arrays and named-key maps deserialize into the same iteration path.
   - All branches panic if the resulting dataset is empty, and call `_<original_name>(...)` once per row, `.await`-ing it if the original function was `async`.
4. Function parameter names/types are extracted directly from the decorated function's signature (`item.sig.inputs`) and reused both as the generated struct's fields and as the call arguments passed to `_<original_name>`.

## Tests

Integration tests live in `tests/` and double as the crate's usage examples (see `tests/all_types.rs` for one test repeated across every supported format, `tests/complex_types.rs` for nested/optional/collection parameters, `tests/tokio.rs` for async tests, `tests/insta.rs` for snapshot-based assertions via `insta`). Sample data files consumed by these tests live in `tests/samples/`; the file's data shape must line up with the parameter list of the decorated test function. When adding a new supported `kind`, update `SUPPORTED_KINDS` in `src/lib.rs` and the format table in `README.md`.

The `tests/examples_*.rs` binaries (`examples_scalars.rs`, `examples_nested.rs`, `examples_async.rs`) are the realistic counterparts that back the `README.md` walkthrough: one signup/account-validation domain expressed across every format, with data in `tests/examples/`. Keep the README code blocks and these files in sync.

The doctest in `src/lib.rs` (on `test_data_file` itself) exercises `tests/samples/test_me.yaml` and must stay in sync with that file's shape.
