use serde_cursor::Cursor;
use serde_json::json;

#[test]
fn lol() {
    let value = json!({
        "a": {
            "hello world": {
                "c": [false, false, true, false]
            }
        }
    });

    type X = ::serde_cursor::Cursor<
        bool,
        ::serde_cursor::Cons<
            ::serde_cursor::FieldName<(::serde_cursor::StrLen<1>, ::serde_cursor::C1<'a'>)>,
            ::serde_cursor::Cons<
                ::serde_cursor::FieldName<(::serde_cursor::StrLen<0>, ())>,
                ::serde_cursor::Cons<
                    ::serde_cursor::FieldName<(::serde_cursor::StrLen<1>, ::serde_cursor::C1<'c'>)>,
                    ::serde_cursor::Cons<::serde_cursor::Index<2>, ::serde_cursor::Nil>,
                >,
            >,
        >,
    >;

    let x = serde_json::from_value::<Cursor!(a."".c.2: bool)>(value)
        .unwrap()
        .value;
}
