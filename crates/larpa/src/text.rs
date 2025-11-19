use std::iter;

use crate::writer::ZWSP;

/// Calculates the display width of a chunk of text.
///
/// This takes a byte slice and tries to take invalid UTF-8 into account.
///
/// This is fundamentally an approximation, since there is no way to know which font the output is
/// using, and the specific behavior of terminal emulators may differ.
///
/// The result of this method is only meaningful when called on a string that does not contain `\n`
/// or `\r`.
pub fn width(text: &[u8]) -> usize {
    let mut width = 0;
    for chunk in text.utf8_chunks() {
        width += str_width(chunk.valid());

        if !chunk.invalid().is_empty() {
            // We assume that all invalid chunks of data are replaced with a single 2-wide
            // replacement character.
            width += 2;
        }
    }
    width
}

pub fn char_width(c: char) -> usize {
    let mut buf = [0; 4];
    str_width(c.encode_utf8(&mut buf))
}

// FIXME: `#[cfg()]` proper unicode-width support
fn str_width(s: &str) -> usize {
    let mut is_escape = false;
    s.chars()
        .map(|ch| match (ch, is_escape) {
            (ZWSP, _) => 0,
            ('m', true) => {
                is_escape = false;
                0
            }
            ('\x1b', _) => {
                is_escape = true;
                0
            }
            (_, true) => 0,
            (_, false) => 1,
        })
        .sum()
}

pub struct Wrapper<'a> {
    paragraph: &'a [u8],
    max_width: usize,
    newline: &'a [u8],
    newline_width: usize,
}

impl<'a> Wrapper<'a> {
    pub fn new(paragraph: &'a [u8], max_width: usize) -> Self {
        Self {
            paragraph,
            max_width,
            newline: b"\n",
            newline_width: 0,
        }
    }

    /// Intersperse `newline` when wrapping.
    ///
    /// `newline` is considered to take up `newline_width` of space on the *next* line.
    pub fn with_newline(self, newline: &'a [u8], newline_width: usize) -> Self {
        Self {
            newline,
            newline_width,
            ..self
        }
    }

    pub fn wrap(self) -> impl Iterator<Item = &'a [u8]> {
        let mut words = words(self.paragraph).peekable();
        let mut width = 0;
        let mut inject_newline = false;

        iter::from_fn(move || -> Option<&[u8]> {
            if inject_newline {
                inject_newline = false;
                return Some(self.newline);
            }

            // When we get here, `cur` will definitely go on *this* line, but we might wrap *after* it.
            let cur = words.next()?;
            let next = words.peek().copied().unwrap_or(&[]);

            let cur_width = self::width(cur);

            let nowrap_width = width + cur_width + self::width(next.trim_end_spaces());
            if nowrap_width > self.max_width
                && width + self::width(cur.trim_end_spaces()) > self.newline_width
            {
                // Putting `cur` and `next` on the same line would overflow.
                // If `next` exists, wrap after `cur`.
                inject_newline = words.peek().is_some();
                width = self.newline_width;
                Some(cur.trim_end_spaces())
            } else {
                width += cur_width;
                Some(cur)
            }
        })
    }
}

trait BytesExt {
    fn trim_end_spaces(&self) -> &Self;
}
impl BytesExt for [u8] {
    fn trim_end_spaces(&self) -> &Self {
        const ZWSP_B: [u8; ZWSP.len_utf8()] = {
            let mut buf = [0; ZWSP.len_utf8()];
            ZWSP.encode_utf8(&mut buf);
            buf
        };

        let mut slice = self;
        loop {
            if let Some(sl) = slice.strip_suffix(b" ") {
                slice = sl;
            } else if let Some(sl) = slice.strip_suffix(&ZWSP_B) {
                slice = sl;
            } else {
                return slice;
            }
        }
    }
}

