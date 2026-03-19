#![feature(adt_const_params)]
#![feature(unsized_const_params)]
#![allow(incomplete_features)]

use std::{marker::PhantomData, mem};

use serde::{Deserialize, de::DeserializeSeed};

use crate::field::FieldVisitor;

mod const_str;
mod field;

trait Path {}

impl<T> Path for T {}

struct Cursor<D, P> {
    pub value: D,
    _path: PhantomData<P>,
}

impl<'de, D: Deserialize<'de>, P: Path> Deserialize<'de> for Cursor<D, P> {
    fn deserialize<De>(deserializer: De) -> Result<Self, De::Error>
    where
        De: serde_core::Deserializer<'de>,
    {
        let mut value: Option<D> = None;

        <DeserializePathSegment<D> as DeserializeSeed>::deserialize(
            DeserializePathSegment(&mut value),
            deserializer,
        )?;

        let value = value.unwrap();

        Ok(Self {
            value,
            _path: PhantomData,
        })
    }
}

// impl Split for () {
//     const HEAD: Option<PathSegment> = None;

//     type Tail = ();
// }

// impl<const P1: PathSegment> Split for (P1,) {
//     const HEAD: Option<PathSegment> = Some(P1);

//     type Tail = ();
// }

struct seg<const S: PathSegment>;

impl<const S: PathSegment> Split for seg<S> {
    type Type = [u8; size_of::<PathSegment>()];

    // SAFETY: `PathSegment` is #[repr(C)]
    const BYTES: Self::Type = unsafe { mem::transmute(S) };
}

trait Split {
    type Type;
    const BYTES: Self::Type;
}

#[repr(C)]
struct Concat2<A, B>(A, B);

impl<A, B> Split for (A, B)
where
    A: Split,
    B: Split,
{
    type Type = Concat2<A::Type, B::Type>;
    const BYTES: Self::Type = Concat2(A::BYTES, B::BYTES);
}

#[repr(C)]
struct Concat3<A, B, C>(A, B, C);

impl<A, B, C> Split for (A, B, C)
where
    A: Split,
    B: Split,
    C: Split,
{
    type Type = Concat3<A::Type, B::Type, C::Type>;
    const BYTES: Self::Type = Concat3(A::BYTES, B::BYTES, C::BYTES);
}

// Cursor<bool, ("a",)>
// Cursor<bool, ("a", "b")>
// Cursor<bool, ("a", "b",)>
// Cursor<bool, ("a", "b", "c")>
// Cursor<bool, ("a", ("b", "c"))>

/// Deserializes a single path segment.
struct DeserializePathSegment<'query, D>(&'query mut Option<D>);

impl<'de, 'query, D: Deserialize<'de>> DeserializeSeed<'de> for DeserializePathSegment<'query, D> {
    type Value = ();

    fn deserialize<De>(self, deserializer: De) -> Result<Self::Value, De::Error>
    where
        De: serde::Deserializer<'de>,
    {
        match current_field() {
            Some(PathSegment::Field(_)) => {
                deserializer.deserialize_map(FieldVisitor(self.0, PhantomData))?;
                Ok(())
            }
            None => {
                *self.0 = Some(<D as Deserialize>::deserialize(deserializer)?);
                Ok(())
            }
            _ => todo!(),
        }
    }
}

const FIELDS: [&str; 3] = ["a", "b", "c"];
static mut FIELD_COUNT: usize = 0;

fn current_field() -> Option<PathSegment> {
    Some(PathSegment::Field(FIELDS.get(unsafe { FIELD_COUNT })?))
}

fn next_field() {
    unsafe { FIELD_COUNT += 1 }
}

#[repr(C)]
#[derive(std::marker::ConstParamTy, PartialEq, Eq)]
enum PathSegment {
    Field(&'static str),
    ArrayExact(usize),
    Array,
}

const _: () = {
    assert!(size_of::<PathSegment>().is_multiple_of(align_of::<PathSegment>()));
};

#[macro_export]
macro_rules! Cursor {
    ($($field:ident).+) => {
        $crate::Cursor!(@ $($field).+, _)
    };
    ($($field:ident).+ , $ty:ty) => {
        $crate::Cursor!(@ $($field).+, $ty)
    };
    (@ $($field:ident).+ , $ty:ty) => {
        $crate::Cursor<$ty, ()>
    }
}

pub use core::result::Result;

#[macro_export]
macro_rules! cursored {
    ($f:expr, $($tt:tt)*) => {{
        let o: $crate::Result<$crate::Cursor!($($tt)*), _> = $f;
        $crate::Result::map(o, |it| it.value)
    }};
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_name() {
        let value = serde_json::json!({
            "a": {
                "b": {
                    "c": true
                }
            }
        });

        let x = cursored!(serde_json::from_value(value), a.b.c, bool);

        // assert!(value);
    }
}
