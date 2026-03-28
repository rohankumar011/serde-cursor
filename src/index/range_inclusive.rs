use core::marker::PhantomData;

use serde_core::Deserialize;
use serde_core::Deserializer;
use serde_core::Serializer;

use super::RangeVisitor;
use crate::DeserializePath;
use crate::Path;
use crate::Sequence;
use crate::SerializePath;
use crate::index::RangeEndInclusive;
use crate::index::RangeStartInclusive;

/// Access an inclusive range of elements in a sequence.
/// Represents the `[4..=7]` in `Cursor!(package[4..=7].dependencies[0])`.
pub struct RangeInclusive<const START: usize, const END: usize>;

impl<'de, const START: usize, const END: usize, P, C> DeserializePath<'de, C>
    for Path<RangeInclusive<START, END>, P>
where
    C: Sequence,
    P: DeserializePath<'de, C::Item>,
    C::Item: Deserialize<'de>,
{
    fn deserialize<D>(deserializer: D) -> Result<C, D::Error>
    where
        D: Deserializer<'de>,
    {
        deserializer.deserialize_seq(RangeVisitor::<
            RangeStartInclusive<START>,
            RangeEndInclusive<END>,
            P,
            C,
        > {
            _marker: PhantomData,
        })
    }
}

impl<const START: usize, const END: usize, P, T, C> SerializePath<C>
    for Path<RangeInclusive<START, END>, P>
where
    for<'a> &'a C: IntoIterator<Item = &'a T, IntoIter: ExactSizeIterator>,
    P: SerializePath<T>,
{
    fn serialize<S>(value: &C, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        super::serialize::<RangeStartInclusive<START>, RangeEndInclusive<END>, S, P, T, _>(
            value.into_iter(),
            serializer,
        )
    }
}
