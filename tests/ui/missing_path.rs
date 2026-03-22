use serde_cursor::Path;

// path must exist
type X<T> = Path!(+ T);

fn main() {}
