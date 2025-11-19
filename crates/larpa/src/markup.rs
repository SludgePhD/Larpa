use std::fmt::Write;

use crate::writer::{BOLD, FAINT, ITALIC, RED, RESET, Style, UNDERLINE, YELLOW};

/// Replaces ANSI escape codes with HTML-like tags for testing the markup.
///
/// Generated tags are not necessarily properly nested, so some cooperating of the printing code may
/// be required.
#[cfg(test)]
pub(crate) fn markup(buf: &str) -> String {
    markup_impl(Markup::Test, buf)
}

pub(crate) fn markup_html(buf: &str) -> String {
    markup_impl(Markup::Html, buf)
}

fn markup_impl(markup: Markup, buf: &str) -> String {
    let mut last = 0;
    let mut out = String::new();
    let mut style = Style::NONE;

    for (start, _) in buf.match_indices("\x1b[") {
        out.push_str(&buf[last..start]);

        let (end, _) = buf[start + 2..].match_indices('m').next().unwrap();
        let esc = &buf[start + 2..][..end];
        last = start + 2 + end + 1;

        let mut new_style = Style::NONE;
        for part in esc.split(';') {
            let idx = part.parse::<u64>().unwrap();
            new_style.0 |= 1 << idx;
        }

        if new_style == RESET {
            for style in bits(style) {
                let name = markup.tag(style).split(' ').next().unwrap();
                write!(out, "</{name}>").unwrap();
            }
            style = Style::NONE;
        } else {
            for style in bits(new_style).rev() {
                let name = markup.tag(style);
                write!(out, "<{name}>").unwrap();
            }
            style |= new_style;
        }
    }
    out.push_str(&buf[last..]);

    for style in bits(style) {
        let name = markup.tag(style).split(' ').next().unwrap();
        write!(out, "</{name}>").unwrap();
    }

    out
}

enum Markup {
    /// Shortened faux HTML used in snapshot tests.
    #[allow(dead_code)]
    Test,
    /// Real HTML.
    Html,
}

impl Markup {
    fn tag(&self, s: Style) -> &'static str {
        match self {
            Markup::Test => match s {
                BOLD => "b",
                UNDERLINE => "u",
                ITALIC => "i",
                RED => "red",
                YELLOW => "yellow",
                FAINT => "faint",
                _ => panic!("unexpected style: {s:?}"),
            },
            Markup::Html => match s {
                BOLD => "b",
                UNDERLINE => "u",
                ITALIC => "i",
                RED => "span style=\"color:red\"",
                YELLOW => "span style=\"color:yellow\"",
                FAINT => "span style=\"color:gray\"",
                _ => panic!("unexpected style: {s:?}"),
            },
        }
    }
}

fn bits(s: Style) -> impl DoubleEndedIterator<Item = Style> {
    (0..64).filter_map(move |i| {
        if s.0 & (1 << i) != 0 {
            Some(Style(1 << i))
        } else {
            None
        }
    })
}
