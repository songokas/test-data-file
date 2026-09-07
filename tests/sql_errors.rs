use test_data_file::test_data_file;

#[allow(dead_code)]
#[derive(Debug, serde::Deserialize)]
struct Product {
    name: String,
    price: u32,
    in_stock: bool,
}

#[allow(dead_code)]
#[derive(Debug, serde::Deserialize)]
struct Customer {
    name: String,
    country: String,
    vip: bool,
}

#[test_data_file(path = "tests/samples/empty.sql")]
#[test]
#[should_panic(expected = "Empty test data provided")]
fn test_empty_sql_panics() {}

#[test_data_file(path = "tests/samples/ambiguous_roots_partial.sql")]
#[test]
#[should_panic(expected = "missing field `customer`")]
fn test_multi_table_sql_missing_position_needs_option(product: Product, customer: Customer) {
    // A root row has no `customer` at its position; without `Option`, the
    // macro's internal deserialization fails.
    let _ = (product, customer);
}

#[test_data_file(path = "tests/samples/cyclic_root.sql")]
#[test]
#[should_panic(expected = "no root table found")]
fn test_cyclic_root_sql_panics() {}
