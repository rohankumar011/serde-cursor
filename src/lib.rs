use serde_core::de::{
    Deserialize, DeserializeSeed, Deserializer, IgnoredAny, MapAccess, SeqAccess, Visitor,
};
use std::fmt;
use std::marker::PhantomData;

/// The [`Cursor!`] macro
#[doc(inline)]
pub use serde_cursor_impl::Cursor;

#[doc(hidden)]
pub struct Cursor<T, P> {
    pub value: T,
    _path: PhantomData<P>,
}

impl<'de, T, P> Deserialize<'de> for Cursor<T, P>
where
    T: Deserialize<'de>,
    P: PathNavigator<'de, T>,
{
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        let value = P::navigate(deserializer)?;
        Ok(Self {
            value,
            _path: PhantomData,
        })
    }
}

#[doc(hidden)]
pub use const_str::C1;
#[doc(hidden)]
pub use const_str::C2;
#[doc(hidden)]
pub use const_str::C3;
#[doc(hidden)]
pub use const_str::C4;
#[doc(hidden)]
pub use const_str::ConstStr;
#[doc(hidden)]
pub use const_str::StrLen;

mod const_str;

#[doc(hidden)]
pub enum PathSegment {
    Field(&'static str),
    Index(usize),
}

#[doc(hidden)]
pub trait ConstPathSegment {
    const VALUE: PathSegment;
}

#[doc(hidden)]
pub struct Nil;
#[doc(hidden)]
pub struct Cons<S, T>(PhantomData<(S, T)>);

#[doc(hidden)]
pub trait Path {
    fn head() -> Option<PathSegment>;
    type Tail: Path;
}

#[doc(hidden)]
impl Path for Nil {
    fn head() -> Option<PathSegment> {
        None
    }
    type Tail = Nil;
}

impl<S: ConstPathSegment, P: Path> Path for Cons<S, P> {
    type Tail = P;
    fn head() -> Option<PathSegment> {
        Some(S::VALUE)
    }
}

#[doc(hidden)]
pub trait PathNavigator<'de, T>: Path {
    fn navigate<D>(deserializer: D) -> Result<T, D::Error>
    where
        D: Deserializer<'de>;
}

// base case: we are at the target property
impl<'de, T: Deserialize<'de>> PathNavigator<'de, T> for Nil {
    fn navigate<D>(deserializer: D) -> Result<T, D::Error>
    where
        D: Deserializer<'de>,
    {
        T::deserialize(deserializer)
    }
}

// step case: we are still digging into the object
impl<'de, S, P, T> PathNavigator<'de, T> for Cons<S, P>
where
    S: ConstPathSegment,
    P: PathNavigator<'de, T>,
    T: Deserialize<'de>,
{
    fn navigate<D>(deserializer: D) -> Result<T, D::Error>
    where
        D: Deserializer<'de>,
    {
        match S::VALUE {
            PathSegment::Field(name) => deserializer.deserialize_map(FieldVisitor::<P, T> {
                target: name,
                _marker: PhantomData,
            }),
            PathSegment::Index(index) => deserializer.deserialize_seq(SequenceVisitor::<P, T> {
                target_index: index,
                _marker: PhantomData,
            }),
        }
    }
}

struct SequenceVisitor<P, T> {
    target_index: usize,
    _marker: PhantomData<(P, T)>,
}

impl<'de, P, T> Visitor<'de> for SequenceVisitor<P, T>
where
    P: PathNavigator<'de, T>,
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
    P: PathNavigator<'de, T>,
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

        result.ok_or_else(|| {
            serde_core::de::Error::custom(format!("field '{}' not found", self.target))
        })
    }
}

struct PathSeed<P, D>(PhantomData<(P, D)>);

impl<'de, P, D> DeserializeSeed<'de> for PathSeed<P, D>
where
    P: PathNavigator<'de, D>,
    D: Deserialize<'de>,
{
    type Value = D;

    fn deserialize<De>(self, deserializer: De) -> Result<Self::Value, De::Error>
    where
        De: Deserializer<'de>,
    {
        P::navigate(deserializer)
    }
}

#[doc(hidden)]
pub struct FieldName<S: ConstStr>(PhantomData<S>);
#[doc(hidden)]
pub struct Index<const N: usize>;

#[doc(hidden)]
impl<S: ConstStr> ConstPathSegment for FieldName<S> {
    const VALUE: PathSegment = PathSegment::Field(S::VALUE);
}

#[doc(hidden)]
impl<const N: usize> ConstPathSegment for Index<N> {
    const VALUE: PathSegment = PathSegment::Index(N);
}
