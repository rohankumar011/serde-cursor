use serde_cursor::Cursor;

#[test]
fn test_index() {
    let document = serde_json::json!([
        "abc",
        42,
        {
            "key": "value",
        }
    ])
    .to_string();

    let data: i64 = serde_json::from_str::<Cursor!(1)>(&document).unwrap().0;

    assert_eq!(data, 42);
}
