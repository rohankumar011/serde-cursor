use core::marker::PhantomData;

use serde_core::ser::Serialize;
use serde_core::ser::SerializeMap;
use serde_core::ser::SerializeSeq;
use serde_core::ser::Serializer;

use crate::ConstPathSegment;
use crate::Cursor;
use crate::Path;
use crate::PathEnd;
use crate::PathSegment;

impl<T, P> serde_core::Serialize for Cursor<T, P>
where
    P: SerializePath<T>,
{
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: serde_core::Serializer,
    {
        <P as SerializePath<T>>::serialize(&self.0, serializer)
    }
}

/// Serializes the path to the type returned by [`Cursor!`].
///
/// For more information, see the [crate-level](crate) documentation.
pub trait SerializePath<T> {
    fn serialize<S>(value: &T, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer;
}

// base case: no more path segments to wrap, just serialize the value
//
// Cursor!(package.name: String)
//                     ^ we are here
impl<T: Serialize> SerializePath<T> for PathEnd {
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
impl<S, P, T> SerializePath<T> for Path<S, P>
where
    S: ConstPathSegment,
    P: SerializePath<T>,
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
                map.serialize_entry(
                    name,
                    &DelegateSerializeToSerealizePath::<P, T>(value, PhantomData),
                )?;

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
                seq.serialize_element(&DelegateSerializeToSerealizePath::<P, T>(
                    value,
                    PhantomData,
                ))?;

                seq.end()
            }
        }
    }
}

pub(crate) struct DelegateSerializeToSerealizePath<'a, P, T>(
    pub(crate) &'a T,
    pub(crate) PhantomData<P>,
);

impl<'a, P, T> Serialize for DelegateSerializeToSerealizePath<'a, P, T>
where
    P: SerializePath<T>,
{
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        <P as SerializePath<T>>::serialize(self.0, serializer)
    }
}

#[cfg(feature = "serde_with")]
impl<T, P> serde_with::SerializeAs<T> for Cursor<T, P>
where
    T: Serialize,
    P: SerializePath<T>,
{
    fn serialize_as<S>(source: &T, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: serde_core::Serializer,
    {
        <P as SerializePath<T>>::serialize(source, serializer)
    }
}
