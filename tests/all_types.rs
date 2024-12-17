use test_data_file::test_data_file;

fn is_name_above_max_size(name: Option<&str>, max_size: usize) -> bool {
    name.map(|n| n.len()) > Some(max_size)
}

#[test_data_file(path = "tests/samples/test_me.list")]
#[test]
fn test_test_me_with_list(name: String, max_size: usize, is_above: bool) {
    let name = if name == "None" {
        None
    } else {
        name.as_str().into()
    };
    assert_eq!(
        is_name_above_max_size(name, max_size),
        is_above,
        "failed for {max_size}"
    );
}

#[test_data_file(path = "tests/samples/test_me.yaml")]
#[test]
fn test_test_me_with_yaml(name: Option<String>, max_size: usize, is_above: bool) {
    assert_eq!(
        is_name_above_max_size(name.as_deref(), max_size),
        is_above,
        "failed for {max_size}"
    );
}

#[test_data_file(path = "tests/samples/test_me.json")]
#[test]
fn test_test_me_with_json(name: Option<String>, max_size: usize, is_above: bool) {
    assert_eq!(
        is_name_above_max_size(name.as_deref(), max_size),
        is_above,
        "failed for {max_size}"
    );
}

#[test_data_file(path = "tests/samples/test_me.ron")]
#[test]
fn test_test_me_with_ron(name: Option<String>, max_size: usize, is_above: bool) {
    assert_eq!(
        is_name_above_max_size(name.as_deref(), max_size),
        is_above,
        "failed for {max_size}"
    );
}

#[test_data_file(path = "tests/samples/test_me.toml")]
#[test]
fn test_test_me_with_toml(name: Option<String>, max_size: usize, is_above: bool) {
    assert_eq!(
        is_name_above_max_size(name.as_deref(), max_size),
        is_above,
        "failed for {max_size}"
    );
}

#[test_data_file(path = "tests/samples/test_me.csv")]
#[test]
fn test_test_me_with_csv(name: Option<String>, max_size: usize, is_above: bool) {
    assert_eq!(
        is_name_above_max_size(name.as_deref(), max_size),
        is_above,
        "failed for {max_size}"
    );
}
