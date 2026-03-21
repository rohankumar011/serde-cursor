use std::marker::PhantomData;

use serde_core::ser::Serialize;
use serde_core::ser::SerializeMap;
use serde_core::ser::SerializeSeq;
use serde_core::ser::Serializer;

use crate::Cons;
use crate::ConstPathSegment;
use crate::Cursor;
use crate::Nil;
use crate::PathSegment;
use crate::Wildcard;

impl<T, P> serde_core::Serialize for Cursor<T, P>
where
    P: SerializeCursor<T>,
{
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: serde_core::Serializer,
    {
        <P as SerializeCursor<T>>::serialize(&self.0, serializer)
    }
}

/// Serializes the path to the type returned by [`Cursor!`].
///
/// For more information, see the [crate-level](crate) documentation.
pub trait SerializeCursor<T> {
    fn serialize<S>(value: &T, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer;
}

// base case: no more path segments to wrap, just serialize the value
//
// Cursor!(package.name: String)
//                     ^ we are here
impl<T: Serialize> SerializeCursor<T> for Nil {
    fn serialize<S>(value: &T, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        // The recursion ends here.
        <T as Serialize>::serialize(value, serializer)
    }
}

// step case: wrap the inner value in a map ([`Field`]) or a sequence ([`Index`])
//
// Cursor!(package.version: String)
//         ^^^^^^^ we are here
//
// This produces: { "package": <rest of path> }
impl<S, P, T> SerializeCursor<T> for Cons<S, P>
where
    S: ConstPathSegment,
    P: SerializeCursor<T>,
{
    fn serialize<Ser>(value: &T, serializer: Ser) -> Result<Ser::Ok, Ser::Error>
    where
        Ser: Serializer,
    {
        match <S as ConstPathSegment>::VALUE {
            // The current segment is a named field.
            //
            // { "field": ... }
            //   ^^^^^^^ `name`
            PathSegment::Field(name) => {
                let mut map = serializer.serialize_map(Some(1))?;

                // We wrap the value in ToCursorWrapper to continue the path
                // recursion for the entry's value.
                //
                // { "field": "value", ... }
                //    ^^^^^^^^^^^^^^^ serialize all of this
                map.serialize_entry(name, &ToCursorWrapper::<P, T>(value, PhantomData))?;

                map.end()
            }

            // The current segment is a specific index.
            //
            // [null, null, null, null, ..., ...]
            //                          ^^^ index
            PathSegment::Index(index) => {
                let mut seq = serializer.serialize_seq(Some(index + 1))?;

                // If we have e.g. Cursor!(4), we need to fill indices 0-3
                // with something so that our target ends up at index 4.
                //
                // [ null, null, null, null, <VALUE> ]
                //   0     1     2     3     4
                for _ in 0..index {
                    seq.serialize_element(&())?; // null
                }

                // serialize the actual element at the target index.
                seq.serialize_element(&ToCursorWrapper::<P, T>(value, PhantomData))?;

                seq.end()
            }
        }
    }
}

// now for the Wildcard step: Cursor!(packages.*.name: Vec<String>)
//                                             ^ we are here
//
// If the value is ["a", "b"], this produces:
//
// [
//   { "name": "a" },
//   { "name": "b" }
// ]
impl<P, T, C> SerializeCursor<C> for Cons<Wildcard, P>
where
    for<'a> &'a C: IntoIterator<Item = &'a T>,
    P: SerializeCursor<T>,
{
    fn serialize<S>(value: &C, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        let mut seq = serializer.serialize_seq(None)?;

        for item in value {
            // every item in the collection is wrapped with the remaining path P.
            seq.serialize_element(&ToCursorWrapper::<P, T>(item, PhantomData))?;
        }

        seq.end()
    }
}

// helper to bridge the recursion
struct ToCursorWrapper<'a, P, T>(&'a T, PhantomData<P>);

impl<'a, P, T> Serialize for ToCursorWrapper<'a, P, T>
where
    P: SerializeCursor<T>,
{
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        <P as SerializeCursor<T>>::serialize(self.0, serializer)
    }
}

#[cfg(feature = "serde_with")]
impl<T, P> serde_with::SerializeAs<T> for Cursor<T, P>
where
    T: Serialize,
    P: SerializeCursor<T>,
{
    fn serialize_as<S>(source: &T, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: serde_core::Serializer,
    {
        <P as SerializeCursor<T>>::serialize(source, serializer)
    }
}
