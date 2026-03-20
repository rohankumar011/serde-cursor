use proc_macro::{Ident, Literal, Punct, Spacing, Span, TokenStream, TokenTree};

mod const_str;

#[proc_macro]
#[allow(nonstandard_style)]
pub fn Cursor(input: TokenStream) -> TokenStream {
    let mut input = input.into_iter().peekable();

    // every path segment individually
    //
    // Cursor!(a.0.c: bool)
    //         ^ ^ ^
    let mut path_segments = Vec::new();

    // this is needed to know if we should expect a "." before
    // the first path segment, or not
    let mut started = false;

    // Parse path segments
    //
    // Cursor!(a.0.c: bool)
    //         ^^^^^
    while let Some(tt) = input.peek() {
        // the "." is not requires for the first path
        //
        // Cursor!(a.0.c: bool)
        //         ^
        if !started {
            parse_path_segment(&mut input, &mut path_segments);

            started = true;

            continue;
        }

        match tt {
            // Path ends at a colon
            //
            // Cursor!(a.b.c: bool)
            //              ^
            TokenTree::Punct(p) if p.as_char() == ':' => {
                input.next();

                break;
            }
            // A single path segment
            //
            // Cursor!(a.0.c: bool)
            //          ^^
            TokenTree::Punct(p) if p.as_char() == '.' => {
                input.next();

                parse_path_segment(&mut input, &mut path_segments);
            }
            _ => break,
        }
    }

    // These tokens make up the actual Type.
    //
    // Cursor!(a.0.c: HashMap<&str, &str>)
    //                ^^^^^^^^^^^^^^^^^^^
    let type_tokens: TokenStream = if input.peek().is_none() {
        TokenStream::from_iter([punct('_')])
    } else {
        input.collect()
    };

    // Type path: `Cons<_, Cons<_, Nil>>`
    let path = path_segments
        .into_iter()
        .rev()
        .fold(path([ident("Nil")]), |p, segment| {
            TokenStream::from_iter(
                path([ident("Cons")])
                    .into_iter()
                    .chain([punct('<')])
                    .chain(TokenStream::from_iter(
                        segment.to_tokens().into_iter().chain([punct(',')]).chain(p),
                    ))
                    .chain([punct('>')]),
            )
        });

    TokenStream::from_iter(
        [
            punct(':'),
            punct(':'),
            ident("serde_cursor"),
            punct(':'),
            punct(':'),
            ident("Cursor"),
            punct('<'),
        ]
        .into_iter()
        .chain(type_tokens)
        .chain([punct(',')])
        .chain(path)
        .chain([punct('>')]),
    )
}

enum PathSegment {
    Field(String, Span),
    Index(u128, Span),
}

impl PathSegment {
    fn to_tokens(&self) -> TokenStream {
        match self {
            PathSegment::Field(field, span) => const_str::encode(field, *span),
            PathSegment::Index(index, span) => {
                let mut ts = path([ident("Index")]);
                ts.extend([punct('<')]);
                let mut lit = Literal::u128_unsuffixed(*index);
                lit.set_span(*span);
                ts.extend(Some(TokenTree::Literal(lit)));
                ts.extend([punct('>')]);
                ts
            }
        }
    }
}

fn parse_path_segment(
    input: &mut std::iter::Peekable<proc_macro::token_stream::IntoIter>,
    path_segments: &mut Vec<PathSegment>,
) {
    match input.peek().unwrap() {
        // Identifier fields
        //
        // Cursor!(a.b.c: bool)
        //         ^
        TokenTree::Ident(_) => {
            let Some(TokenTree::Ident(field)) = input.next() else {
                unreachable!()
            };
            path_segments.push(PathSegment::Field(field.to_string(), field.span()));
        }
        TokenTree::Literal(lit) => {
            match litrs::Literal::from(lit) {
                // Integer index
                //
                // Cursor!(a.0.c: bool)
                //           ^
                litrs::Literal::Integer(index) => {
                    path_segments.push(PathSegment::Index(
                        index.value::<u128>().unwrap(),
                        lit.span(),
                    ));
                    input.next();
                }
                // Integer index
                //
                // Cursor!(a."hello world".c: bool)
                //           ^^^^^^^^^^^^^
                litrs::Literal::String(field) => {
                    path_segments.push(PathSegment::Field(field.value().to_string(), lit.span()));
                    input.next();
                }
                _ => panic!(),
            };
        }
        _ => panic!(),
    }
}

fn ident(name: &str) -> TokenTree {
    TokenTree::Ident(Ident::new(name, Span::call_site()))
}

fn punct(char: char) -> TokenTree {
    TokenTree::Punct(Punct::new(char, Spacing::Joint))
}

/// Returns path at `::serde_cursor`
fn path(segments: impl IntoIterator<Item = TokenTree>) -> TokenStream {
    segments.into_iter().enumerate().fold(
        TokenStream::from_iter([
            punct(':'),
            punct(':'),
            ident("serde_cursor"),
            punct(':'),
            punct(':'),
        ]),
        |mut ts, (i, path_segment)| {
            if i > 0 {
                ts.extend([punct(':'), punct(':')]);
            }
            ts.extend(Some(path_segment));
            ts
        },
    )
}
