# SQL INSERT format support — design

## Summary

Add a new `kind = "sql"` to `test_data_file`, auto-detected from a `.sql`
extension, that reads test data from `INSERT INTO ... VALUES ...;`
statements. It supports two shapes: a flat single-table shape (mirrors the
existing CSV/list formats) and a nested multi-table shape using a
parent/child foreign-key convention (mirrors the existing nested-struct JSON
example in `tests/complex_types.rs`).

## Motivation

The crate already supports csv, json, yaml, ron, toml, and list. SQL dump
snippets are a natural, readable way to hand-author relational-shaped test
fixtures (especially for nested struct parameters), and are a format users
may already have on hand (e.g. copied from a seed script or a DB dump).

## Recognition & dependencies

- Add `"sql"` to `SUPPORTED_KINDS` in `src/lib.rs`.
- `.sql` extension is auto-detected the same way other kinds are (via
  `TestFileDataAttributes::parse`); `kind = "sql"` can also be specified
  explicitly for files without that extension.
- The macro crate itself (`syn`/`quote`/`proc-macro2`) gains no new
  dependency — parsing happens in the *generated code*, which runs in the
  consumer's test binary, same as how `csv`/`serde_yaml`/`toml` parsing
  happens today. Consumers using `kind = "sql"` must add two dev-dependencies:
  - `sqlparser` — parses the `.sql` file text into an AST.
  - `serde_json` — its `Value` type is the intermediate representation each
    row is built into before final deserialization into the generated
    `_Data` struct (reusing `serde_json::from_value`).

## File shapes

### Flat case

Mirrors `tests/samples/test_me.csv` / `tests/all_types.rs`. One (or more)
`INSERT INTO <any_name> (col1, col2, ...) VALUES (...), (...), ...;`
statement. The table name is not inspected. Columns map directly by name to
the decorated function's parameter names. Multiple `INSERT` statements
targeting the same table simply append rows.

```sql
INSERT INTO test_me (name, max_size, is_above)
VALUES
  (NULL, 0, false),
  ('name', 4, false),
  ('name', 3, true),
  ('a', 0, true);
```

### Nested case

Mirrors `tests/samples/valid_users.json` / `tests/complex_types.rs`
(`user: User { is_cool: bool, address: Address }`).

- A **parent** table whose name matches the top-level parameter (e.g.
  `user`), with an `id` column.
- A **child** table whose name matches the nested field (e.g. `address`),
  with a `<parent_table>_id` foreign-key column (e.g. `user_id`) referencing
  the parent's `id`.

```sql
INSERT INTO user (id, is_cool) VALUES (1, true), (2, false);

INSERT INTO address (user_id, town, phone, country) VALUES
  (1, 'Kentucky', NULL, 'US'),
  (2, 'Unknown', NULL, 'DE');
```

For each parent row, matching child rows (`child.<parent>_id == parent.id`)
are embedded into that parent row under a key equal to the child table's
name:
- exactly one matching child row → embedded as a JSON object (for a
  singular nested struct field, e.g. `address: Address`)
- more than one matching child row → embedded as a JSON array (for a
  `Vec<T>` nested field)

The macro does not know the actual Rust field types (proc-macro attribute
expansion only sees the decorated function's parameter list, not the body
of `User`/`Address`), so the number of matching child rows the `.sql` file
supplies determines object-vs-array shape; a mismatch against the real
struct surfaces as a normal `serde` deserialization panic at test run time,
same as a malformed JSON/YAML file today.

### Root table detection

Exactly one table in the file must not be a "child" (i.e. not the source of
a `<x>_id` FK column). That table is the root, and its rows become the test
cases fed to the decorated function.

- Purely flat file (single table, no FK columns) → that table is trivially
  root, regardless of its name.
- Nested file → the parent table (not referenced by any `_id` FK it emits,
  but referenced *by* a child) is root.
- More than one root-candidate table found → panic with a message naming
  the ambiguous tables, at test run time (this is a data-authoring error,
  not something the macro can resolve).

### Column stripping

Applied before each row is handed to `serde_json::from_value`:
- A child table's `<parent_table>_id` FK column is always stripped (pure
  join plumbing, never a real struct field).
- A table's `id` column is stripped **only if** it is the target of at
  least one other table's FK (i.e. it's acting as a join key). A root table
  that is never referenced by a child keeps its `id` column as ordinary
  data.

## Value mapping

SQL literal → JSON value, same semantics the JSON/YAML/RON paths already
rely on for `Option<T>` handling:

| SQL literal | JSON value |
|---|---|
| `NULL` | `null` (→ `None` for `Option<T>`, same as a missing/null JSON key) |
| `'quoted string'` | string |
| numeric literal | number |
| `TRUE` / `FALSE` | bool |

## Empty data

If the root table has zero rows after parsing, panic with
`"Empty test data provided in {file_path}"`, matching every other kind's
existing behavior.

## Code generation shape

Follows the same pattern as the existing non-csv/non-list branch in
`impl_test_data_file`: define a `_Data` struct from the decorated function's
parameter names/types, then generate a runtime body that:
1. Reads the `.sql` file to a string.
2. Parses it with `sqlparser` (`GenericDialect`) into a list of `Statement::Insert`.
3. Groups rows by table name, builds each row as a `serde_json::Map`
   (column name → JSON value per the value-mapping table above).
4. Determines the root table (per "Root table detection"), embeds matching
   child rows into it (per "Nested case"), strips FK/id columns (per
   "Column stripping").
5. Panics if the root row set is empty.
6. For each resulting root row, deserializes into `_Data` via
   `serde_json::from_value` and calls `_<fn_name>(...)`, `.await`-ing if the
   original function is `async` — identical tail behavior to the existing
   json/yaml/ron/toml branch.

## Tests & docs

- `tests/samples/test_me.sql` — flat shape, same 4 rows as
  `test_me.csv`/`test_me.yaml`/etc. Wired into `tests/all_types.rs` as
  `test_test_me_with_sql`, alongside the other 6 formats.
- New nested-shape `.sql` samples mirroring `valid_users.json` /
  `invalid_users.json` (parent/child `user`/`address` tables), wired into
  `tests/complex_types.rs` (or a new dedicated test file if that reads
  cleaner once written).
- `README.md`: add `sql` to the "Supported file formats" table, and a
  worked example in the numbered example list (flat, plus a nested
  variant reusing the existing `User`/`Address` example from section 4).
  Quick-start dev-dependencies snippet gets a note that `sql` needs
  `sqlparser` + `serde_json`.
- `CLAUDE.md`: update `SUPPORTED_KINDS` mention and the architecture
  section's per-kind branch description.

## Out of scope / limitations (v1)

- No support for more than two relational levels (grandchild tables via
  chained FKs) — only direct parent/child.
- No support for a table being both a parent and a child simultaneously
  (i.e. exactly two tiers: root + its direct children).
- No `UPDATE`/`DELETE`/multi-value-per-column SQL expressions — only
  literal values in `INSERT ... VALUES`.
- Ambiguous multi-root files are a hard panic, not an error the macro can
  auto-resolve.
