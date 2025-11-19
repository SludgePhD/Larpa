//! Parser for raw arguments.
//!
//! This yields a sequence of [`RawArg`]s to the caller, which is responsible for all the rest.

use std::{
    ffi::{OsStr, OsString},
    fmt, str,
};

use crate::error::ErrorKind;

// FIXME: uses a little bit of unsafe for `from_encoded_bytes_unchecked`
// Once `os_str_slice` is stable we can get rid of that.

#[derive(Clone, Copy)]
pub enum ParseState {
    /// Not inside any option.
    Default { chunk: usize },
    /// After `--`.
    Done { chunk: usize },
    /// Inside a group of short options (`-lsH`).
    Shorts { chunk: usize, offset: usize },
    /// `--long=value`, where `RawArg::Long` has been yielded.
    LongEq { chunk: usize, offset: usize },
}

impl Default for ParseState {
    fn default() -> Self {
        Self::Default { chunk: 0 }
    }
}

#[derive(Debug, PartialEq, Eq)]
pub enum RawArg<'a> {
    Short(char),
    /// After receiving this, the caller *must* fetch the argument value via `peek_value`.
    Long(&'a str),
    Value(&'a OsStr),
    Eof,
}

impl<'a> fmt::Display for RawArg<'a> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            RawArg::Short(ch) => write!(f, "argument `-{ch}`"),
            RawArg::Long(name) => write!(f, "argument `--{name}`"),
            RawArg::Value(os_str) => write!(f, "argument `{}`", os_str.to_string_lossy()),
            RawArg::Eof => f.write_str("end of invocation"),
        }
    }
}

impl ParseState {
    pub fn chunk_index(&self) -> usize {
        match self {
            ParseState::Default { chunk }
            | ParseState::Done { chunk }
            | ParseState::Shorts { chunk, .. }
            | ParseState::LongEq { chunk, .. } => *chunk,
        }
    }

    pub fn peek_next<'a>(
        &self,
        chunks: &'a [OsString],
    ) -> Result<(RawArg<'a>, ParseState), ErrorKind> {
        let mut state = *self;
        loop {
            match &mut state {
                ParseState::Default { chunk: next } => {
                    let Some(chunk) = chunks.get(*next) else {
                        return Ok((RawArg::Eof, state));
                    };
                    *next += 1;

                    match chunk.as_encoded_bytes() {
                        [] | [b'-'] => return Ok((RawArg::Value(chunk), state)),
                        [b'-', b'-'] => state = ParseState::Done { chunk: *next },
                        [b'-', b'-', long @ ..] => match long.iter().position(|b| *b == b'=') {
                            Some(pos) => {
                                let name =
                                    str::from_utf8(&long[..pos]).map_err(ErrorKind::Utf8Error)?;
                                state = ParseState::LongEq {
                                    chunk: *next - 1,
                                    offset: name.len() + 3,
                                };
                                return Ok((RawArg::Long(name), state));
                            }
                            None => {
                                return Ok((
                                    RawArg::Long(
                                        str::from_utf8(long).map_err(ErrorKind::Utf8Error)?,
                                    ),
                                    state,
                                ));
                            }
                        },
                        [b'-', rest @ ..] => {
                            let first = first_char(rest)?;
                            if rest.len() > first.len_utf8() {
                                state = ParseState::Shorts {
                                    chunk: *next - 1,
                                    offset: 1 + first.len_utf8(),
                                };
                            }
                            return Ok((RawArg::Short(first), state));
                        }
                        _ => return Ok((RawArg::Value(chunk), state)),
                    }
                }
                ParseState::Shorts { chunk, offset } => {
                    let bytes = chunks[*chunk].as_encoded_bytes();
                    let rem = &bytes[*offset..];
                    let c = first_char(rem)?;
                    *offset += c.len_utf8();
                    if *offset == bytes.len() {
                        state = ParseState::Default { chunk: *chunk + 1 };
                    }
                    return Ok((RawArg::Short(c), state));
                }
                ParseState::LongEq { chunk, offset } => {
                    // `--long=value`, but `value` hasn't been retrieved using `Parser::value`.
                    // This means the caller would be interpreting it as a positional arg if we
                    // yielded it here, which is wrong, so we error.
                    let bytes = chunks[*chunk].as_encoded_bytes();
                    let name = str::from_utf8(&bytes[2..*offset - 1]).unwrap();
                    panic!("internal error: `--{name}` was provided a value that wasn't retrieved");
                }
                ParseState::Done { chunk: next } => match chunks.get(*next) {
                    Some(val) => {
                        *next += 1;
                        return Ok((RawArg::Value(val), state));
                    }
                    None => return Ok((RawArg::Eof, state)),
                },
            }
        }
    }

    /// Returns `true` if the last [`RawArg`] was a long argument with an `=` sign (`--long=`).
    pub fn after_eq(&self) -> bool {
        match self {
            ParseState::LongEq { .. } => true,
            _ => false,
        }
    }

    pub fn peek_value<'a>(&self, chunks: &'a [OsString]) -> Option<(&'a OsStr, ParseState)> {
        let mut state = *self;
        match &mut state {
            ParseState::Default { chunk: next } | ParseState::Done { chunk: next } => {
                let chunk = chunks.get(*next)?;
                *next += 1;
                Some((chunk, state))
            }
            ParseState::Shorts { chunk, offset } | ParseState::LongEq { chunk, offset } => {
                let new_state = ParseState::Default { chunk: *chunk + 1 };
                let chunk = chunks[*chunk].as_encoded_bytes();
                let val = unsafe { OsStr::from_encoded_bytes_unchecked(&chunk[*offset..]) };
                Some((val, new_state))
            }
        }
    }
}

