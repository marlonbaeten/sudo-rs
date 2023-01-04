use std::iter::Peekable;

// contract: if the accept method returns None, the iterator is not advanced; otherwise it is advanced beyond the accepted part of the input
pub trait Parse {
    fn parse(stream: &mut Peekable<impl Iterator<Item = char>>) -> Option<Self>
    where
        Self: Sized;
}

// primitive function
fn accept_if(
    predicate: impl Fn(char) -> bool,
    stream: &mut Peekable<impl Iterator<Item = char>>,
) -> Option<char> {
    let &c = stream.peek()?;
    if predicate(c) {
        stream.next();
        Some(c)
    } else {
        None
    }
}

#[derive(Debug)]
struct Whitespace;

impl Parse for Whitespace {
    fn parse(stream: &mut Peekable<impl Iterator<Item = char>>) -> Option<Self> {
        let mut eat_space = || accept_if(char::is_whitespace, stream);
        eat_space()?;
        while let Some(_) = eat_space() {}
        Some(Whitespace {})
    }
}

// same as accept_if, but parses whitespace
pub fn maybe_syntax(syntax: char, stream: &mut Peekable<impl Iterator<Item = char>>) -> Option<()> {
    accept_if(|c| c == syntax, stream)?;
    Whitespace::parse(stream);
    Some(())
}

pub fn require_syntax(syntax: char, stream: &mut Peekable<impl Iterator<Item = char>>) {
    if maybe_syntax(syntax, stream).is_none() {
        let str = if let Some(c) = stream.peek() {
            c.to_string()
        } else {
            "EOL".to_string()
        };
        panic!("parse error: expecting `{}' but found `{}'", syntax, str)
    }
}

pub fn maybe<T: Parse>(stream: &mut Peekable<impl Iterator<Item = char>>) -> Option<T> {
    T::parse(stream)
}

pub fn require<T: Parse>(stream: &mut Peekable<impl Iterator<Item = char>>) -> T {
    let Some(result) = maybe(stream) else {
        panic!("parse error: expected `{}'", std::any::type_name::<T>())
    };
    result
}

pub trait Token {
    const IDENT: fn(String) -> Self;
    const MAX_LEN: usize = 255;

    fn accept(c: char) -> bool;
    fn accept_1st(c: char) -> bool {
        Self::accept(c)
    }

    const ESCAPE: char = '\0';
    fn escaped(_: char) -> bool {
        false
    }
}

impl<T: Token> Parse for T {
    fn parse(stream: &mut Peekable<impl Iterator<Item = char>>) -> Option<Self> {
        let mut str = accept_if(T::accept_1st, stream)?.to_string();
        loop {
            if let Some(c) = accept_if(T::accept, stream) {
                str.push(c)
            } else if let Some(_) = accept_if(|c| c == T::ESCAPE, stream) {
                if let Some(c) = accept_if(T::escaped, stream) {
                    str.push(c)
                } else {
                    panic!("tokenizer: illegal escape sequence")
                }
            } else {
                break;
            }
            if str.len() >= T::MAX_LEN {
                panic!("tokenizer: exceeded safety margin")
            }
        }
        Whitespace::parse(stream);
        Some(T::IDENT(str))
    }
}

// I would recommend not using this for anything that has more than two alternatives
impl<T1: Token, T2: Parse> Parse for Result<T1, T2> {
    fn parse(stream: &mut Peekable<impl Iterator<Item = char>>) -> Option<Self> {
        let &c = stream.peek()?;
        if T1::accept(c) {
            T1::parse(stream).map(Ok)
        } else {
            T2::parse(stream).map(Err)
        }
    }
}

fn parse_list<T: Parse>(
    sep_by: char,
    max: usize,
    stream: &mut Peekable<impl Iterator<Item = char>>,
) -> Option<Vec<T>> {
    let mut elems = Vec::new();
    elems.push(maybe(stream)?);
    while maybe_syntax(sep_by, stream).is_some() {
        if elems.len() >= max {
            panic!("parse_list: parsing multiple items: safety margin exceeded")
        }
        elems.push(require(stream));
    }
    return Some(elems);
}

pub trait Many {
    const SEP: char = ',';
    const LIMIT: usize = 127;
}

impl<T: Parse + Many> Parse for Vec<T> {
    fn parse(stream: &mut Peekable<impl Iterator<Item = char>>) -> Option<Self> {
        parse_list(T::SEP, T::LIMIT, stream)
    }
}

#[allow(dead_code)]
pub fn end_of_parse(stream: &mut Peekable<impl Iterator<Item = char>>) -> Option<()> {
    match stream.peek() {
        Some(_) => None,
        None => Some(()),
    }
}
