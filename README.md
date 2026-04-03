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
```

# Examples

## 1. Basic scalar parameters

`tests/samples/password_rules.yaml`:

```yaml
- password: ""
  min_length: 8
  expected: false
- password: "short"
  min_length: 8
  expected: false
- password: "correct-horse"
  min_length: 8
  expected: true
- password: "correct-horse"
  min_length: 20
  expected: false
```

```rust
use test_data_file::test_data_file;

fn is_password_valid(password: &str, min_length: usize) -> bool {
    password.len() >= min_length
}

#[test_data_file(path = "tests/samples/password_rules.yaml")]
#[test]
fn test_password_validation(password: String, min_length: usize, expected: bool) {
    assert_eq!(
        is_password_valid(&password, min_length),
        expected,
        "password={password:?} min_length={min_length}"
    );
}
```

## 2. Optional fields

Fields that may be absent in some rows can be typed as `Option<T>`. The macro deserialises missing JSON keys as `None`.

`tests/samples/users.json`:

```json
[
    { "username": "alice", "email": "alice@example.com", "age": 30 },
    { "username": "bob",   "email": "bob@example.com" },
    { "username": "carol", "age": 25 }
]
```

```rust
use test_data_file::test_data_file;

fn is_profile_complete(email: Option<&str>, age: Option<u32>) -> bool {
    email.is_some() && age.is_some()
}

#[test_data_file(path = "tests/samples/users.json")]
#[test]
fn test_profile_completeness(username: String, email: Option<String>, age: Option<u32>) {
    let complete = is_profile_complete(email.as_deref(), age);
    // only alice has both fields
    assert_eq!(complete, username == "alice", "failed for {username}");
}
```

## 3. Named test cases (TOML)


`tests/samples/discount_rules.toml`:

```toml
[new_customer]
order_total = 100.0
is_member   = false
expected_discount = 0.0

[member_small_order]
order_total = 40.0
is_member   = true
expected_discount = 0.0

[member_large_order]
order_total = 120.0
is_member   = true
expected_discount = 12.0
```

```rust
use test_data_file::test_data_file;

fn calculate_discount(order_total: f64, is_member: bool) -> f64 {
    if is_member && order_total >= 100.0 { order_total * 0.10 } else { 0.0 }
}

#[test_data_file(path = "tests/samples/discount_rules.toml")]
#[test]
fn test_discount(order_total: f64, is_member: bool, expected_discount: f64) {
    let discount = calculate_discount(order_total, is_member);
    assert!(
        (discount - expected_discount).abs() < f64::EPSILON,
        "order_total={order_total} is_member={is_member}"
    );
}
```

## 4. Nested / complex types

Parameters can be arbitrary `serde::Deserialize` types, including your own structs.

`tests/samples/valid_users.json`:

```json
{
    "user is cool": {
        "user": { "is_cool": true,  "address": { "town": "Kentucky", "country": "US" } }
    },
    "user is from the right country": {
        "user": { "is_cool": false, "address": { "town": "Unknown",  "country": "DE" } }
    }
}
```

```rust
use test_data_file::test_data_file;

#[derive(Debug, serde::Deserialize)]
struct Address { town: String, country: String }

#[derive(Debug, serde::Deserialize)]
struct User { is_cool: bool, address: Address }

fn is_user_country_supported(user: &User) -> bool {
    user.is_cool || user.address.country == "DE"
}

#[test_data_file(path = "tests/samples/valid_users.json")]
#[test]
fn test_supported_users(user: User) {
    assert!(
        is_user_country_supported(&user),
        "expected support for user in {}",
        user.address.country
    );
}
```

## 5. Collection parameters

A parameter can itself be a `Vec<T>`. When the data file has no recognised extension, specify `kind` explicitly.

`tests/samples/tag_sets` (no extension):

```json
{
    "non_empty_tags":  { "tags": ["rust", "testing", "macros"] },
    "single_tag":      { "tags": ["rust"] }
}
```

```rust
use test_data_file::test_data_file;

#[test_data_file(kind = "json", path = "tests/samples/tag_sets")]
#[test]
fn test_tags_not_empty(tags: Vec<String>) {
    assert!(!tags.is_empty());
    assert!(tags.iter().all(|t| !t.is_empty()), "blank tag found");
}
```

## 6. CSV — tabular data

`tests/samples/conversions.csv`:

```csv
celsius,expected_fahrenheit
0,32
100,212
-40,-40
37,98.6
```

```rust
use test_data_file::test_data_file;

fn celsius_to_fahrenheit(c: f64) -> f64 { c * 9.0 / 5.0 + 32.0 }

#[test_data_file(path = "tests/samples/conversions.csv")]
#[test]
fn test_temperature_conversion(celsius: f64, expected_fahrenheit: f64) {
    let result = celsius_to_fahrenheit(celsius);
    assert!(
        (result - expected_fahrenheit).abs() < 0.01,
        "{celsius}°C → expected {expected_fahrenheit}°F, got {result}"
    );
}
```

## 7. Space-separated list

The `list` format is a plain text file: the first line is a header (used only to map parameters), subsequent lines are space-separated values parsed with `FromStr`.

`tests/samples/ip_ports.list`:

```
host         port  is_https
127.0.0.1    80    false
example.com  443   true
localhost    8080  false
```

```rust
use test_data_file::test_data_file;

fn default_scheme(is_https: bool) -> &'static str {
    if is_https { "https" } else { "http" }
}

#[test_data_file(path = "tests/samples/ip_ports.list")]
#[test]
fn test_scheme_detection(host: String, port: u16, is_https: bool) {
    let scheme = default_scheme(is_https);
    let url = format!("{scheme}://{host}:{port}");
    assert!(url.starts_with(scheme), "bad url: {url}");
}
```

## 8. Async tests

`tests/samples/endpoints.yaml`:

```yaml
- url: "/health"
  expect_status: 200
- url: "/not-found"
  expect_status: 404
```

```rust
use test_data_file::test_data_file;

async fn fetch_status(url: &str) -> u16 {
    // ... real HTTP call here
    if url == "/health" { 200 } else { 404 }
}

#[test_data_file(path = "tests/samples/endpoints.yaml")]
#[tokio::test]
async fn test_endpoint_status(url: String, expect_status: u16) {
    let status = fetch_status(&url).await;
    assert_eq!(status, expect_status, "unexpected status for {url}");
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

When the file has no extension (or a non-standard one), pass `kind = "<format>"` explicitly:

```rust
#[test_data_file(kind = "json", path = "tests/samples/my_data")]
```

# How it works

The macro renames your function to `_<name>` and generates a new `<name>()` wrapper that:

1. Reads and deserialises the data file at runtime.
2. Iterates over every row, unpacking each entry into your function's parameters.
3. Calls `_<name>(params…)` for each row.

Because the generated function has the same name and attributes as your original, `cargo test`, IDE test runners all work without any extra configuration.
