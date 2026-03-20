use serde_cursor::Cursor;

#[test]
fn test_field_name() {
    let document = serde_json::json!({
        "field name with spaces": 42,
    })
    .to_string();

    let data: i64 = serde_json::from_str::<Cursor!("field name with spaces")>(&document)
        .unwrap()
        .0;

    assert_eq!(data, 42);
}
