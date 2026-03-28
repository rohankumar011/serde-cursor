use core::marker::PhantomData;

use serde_core::Deserialize;
use serde_core::Deserializer;
use serde_core::Serializer;

use crate::index::RangeEndInclusive;
use crate::index::RangeStartUnbounded;
use crate::index::RangeVisitor;
use crate::DeserializePath;
use crate::Path;
use crate::Sequence;
use crate::SerializePath;

/// Access all elements in a sequence until the given index, inclusive.
/// Represents the `[..=7]` in `Cursor!(package[..=7].dependencies[0])`.
pub struct RangeToInclusive<const END: usize>;

impl<'de, const END: usize, P, C> DeserializePath<'de, C> for Path<RangeToInclusive<END>, P>
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
            RangeEndInclusive<END>,
            P,
            C,
        > {
            _marker: PhantomData,
        })
    }
}

impl<const END: usize, P, T, C> SerializePath<C> for Path<RangeToInclusive<END>, P>
where
    for<'a> &'a C: IntoIterator<Item = &'a T, IntoIter: ExactSizeIterator>,
    P: SerializePath<T>,
{
    fn serialize<S>(value: &C, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        super::serialize::<RangeStartUnbounded, RangeEndInclusive<END>, S, P, T, _>(
            value.into_iter(),
            serializer,
        )
    }
}
