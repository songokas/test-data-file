//! Realistic nested-struct and collection examples. Data files live in
//! `tests/examples/`. JSON carries the nesting inline; SQL builds the same
//! shape from a parent table joined to a child table.

use test_data_file::test_data_file;

#[allow(dead_code)]
#[derive(Debug, serde::Deserialize)]
struct Profile {
    country: String,
    plan: String,
}

#[derive(Debug, serde::Deserialize)]
struct Account {
    username: String,
    is_active: bool,
    profile: Profile,
}

fn can_access_beta(account: &Account) -> bool {
    account.is_active && account.profile.plan == "pro"
}

#[test_data_file(path = "tests/examples/accounts.json")]
#[test]
fn beta_access_json(account: Account) {
    // only the active "pro" account qualifies
    assert_eq!(
        can_access_beta(&account),
        account.username == "ada",
        "failed for {}",
        account.username
    );
}

#[test_data_file(path = "tests/examples/accounts.sql")]
#[test]
fn beta_access_sql(account: Account) {
    assert_eq!(
        can_access_beta(&account),
        account.username == "ada",
        "failed for {}",
        account.username
    );
}

const KNOWN_ROLES: [&str; 4] = ["admin", "billing", "editor", "viewer"];

/// Collection parameter: each entry's `roles` array becomes a `Vec<String>`.
/// The file has no extension, so `kind` is given explicitly.
#[test_data_file(kind = "json", path = "tests/examples/role_sets")]
#[test]
fn role_sets_are_known(roles: Vec<String>) {
    assert!(!roles.is_empty(), "role set must not be empty");
    for role in &roles {
        assert!(KNOWN_ROLES.contains(&role.as_str()), "unknown role: {role}");
    }
}

#[allow(dead_code)]
#[derive(Debug, serde::Deserialize)]
struct Member {
    name: String,
    role: String,
}

#[derive(Debug, serde::Deserialize)]
struct Team {
    name: String,
    members: Vec<Member>,
}

/// A parent row with more than one matching child row embeds the children as
/// an array, so `members` deserializes into `Vec<Member>`.
#[test_data_file(path = "tests/examples/teams.sql")]
#[test]
fn team_has_one_admin(team: Team) {
    let admins = team.members.iter().filter(|m| m.role == "admin").count();
    assert_eq!(
        admins, 1,
        "team {} should have exactly one admin",
        team.name
    );
}