/// Splits `buf` into words separated by spaces (` `). Trailing spaces are included.
fn words(buf: &[u8]) -> impl Iterator<Item = &[u8]> {
    let mut start = 0;
    let mut was_space = false;

    iter::from_fn(move || {
        for chunk in buf[start..].utf8_chunks() {
            let valid_len = chunk.valid().len();
            let spaces = chunk
                .valid()
                .char_indices()
                .map(|(i, ch)| (i, ch == ' ' || ch == ZWSP))
                .chain(
                    chunk
                        .invalid()
                        .iter()
                        .enumerate()
                        .map(|(i, _)| (valid_len + i, false)),
                );
            for (i, is_space) in spaces {
                if was_space && !is_space {
                    was_space = is_space;
                    let word = &buf[start..start + i];
                    start += i;
                    return Some(word);
                }

                was_space = is_space;
            }
        }

        if start < buf.len() {
            // Yield the remainder.
            let word = &buf[start..];
            start = buf.len();
            return Some(word);
        }
        None
    })
}

#[cfg(test)]
mod tests {
    use crate::writer::{BOLD, NBSP, RESET};

    use super::*;

    #[test]
    fn test_width() {
        assert_eq!(width(b""), 0);
        assert_eq!(width("ü".as_bytes()), 1);
        assert_eq!(width("위".as_bytes()), 1);
        assert_eq!(width("💖".as_bytes()), 1);

        assert_eq!(width(b"b\xff"), 3);

        // ANSI escape sequences do not count towards the text width.
        assert_eq!(width(format!("{BOLD}BOLD{RESET}").as_bytes()), 4);
    }

    #[test]
    fn test_trim() {
        assert_eq!(b"".trim_end_spaces(), b"");
        assert_eq!(b"\n ".trim_end_spaces(), b"\n");
        assert_eq!(b"abc  ".trim_end_spaces(), b"abc");
        assert_eq!(
            format!("abc{NBSP}").as_bytes().trim_end_spaces(),
            format!("abc{NBSP}").as_bytes()
        );
        assert_eq!(format!("abc{ZWSP}").as_bytes().trim_end_spaces(), b"abc");
    }

    #[test]
    fn test_words() {
        assert_eq!(
            words(b"  abc def  gh   a\xffb ").collect::<Vec<_>>(),
            vec![&b"  "[..], b"abc ", b"def  ", b"gh   ", b"a\xffb "],
        );
        assert_eq!(
            words(format!("abc{ZWSP}123").as_bytes()).collect::<Vec<_>>(),
            vec![format!("abc{ZWSP}").as_bytes(), b"123"],
        );
    }

    #[test]
    fn test_wrap() {
        fn check_indent(indent: usize, text: &[u8], wrapped: &[u8]) {
            let mut newline = vec![b'\n'];
            newline.resize(indent + 1, b' ');
            let actual = Wrapper::new(text, 10)
                .with_newline(&newline, indent)
                .wrap()
                .flatten()
                .copied()
                .collect::<Vec<_>>();

            assert_eq!(
                actual,
                wrapped,
                r#"expected "{}", got "{}""#,
                wrapped.escape_ascii(),
                actual.escape_ascii()
            );
        }
        fn check(text: &[u8], wrapped: &[u8]) {
            check_indent(0, text, wrapped);
        }

        check(b"12345 7890 abc def", b"12345 7890\nabc def");
        check(b"1234567890 bla", b"1234567890\nbla");
        check(b"12345678901234567890", b"12345678901234567890");
        check(b"1 2 3 4 5 longtext", b"1 2 3 4 5\nlongtext");

        check_indent(4, b"123 456 789 abcdef", b"123 456\n    789\n    abcdef");

        // If wrapping doesn't allow the text to fit, we don't wrap.
        check_indent(
            4,
            b"1234 toolongtofitononeline",
            b"1234 toolongtofitononeline",
        );

        check(
            format!("1234567890 1234567890{NBSP}abcd").as_bytes(),
            format!("1234567890\n1234567890{NBSP}abcd").as_bytes(),
        );
        check(
            format!("1234567890{ZWSP}1234567890{NBSP}abcd").as_bytes(),
            format!("1234567890\n1234567890{NBSP}abcd").as_bytes(),
        );
    }
}
