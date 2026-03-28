use core::fmt;
use std::marker::PhantomData;

use serde_core::de::IgnoredAny;
use serde_core::de::SeqAccess;
use serde_core::de::Visitor;
use serde_core::ser::SerializeSeq as _;
use serde_core::Deserialize;
use serde_core::Serializer;

use crate::de::PathSeed;
use crate::ser::DelegateSerializeToSerealizePath;
use crate::DeserializePath;
use crate::Sequence;
use crate::SerializePath;

pub(crate) mod range;
pub(crate) mod range_from;
pub(crate) mod range_full;
pub(crate) mod range_inclusive;
pub(crate) mod range_to;
pub(crate) mod range_to_inclusive;

#[derive(Debug)]
enum RangeStart {
    Inclusive(usize),
    Unbounded,
}

trait ConstRangeStart {
    const VALUE: RangeStart;
}

struct RangeStartInclusive<const N: usize>;

impl<const N: usize> ConstRangeStart for RangeStartInclusive<N> {
    const VALUE: RangeStart = RangeStart::Inclusive(N);
}

struct RangeStartUnbounded;

impl ConstRangeStart for RangeStartUnbounded {
    const VALUE: RangeStart = RangeStart::Unbounded;
}

#[derive(Debug)]
enum RangeEnd {
    Inclusive(usize),
    Exclusive(usize),
    Unbounded,
}

trait ConstRangeEnd {
    const VALUE: RangeEnd;
}

struct RangeEndInclusive<const N: usize>;

impl<const N: usize> ConstRangeEnd for RangeEndInclusive<N> {
    const VALUE: RangeEnd = RangeEnd::Inclusive(N);
}

struct RangeEndExclusive<const N: usize>;

impl<const N: usize> ConstRangeEnd for RangeEndExclusive<N> {
    const VALUE: RangeEnd = RangeEnd::Exclusive(N);
}

struct RangeEndUnbounded;

impl ConstRangeEnd for RangeEndUnbounded {
    const VALUE: RangeEnd = RangeEnd::Unbounded;
}

/// Visitor for range-based path segments.
pub(super) struct RangeVisitor<S, E, P, C> {
    pub(super) _marker: PhantomData<(S, E, P, C)>,
}

impl<'de, Start: ConstRangeStart, End: ConstRangeEnd, P, C> Visitor<'de>
    for RangeVisitor<Start, End, P, C>
where
    C: Sequence,
    P: DeserializePath<'de, C::Item>,
    C::Item: Deserialize<'de>,
{
    type Value = C;

    fn expecting(&self, f: &mut fmt::Formatter) -> fmt::Result {
        f.write_fmt(format_args!("a sequence with `"))?;

        match Start::VALUE {
            RangeStart::Inclusive(start) => {
                f.write_fmt(format_args!("{start}"))?;
            }
            RangeStart::Unbounded => {}
        }

        f.write_fmt(format_args!(".."))?;

        match End::VALUE {
            RangeEnd::Exclusive(end) => {
                f.write_fmt(format_args!("{end}`"))?;
            }
            RangeEnd::Inclusive(end) => {
                f.write_fmt(format_args!("={end}`"))?;
            }
            RangeEnd::Unbounded => {}
        }

        f.write_fmt(format_args!("` elements"))?;

        Ok(())
    }

    fn visit_seq<A>(self, mut seq: A) -> Result<Self::Value, A::Error>
    where
        A: SeqAccess<'de>,
    {
        let start = match Start::VALUE {
            RangeStart::Inclusive(start) => start,
            RangeStart::Unbounded => 0,
        };

        let end = match End::VALUE {
            RangeEnd::Inclusive(end) => Some(end + 1),
            RangeEnd::Exclusive(end) => Some(end),
            RangeEnd::Unbounded => None,
        };

        let mut index = 0;

        // Say we have a range like 2..4
        // 0 1 2 3 4

        // First 2 items are skipped
        //
        // 0 1 2 3 4
        // ^ ^
        // ^ START ^ END
        while index < start {
            let element = seq.next_element::<IgnoredAny>();
            index += 1;

            match element {
                Ok(Some(IgnoredAny)) => {
                    // continue
                }
                Ok(None) => {
                    return Err(serde_core::de::Error::custom(format_args!(
                        "[{index}]: expected at least {} elements, but found {}",
                        start + 1,
                        index + 1
                    )))
                }
                Err(err) => {
                    return Err(serde_core::de::Error::custom(format_args!(
                        "[{index}]{err}"
                    )))
                }
            }
        }

        let mut elements = if let Some(end) = end {
            <C as Sequence>::with_capacity(end.saturating_sub(start))
        } else if let Some(capacity) = seq.size_hint() {
            <C as Sequence>::with_capacity(capacity.saturating_sub(start))
        } else {
            <C as Default>::default()
        };

        let Some(end) = end else {
            // Deserialize every remaining item
            while let Some(element) = seq
                .next_element_seed(PathSeed::<P, C::Item>(PhantomData))
                .map_err(|e| serde_core::de::Error::custom(format!("[{}]{}", index, e)))?
            {
                elements.push(element);
                index += 1;
            }

            return Ok(elements);
        };

        // Remaining items are collected
        //
        // 0 1 2 3 4
        //     ^ ^
        // ^ START ^ END
        while index < end {
            let element = seq.next_element_seed(PathSeed::<P, C::Item>(PhantomData));
            index += 1;

            match element {
                Ok(Some(element)) => {
                    elements.push(element);
                }
                Ok(None) => {
                    return Err(serde_core::de::Error::custom(format_args!(
                        "[{index}]: expected at least {} elements, but found {}",
                        start + 1,
                        index + 1
                    )))
                }
                Err(err) => {
                    return Err(serde_core::de::Error::custom(format_args!(
                        "[{index}]{err}"
                    )))
                }
            }
        }

        // Exhaust the sequence. Some formats require the visitor to finish the entire sequence.
        while seq.next_element::<IgnoredAny>()?.is_some() {}

        Ok(elements)
    }
}

fn serialize<'a, Start, End, S, P, T, I>(elements: I, serializer: S) -> Result<S::Ok, S::Error>
where
    Start: ConstRangeStart,
    End: ConstRangeEnd,
    S: Serializer,
    P: SerializePath<T>,
    I: Iterator<Item = &'a T> + ExactSizeIterator,
    T: 'a,
{
    let start = match dbg!(Start::VALUE) {
        RangeStart::Inclusive(start) => start,
        RangeStart::Unbounded => 0,
    };

    let end = match dbg!(End::VALUE) {
        RangeEnd::Inclusive(end) => end + 1,
        RangeEnd::Exclusive(end) => end,
        RangeEnd::Unbounded => elements.len() + 1,
    };

    // for `start > end` ranges, this will be `0`, an empty range
    let len = end.saturating_sub(start);

    let mut seq = serializer.serialize_seq(Some(len))?;

    if elements.len() < len {
        return Err(serde_core::ser::Error::custom(format_args!(
            "expected at least `{len}` elements but found `{}`",
            elements.len()
        )));
    }

    // pad prefix
    //
    // [null, null, 2, 3, 4]
    //  ^^^^^^^^^^
    for _ in 0..start {
        seq.serialize_element(&())?;
    }

    // the actual elements in range `2..5`
    //
    // [null, null, 2, 3, 4]
    //              ^^^^^^^
    for element in elements.into_iter().take(len) {
        seq.serialize_element(&DelegateSerializeToSerealizePath::<P, T>(
            element,
            std::marker::PhantomData,
        ))?;
    }

    seq.end()
}
