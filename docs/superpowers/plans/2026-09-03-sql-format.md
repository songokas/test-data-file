# SQL INSERT Format Support Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Add `kind = "sql"` to the `test_data_file` proc-macro so test data can be authored as `INSERT INTO ... VALUES ...;` statements, supporting both a flat single-table shape and a nested parent/child shape joined via a `<table>_id` foreign-key naming convention.

**Architecture:** Follows the existing per-kind branch pattern in `impl_test_data_file` (`src/lib.rs`). All new logic is generated Rust source emitted via `quote!` — it runs at *test* runtime in the consuming crate's binary, not at macro-expansion time, exactly like the existing csv/json/yaml/ron/toml/list branches. No new dependency is added to the macro crate's own `[dependencies]`; `sqlparser` is added only as a `[dev-dependencies]` entry (needed for the macro crate's own tests, and documented as a requirement for any consumer using `kind = "sql"`).

**Tech Stack:** Rust, `syn`/`quote`/`proc-macro2` (existing), `sqlparser = "0.62"` (new, dev-only), `serde_json` (existing dev-dependency, reused as the row intermediate representation).

## Global Constraints

- `sqlparser` version is pinned to `"0.62"` — its `Insert`/`Expr`/`Value` AST shapes (`TableObject::TableName`, `Expr::Value(ValueWithSpan)`, `ObjectName(Vec<ObjectNamePart>)`, columns as `Vec<ObjectName>`) are specific to this version and were verified against the actual crate source; do not assume API compatibility with other versions.
- All generated-code panics must include `{file_path}` in the message, matching every existing kind's convention (e.g. `"Empty test data provided in {file_path}"`).
- `cargo fmt --check` and `cargo clippy` (bare, no `--tests`/`--all-targets`) must stay clean on `src/lib.rs` — CI runs exactly `cargo fmt --check && cargo clippy && cargo test` (`.github/workflows/linux.yml`).
- No support beyond two relational tiers (root + direct children); no `UPDATE`/`DELETE`; only literal values (`NULL`, quoted strings, numbers, `TRUE`/`FALSE`) in `VALUES` — anything else panics with a clear message. This is intentional scope from the design spec (`docs/superpowers/specs/2026-09-03-sql-format-design.md`).

---

### Task 1: Recognize `kind = "sql"` and implement the flat-table case

**Files:**
- Modify: `Cargo.toml`
- Modify: `src/lib.rs` (`SUPPORTED_KINDS` around line 9; `impl_test_data_file` around line 107-241)
- Create: `tests/samples/test_me.sql`
- Modify: `tests/all_types.rs`

**Interfaces:**
- Produces: a new `kind_str == "sql"` branch inside `impl_test_data_file`'s `body` computation, generating (per call site) three local `fn` items — `__test_data_file_sql_object_name(parts: &[sqlparser::ast::ObjectNamePart]) -> String`, `__test_data_file_sql_expr_to_json(expr: &sqlparser::ast::Expr) -> serde_json::Value`, `__test_data_file_load_sql_rows(file_path: &str, sql_text: &str) -> Vec<serde_json::Value>` — plus the same `_Data` struct / `#call_ident(...)` / `#func_await` tail used by the other branches. Later tasks (2, 3) rely on `__test_data_file_load_sql_rows`'s exact panic-message text, defined here.

- [ ] **Step 1: Add the `sqlparser` dev-dependency**

Edit `Cargo.toml`, in `[dev-dependencies]`, add a line after `serde_yaml = "0.9"`:

```toml
sqlparser = "0.62"
```

- [ ] **Step 2: Add `"sql"` to `SUPPORTED_KINDS`**

In `src/lib.rs`, change:

```rust
const SUPPORTED_KINDS: [&str; 6] = ["csv", "json", "yaml", "ron", "toml", "list"];
```

to:

```rust
const SUPPORTED_KINDS: [&str; 7] = ["csv", "json", "yaml", "ron", "toml", "list", "sql"];
```

- [ ] **Step 3: Write the failing test — flat SQL sample and integration test**

Create `tests/samples/test_me.sql`:

```sql
INSERT INTO test_me (name, max_size, is_above)
VALUES
  (NULL, 0, false),
  ('name', 4, false),
  ('name', 3, true),
  ('a', 0, true);
```

In `tests/all_types.rs`, add after `test_test_me_with_csv` (end of file):

```rust

#[test_data_file(path = "tests/samples/test_me.sql")]
#[test]
fn test_test_me_with_sql(name: Option<String>, max_size: usize, is_above: bool) {
    assert_eq!(
        is_name_above_max_size(name.as_deref(), max_size),
        is_above,
        "failed for {max_size}"
    );
}
```

- [ ] **Step 4: Run the test to verify it fails**

Run: `cargo test --test all_types test_test_me_with_sql`
Expected: **compile failure** — with only `SUPPORTED_KINDS` updated so far, `kind_str == "sql"` falls through every existing `if`/`else if` and hits the final generic `else` branch, which generates a call through a nonexistent `sql::de::from_reader(...)` path (it treats `"sql"` as if it were a serde-crate name, e.g. `serde_yaml`/`ron`/`toml`). Expect an error like `failed to resolve: use of undeclared crate or module `sql`` when compiling the test binary.

- [ ] **Step 5: Implement the `"sql"` branch**

In `src/lib.rs`, inside `impl_test_data_file`, the `body` is currently:

```rust
    let body = if kind_str == "csv" {
        quote! { ... }
    } else if kind_str == "list" {
        quote! { ... }
    } else {
        // generic json/yaml/ron/toml branch
        ...
    };
```

Insert a new `else if kind_str == "sql"` branch **between** the `"list"` branch and the final `else`:

```rust
    } else if kind_str == "sql" {
        quote! {
            #[derive(Debug, serde::Deserialize)]
            struct _Data {
                #(#field_names: #field_types,)*
            }

            fn __test_data_file_sql_object_name(parts: &[sqlparser::ast::ObjectNamePart]) -> String {
                match parts.last() {
                    Some(sqlparser::ast::ObjectNamePart::Identifier(ident)) => ident.value.clone(),
                    _ => panic!("unsupported object name"),
                }
            }

            fn __test_data_file_sql_expr_to_json(expr: &sqlparser::ast::Expr) -> serde_json::Value {
                match expr {
                    sqlparser::ast::Expr::Value(vws) => match &vws.value {
                        sqlparser::ast::Value::Null => serde_json::Value::Null,
                        sqlparser::ast::Value::Boolean(b) => serde_json::Value::Bool(*b),
                        sqlparser::ast::Value::Number(n, _) => n
                            .parse::<i64>()
                            .map(serde_json::Value::from)
                            .or_else(|_| n.parse::<f64>().map(serde_json::Value::from))
                            .unwrap_or_else(|_| panic!("invalid numeric literal {n}")),
                        sqlparser::ast::Value::SingleQuotedString(s) => serde_json::Value::String(s.clone()),
                        other => panic!("unsupported sql literal {other:?}"),
                    },
                    other => panic!("unsupported sql expression {other:?}"),
                }
            }

            fn __test_data_file_load_sql_rows(file_path: &str, sql_text: &str) -> Vec<serde_json::Value> {
                use std::collections::{HashMap, HashSet};
                use serde_json::{Map, Value};
                use sqlparser::ast::{SetExpr, Statement, TableObject};
                use sqlparser::dialect::GenericDialect;
                use sqlparser::parser::Parser;

                let dialect = GenericDialect {};
                let statements = Parser::parse_sql(&dialect, sql_text)
                    .unwrap_or_else(|e| panic!("failed to parse sql in {file_path} {e}"));

                let mut table_columns: HashMap<String, Vec<String>> = HashMap::new();
                let mut table_rows: HashMap<String, Vec<Map<String, Value>>> = HashMap::new();

                for statement in statements {
                    let Statement::Insert(insert) = statement else { continue; };
                    let table_name = match &insert.table {
                        TableObject::TableName(name) => __test_data_file_sql_object_name(&name.0),
                        _ => panic!("unsupported insert target in {file_path}"),
                    };
                    let columns: Vec<String> = insert
                        .columns
                        .iter()
                        .map(|c| __test_data_file_sql_object_name(&c.0))
                        .collect();

                    let query = insert
                        .source
                        .unwrap_or_else(|| panic!("INSERT INTO {table_name} has no VALUES source in {file_path}"));
                    let SetExpr::Values(values) = *query.body else {
                        panic!("INSERT INTO {table_name} must use VALUES in {file_path}");
                    };

                    for row in &values.rows {
                        let mut map = Map::new();
                        for (col, expr) in columns.iter().zip(row.iter()) {
                            map.insert(col.clone(), __test_data_file_sql_expr_to_json(expr));
                        }
                        table_rows.entry(table_name.clone()).or_default().push(map);
                    }
                    table_columns.entry(table_name.clone()).or_default().extend(columns);
                }

                if table_columns.is_empty() {
                    panic!("Empty test data provided in {file_path}");
                }

                let table_names: HashSet<&String> = table_columns.keys().collect();

                let mut child_of: HashMap<String, String> = HashMap::new();
                for (table, columns) in &table_columns {
                    for other in &table_names {
                        if *other == table {
                            continue;
                        }
                        let fk_col = format!("{other}_id");
                        if columns.iter().any(|c| c == &fk_col) {
                            child_of.insert(table.clone(), (*other).clone());
                        }
                    }
                }

                let root_candidates: Vec<&String> = table_columns
                    .keys()
                    .filter(|t| !child_of.contains_key(*t))
                    .collect();
                let root_table = match root_candidates.as_slice() {
                    [only] => (*only).clone(),
                    [] => panic!(
                        "no root table found in {file_path}: every table has an outbound '<table>_id' foreign key (check for a reference cycle)"
                    ),
                    many => panic!(
                        "ambiguous root table in {file_path}, none of these reference each other: {}",
                        many.iter().map(|s| s.as_str()).collect::<Vec<_>>().join(", ")
                    ),
                };

                let targeted_tables: HashSet<&String> = child_of.values().collect();
                let strip_root_id = targeted_tables.contains(&root_table);

                let children: Vec<&String> = child_of
                    .iter()
                    .filter(|(_, parent)| **parent == root_table)
                    .map(|(child, _)| child)
                    .collect();

                let root_rows = table_rows
                    .get(&root_table)
                    .unwrap_or_else(|| panic!("unreachable: root table {root_table} has no rows in {file_path}"));

                let mut results = Vec::new();
                for root_row in root_rows {
                    let root_id = root_row.get("id").cloned();
                    let mut out = root_row.clone();
                    if strip_root_id {
                        out.remove("id");
                    }

                    for child_table in &children {
                        let fk_col = format!("{root_table}_id");
                        let child_rows = table_rows.get(*child_table).map(|v| v.as_slice()).unwrap_or(&[]);
                        let matches: Vec<Map<String, Value>> = child_rows
                            .iter()
                            .filter(|row| row.get(&fk_col) == root_id.as_ref())
                            .map(|row| {
                                let mut row = row.clone();
                                row.remove(&fk_col);
                                row
                            })
                            .collect();
                        match matches.len() {
                            0 => {}
                            1 => {
                                out.insert((*child_table).clone(), Value::Object(matches.into_iter().next().unwrap()));
                            }
                            _ => {
                                out.insert(
                                    (*child_table).clone(),
                                    Value::Array(matches.into_iter().map(Value::Object).collect()),
                                );
                            }
                        }
                    }

                    results.push(Value::Object(out));
                }

                results
            }

            let file_path = #path;
            let sql_text = std::fs::read_to_string(file_path).unwrap();
            let rows = __test_data_file_load_sql_rows(file_path, &sql_text);

            for row in rows {
                let test_data: _Data = serde_json::from_value(row)
                    .map_err(|e| format!("Failed to load data in {file_path} {e}"))
                    .unwrap();
                let _Data { #(#field_names,)* } = test_data;
                #call_ident(#(#field_names,)*)#func_await;
            }
        }
    } else {
```

(The trailing `} else {` is the existing generic branch — leave its body untouched.)

- [ ] **Step 6: Run the test to verify it passes**

Run: `cargo test --test all_types test_test_me_with_sql`
Expected: PASS (1 test run, 0 failed). Also run `cargo test --test all_types` to confirm the other 6 format tests in the same file are unaffected.

- [ ] **Step 7: Format and lint**

Run: `cargo fmt` then `cargo fmt --check` (expect clean) and `cargo clippy` (expect no warnings/errors).

- [ ] **Step 8: Commit**

```bash
git add Cargo.toml Cargo.lock src/lib.rs tests/samples/test_me.sql tests/all_types.rs
git commit -m "Add kind = \"sql\" support for flat INSERT INTO test data"
```

---

### Task 2: Nested parent/child SQL support (foreign-key join)

**Files:**
- Create: `tests/samples/valid_users.sql`
- Create: `tests/samples/invalid_users.sql`
- Modify: `tests/complex_types.rs`

**Interfaces:**
- Consumes: `__test_data_file_load_sql_rows` from Task 1 — its root-detection/FK-join/column-stripping logic is exercised here for the first time.

**Deviation from plan (discovered while executing this task):** Task 1's original code did *not* wrap the root row under a key matching the decorated function's parameter name. For a single-parameter function like `fn f(user: User)`, `_Data` is `struct _Data { user: User }`, so each row handed to `serde_json::from_value` must be `{"user": {...}}`, not `{...}` directly — exactly like the existing JSON nested example wraps each entry under `"user"`. Task 1's flat-case test didn't catch this because it has 3 parameters, so no wrapping was ever needed there. Fixed by threading the decorated function's parameter names into `__test_data_file_load_sql_rows` (as a new `target_field_names: &[&str]` parameter, passed at the call site via `&[#(stringify!(#field_names)),*]`) and wrapping the root row under the root table's name when `target_field_names` has exactly one entry matching the root table's name. This was applied directly to `src/lib.rs` (the Task 1 code block, already committed) as its own small fix commit before Task 2's own commit, since it's a correction to Task 1's implementation, not new functionality.

- [ ] **Step 1: Add nested sample files**

Create `tests/samples/valid_users.sql`:

```sql
INSERT INTO user (id, is_cool) VALUES
  (1, true),
  (2, false),
  (3, true);

INSERT INTO address (user_id, town, country) VALUES
  (1, 'Kentucky', 'US'),
  (2, 'Unknown', 'DE'),
  (3, 'Unknown', 'DE');
```

Create `tests/samples/invalid_users.sql`:

```sql
INSERT INTO user (id, is_cool) VALUES
  (1, false),
  (2, false);

INSERT INTO address (user_id, town, country) VALUES
  (1, 'Kentucky', 'US'),
  (2, 'Unknown', 'BE');
```

(Mirrors the `is_cool`/`country` combinations already used in `tests/samples/valid_users.json` / `invalid_users.json`; `phone` is intentionally omitted from every row, same as the JSON fixtures, to keep exercising `Address.phone: Option<String>` → `None`.)

- [ ] **Step 2: Add the tests**

In `tests/complex_types.rs`, inside `mod tests { ... }`, add after `test_is_user_country_not_supported`:

```rust

    #[test_data_file(path = "tests/samples/valid_users.sql")]
    #[test]
    fn test_is_user_country_supported_sql(user: User) {
        assert!(is_user_country_supported(&user), "{}", user.address.country);
    }

    #[test_data_file(path = "tests/samples/invalid_users.sql")]
    #[test]
    fn test_is_user_country_not_supported_sql(user: User) {
        assert!(
            !is_user_country_supported(&user),
            "{}",
            user.address.country
        );
    }
```

Note: `User`/`Address` (defined at the top of the file, outside `mod tests`) don't use `#[serde(deny_unknown_fields)]`, so these two tests alone would still pass even if the `id`/`user_id` join columns leaked into the deserialized row — serde silently ignores unrecognized fields by default. Step 3 below adds a deny-unknown-fields variant specifically to prove the join columns are actually stripped, not just harmlessly ignored.

- [ ] **Step 3: Add a strict-deserialization test proving join columns are stripped**

In `tests/complex_types.rs`, add near the top (after the existing `User`/`Address` definitions, before `fn is_user_country_supported`):

```rust
#[allow(dead_code)]
#[derive(Debug)]
#[cfg_attr(test, derive(serde::Deserialize))]
#[cfg_attr(test, serde(deny_unknown_fields))]
struct StrictAddress {
    town: String,
    country: Country,
}

#[allow(dead_code)]
#[derive(Debug)]
#[cfg_attr(test, derive(serde::Deserialize))]
#[cfg_attr(test, serde(deny_unknown_fields))]
struct StrictUser {
    is_cool: bool,
    address: StrictAddress,
}
```

Then inside `mod tests { ... }`, add after the two tests from Step 2:

```rust

    #[test_data_file(path = "tests/samples/valid_users.sql")]
    #[test]
    fn test_sql_join_columns_are_stripped(user: StrictUser) {
        // If `id` (on `user`) or `user_id` (on `address`) leaked into the
        // deserialized row, `deny_unknown_fields` would make the macro's
        // internal `serde_json::from_value(...).unwrap()` panic before this
        // body ever runs.
        let _ = user;
    }
```

- [ ] **Step 4: Run the tests and verify they pass**

Run: `cargo test --test complex_types`
Expected: all 5 tests pass (2 pre-existing JSON-based, 2 SQL-based from Step 2, 1 strict-deserialization test from Step 3), confirming the parent (`user`) / child (`address`) join, `id`/`user_id` column stripping, and one-to-one embedding all work without further code changes.

- [ ] **Step 5: Format and lint**

Run: `cargo fmt --check && cargo clippy` — expect clean (test files aren't linted by bare `cargo clippy`, but keep formatting consistent).

- [ ] **Step 6: Commit**

```bash
git add tests/samples/valid_users.sql tests/samples/invalid_users.sql tests/complex_types.rs
git commit -m "Add nested parent/child SQL test data support"
```

---

### Task 3: Edge-case panics (empty file, ambiguous root, cyclic FK reference)

**Files:**
- Create: `tests/samples/empty.sql`
- Create: `tests/samples/ambiguous_root.sql`
- Create: `tests/samples/cyclic_root.sql`
- Create: `tests/sql_errors.rs`

**Interfaces:**
- Consumes: the three panic paths inside `__test_data_file_load_sql_rows` (Task 1): `"Empty test data provided in {file_path}"`, `"ambiguous root table in {file_path}, ..."`, `"no root table found in {file_path}: ..."`.

- [ ] **Step 1: Add the fixture files**

Create `tests/samples/empty.sql` (comment-only, zero `INSERT` statements):

```sql
-- intentionally empty: no INSERT statements
```

Create `tests/samples/ambiguous_root.sql` (two tables, neither references the other):

```sql
INSERT INTO a (x) VALUES (1);
INSERT INTO b (y) VALUES (2);
```

Create `tests/samples/cyclic_root.sql` (two tables that reference each other):

```sql
INSERT INTO a (id, b_id) VALUES (1, 1);
INSERT INTO b (id, a_id) VALUES (1, 1);
```

- [ ] **Step 2: Write the tests**

Create `tests/sql_errors.rs`:

```rust
use test_data_file::test_data_file;

#[test_data_file(path = "tests/samples/empty.sql")]
#[test]
#[should_panic(expected = "Empty test data provided")]
fn test_empty_sql_panics() {}

#[test_data_file(path = "tests/samples/ambiguous_root.sql")]
#[test]
#[should_panic(expected = "ambiguous root table")]
fn test_ambiguous_root_sql_panics() {}

#[test_data_file(path = "tests/samples/cyclic_root.sql")]
#[test]
#[should_panic(expected = "no root table found")]
fn test_cyclic_root_sql_panics() {}
```

- [ ] **Step 3: Run the tests and verify they pass**

Run: `cargo test --test sql_errors`
Expected: 3 passed. Each test's annotated function body is empty (`{}`) — the panic happens inside data loading, before the (never-reached) function body would run, same mechanism the macro already relies on for `#[should_panic]` support (`#[test]`/`#[should_panic]` are kept on the generated wrapper, stripped from the renamed `_<fn>`).

- [ ] **Step 4: Format and lint**

Run: `cargo fmt --check && cargo clippy` — expect clean.

- [ ] **Step 5: Commit**

```bash
git add tests/samples/empty.sql tests/samples/ambiguous_root.sql tests/samples/cyclic_root.sql tests/sql_errors.rs
git commit -m "Add panic-path tests for SQL root-table detection edge cases"
```

---

### Task 4: Documentation

**Files:**
- Modify: `README.md`
- Modify: `CLAUDE.md`

**Interfaces:**
- None (docs only; no code interfaces produced or consumed).

- [ ] **Step 1: Update the README's supported-formats table**

In `README.md`, change:

```markdown
| Format | Extension | Notes |
|--------|-----------|-------|
| YAML   | `.yaml`   | array or named-key map at the top level |
| JSON   | `.json`   | array or named-key map at the top level |
| TOML   | `.toml`   | named-key map at the top level |
| RON    | `.ron`    | array or named-key map at the top level |
| CSV    | `.csv`    | first row is the header that specifies data mapping |
| List   | `.list`   | first line is a header that specifies data mapping words are separated by space |
```

to:

```markdown
| Format | Extension | Notes |
|--------|-----------|-------|
| YAML   | `.yaml`   | array or named-key map at the top level |
| JSON   | `.json`   | array or named-key map at the top level |
| TOML   | `.toml`   | named-key map at the top level |
| RON    | `.ron`    | array or named-key map at the top level |
| CSV    | `.csv`    | first row is the header that specifies data mapping |
| List   | `.list`   | first line is a header that specifies data mapping words are separated by space |
| SQL    | `.sql`    | `INSERT INTO` statements; a parent/child table pair joined via a `<table>_id` -> `id` foreign key supplies nested struct fields |
```

- [ ] **Step 2: Update the Quick start dev-dependencies snippet**

In `README.md`, change:

```toml
[dev-dependencies]
test-data-file = "0.1"
serde = { version = "1", features = ["derive"] }
serde_json = "1"   # or serde_yaml / toml / ron / csv — whichever formats you use
```

to:

```toml
[dev-dependencies]
test-data-file = "0.1"
serde = { version = "1", features = ["derive"] }
serde_json = "1"   # or serde_yaml / toml / ron / csv — whichever formats you use
sqlparser = "0.62" # only needed for kind = "sql"
```

- [ ] **Step 3: Add a numbered example**

In `README.md`, insert a new section after "## 8. Async tests" and before "# Supported file formats":

```markdown
## 9. SQL INSERT statements

`tests/samples/test_me.sql`:

\`\`\`sql
INSERT INTO test_me (name, max_size, is_above)
VALUES
  (NULL, 0, false),
  ('name', 4, false),
  ('name', 3, true),
  ('a', 0, true);
\`\`\`

\`\`\`rust
use test_data_file::test_data_file;

fn is_name_above_max_size(name: Option<&str>, max_size: usize) -> bool {
    name.map(|n| n.len()) > Some(max_size)
}

#[test_data_file(path = "tests/samples/test_me.sql")]
#[test]
fn test_test_me_with_sql(name: Option<String>, max_size: usize, is_above: bool) {
    assert_eq!(
        is_name_above_max_size(name.as_deref(), max_size),
        is_above,
        "failed for {max_size}"
    );
}
\`\`\`

Nested struct parameters (like the `User`/`Address` example in section 4) are
supported by splitting the data across a parent table and a child table,
joined by a `<parent_table>_id` foreign key referencing the parent's `id`:

\`\`\`sql
INSERT INTO user (id, is_cool) VALUES (1, true), (2, false);

INSERT INTO address (user_id, town, country) VALUES
  (1, 'Kentucky', 'US'),
  (2, 'Unknown', 'DE');
\`\`\`

A parent row with exactly one matching child row gets that child embedded as
a nested object (`address: Address`); a parent row with more than one
matching child row gets them embedded as an array (`Vec<Address>`). Requires
`sqlparser` and `serde_json` as dev-dependencies (see Quick start).
```

(Use literal triple-backtick fences, not escaped, when editing the file — the `\`\`\`` above is only to delimit this instruction block.)

- [ ] **Step 4: Update CLAUDE.md**

In `CLAUDE.md`, change:

```markdown
`test-data-file` is a Rust proc-macro crate (single crate, `proc-macro = true`). It provides the `#[test_data_file(path = "...")]` attribute macro, which loads test data from a file (csv, json, yaml, ron, toml, or a space-separated `list` format) and calls the decorated test function once per row/entry. All macro logic lives in `src/lib.rs` — there is no other source file.
```

to:

```markdown
`test-data-file` is a Rust proc-macro crate (single crate, `proc-macro = true`). It provides the `#[test_data_file(path = "...")]` attribute macro, which loads test data from a file (csv, json, yaml, ron, toml, sql, or a space-separated `list` format) and calls the decorated test function once per row/entry. All macro logic lives in `src/lib.rs` — there is no other source file.
```

And change:

```markdown
1. **Argument parsing** (`TestFileDataAttributes::parse`): reads `path` (required, checked to exist on disk *at macro-expansion time*) and `kind` (optional). If `kind` is omitted, it's inferred from the file extension against `SUPPORTED_KINDS = ["csv", "json", "yaml", "ron", "toml", "list"]`. Files with no/unrecognized extension require `kind` explicitly.
2. **Function transformation**: the original function is renamed to `_<original_name>` (its `#[test]`/`#[should_panic]`/`#[tokio::*]` attributes are stripped), and a new function named `<original_name>` is generated that keeps the original attributes. This is what lets `cargo test` and IDE test runners discover the wrapper transparently.
3. **Body generation per `kind`**, branching in `impl_test_data_file`:
   - `csv`: builds an internal `_Data` struct (fields = the original function's parameters/types) and deserializes each row with `csv::ReaderBuilder`.
   - `list`: no serde struct — reads lines with `BufRead`, skips the header line, splits each subsequent line on spaces, and parses each field with `FromStr`.
   - everything else (`json`, `yaml`, `ron`, `toml`): also builds a `_Data` struct, plus an untagged `Collection` enum (`Index(Vec<_Data>)` or `Map(HashMap<String, _Data>)`) so both top-level arrays and named-key maps deserialize into the same iteration path.
   - All branches panic if the resulting dataset is empty, and call `_<original_name>(...)` once per row, `.await`-ing it if the original function was `async`.
4. Function parameter names/types are extracted directly from the decorated function's signature (`item.sig.inputs`) and reused both as the generated struct's fields and as the call arguments passed to `_<original_name>`.
```

to:

```markdown
1. **Argument parsing** (`TestFileDataAttributes::parse`): reads `path` (required, checked to exist on disk *at macro-expansion time*) and `kind` (optional). If `kind` is omitted, it's inferred from the file extension against `SUPPORTED_KINDS = ["csv", "json", "yaml", "ron", "toml", "list", "sql"]`. Files with no/unrecognized extension require `kind` explicitly.
2. **Function transformation**: the original function is renamed to `_<original_name>` (its `#[test]`/`#[should_panic]`/`#[tokio::*]` attributes are stripped), and a new function named `<original_name>` is generated that keeps the original attributes. This is what lets `cargo test` and IDE test runners discover the wrapper transparently.
3. **Body generation per `kind`**, branching in `impl_test_data_file`:
   - `csv`: builds an internal `_Data` struct (fields = the original function's parameters/types) and deserializes each row with `csv::ReaderBuilder`.
   - `list`: no serde struct — reads lines with `BufRead`, skips the header line, splits each subsequent line on spaces, and parses each field with `FromStr`.
   - `sql`: parses `INSERT INTO ... VALUES ...;` statements at runtime via `sqlparser`, groups rows by table name into `serde_json::Value` maps, auto-detects a single "root" table (the one table with no outbound `<other_table>_id` column), and embeds any child table's matching rows (joined on `<root_table>_id` = root's `id`) under a key named after the child table — one match embeds as an object, multiple matches embed as an array. `id`/`<parent>_id` join columns are stripped before the final `_Data` deserialization. Requires the consumer to add `sqlparser` (and `serde_json`, for the intermediate `Value` representation) as dev-dependencies.
   - everything else (`json`, `yaml`, `ron`, `toml`): also builds a `_Data` struct, plus an untagged `Collection` enum (`Index(Vec<_Data>)` or `Map(HashMap<String, _Data>)`) so both top-level arrays and named-key maps deserialize into the same iteration path.
   - All branches panic if the resulting dataset is empty, and call `_<original_name>(...)` once per row, `.await`-ing it if the original function was `async`.
4. Function parameter names/types are extracted directly from the decorated function's signature (`item.sig.inputs`) and reused both as the generated struct's fields and as the call arguments passed to `_<original_name>`.
```

- [ ] **Step 5: Full verification pass**

Run: `cargo fmt --check && cargo clippy && cargo test`
Expected: all pass — this is the same sequence CI runs, confirming nothing in Tasks 1-4 broke any existing behavior (doctest included, since `src/lib.rs`'s doctest still targets `tests/samples/test_me.yaml`, untouched by this plan).

- [ ] **Step 6: Commit**

```bash
git add README.md CLAUDE.md
git commit -m "Document kind = \"sql\" support in README and CLAUDE.md"
```
