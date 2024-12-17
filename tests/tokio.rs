use test_data_file::test_data_file;

#[test_data_file(kind = "json", path = "tests/samples/check_array")]
#[tokio::test]
async fn test_check_array(value: Vec<String>) {
    assert!(!value.is_empty());
}
