//! Terminal writer with minimal styling support.

use std::{
    env, fmt,
    io::{self, IsTerminal, Write},
    ops,
};

use crate::text::Wrapper;

fn enable_color<T: IsTerminal>(fd: &T) -> bool {
    if env::var_os("NO_COLOR").is_some() {
        return false;
    }

    if env::var_os("CLICOLOR_FORCE").is_some() {
        return true;
    }

    if let Some(term) = env::var_os("TERM")
        && term == "dumb"
    {
        return false;
    }

    // TODO: check $COLORTERM

    fd.is_terminal()
}

struct FmtAdapter<F: fmt::Write>(F);

impl<F: fmt::Write> io::Write for FmtAdapter<F> {
    fn write(&mut self, buf: &[u8]) -> io::Result<usize> {
        for chunk in buf.utf8_chunks() {
            self.0
                .write_str(chunk.valid())
                .map_err(|_| io::ErrorKind::Other)?;
            if !chunk.invalid().is_empty() {
                self.0
                    .write_char('\u{fffe}')
                    .map_err(|_| io::ErrorKind::Other)?;
            }
        }

        Ok(buf.len())
    }

    fn flush(&mut self) -> io::Result<()> {
        Ok(())
    }
}

pub const DEFAULT_LINE_WIDTH: usize = 100;

pub struct Writer<'a> {
    out: Box<dyn Write + 'a>,
    enable_color: bool,
    max_line_width: usize,
    /// Paragraph buffer.
    paragraph: Vec<u8>,
    /// Indentation to put at the beginning of each auto-wrapped line.
    ///
    /// This contains `\n`, followed by a number of space characters that make up the indentation.
    indentation: Vec<u8>,
}

impl<'a> Writer<'a> {
    /// Creates a [`Writer`] that wraps a [`fmt::Write`] implementor.
    ///
    /// Can be used to implement [`fmt::Display`] with printing code that uses [`Writer`].
    pub fn display(f: impl fmt::Write + 'a) -> Self {
        Self::new(FmtAdapter(f), false)
    }

    /// Creates a [`Writer`] that writes to a file descriptor (potentially a terminal).
    ///
    /// The provided type's [`io::Write`] implementation is expected to write to the file descriptor
    /// returned by its [`AsFd`] implementation.
    ///
    /// If the environment is configured for automatic colorization, and the file descriptor is a
    /// terminal, colors will be enabled.
    pub fn fd(fd: impl Write + IsTerminal + 'a) -> Self {
        let colors = enable_color(&fd);
        Self::new(fd, colors)
    }

    /// Creates a [`Writer`] that writes to an [`io::Write`] implementor that does *not* refer to a
    /// terminal.
    ///
    /// Colors will be disabled.
    pub fn io(w: impl Write + 'a) -> Self {
        Self::new(w, false)
    }

    fn new(out: impl Write + 'a, enable_color: bool) -> Self {
        Self {
            out: Box::new(out),
            enable_color,
            max_line_width: DEFAULT_LINE_WIDTH,
            paragraph: Vec::new(),
            indentation: vec![b'\n'],
        }
    }

    pub fn max_line_width(&self) -> usize {
        self.max_line_width
    }

    pub fn set_max_line_width(&mut self, width: usize) {
        self.max_line_width = width;
    }

    pub fn force_color(&mut self, color: bool) {
        self.enable_color = color;
    }

    pub fn indentation(&self) -> usize {
        self.indentation.len() - 1
    }

    pub fn set_indentation(&mut self, indent: usize) {
        self.indentation.resize(indent + 1, b' ');
    }

    fn finish_paragraph(&mut self) -> io::Result<()> {
        // Softwrap `self.paragraph` and apply indentation.
        let iter = Wrapper::new(&self.paragraph, self.max_line_width)
            .with_newline(&self.indentation, self.indentation.len() - 1)
            .wrap();

        for frag in iter {
            for chunk in frag.utf8_chunks() {
                for (i, ch) in chunk.valid().char_indices() {
                    match ch {
                        NBSP => self.out.write_all(b" ")?,
                        ZWSP => continue,
                        _ => self
                            .out
                            .write_all(&chunk.valid().as_bytes()[i..i + ch.len_utf8()])?,
                    }
                }
                if !chunk.invalid().is_empty() {
                    self.out.write_all(frag)?;
                }
            }
        }

        self.paragraph.clear();
        Ok(())
    }
}

impl Write for Writer<'_> {
    fn write(&mut self, buf: &[u8]) -> io::Result<usize> {
        // Assumption: ANSI escapes are not torn apart when being written to the `Writer`.
        // This simplifies stripping the escapes when colors are disabled.
        // `Style` makes sure that the whole escape sequence is written in one go.

        for chunk in buf.utf8_chunks() {
            for line in chunk.valid().split_inclusive('\n') {
                if self.enable_color {
                    self.paragraph.extend_from_slice(line.as_bytes());
                } else {
                    strip_escapes(line.as_bytes(), &mut self.paragraph);
                }
                if line.ends_with('\n') {
                    self.finish_paragraph()?;
                }
            }

            // Replace invalid UTF-8 with a replacement char.
            if !chunk.invalid().is_empty() {
                self.paragraph.extend_from_slice("\u{fffe}".as_bytes());
            }
        }

        Ok(buf.len())
    }

    fn flush(&mut self) -> io::Result<()> {
        self.finish_paragraph()?;
        self.out.flush()
    }
}

