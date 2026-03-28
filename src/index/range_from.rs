use core::marker::PhantomData;

use serde_core::Deserialize;
use serde_core::Deserializer;
use serde_core::Serializer;

use crate::DeserializePath;
use crate::Path;
use crate::Sequence;
use crate::SerializePath;
use crate::index::RangeEndUnbounded;
use crate::index::RangeStartInclusive;
use crate::index::RangeVisitor;

/// Access all elements in a sequence starting from the given index.
/// Represents the `[1..]` in `Cursor!(package[1..].dependencies[0])`.
pub struct RangeFrom<const START: usize>;

impl<'de, const START: usize, P, C> DeserializePath<'de, C> for Path<RangeFrom<START>, P>
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
            RangeEndUnbounded,
            P,
            C,
        > {
            _marker: PhantomData,
        })
    }
}

impl<const START: usize, P, T, C> SerializePath<C> for Path<RangeFrom<START>, P>
where
    for<'a> &'a C: IntoIterator<Item = &'a T, IntoIter: ExactSizeIterator>,
    P: SerializePath<T>,
{
    fn serialize<S>(value: &C, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        super::serialize::<RangeStartInclusive<START>, RangeEndUnbounded, S, P, T, _>(
            value.into_iter(),
            serializer,
        )
    }
}
