use test_file_data::test_file_data;

fn test_me(max_value: u64) -> bool {
    max_value > 10
}

#[test_file_data(path = "tests/samples/test_me.list")]
#[test]
fn test_test_me_with_list(max_value: u64, supported: bool) {
    assert_eq!(test_me(max_value), supported, "failed for {max_value}");
}

// #[test_file_data(path = "tests/samples/test_me.yaml")]
// #[test]
// fn test_test_me_with_yaml(max_value: u64, supported: bool) {
//     assert_eq!(test_me(max_value), supported, "failed for {max_value}");
// }

#[test_file_data(path = "tests/samples/test_me.json")]
#[test]
fn test_test_me_with_json(max_value: u64, supported: bool) {
    assert_eq!(test_me(max_value), supported, "failed for {max_value}");
}

#[test_file_data(path = "tests/samples/test_me.ron")]
#[test]
fn test_test_me_with_ron(max_value: u64, supported: bool) {
    assert_eq!(test_me(max_value), supported, "failed for {max_value}");
}

#[test_file_data(path = "tests/samples/test_me.toml")]
#[test]
fn test_test_me_with_toml(max_value: u64, supported: bool) {
    assert_eq!(test_me(max_value), supported, "failed for {max_value}");
}