/// Decodes the first UTF-8 character in the byte slice.
fn first_char(s: &[u8]) -> Result<char, ErrorKind> {
    let len = s[0].leading_ones().max(1);
    let s = str::from_utf8(&s[..len as usize]).map_err(ErrorKind::Utf8Error)?;
    Ok(s.chars().next().unwrap())
}

#[cfg(test)]
mod tests {
    use super::*;

    struct Parser {
        chunks: Vec<OsString>,
        state: ParseState,
    }

    impl Parser {
        fn from_iter<I>(iter: impl IntoIterator<Item = I>) -> Self
        where
            I: Into<OsString>,
        {
            Self {
                chunks: iter.into_iter().map(Into::into).collect::<Vec<_>>(),
                state: ParseState::Default { chunk: 0 },
            }
        }

        fn value(&mut self) -> Option<&OsStr> {
            let (str, state) = self.state.peek_value(&self.chunks)?;
            self.state = state;
            Some(str)
        }

        fn next(&mut self) -> Result<RawArg<'_>, ErrorKind> {
            let (arg, state) = self.state.peek_next(&self.chunks)?;
            self.state = state;
            Ok(arg)
        }
    }

    #[track_caller]
    fn check_args<I>(chunks: impl IntoIterator<Item = I>, args: &[RawArg<'_>])
    where
        I: Into<OsString>,
    {
        let mut p = Parser::from_iter(chunks);
        for (i, expected) in args.iter().enumerate() {
            let got = p.next().unwrap();
            assert_eq!(*expected, got, "argument {i} doesn't match");
        }
        assert_eq!(p.next().unwrap(), RawArg::Eof);
    }

    #[test]
    fn short_args() {
        check_args([""], &[RawArg::Value(OsStr::new(""))]);
        check_args(["-"], &[RawArg::Value(OsStr::new("-"))]);
        check_args(["--", "-a"], &[RawArg::Value(OsStr::new("-a"))]);

        check_args(["-="], &[RawArg::Short('=')]);

        assert_eq!("ü".len(), 2);
        assert_eq!("위".len(), 3);
        assert_eq!("💖".len(), 4);
        check_args(
            ["-ü위💖"],
            &[RawArg::Short('ü'), RawArg::Short('위'), RawArg::Short('💖')],
        );
    }

    #[test]
    fn short_values() {
        let mut p = Parser::from_iter(["-abval1", "--long", "val2", "-s", "val3", "-aS", "val4"]);

        assert_eq!(p.next().unwrap(), RawArg::Short('a'));
        assert_eq!(p.next().unwrap(), RawArg::Short('b'));
        assert_eq!(p.value().unwrap(), OsStr::new("val1"));
        assert_eq!(p.next().unwrap(), RawArg::Long("long"));
        assert_eq!(p.value().unwrap(), OsStr::new("val2"));
        assert_eq!(p.next().unwrap(), RawArg::Short('s'));
        assert_eq!(p.value().unwrap(), OsStr::new("val3"));
        assert_eq!(p.next().unwrap(), RawArg::Short('a'));
        assert_eq!(p.next().unwrap(), RawArg::Short('S'));
        assert_eq!(p.value().unwrap(), OsStr::new("val4"));
        assert_eq!(p.next().unwrap(), RawArg::Eof);
    }

    #[test]
    fn long_args() {
        check_args(
            ["--long", "value"],
            &[RawArg::Long("long"), RawArg::Value(OsStr::new("value"))],
        );

        let mut p = Parser::from_iter(["--long=value"]);
        assert_eq!(p.next().unwrap(), RawArg::Long("long"));
        assert_eq!(p.value().unwrap(), "value");

        let mut p = Parser::from_iter(["--long=", "value"]);
        assert_eq!(p.next().unwrap(), RawArg::Long("long"));
        assert_eq!(p.value().unwrap(), "");
    }

    #[test]
    fn empty_parser() {
        let mut p = Parser::from_iter([""; 0]);
        assert_eq!(p.next().unwrap(), RawArg::Eof);
    }
}
