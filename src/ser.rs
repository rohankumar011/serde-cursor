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
        P::serialize(&self.0, serializer)
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

// base case: no more path, just serialize the value
impl<T: Serialize> SerializeCursor<T> for Nil {
    fn serialize<S>(value: &T, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        value.serialize(serializer)
    }
}

// step case: wrap in a map (field) or seq (Index)
impl<S, P, T> SerializeCursor<T> for Cons<S, P>
where
    S: ConstPathSegment,
    P: SerializeCursor<T>,
{
    fn serialize<Ser>(value: &T, serializer: Ser) -> Result<Ser::Ok, Ser::Error>
    where
        Ser: Serializer,
    {
        match S::VALUE {
            PathSegment::Field(name) => {
                let mut map = serializer.serialize_map(Some(1))?;

                map.serialize_entry(name, &ToCursorWrapper::<P, T>(value, PhantomData))?;

                map.end()
            }
            PathSegment::Index(index) => {
                let mut seq = serializer.serialize_seq(Some(index + 1))?;

                // so if we have e.g. Query!(4), which accesses the 4th element,
                // when serializing we will create 3 "null"s and serialize the 4th element
                for _ in 0..index {
                    seq.serialize_element(&())?; // null
                }

                // the actual element that we are serializing
                seq.serialize_element(&ToCursorWrapper::<P, T>(value, PhantomData))?;

                seq.end()
            }
        }
    }
}

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
        P::serialize(self.0, serializer)
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
        P::serialize(source, serializer)
    }
}
