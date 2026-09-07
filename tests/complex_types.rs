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

#[allow(dead_code)]
#[derive(Debug)]
#[cfg_attr(test, derive(serde::Deserialize))]
struct Product {
    name: String,
    price: u32,
    in_stock: bool,
}

#[allow(dead_code)]
#[derive(Debug)]
#[cfg_attr(test, derive(serde::Deserialize))]
struct Customer {
    name: String,
    country: Country,
    vip: bool,
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

    #[test_data_file(path = "tests/samples/ambiguous_roots.sql")]
    #[test]
    fn test_sql_multi_table_becomes_struct_fields(product: Product, customer: Customer) {
        // No table references another, so each table is deserialized into its
        // own struct and passed as the parameter matching the table name. Both
        // tables have one row, so the test runs once.
        assert_eq!(product.name, "Widget");
        assert!(product.in_stock);
        assert_eq!(customer.country, Country::US);
        assert!(customer.vip);
    }

    #[test_data_file(path = "tests/samples/ambiguous_roots_paired.sql")]
    #[test]
    fn test_sql_multi_table_pairs_rows_by_position(product: Product, customer: Customer) {
        // `product` is the root (first table); `customer` row i pairs with
        // product row i. The data is arranged so this holds only when the
        // pairing is by position (Widget<->Ada, Gadget<->Bea).
        assert_eq!(
            product.in_stock, customer.vip,
            "mispaired {} with {}",
            product.name, customer.name
        );
    }

    #[test_data_file(path = "tests/samples/ambiguous_roots_partial.sql")]
    #[test]
    fn test_sql_multi_table_missing_position_is_none(product: Product, customer: Option<Customer>) {
        // `customer` has fewer rows than the root `product`, so the root row
        // with no customer at its position gets `None`.
        assert_eq!(
            customer.is_some(),
            product.name == "Widget",
            "unexpected customer for {}",
            product.name
        );
    }

    #[test_data_file(path = "tests/samples/ambiguous_roots_extra.sql")]
    #[test]
    fn test_sql_multi_table_extra_non_root_rows_ignored(
        product: Product,
        customer: Option<Customer>,
    ) {
        // `customer` has more rows than the root; positions past the last root
        // row are never referenced. Every root row still pairs by position.
        let customer = customer.expect("customer present at every root position");
        assert_eq!(product.in_stock, customer.vip, "mispaired {}", product.name);
    }

    #[test_data_file(path = "tests/samples/valid_users.sql")]
    #[test]
    fn test_sql_join_columns_are_stripped(user: StrictUser) {
        // If `id` (on `user`) or `user_id` (on `address`) leaked into the
        // deserialized row, `deny_unknown_fields` would make the macro's
        // internal `serde_json::from_value(...).unwrap()` panic before this
        // body ever runs.
        let _ = user;
    }
}
