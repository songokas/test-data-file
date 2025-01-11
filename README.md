# About

Separate your tests from a testing data.

Use a simple sample file to drive your tests.

Concern yourself with providing a good sample data instead of writing a test itself.

Testing data readability matters.

Reuse testing data where it makes sense.

# Example

```rust
#[test_data_file(path = "tests/samples/test_me.yaml")]
#[test]
fn test_is_name_above_max_size(name: Option<String>, max_size: usize, is_above: bool) {
    assert_eq!(
        is_name_above_max_size(name.as_deref(), max_size),
        is_above,
        "failed for {max_size}"
    );
}
```

# Supported sample file formats

csv,toml,json,yaml,ron,list

# How it works

Macro simply renames your test function with prefix \_ and creates the same function
which calls your original function with the testing data supplied by the sample.
So there are no surprises and `cargo test --test test_test_me_with_yaml`
and running tests from your editor works as expected.

So instead of writing everywhere something like:

```rust
#[test]
fn test_test_me_with_yaml() {
    let data = [
        (Some("John".to_string()), 3, false),
        (Some("John".to_string(), 0, false),
        (Some("John".to_string(), 4, false),
        (Some("John".to_string(), 5, true),
        (Some("".to_string(), 3, false),
        (Some("".to_string(), 0, false),
        (None, 3, false),
    ]
    for (name, max_size, is_above) in data {
        assert_eq!(
            is_name_above_max_size(name.as_deref(), max_size),
            is_above,
            "failed for {max_size}"
        );
    }
}
```

You focus on the testing data
