#![allow(clippy::identity_op)]

use serde_cursor::Cursor;
use serde_json::json;

/// `C` satisfies any type returned by `Cursor!` macro
#[track_caller]
fn assert_roundtrip<T, C>(json: serde_json::Value, expected_cursor: T)
where
    T: PartialEq + std::fmt::Debug,
    C: serde::de::DeserializeOwned + serde::Serialize + std::ops::Deref<Target = T>,
{
    // JSON -> Cursor
    let cursor: C = serde_json::from_value(json.clone()).unwrap();

    assert_eq!(*cursor, expected_cursor, "deserialization failed");

    // Cursor -> JSON
    let json = serde_json::to_value(&cursor).unwrap();

    // JSON -> Cursor
    let cursor_2: C = serde_json::from_value(json).unwrap();

    // The property that we care about is that we can repeatedly serialize
    // and deserialize JSON to cursor and then back.
    assert_eq!(*cursor, *cursor_2, "roundtrip failed");
}

#[test]
fn deep_field_path() {
    let json = json!({
        "a": { "b": { "c": 100 } }
    });

    assert_roundtrip::<i32, Cursor!(a.b.c)>(json, 100);
}

#[cfg(feature = "alloc")]
macro_rules! test_range {
    ($range:tt, $expected:tt) => {{
        let json = json!([[0], [1], [2], [3], [4]]);

        assert_roundtrip::<Vec<i32>, Cursor!($range[0])>(json, vec! $expected);

        let json = json!({ "foo": [{ "item": 0 }, { "item": 1 }, { "item": 2 }, { "item": 3 }, { "item": 4 }] });

        assert_roundtrip::<Vec<i32>, Cursor!(foo $range.item)>(json, vec! $expected);
    }};
}

const X: usize = 1;

#[test]
#[cfg(feature = "alloc")]
fn range() {
    test_range!([1..3], [1, 2]);
    test_range!([X..2 + X], [1, 2]);
}

#[cfg(feature = "alloc")]
#[test]
fn range_inclusive() {
    test_range!([1..=3], [1, 2, 3]);
    test_range!([X..=2 + X], [1, 2, 3]);
}

#[cfg(feature = "alloc")]
#[test]
fn range_to() {
    test_range!([..3], [0, 1, 2]);
    test_range!([..2 + X], [0, 1, 2]);
}

#[cfg(feature = "alloc")]
#[test]
fn range_to_inclusive() {
    test_range!([..=3], [0, 1, 2, 3]);
    test_range!([..=2 + X], [0, 1, 2, 3]);
}

#[test]
#[cfg(feature = "alloc")]
fn range_from() {
    test_range!([1..], [1, 2, 3, 4]);
    test_range!([0 + X..], [1, 2, 3, 4]);
}

#[test]
fn array_index_path() {
    // indices create null-padding for preceding elements during serialization
    let json = json!({
        "arr": [null, null, "found me"]
    });

    assert_roundtrip::<String, Cursor!(arr[2])>(json, "found me".to_string());
}

#[test]
fn crab() {
    let json = json!({
        "🦀": "crab"
    });

    assert_roundtrip::<String, Cursor!("🦀")>(json, "crab".to_string());
}

#[test]
fn with_dashes() {
    let json = json!({
        "--dev-dependencies": {
            "--yes": true
        }
    });

    assert_roundtrip::<bool, Cursor!(--dev-dependencies.--yes)>(json, true);
}

#[cfg(feature = "alloc")]
#[test]
fn index_all_collection() {
    let json = json!([
        { "val": 10 },
        { "val": 20 }
    ]);

    assert_roundtrip::<Vec<i32>, Cursor!([].val)>(json, vec![10, 20]);
}

#[test]
fn mixed_nested_path() {
    let json = json!({
        "users": [
            null,
            { "meta": { "id": "uuid-1" } }
        ]
    });

    assert_roundtrip::<String, Cursor!(users[1].meta.id)>(json, "uuid-1".to_string());
}

#[cfg(feature = "alloc")]
#[test]
fn nested_index_all() {
    let json = json!({
        "groups": [
            { "members": [{ "name": "A" }, { "name": "B" }] },
            { "members": [{ "name": "C" }] }
        ]
    });

    let expected = vec![vec!["A".to_string(), "B".to_string()], vec![
        "C".to_string(),
    ]];

    assert_roundtrip::<Vec<Vec<String>>, Cursor!(groups[].members[].name)>(json, expected);
}

#[cfg(feature = "alloc")]
#[test]
fn complex_index_all_objects() {
    let json = json!({
        "data": [
            { "info": { "code": 1 } },
            { "info": { "code": 2 } }
        ]
    });
    assert_roundtrip::<Vec<i32>, Cursor!(data[].info.code)>(json, vec![1, 2]);
}
