use serde_cursor::Cursor;

#[test]
fn field_name() {
    use serde_query::{DeserializeQuery, Query};

    #[derive(DeserializeQuery)]
    struct Data {
        #[query(r#".["field name with spaces"]"#)]
        with_space: i64,
    }

    // serde_json::from_str< Cursor!("field name with spaces") >(&document);

    let document = serde_json::json!({
        "field name with spaces": 42,
    })
    .to_string();

    let data: Data = serde_json::from_str::<Query<Data>>(&document)
        .unwrap()
        .into();

    assert_eq!(data.with_space, 42);
}
