# test-data-file

A proc-macro attribute that drives your test functions with data loaded from a file.

Separate your test logic from your test data. Write the test once, supply as many cases as you need in a data file, and let the macro call your test function for each row.

- Keep tests readable — no giant inline arrays of tuples
- Add or tweak test cases by editing a file, not Rust code (no recompiliation)
- Reuse the same data file across multiple tests

# Quick start

Add to `Cargo.toml`:

```toml
[dev-dependencies]
test-data-file = "0.1"
serde = { version = "1", features = ["derive"] }
serde_json = "1"   # or serde_yaml / toml / ron / csv — whichever formats you use
sqlparser = "0.62" # only needed for kind = "sql"
```

# Examples

All examples below drive the same signup-validation logic. The full data files
live in [`tests/examples/`](tests/examples).

## 1. Basic scalar parameters

`tests/examples/signups.yaml`:

```yaml
- email: "ada@example.com"
  password: "horse-battery-staple"
  age: 34
  accepted_terms: true
  expected_valid: true
- email: "not-an-email"
  password: "Hopper1906"
  age: 29
  accepted_terms: true
  expected_valid: false
- email: "kay@example.com"
  password: "secret"
  age: 22
  accepted_terms: true
  expected_valid: false
```

```rust
use test_data_file::test_data_file;

/// Returns the first validation error, or `None` when the signup is valid.
fn signup_error(email: &str, password: &str, age: u8, accepted_terms: bool) -> Option<&'static str> {
    if !email.contains('@') {
        Some("invalid email")
    } else if password.len() < 8 {
        Some("password too short")
    } else if age < 13 {
        Some("under minimum age")
    } else if !accepted_terms {
        Some("terms not accepted")
    } else {
        None
    }
}

#[test_data_file(path = "tests/examples/signups.yaml")]
#[test]
fn test_signup_validation(
    email: String,
    password: String,
    age: u8,
    accepted_terms: bool,
    expected_valid: bool,
) {
    assert_eq!(
        signup_error(&email, &password, age, accepted_terms).is_none(),
        expected_valid,
        "email={email:?}"
    );
}
```

## 2. Optional fields

Fields that may be absent in some rows can be typed as `Option<T>`. The macro deserialises missing JSON keys as `None`.

`tests/examples/contacts.json`:

```json
[
    { "username": "ada",   "email": "ada@example.com", "age": 34 },
    { "username": "grace", "email": "grace@example.com" },
    { "username": "lin",   "age": 29 }
]
```

```rust
use test_data_file::test_data_file;

fn has_complete_contact(email: Option<&str>, age: Option<u32>) -> bool {
    email.is_some() && age.is_some()
}

#[test_data_file(path = "tests/examples/contacts.json")]
#[test]
fn test_contact_completeness(username: String, email: Option<String>, age: Option<u32>) {
    let complete = has_complete_contact(email.as_deref(), age);
    // only ada supplies both an email and an age
    assert_eq!(complete, username == "ada", "failed for {username}");
}
```

## 3. Named test cases (TOML)


Each table becomes one named case, which shows up in test output. Same dataset
as section 1, keyed by scenario.

`tests/examples/signups.toml`:

```toml
[valid_passphrase]
email = "ada@example.com"
password = "horse-battery-staple"
age = 34
accepted_terms = true
expected_valid = true

[password_too_short]
email = "kay@example.com"
password = "secret"
age = 22
accepted_terms = true
expected_valid = false

[terms_declined]
email = "lin@example.com"
password = "trombone-clip"
age = 29
accepted_terms = false
expected_valid = false
```

```rust
use test_data_file::test_data_file;

// `signup_error` as defined in section 1

#[test_data_file(path = "tests/examples/signups.toml")]
#[test]
fn test_signup_cases(
    email: String,
    password: String,
    age: u8,
    accepted_terms: bool,
    expected_valid: bool,
) {
    assert_eq!(
        signup_error(&email, &password, age, accepted_terms).is_none(),
        expected_valid,
    );
}
```

## 4. Nested / complex types

Parameters can be arbitrary `serde::Deserialize` types, including your own structs.

`tests/examples/accounts.json`:

