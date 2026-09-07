//! Realistic scalar-parameter examples: the same signup-validation dataset
//! expressed in every supported flat format. Data files live in `tests/examples/`.

use test_data_file::test_data_file;

/// Returns the first validation error for a signup, or `None` when it is valid.
fn signup_error(
    email: &str,
    password: &str,
    age: u8,
    accepted_terms: bool,
) -> Option<&'static str> {
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

fn assert_signup(email: &str, password: &str, age: u8, accepted_terms: bool, expected_valid: bool) {
    assert_eq!(
        signup_error(email, password, age, accepted_terms).is_none(),
        expected_valid,
        "email={email:?} password_len={} age={age} accepted_terms={accepted_terms}",
        password.len()
    );
}

#[test_data_file(path = "tests/examples/signups.csv")]
#[test]
fn signup_rules_csv(
    email: String,
    password: String,
    age: u8,
    accepted_terms: bool,
    expected_valid: bool,
) {
    assert_signup(&email, &password, age, accepted_terms, expected_valid);
}

#[test_data_file(path = "tests/examples/signups.list")]
#[test]
fn signup_rules_list(
    email: String,
    password: String,
    age: u8,
    accepted_terms: bool,
    expected_valid: bool,
) {
    assert_signup(&email, &password, age, accepted_terms, expected_valid);
}

#[test_data_file(path = "tests/examples/signups.json")]
#[test]
fn signup_rules_json(
    email: String,
    password: String,
    age: u8,
    accepted_terms: bool,
    expected_valid: bool,
) {
    assert_signup(&email, &password, age, accepted_terms, expected_valid);
}

#[test_data_file(path = "tests/examples/signups.yaml")]
#[test]
fn signup_rules_yaml(
    email: String,
    password: String,
    age: u8,
    accepted_terms: bool,
    expected_valid: bool,
) {
    assert_signup(&email, &password, age, accepted_terms, expected_valid);
}

#[test_data_file(path = "tests/examples/signups.ron")]
#[test]
fn signup_rules_ron(
    email: String,
    password: String,
    age: u8,
    accepted_terms: bool,
    expected_valid: bool,
) {
    assert_signup(&email, &password, age, accepted_terms, expected_valid);
}

#[test_data_file(path = "tests/examples/signups.toml")]
#[test]
fn signup_rules_toml(
    email: String,
    password: String,
    age: u8,
    accepted_terms: bool,
    expected_valid: bool,
) {
    assert_signup(&email, &password, age, accepted_terms, expected_valid);
}

#[test_data_file(path = "tests/examples/signups.sql")]
#[test]
fn signup_rules_sql(
    email: String,
    password: String,
    age: u8,
    accepted_terms: bool,
    expected_valid: bool,
) {
    assert_signup(&email, &password, age, accepted_terms, expected_valid);
}

/// Optional fields: rows that omit a key deserialize it as `None`.
#[test_data_file(path = "tests/examples/contacts.json")]
#[test]
fn contact_completeness(username: String, email: Option<String>, age: Option<u32>) {
    let complete = email.is_some() && age.is_some();
    // only "ada" supplies both an email and an age
    assert_eq!(complete, username == "ada", "failed for {username}");
}
