use core::marker::PhantomData;

use serde_core::Deserialize;
use serde_core::Deserializer;
use serde_core::Serializer;

use crate::index::RangeEndExclusive;
use crate::index::RangeStartUnbounded;
use crate::index::RangeVisitor;
use crate::DeserializePath;
use crate::Path;
use crate::Sequence;
use crate::SerializePath;

/// Access all elements in a sequence until the given index.
/// Represents the `[..7]` in `Cursor!(package[..=7].dependencies[0])`.
pub struct RangeTo<const END: usize>;

impl<'de, const END: usize, P, C> DeserializePath<'de, C> for Path<RangeTo<END>, P>
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
            RangeStartUnbounded,
            RangeEndExclusive<END>,
            P,
            C,
        > {
            _marker: PhantomData,
        })
    }
}

impl<const END: usize, P, T, C> SerializePath<C> for Path<RangeTo<END>, P>
where
    for<'a> &'a C: IntoIterator<Item = &'a T, IntoIter: ExactSizeIterator>,
    P: SerializePath<T>,
{
    fn serialize<S>(value: &C, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        super::serialize::<RangeStartUnbounded, RangeEndExclusive<END>, S, P, T, _>(
            value.into_iter(),
            serializer,
        )
    }
}