```json
{
    "active pro account": {
        "account": { "username": "ada", "is_active": true, "profile": { "country": "US", "plan": "pro" } }
    },
    "active free account": {
        "account": { "username": "lin", "is_active": true, "profile": { "country": "GB", "plan": "free" } }
    },
    "inactive pro account": {
        "account": { "username": "sam", "is_active": false, "profile": { "country": "DE", "plan": "pro" } }
    }
}
```

```rust
use test_data_file::test_data_file;

#[derive(Debug, serde::Deserialize)]
struct Profile { country: String, plan: String }

#[derive(Debug, serde::Deserialize)]
struct Account { username: String, is_active: bool, profile: Profile }

fn can_access_beta(account: &Account) -> bool {
    account.is_active && account.profile.plan == "pro"
}

#[test_data_file(path = "tests/examples/accounts.json")]
#[test]
fn test_beta_access(account: Account) {
    // only the active "pro" account qualifies
    assert_eq!(
        can_access_beta(&account),
        account.username == "ada",
        "failed for {}",
        account.username
    );
}
```

## 5. Collection parameters

A parameter can itself be a `Vec<T>`. When the data file has no recognised extension, specify `kind` explicitly.

`tests/examples/role_sets` (no extension):

```json
{
    "admin_and_billing": { "roles": ["admin", "billing"] },
    "single_viewer":     { "roles": ["viewer"] },
    "editor_and_viewer": { "roles": ["editor", "viewer"] }
}
```

```rust
use test_data_file::test_data_file;

const KNOWN_ROLES: [&str; 4] = ["admin", "billing", "editor", "viewer"];

#[test_data_file(kind = "json", path = "tests/examples/role_sets")]
#[test]
fn test_role_sets_are_known(roles: Vec<String>) {
    assert!(!roles.is_empty(), "role set must not be empty");
    assert!(
        roles.iter().all(|r| KNOWN_ROLES.contains(&r.as_str())),
        "unknown role in {roles:?}"
    );
}
```

## 6. CSV — tabular data

The same signup dataset as section 1, in tabular form. The first row is the header.

`tests/examples/signups.csv`:

```csv
email,password,age,accepted_terms,expected_valid
ada@example.com,horse-battery-staple,34,true,true
grace@example.com,Hopper1906,47,true,true
not-an-email,Hopper1906,29,true,false
kay@example.com,secret,22,true,false
```

```rust
use test_data_file::test_data_file;

// `signup_error` as defined in section 1

#[test_data_file(path = "tests/examples/signups.csv")]
#[test]
fn test_signup_validation_csv(
    email: String,
    password: String,
    age: u8,
    accepted_terms: bool,
    expected_valid: bool,
) {
    assert_eq!(
        signup_error(&email, &password, age, accepted_terms).is_none(),
        expected_valid,
    );
}
```

## 7. Space-separated list

The `list` format is a plain text file: the first line is a header (used only to
map parameters), subsequent lines are space-separated values parsed with
`FromStr`. Because values are whitespace-separated, no field may contain a space.

`tests/examples/signups.list`:

```
email             password              age  accepted_terms  expected_valid
ada@example.com    horse-battery-staple  34   true            true
grace@example.com  Hopper1906            47   true            true
not-an-email       Hopper1906            29   true            false
kay@example.com    secret                22   true            false
```

```rust
use test_data_file::test_data_file;

// `signup_error` as defined in section 1

#[test_data_file(path = "tests/examples/signups.list")]
#[test]
fn test_signup_validation_list(
    email: String,
    password: String,
    age: u8,
    accepted_terms: bool,
    expected_valid: bool,
) {
    assert_eq!(
        signup_error(&email, &password, age, accepted_terms).is_none(),
        expected_valid,
    );
}
```

## 8. Async tests

`tests/examples/welcome_emails.yaml`:

```yaml
- email: "ada@example.com"
  locale: "en-US"
  expect_sent: true
- email: "not-an-email"
  locale: "en-US"
  expect_sent: false
```

