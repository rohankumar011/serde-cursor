use crate::ConstPathSegment;
use crate::Nil;
use crate::PathSegment;
use core::fmt;
use std::marker::PhantomData;

use serde_core::de::DeserializeSeed;
use serde_core::de::MapAccess;
use serde_core::{
    Deserialize, Deserializer,
    de::{IgnoredAny, SeqAccess, Visitor},
};

use crate::Cursor;
use crate::{Cons, Sequence, Wildcard};

struct SequenceVisitor<P, T> {
    target_index: usize,
    _marker: PhantomData<(P, T)>,
}

impl<'de, P, T> Visitor<'de> for SequenceVisitor<P, T>
where
    P: DeserializePath<'de, T>,
    T: Deserialize<'de>,
{
    type Value = T;

    fn expecting(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(
            f,
            "a sequence containing at least {} elements",
            self.target_index + 1
        )
    }

    fn visit_seq<A>(self, mut seq: A) -> Result<Self::Value, A::Error>
    where
        A: SeqAccess<'de>,
    {
        // skip elements before the target index
        for i in 0..self.target_index {
            if seq.next_element::<IgnoredAny>()?.is_none() {
                return Err(serde_core::de::Error::custom(format!(
                    "index {} out of bounds (length {})",
                    self.target_index, i
                )));
            }
        }

        // found the index, recurse to the next part of the path
        let result = seq
            .next_element_seed(PathSeed::<P, T>(PhantomData))?
            .ok_or_else(|| {
                serde_core::de::Error::custom(format!("index {} out of bounds", self.target_index))
            })?;

        // consume the rest of the sequence
        // some deserializers (like serde_json) will error if the sequence isn't exhausted
        while seq.next_element::<IgnoredAny>()?.is_some() {}

        Ok(result)
    }
}

struct FieldVisitor<P, D> {
    target: &'static str,
    _marker: PhantomData<(P, D)>,
}

impl<'de, P, T> Visitor<'de> for FieldVisitor<P, T>
where
    P: DeserializePath<'de, T>,
    T: Deserialize<'de>,
{
    type Value = T;

    fn expecting(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "map with field '{}'", self.target)
    }

    fn visit_map<A>(self, mut map: A) -> Result<Self::Value, A::Error>
    where
        A: MapAccess<'de>,
    {
        let mut result = None;

        while let Some(key) = map.next_key::<String>()? {
            if key == self.target && result.is_none() {
                result = Some(map.next_value_seed(PathSeed::<P, T>(PhantomData))?);
            } else {
                map.next_value::<IgnoredAny>()?;
            }
        }

        match result {
            Some(val) => Ok(val),
            // This allows Option<T> to become None instead of failing.
            None => T::deserialize(serde_core::de::value::UnitDeserializer::<A::Error>::new())
                .map_err(|_| {
                    <A::Error as serde_core::de::Error>::custom(format!(
                        "missing field '{}'",
                        self.target
                    ))
                }),
        }
    }
}

struct PathSeed<P, T>(PhantomData<(P, T)>);

impl<'de, P, T> DeserializeSeed<'de> for PathSeed<P, T>
where
    P: DeserializePath<'de, T>,
    T: Deserialize<'de>,
{
    type Value = T;

    fn deserialize<D>(self, deserializer: D) -> Result<Self::Value, D::Error>
    where
        D: Deserializer<'de>,
    {
        P::deserialize(deserializer)
    }
}

/// Deserializes the path to the type returned by [`Cursor!`]
///
/// For more information, see the [crate-level](crate) documentation.
#[diagnostic::on_unimplemented(
    message = "`{T}` doesn't implement `serde_cursor::Sequence`",
    note = "try: `Vec<{T}>`"
)]
pub trait DeserializePath<'de, T> {
    fn deserialize<D>(deserializer: D) -> Result<T, D::Error>
    where
        D: Deserializer<'de>;
}

// base case: we are at the target property
impl<'de, T: Deserialize<'de>> DeserializePath<'de, T> for Nil {
    fn deserialize<D>(deserializer: D) -> Result<T, D::Error>
    where
        D: Deserializer<'de>,
    {
        T::deserialize(deserializer)
    }
}

// step case: we are still digging into the object
impl<'de, S, P, T> DeserializePath<'de, T> for Cons<S, P>
where
    S: ConstPathSegment,
    P: DeserializePath<'de, T>,
    T: Deserialize<'de>,
{
    fn deserialize<D>(deserializer: D) -> Result<T, D::Error>
    where
        D: Deserializer<'de>,
    {
        use serde_core::de::Error;
        let segment = S::VALUE;

        let result = match segment {
            PathSegment::Field(name) => deserializer.deserialize_map(FieldVisitor::<P, T> {
                target: name,
                _marker: PhantomData,
            }),
            PathSegment::Index(index) => deserializer.deserialize_seq(SequenceVisitor::<P, T> {
                target_index: index,
                _marker: PhantomData,
            }),
        };

        // wrap the error with the current path segment
        result.map_err(|e| {
            let err_str = e.to_string();
            let path_str = match segment {
                PathSegment::Field(name) => format!(".{}", name),
                PathSegment::Index(i) => format!("[{}]", i),
            };

            // if the error is a nested path (starts with . or [), just join them
            if err_str.starts_with('.') || err_str.starts_with('[') {
                D::Error::custom(format!("{}{}", path_str, err_str))
            }
            // if this is the "top" level of the path, ensure it starts with a dot
            else {
                let prefix = if path_str.starts_with('[') {
                    format!(".{}", path_str)
                } else {
                    path_str
                };
                D::Error::custom(format!("{}: {}", prefix, err_str))
            }
        })
    }
}

struct WildcardVisitor<P, C> {
    _marker: PhantomData<(P, C)>,
}

impl<'de, P, C> DeserializePath<'de, C> for Cons<Wildcard, P>
where
    C: Sequence,
    P: DeserializePath<'de, C::Item>,
    C::Item: Deserialize<'de>,
{
    fn deserialize<D>(deserializer: D) -> Result<C, D::Error>
    where
        D: Deserializer<'de>,
    {
        use serde_core::de::Error;
        deserializer
            .deserialize_seq(WildcardVisitor::<P, C> {
                _marker: PhantomData,
            })
            .map_err(|e| D::Error::custom(format!("{}", e)))
    }
}

impl<'de, P, C> Visitor<'de> for WildcardVisitor<P, C>
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
        let mut items = C::default();
        let mut index = 0;
        while let Some(item) = seq
            .next_element_seed(PathSeed::<P, C::Item>(PhantomData))
            .map_err(|e| serde_core::de::Error::custom(format!("[{}]{}", index, e)))?
        {
            items.push(item);
            index += 1;
        }
        Ok(items)
    }
}

impl<'de, T, P> Deserialize<'de> for Cursor<T, P>
where
    T: Deserialize<'de>,
    P: DeserializePath<'de, T>,
{
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        let value = P::deserialize(deserializer)?;
        Ok(Self(value, PhantomData))
    }
}

#[cfg(feature = "serde_with")]
impl<'de, T, P> serde_with::DeserializeAs<'de, T> for Cursor<T, P>
where
    T: Deserialize<'de>,
    P: DeserializePath<'de, T>,
{
    fn deserialize_as<D>(deserializer: D) -> Result<T, D::Error>
    where
        D: Deserializer<'de>,
    {
        P::deserialize(deserializer)
    }
}
