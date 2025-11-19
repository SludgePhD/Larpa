use std::{
    ffi::OsStr,
    fmt::{self, Write as _},
    io::{self, Write},
};

use crate::{
    CommandDesc,
    desc::ArgumentDesc,
    text,
    writer::{BOLD, NBSP, RESET, Style, UNDERLINE, Writer, ZWSP},
};

impl CommandDesc {
    pub fn usage(&self) -> Usage<'_> {
        Usage {
            desc: self,
            invocation: None,
            prefix: String::new(),
            highlight_arg: None,
            highlight_subcommand: false,
        }
    }
}

/// A usage string generator.
///
/// Returned by [`CommandDesc::usage`].
pub struct Usage<'a> {
    desc: &'a CommandDesc,
    invocation: Option<&'a OsStr>,
    prefix: String,
    highlight_arg: Option<usize>,
    highlight_subcommand: bool,
}

impl<'a> Usage<'a> {
    pub(crate) fn with_invocation(self, invocation: &'a OsStr) -> Self {
        Self {
            invocation: Some(invocation),
            ..self
        }
    }

    pub(crate) fn with_prefix(self, prefix: String) -> Self {
        Self { prefix, ..self }
    }

    pub(crate) fn highlight_arg(self, index: usize) -> Self {
        Self {
            highlight_arg: Some(index),
            ..self
        }
    }

    pub(crate) fn highlight_subcommand(self) -> Self {
        Self {
            highlight_subcommand: true,
            ..self
        }
    }

    fn args(&self) -> impl Iterator<Item = (bool, &ArgumentDesc)> + Clone {
        self.desc
            .args()
            .iter()
            .enumerate()
            .map(|(i, desc)| (Some(i) == self.highlight_arg, desc))
    }

    pub(crate) fn write_to(&self, f: &mut Writer<'_>) -> io::Result<()> {
        let argv0 = self
            .invocation
            .unwrap_or(OsStr::new(self.desc.canonical_name()));
        write!(f, "{}{BOLD}", self.prefix)?;
        f.write_all(argv0.as_encoded_bytes())?;
        write!(f, "{RESET}")?;
        let prefix_width = text::width(self.prefix.as_bytes());
        let namelen = text::width(argv0.as_encoded_bytes()) + prefix_width;
        if namelen < f.max_line_width() / 3 {
            // If more than 2/3 of the line is still free, indent the arguments so that they line up
            // nicely.
            // If the line is too narrow (for example because the command name is long) we don't do
            // this, to avoid all arguments getting crammed into the rightmost part of the terminal.
            f.set_indentation(namelen + 1);
        } else {
            f.set_indentation(prefix_width);
        }

        // Flags first.
        let mut short_flags = String::new();
        for (highlight, arg) in self.args() {
            if arg.takes_value() {
                continue;
            }

            if let Some(c) = arg.short() {
                match highlight {
                    true => write!(short_flags, "{UNDERLINE}{c}{RESET}").unwrap(),
                    false => short_flags.push(c),
                }
            }
        }
        if !short_flags.is_empty() {
            write!(f, " [-{short_flags}]")?;
        }
        for (highlight, arg) in self.args() {
            if arg.takes_value() {
                continue;
            }

            if let Some(long) = arg.long() {
                match highlight {
                    true => write!(f, " [{UNDERLINE}--{long}{RESET}]")?,
                    false => write!(f, " [--{long}]")?,
                }
            }
        }

        // Named args.
        let mut named_arg = |highlight: bool, arg: &ArgumentDesc| -> io::Result<()> {
            let val = arg.value_name().unwrap();
            write!(f, " ")?;
            if arg.is_optional() {
                write!(f, "[")?;
            } else {
                write!(f, "{BOLD}")?;
            }
            if highlight {
                write!(f, "{UNDERLINE}")?;
            }
            match (arg.short(), arg.long()) {
                (Some(short), None) => write!(f, "-{short}{NBSP}<{val}>")?,
                (None, Some(long)) => write!(f, "--{long}=<{val}>")?,
                (Some(short), Some(long)) => write!(f, "-{short}/--{long}{NBSP}<{val}>")?,
                (None, None) => unreachable!(),
            };
            write!(f, "{RESET}")?;
            if arg.is_optional() {
                write!(f, "]")?;
            }
            if arg.is_repeating() {
                write!(f, "...")?;
            }
            Ok(())
        };
        // Print optionals first, then mandatory ones, which fits more nicely with the optional
        // flags coming first, and positionals coming last (which are often mandatory).
        let named_args = self
            .args()
            .filter(|(_, arg)| !arg.is_positional() && arg.takes_value());
        for (hl, arg) in named_args.clone() {
            if arg.is_optional() {
                named_arg(hl, arg)?;
            }
        }
        for (hl, arg) in named_args.clone() {
            if !arg.is_optional() {
                named_arg(hl, arg)?;
            }
        }

        // Positional args after named args and flags.
        for (highlight, arg) in self.args() {
            if !arg.is_positional() {
                continue;
            }
            let Some(value_name) = arg.value_name() else {
                continue;
            };

            write!(f, " ")?;
            if highlight {
                write!(f, "{UNDERLINE}")?;
            }
            if arg.is_optional() {
                write!(f, "[")?;
            } else {
                write!(f, "{BOLD}<")?;
            }
            write!(f, "{value_name}")?;
            if arg.is_optional() {
                write!(f, "]")?;
            } else {
                write!(f, ">")?;
            }
            if arg.is_repeating() {
                write!(f, "...")?;
            }
            write!(f, "{RESET}")?;
        }

        // Subcommands at the very end.
        if !self.desc.subcommands().is_empty() || self.desc.has_subcommand_fallback() {
            write!(f, " ")?;
            let opt = self.desc.is_subcommand_optional();
            let mut style = Style::NONE;
            if !opt {
                style |= BOLD;
            }
            if self.highlight_subcommand {
                style |= UNDERLINE;
            }

            if opt {
                write!(f, "[")?;
            }
            let mut wrote_cmd = false;
            for subcmd in self.desc.subcommands() {
                if wrote_cmd {
                    write!(f, "{RESET}|{ZWSP}")?;
                }
                write!(f, "{style}{}", subcmd.canonical_name())?;
                wrote_cmd = true;
            }
            if self.desc.has_subcommand_fallback() {
                for subcmd in self.desc.discover_subcommands() {
                    if wrote_cmd {
                        write!(f, "{RESET}|{ZWSP}")?;
                    }
                    write!(f, "{style}{}", subcmd.name().display())?;
                    wrote_cmd = true;
                }
                if wrote_cmd {
                    write!(f, "{RESET}|{ZWSP}")?;
                }
                write!(f, "{style}...")?;
            }
            write!(f, "{RESET}")?;
            if self.desc.is_subcommand_optional() {
                write!(f, "]")?;
            }
        }

        Ok(())
    }
}