```rust
use test_data_file::test_data_file;

async fn send_welcome_email(email: &str, _locale: &str) -> bool {
    // ... real async mail enqueue here
    email.contains('@')
}

#[test_data_file(path = "tests/examples/welcome_emails.yaml")]
#[tokio::test]
async fn test_welcome_email_delivery(email: String, locale: String, expect_sent: bool) {
    assert_eq!(
        send_welcome_email(&email, &locale).await,
        expect_sent,
        "failed for {email}"
    );
}
```

## 9. SQL INSERT statements

A flat `INSERT INTO` maps each column to a parameter — the same signup dataset
as section 1.

`tests/examples/signups.sql`:

```sql
INSERT INTO signups (email, password, age, accepted_terms, expected_valid)
VALUES
  ('ada@example.com', 'horse-battery-staple', 34, true, true),
  ('not-an-email', 'Hopper1906', 29, true, false),
  ('kay@example.com', 'secret', 22, true, false);
```

```rust
use test_data_file::test_data_file;

// `signup_error` as defined in section 1

#[test_data_file(path = "tests/examples/signups.sql")]
#[test]
fn test_signup_validation_sql(
    email: String,
    password: String,
    age: u8,
    accepted_terms: bool,
    expected_valid: bool,
) {
    assert_eq!(
        signup_error(&email, &password, age, accepted_terms).is_none(),
        expected_valid,
    );
}
```

Nested struct parameters (like the `Account`/`Profile` example in section 4) are
supported by splitting the data across a parent table and a child table, joined
by a `<parent_table>_id` foreign key referencing the parent's `id`.

`tests/examples/accounts.sql`:

```sql
INSERT INTO account (id, username, is_active) VALUES
  (1, 'ada', true),
  (2, 'lin', true);

INSERT INTO profile (account_id, country, plan) VALUES
  (1, 'US', 'pro'),
  (2, 'GB', 'free');
```

A parent row with exactly one matching child row gets that child embedded as
a nested object (`profile: Profile`); a parent row with more than one
matching child row gets them embedded as an array (`Vec<Profile>`). The `id`
and `<parent>_id` join columns are stripped before deserialization. Requires
`sqlparser` and `serde_json` as dev-dependencies (see Quick start).

When no single root table can be found because none of the tables reference
another, each table is deserialized into its own struct and passed as the
parameter whose name matches the table. The **first table in the file is the
root**: the test runs once per root row. Every other table pairs by position
— root row `i` gets that table's row `i`, and can have any number of rows.
When a table has no row at position `i`, that parameter must be an
`Option<_>` and deserializes to `None`; rows past the last root row are
ignored.

```sql
INSERT INTO product (name, price, in_stock) VALUES
  ('Widget', 999, true),
  ('Gadget', 1499, false);

INSERT INTO customer (name, country, vip) VALUES
  ('Ada', 'US', true);
```

```rust
#[test_data_file(path = "tests/examples/catalog.sql")]
#[test]
fn test_catalog(product: Product, customer: Option<Customer>) {
    // runs twice: (Widget, Some(Ada)) then (Gadget, None)
}
```

# Supported file formats

| Format | Extension | Notes |
|--------|-----------|-------|
| YAML   | `.yaml`   | array or named-key map at the top level |
| JSON   | `.json`   | array or named-key map at the top level |
| TOML   | `.toml`   | named-key map at the top level |
| RON    | `.ron`    | array or named-key map at the top level |
| CSV    | `.csv`    | first row is the header that specifies data mapping |
| List   | `.list`   | first line is a header that specifies data mapping words are separated by space |
| SQL    | `.sql`    | `INSERT INTO` statements; a parent/child table pair joined via a `<table>_id` -> `id` foreign key supplies nested struct fields; unrelated tables each become a struct parameter named after the table, the first table driving iteration and the rest pairing by row position (`Option<_>` where a position has no row) |

When the file has no extension (or a non-standard one), pass `kind = "<format>"` explicitly:

```rust
#[test_data_file(kind = "json", path = "tests/examples/my_data")]
```

# How it works

The macro renames your function to `_<name>` and generates a new `<name>()` wrapper that:

1. Reads and deserialises the data file at runtime.
2. Iterates over every row, unpacking each entry into your function's parameters.
3. Calls `_<name>(params…)` for each row.

Because the generated function has the same name and attributes as your original, `cargo test`, IDE test runners all work without any extra configuration.
