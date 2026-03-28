use core::fmt;
use core::marker::PhantomData;

use serde_core::Deserialize;
use serde_core::Deserializer;
use serde_core::Serializer;
use serde_core::de::SeqAccess;
use serde_core::de::Visitor;
use serde_core::ser::SerializeSeq as _;

use crate::DeserializePath;
use crate::Path;
use crate::Sequence;
use crate::SerializePath;
use crate::de::PathSeed;
use crate::ser::DelegateSerializeToSerealizePath;

/// Access all elements of a sequence.
/// Represents the `[]` in `Cursor!(package[].dependencies[0])`.
pub struct RangeFull;

impl<'de, P, C> DeserializePath<'de, C> for Path<RangeFull, P>
where
    C: Sequence,
    P: DeserializePath<'de, C::Item>,
    C::Item: Deserialize<'de>,
{
    fn deserialize<D>(deserializer: D) -> Result<C, D::Error>
    where
        D: Deserializer<'de>,
    {
        deserializer.deserialize_seq(RangeFullVisitor::<P, C> {
            _marker: PhantomData,
        })
    }
}

/// Visitor for the index-all (`[]`) path segment.
///
/// ```txt
/// Cursor!(package[].name: Vec<String>)
///                ^^
/// ```
///
/// Collects multiple items into a sequence `C` implementing [`Sequence`],
///
/// In this case every `name` field corresponds to `String`, all of the
/// `name`s will be collected into a single `Vec<String>`.
struct RangeFullVisitor<P, C> {
    _marker: PhantomData<(P, C)>,
}

impl<'de, P, C> Visitor<'de> for RangeFullVisitor<P, C>
where
    C: Sequence,
    P: DeserializePath<'de, C::Item>,
    C::Item: Deserialize<'de>,
{
    type Value = C;

    fn expecting(&self, f: &mut fmt::Formatter) -> fmt::Result {
        f.write_str("a sequence")
    }

    fn visit_seq<A>(self, mut seq: A) -> Result<Self::Value, A::Error>
    where
        A: SeqAccess<'de>,
    {
        let mut items = if let Some(capacity) = seq.size_hint() {
            <C as Sequence>::with_capacity(capacity)
        } else {
            <C as Default>::default()
        };

        let mut index = 0;

        // Instead of skipping, we visit EVERY element in the sequence.
        //
        // For every element, we apply the rest of the path `P`.
        //
        // Cursor!(packages[].name: Vec<String>)
        //          [
        //            {"name": "serde"}, // Apply ".name" -> "serde"
        //            {"name": "anyhow"} // Apply ".name" -> "anyhow"
        //          ]
        while let Some(item) = seq
            .next_element_seed(PathSeed::<P, C::Item>(PhantomData))
            .map_err(|err| serde_core::de::Error::custom(format_args!("[{index}]{err}")))?
        {
            items.push(item);
            index += 1;
        }

        Ok(items)
    }
}

// now for the IndexAll step: Cursor!(packages[].name: Vec<String>)
//                                            ^^ we are here
//
// If the value is ["a", "b"], this produces:
//
// [
//   { "name": "a" },
//   { "name": "b" }
// ]
impl<P, T, C> SerializePath<C> for Path<RangeFull, P>
where
    for<'a> &'a C: IntoIterator<Item = &'a T>,
    P: SerializePath<T>,
{
    fn serialize<S>(value: &C, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        let mut seq = serializer.serialize_seq(None)?;

        for item in value {
            // every item in the collection is wrapped with the remaining path P.
            seq.serialize_element(&DelegateSerializeToSerealizePath::<P, T>(item, PhantomData))?;
        }

        seq.end()
    }
}
