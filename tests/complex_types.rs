use iso_country::Country;

#[allow(dead_code)]
#[derive(Debug)]
#[cfg_attr(test, derive(serde::Deserialize))]
struct Address {
    town: String,
    phone: Option<String>,
    country: Country,
}

#[derive(Debug)]
#[cfg_attr(test, derive(serde::Deserialize))]
struct User {
    is_cool: bool,
    address: Address,
}

fn is_user_country_supported(user: &User) -> bool {
    user.is_cool || user.address.country == Country::DE
}

#[cfg(test)]
mod tests {
    use test_data_file::test_data_file;

    use super::*;

    #[test_data_file(path = "tests/samples/valid_users.json")]
    #[test]
    fn test_is_user_country_supported(user: User) {
        assert!(is_user_country_supported(&user), "{}", user.address.country);
    }

    #[test_data_file(path = "tests/samples/invalid_users.json")]
    #[test]
    fn test_is_user_country_not_supported(user: User) {
        assert!(
            !is_user_country_supported(&user),
            "{}",
            user.address.country
        );
    }
}
