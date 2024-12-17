use insta::assert_yaml_snapshot;
use test_data_file::test_data_file;

fn split_words(s: &str) -> Vec<&str> {
    s.split_whitespace().collect()
}

#[test_data_file(kind = "csv", path = "tests/samples/snapshot")]
#[test]
fn test_split_words(name: String, words: String) {
    assert_yaml_snapshot!(name, split_words(&words));
}