fn strip_escapes(input: &[u8], output: &mut Vec<u8>) {
    let mut in_escape = false;
    for &byte in input {
        match byte {
            b'\x1b' => in_escape = true,
            b'm' if in_escape => in_escape = false,
            _ if !in_escape => output.push(byte),
            _ => {}
        }
    }
}

impl Drop for Writer<'_> {
    fn drop(&mut self) {
        self.finish_paragraph().ok();
    }
}

/// A non-breaking space character.
///
/// This can be used to prevent automatic line wrapping from happening at this position.
pub const NBSP: char = '\u{00A0}';

/// An invisible, zero-width space that allows line breaking.
///
/// This can be used to mark a place where a line break may be inserted, without taking up any
/// space.
pub const ZWSP: char = '\u{200B}';

const fn style(bit: u64) -> Style {
    Style(1 << bit)
}
pub const RESET: Style = style(0);
pub const BOLD: Style = style(1);
pub const FAINT: Style = style(2);
pub const ITALIC: Style = style(3);
pub const UNDERLINE: Style = style(4);
pub const RED: Style = style(31);
pub const YELLOW: Style = style(33);

/// A set of ANSI escape codes that modify the text style.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Style(pub u64);

impl Style {
    /// No change to style.
    pub const NONE: Style = Style(0);
}

impl ops::BitOr for Style {
    type Output = Style;

    fn bitor(self, rhs: Self) -> Self::Output {
        Self(self.0 | rhs.0)
    }
}
impl ops::BitOrAssign for Style {
    fn bitor_assign(&mut self, rhs: Self) {
        self.0 |= rhs.0;
    }
}

impl fmt::Display for Style {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        if self.0 == 0 {
            return Ok(());
        }

        let mut buf = [0; 128];
        let mut w = &mut buf[..];
        write!(w, "\x1B[").ok();

        let mut first = true;
        for bit in 0..64 {
            if self.0 & (1 << bit) != 0 {
                if !first {
                    write!(w, ";").ok();
                }
                first = false;
                write!(w, "{bit}").ok();
            }
        }
        write!(w, "m").ok();

        let remaining = w.len();
        let used = buf.len() - remaining;
        f.write_str(str::from_utf8(&buf[..used]).unwrap())
    }
}

#[cfg(test)]
mod tests {
    use std::sync::{Arc, Mutex, PoisonError};

    use super::*;

    #[derive(Default, Clone)]
    struct TestSink(Arc<Mutex<Vec<u8>>>);

    impl Write for TestSink {
        fn write(&mut self, buf: &[u8]) -> io::Result<usize> {
            self.0
                .lock()
                .unwrap_or_else(PoisonError::into_inner)
                .write(buf)
        }

        fn flush(&mut self) -> io::Result<()> {
            self.0
                .lock()
                .unwrap_or_else(PoisonError::into_inner)
                .flush()
        }
    }

    #[track_caller]
    fn col_check(color: bool, f: impl FnOnce(&mut Writer), raw: &[u8]) {
        let sink = TestSink::default();
        let mut w = Writer::new(sink.clone(), color);
        f(&mut w);
        w.flush().unwrap();

        let guard = sink.0.lock().unwrap();
        assert_eq!(
            raw,
            guard.as_slice(),
            r#"expected "{}", got "{}""#,
            raw.escape_ascii(),
            guard.as_slice().escape_ascii()
        );
    }

    #[track_caller]
    fn wrapcheck(f: impl FnOnce(&mut Writer), raw: &[u8]) {
        col_check(
            true,
            |w| {
                w.max_line_width = 5;
                f(w)
            },
            raw,
        );
    }

    #[test]
    fn colors() {
        col_check(false, |w| writeln!(w, "abcde").unwrap(), b"abcde\n");
        col_check(true, |w| writeln!(w, "abcde").unwrap(), b"abcde\n");

        col_check(false, |w| writeln!(w, "A{BOLD}B").unwrap(), b"AB\n");
        col_check(true, |w| writeln!(w, "A{BOLD}B").unwrap(), b"A\x1B[1mB\n");
    }

    #[test]
    fn test_wrapping() {
        wrapcheck(
            |w| write!(w, "abc def 1 2 34 56").unwrap(),
            b"abc\ndef 1\n2 34\n56",
        );
    }

    #[test]
    fn test_indentation() {
        wrapcheck(
            |w| {
                w.max_line_width = 10;
                w.set_indentation(5);
                write!(w, "123 456 789 abc").unwrap();
            },
            b"123 456\n     789\n     abc",
        );
    }

    #[test]
    fn test_style() {
        assert_eq!(Style::NONE.to_string(), "");
        assert_eq!((RED | UNDERLINE).to_string(), "\x1B[4;31m");
        assert_eq!(RED.to_string(), "\x1B[31m");
    }
}
