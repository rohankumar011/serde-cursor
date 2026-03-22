use serde_cursor::Path;

// T must be a type parameter that's available in the current scope
type X = Path!(foo.bar + T);

fn main() {}
