use std::marker::PhantomData;

use crate::const_str::ConstStr;

/// A single path segment of the type-level path constructed by [`Cursor!`](macro@crate::Cursor).
///
/// See the [crate-level](crate) documentation for more info.
pub enum PathSegment {
    /// Path segment representing the field of a map.
    ///
    /// ```txt
    /// Cursor!(field.0)
    ///         ^^^^^
    /// ```
    Field(&'static str),
    /// Path segment representing an index into a sequence.
    ///
    /// ```txt
    /// Cursor!(field.0)
    ///               ^
    /// ```
    Index(usize),
}

/// Equivalent to <code>const PATH_SEGMENT: [PathSegment]</code>, except it works on stable Rust.
pub trait ConstPathSegment {
    const VALUE: PathSegment;
}

/// Path segment representing the field of a map.
///
/// ```txt
/// Cursor!(field.0)
///         ^^^^^
/// ```
pub struct FieldName<S: ConstStr>(PhantomData<S>);

/// Path segment representing an index into a sequence.
///
/// ```txt
/// Cursor!(field.0)
///               ^
/// ```
pub struct Index<const N: usize>;

impl<S: ConstStr> ConstPathSegment for FieldName<S> {
    const VALUE: PathSegment = PathSegment::Field(S::VALUE);
}

impl<const N: usize> ConstPathSegment for Index<N> {
    const VALUE: PathSegment = PathSegment::Index(N);
}
