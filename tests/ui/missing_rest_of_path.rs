use serde_cursor::Path;

// expected to see "+ T"
type X<T> = Path!(foo.bar);

fn main() {}
