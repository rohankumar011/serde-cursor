use serde_cursor::Cursor;
use serde_json::from_value;
use serde_json::json;

#[test]
fn test_error_missing_field() {
    let data = json!({ "author": { "id": 1 } });
    let result = from_value::<Cursor!(author.name: String)>(data);
    let err = result.unwrap_err().to_string();
    // Fixed spacing: ".author.name: ..."
    assert_eq!(err, ".author.name: missing field 'name'");
}

#[test]
fn test_error_type_mismatch() {
    let data = json!({ "author": { "id": "not-a-number" } });
    let result = from_value::<Cursor!(author.id: i32)>(data);
    let err = result.unwrap_err().to_string();
    assert!(err.starts_with(".author.id: invalid type: string"));
}

#[test]
fn test_error_wildcard_index_tracking() {
    let data = json!([
        { "name": "alice" },
        { "typo": "bob" }
    ]);

    let result = from_value::<Cursor!(*.name: Vec<String>)>(data);
    let err = result.unwrap_err().to_string();

    assert_eq!(err, "[1].name: missing field 'name'");
}

#[test]
fn test_error_deep_wildcard_mismatch() {
    let data = json!({
        "org": {
            "users": [
                { "id": 1 },
                { "id": "invalid" }
            ]
        }
    });
    let result = from_value::<Cursor!(org.users.*.id: Vec<i32>)>(data);
    let err = result.unwrap_err().to_string();

    assert_eq!(
        err,
        ".org.users[1].id: invalid type: string \"invalid\", expected i32"
    );
}