impl fmt::Display for Usage<'_> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        self.write_to(&mut Writer::display(f))
            .map_err(|_| fmt::Error)
    }
}

// TODO: optional long args with optional values should be printed as `[--long[=<value>]]`

#[cfg(test)]
mod tests {
    use std::ffi::OsString;

    use crate::{
        Command,
        markup::markup,
        writer::{BOLD, RESET, Writer},
    };
    use expect_test::{Expect, expect};

    fn check<C: Command>(expect: Expect) {
        let mut out = Vec::new();
        let mut w = Writer::io(&mut out);
        w.force_color(true);
        C::DESC
            .usage()
            .with_prefix(format!("{BOLD}usage{RESET}: "))
            .write_to(&mut w)
            .unwrap();
        drop(w);
        out.push(b'\n');

        let usage = markup(&str::from_utf8(&out).unwrap());
        expect.assert_eq(&usage);
    }

    #[test]
    fn optional() {
        #[derive(Command)]
        #[larpa(crate = "crate")]
        #[allow(dead_code)]
        struct MyCmd {
            #[larpa(name = "-s", default)]
            short: u8,

            #[larpa(name = "--long", default)]
            long: u8,

            #[larpa(name = ["-m", "--mixed"], default)]
            mixed: u8,

            #[larpa(default)]
            opt_positional: u8,
        }

        check::<MyCmd>(expect![[r#"
            <b>usage</b>: <b>larpa</b> [--help] [-s <SHORT>] [--long=<LONG>] [-m/--mixed <MIXED>] [OPT_POSITIONAL]
        "#]]);
    }

    #[test]
    fn required() {
        #[derive(Command)]
        #[larpa(crate = "crate")]
        #[allow(dead_code)]
        struct MyCmd {
            #[larpa(name = "-s")]
            short: u8,

            #[larpa(name = "--long")]
            long: u8,

            #[larpa(name = ["-m", "--mixed"])]
            mixed: u8,

            required_positional: u8,
        }

        check::<MyCmd>(expect![[r#"
            <b>usage</b>: <b>larpa</b> [--help] <b>-s <SHORT></b> <b>--long=<LONG></b> <b>-m/--mixed <MIXED></b> <b><REQUIRED_POSITIONAL></b>
        "#]]);
    }

    #[test]
    fn repeated() {
        #[derive(Command)]
        #[larpa(crate = "crate")]
        #[allow(dead_code)]
        struct MyCmd {
            #[larpa(name = "-s")]
            short: Vec<u8>,

            #[larpa(name = "--long")]
            long: Vec<u8>,

            #[larpa(name = ["-m", "--mixed"])]
            mixed: Vec<u8>,

            positional: Vec<u8>,
        }

        check::<MyCmd>(expect![[r#"
            <b>usage</b>: <b>larpa</b> [--help] [-s <SHORT>]... [--long=<LONG>]... [-m/--mixed <MIXED>]... [POSITIONAL]...
        "#]]);
    }

    #[test]
    fn flags() {
        #[derive(Command)]
        #[larpa(crate = "crate")]
        #[allow(dead_code)]
        struct MyCmd {
            #[larpa(name = "-s", flag)]
            short_only: u8,

            #[larpa(name = "--long", flag)]
            long: u8,

            #[larpa(name = ["-e", "--either"], flag)]
            either: u8,
        }

        check::<MyCmd>(expect![[r#"
            <b>usage</b>: <b>larpa</b> [-se] [--long] [--either] [--help]
        "#]]);
    }

    #[test]
    fn subcommands() {
        #[derive(Command)]
        #[larpa(crate = "crate", name = "git")]
        #[allow(dead_code)]
        enum Git {
            Push,
            Pull,
            Commit,
        }

        check::<Git>(expect![[r#"
            <b>usage</b>: <b>git</b> <b>push</b>|<b>pull</b>|<b>commit</b>
        "#]]);

        #[derive(Command)]
        #[larpa(crate = "crate", name = "gitwrap")]
        #[allow(dead_code)]
        struct Outer {
            #[larpa(flag, name = "-f")]
            flag: bool,

            #[larpa(subcommand)]
            git: Git,
        }

        check::<Outer>(expect![[r#"
            <b>usage</b>: <b>gitwrap</b> [-f] [--help] <b>push</b>|<b>pull</b>|<b>commit</b>
        "#]]);

        #[derive(Command)]
        #[larpa(crate = "crate", name = "opt")]
        #[allow(dead_code)]
        struct Opt {
            #[larpa(flag, name = "-f")]
            flag: bool,

            #[larpa(subcommand)]
            git: Option<Git>,
        }

        check::<Opt>(expect![[r#"
            <b>usage</b>: <b>opt</b> [-f] [--help] [push|pull|commit]
        "#]]);
    }

    #[test]
    fn subcommands_fallback() {
        #[derive(Command)]
        #[larpa(crate = "crate", name = "fallback-cmd")]
        #[allow(dead_code)]
        enum OnlyFallback {
            #[larpa(fallback)]
            Fallback(Vec<OsString>),
        }

        check::<OnlyFallback>(expect![[r#"
            <b>usage</b>: <b>fallback-cmd</b> <b>...</b>
        "#]]);

        #[derive(Command)]
        #[larpa(crate = "crate", name = "cmd")]
        #[allow(dead_code)]
        enum Cmd {
            List,
            Status,
            #[larpa(fallback)]
            Fallback(Vec<OsString>),
        }

        check::<Cmd>(expect![[r#"
            <b>usage</b>: <b>cmd</b> <b>list</b>|<b>status</b>|<b>...</b>
        "#]]);

        #[derive(Command)]
        #[larpa(crate = "crate", name = "embed")]
        #[allow(dead_code)]
        struct Embed {
            #[larpa(subcommand)]
            fld: Cmd,
        }

        check::<Embed>(expect![[r#"
            <b>usage</b>: <b>embed</b> [--help] <b>list</b>|<b>status</b>|<b>...</b>
        "#]]);

        #[derive(Command)]
        #[larpa(crate = "crate", name = "embed")]
        #[allow(dead_code)]
        struct EmbedOpt {
            #[larpa(subcommand)]
            fld: Option<Cmd>,
        }

        check::<EmbedOpt>(expect![[r#"
            <b>usage</b>: <b>embed</b> [--help] [list|status|...]
        "#]]);
    }
}
