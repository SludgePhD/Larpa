use std::{
    ffi::OsStr,
    fmt::Write as _,
    io::{self, Write},
};

use crate::{
    desc::{ArgumentDesc, CommandDesc},
    text,
    writer::{BOLD, FAINT, RESET, Writer},
};

impl CommandDesc {
    pub(crate) fn help(&self) -> Help<'_> {
        Help {
            desc: self,
            invocation: None,
        }
    }
}

pub(crate) struct Help<'a> {
    desc: &'a CommandDesc,
    invocation: Option<&'a OsStr>,
}

impl<'a> Help<'a> {
    pub(crate) fn with_invocation(self, invocation: &'a OsStr) -> Self {
        Self {
            invocation: Some(invocation),
            ..self
        }
    }

    pub(crate) fn write_to(&self, f: &mut Writer<'_>) -> io::Result<()> {
        if let Some(desc) = self.desc.description()
            && !desc.trim().is_empty()
        {
            write!(f, "{desc}\n\n")?;
        }

        let mut usage = self.desc.usage();
        if let Some(inv) = self.invocation {
            usage = usage.with_invocation(inv);
        }
        usage.write_to(f)?;
        writeln!(f)?;
        f.set_indentation(0);

        // If any positionals have documentation, print all of them.
        // Otherwise this wouldn't add any information that isn't already in the usage string.
        if positionals(self.desc).any(|arg| arg.description().is_some()) {
            writeln!(f, "\n{BOLD}ARGUMENTS{RESET}:")?;

            let name_len = positionals(self.desc)
                .map(|arg| arg.value_name().unwrap().len())
                .max()
                .unwrap_or(0);
            let indent = name_len + 4;
            f.set_indentation(indent);

            for arg in positionals(self.desc) {
                let value = arg.value_name().unwrap();

                write!(f, "  {BOLD}{value}{RESET}")?;
                if let Some(desc) = make_arg_desc(arg) {
                    write!(f, "{}", " ".repeat(indent - value.len() - 2))?;
                    write_multiline(f, &desc)?;
                }
                writeln!(f)?;
            }

            f.set_indentation(0);
        }

        if named(self.desc).next().is_some() {
            writeln!(f, "\n{BOLD}OPTIONS{RESET}:")?;

            // Left column contains "  -s, --long  "
            let longest_len = named(self.desc)
                .map(|arg| {
                    let name = arg.long().unwrap_or("").len();
                    let value = arg.value_name().map_or(0, |n| n.len() + 1);
                    name + value
                })
                .max()
                .unwrap_or(0);
            let indent = "  -s, --".len() + longest_len + "  ".len();
            f.set_indentation(indent);

            for arg in self.desc.args() {
                if arg.is_positional() {
                    continue;
                }

                write!(f, "  {BOLD}")?;
                let mut len = 2;
                len += match (arg.short(), arg.long()) {
                    (None, None) => unreachable!(),
                    (Some(short), None) => {
                        write!(f, "-{short}")?;
                        1 + text::char_width(short)
                    }
                    (None, Some(long)) => {
                        write!(f, "    --{long}")?;
                        6 + text::width(long.as_bytes())
                    }
                    (Some(short), Some(long)) => {
                        write!(f, "-{short}, --{long}")?;
                        6 + text::width(long.as_bytes())
                    }
                };
                write!(f, "{RESET}")?;
                if let Some(val) = arg.value_name() {
                    let sep = if arg.long().is_some() { "=" } else { " " };
                    write!(f, "{sep}{val}")?;
                    len += 1 + text::width(val.as_bytes());
                }
                if let Some(desc) = make_arg_desc(arg) {
                    write!(f, "{}", " ".repeat(indent - len))?;
                    write_multiline(f, &desc)?;
                }
                writeln!(f)?;
            }

            f.set_indentation(0);
        }

        let discovered = self.desc.discover_subcommands();
        if !self.desc.subcommands().is_empty() || !discovered.is_empty() {
            writeln!(f, "\n{BOLD}SUBCOMMANDS{RESET}:")?;

            let name_width = self
                .desc
                .subcommands()
                .iter()
                .map(|cmd| text::width(cmd.canonical_name().as_bytes()))
                .chain(
                    discovered
                        .iter()
                        .map(|cmd| text::width(cmd.name().as_encoded_bytes())),
                )
                .max()
                .unwrap_or(0);
            let indent = name_width + 4;
            f.set_indentation(indent);

            for cmd in self.desc.subcommands() {
                write!(f, "  {BOLD}{}{RESET}", cmd.canonical_name())?;
                if let Some(desc) = cmd.description() {
                    write!(
                        f,
                        "{}",
                        " ".repeat(indent - text::width(cmd.canonical_name().as_bytes()) - 2)
                    )?;
                    write_multiline(f, desc)?;
                }
                writeln!(f)?;
            }
            for cmd in &discovered {
                write!(f, "  {BOLD}")?;
                f.write_all(cmd.name().as_encoded_bytes())?;
                write!(f, "{RESET}")?;
                if let Some(desc) = cmd.description() {
                    write!(
                        f,
                        "{}",
                        " ".repeat(indent - text::width(cmd.name().as_encoded_bytes()) - 2)
                    )?;
                    write_multiline(f, desc)?;
                } else if let Some(path) = cmd.path() {
                    write!(
                        f,
                        "{}",
                        " ".repeat(indent - text::width(cmd.name().as_encoded_bytes()) - 2)
                    )?;
                    f.write_all(path.as_os_str().as_encoded_bytes())?;
                }
                writeln!(f)?;
            }

            f.set_indentation(0);
        }

        if self.desc.authors().is_some()
            || self.desc.homepage().is_some()
            || self.desc.repository().is_some()
            || self.desc.license().is_some()
        {
            writeln!(f)?;
        }
        if let Some(authors) = self.desc.authors() {
            writeln!(f, "{BOLD}Authors{RESET}: {authors}")?;
        }
        if let Some(hp) = self.desc.homepage() {
            writeln!(f, "{BOLD}Homepage{RESET}: {hp}")?;
        }
        if let Some(repo) = self.desc.repository() {
            writeln!(f, "{BOLD}Repository{RESET}: {repo}")?;
        }
        if let Some(license) = self.desc.license() {
            writeln!(f, "{BOLD}License{RESET}: {license}")?;
        }

        Ok(())
    }
}

fn named(desc: &CommandDesc) -> impl Iterator<Item = &ArgumentDesc> {
    desc.args().iter().filter(|arg| !arg.is_positional())
}

fn positionals(desc: &CommandDesc) -> impl Iterator<Item = &ArgumentDesc> {
    desc.args().iter().filter(|arg| arg.is_positional())
}

/// Modifies the argument description with generated content.
fn make_arg_desc(arg: &ArgumentDesc) -> Option<String> {
    let mut supp = String::new();
    if let Some(dfl) = arg.custom_default() {
        write!(supp, "{FAINT}[default: {dfl}]{RESET}").ok();
    }

    let mut out = String::new();
    let mut first = true;
    for line in arg.description().unwrap_or("").split_inclusive('\n') {
        if first {
            first = false;
            if line.ends_with('\n') {
                out.push_str(&line[..line.len() - 1]);
            } else {
                out.push_str(line);
            }
            if !supp.is_empty() {
                out.push(' ');
                out.push_str(&supp);
            }
            if line.ends_with('\n') {
                out.push('\n');
            }
        } else {
            out.push_str(line);
        }
    }

    if out.is_empty() {
        out.push_str(&supp);
    }

    if out.is_empty() { None } else { Some(out) }
}

fn write_multiline(f: &mut Writer<'_>, text: &str) -> io::Result<()> {
    let indent = f.indentation();
    let mut first = true;
    for line in text.split_inclusive('\n') {
        if !first && line != "\n" && line != "" {
            write!(f, "{}", " ".repeat(indent))?;
        }
        first = false;
        write!(f, "{line}")?;
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use std::ffi::OsString;

    use expect_test::{Expect, expect};

    use crate::{Command, desc::DiscoveredSubcommand, markup::markup};

    use super::*;

    fn check<C: Command>(color: bool, expect: Expect) {
        let mut out = Vec::new();
        let mut w = Writer::io(&mut out);
        w.force_color(color);
        C::DESC.help().write_to(&mut w).unwrap();
        drop(w);

        let usage = markup(&str::from_utf8(&out).unwrap());
        expect.assert_eq(&usage);
    }

    #[test]
    fn empty() {
        /// Before.
        #[derive(Command)]
        #[larpa(
            crate = "crate",
            no_repository,
            no_license,
            no_homepage,
            name = "empty"
        )]
        /// After.
        struct Empty;

        check::<Empty>(
            true,
            expect![[r#"
                Before. After.

                <b>empty</b> [--help]

                <b>OPTIONS</b>:
                  <b>    --help</b>  Print help information.
            "#]],
        );
    }

    #[test]
    fn empty_metadata() {
        ///
        #[derive(Command)]
        #[larpa(
            crate = "crate",
            repository = "repo",
            license = "license",
            homepage = "home page",
            name = "empty"
        )]
        struct Empty;

        check::<Empty>(
            true,
            expect![[r#"
                <b>empty</b> [--help]

                <b>OPTIONS</b>:
                  <b>    --help</b>  Print help information.

                <b>Homepage</b>: home page
                <b>Repository</b>: repo
                <b>License</b>: license
            "#]],
        );
    }

    #[test]
    fn smoke() {
        #[allow(dead_code)]
        #[derive(Command)]
        #[larpa(crate = "crate", name = "mycmd", no_generate_tests)]
        struct MyCmd {
            #[larpa(name = "--long")]
            long: u8,

            #[larpa(name = "-s")]
            short: u8,

            #[larpa(name = ["-m", "--mixed"])]
            mixed: u8,

            #[larpa(name = "--def", default = "123")]
            def: u8,

            pos: u8,

            #[larpa(subcommand)]
            _cmd: Subcmd,
        }

        #[derive(Command)]
        #[larpa(crate = "crate")]
        enum Subcmd {
            List,
            Help,
        }

        check::<MyCmd>(
            true,
            expect![[r#"
                Lousy ARgument PArser

                <b>mycmd</b> [--help] [--def=<DEF>] <b>--long=<LONG></b> <b>-s <SHORT></b> <b>-m/--mixed <MIXED></b> <b><POS></b> <b>list</b>|<b>help</b>

                <b>OPTIONS</b>:
                  <b>    --long</b>=LONG
                  <b>-s</b> SHORT
                  <b>-m, --mixed</b>=MIXED
                  <b>    --def</b>=DEF      <faint>[default: 123]</faint>
                  <b>    --help</b>         Print help information.

                <b>SUBCOMMANDS</b>:
                  <b>list</b>
                  <b>help</b>

                <b>Repository</b>: https://github.com/SludgePhD/Larpa
                <b>License</b>: 0BSD
            "#]],
        );
    }

    #[test]
    fn docs() {
        #[allow(dead_code)]
        #[derive(Command)]
        #[larpa(
            crate = "crate",
            no_repository,
            no_license,
            name = "mycmd",
            no_generate_tests
        )]
        struct MyCmd {
            /// Long. Well, not *that* long.
            #[larpa(name = "--long")]
            long: u8,

            /// Short.
            #[larpa(name = "-s")]
            short: u8,

            /// Either.
            #[larpa(name = ["-m", "--mixed"])]
            mixed: u8,

            /// This one has a default and is documented with a very long line that will be broken
            /// up to fit.
            ///
            /// Additional information for `--defaulted2`:
            /// - First item
            /// - Second item
            #[larpa(name = "--defaulted2", default = "123")]
            dfl2: u8,

            #[larpa(name = "--defaulted", default = "123")]
            dfl: u8,

            /// Documented positional with a long text that requires wrapping after the line
            /// runs on for too long.
            ///
            /// And some more text.
            pos: u8,

            positional2: u8,

            /// Another documented positional.
            positional3: u8,

            #[larpa(subcommand)]
            _cmd: Option<Subcmd>,
        }

        #[derive(Command)]
        #[larpa(crate = "crate")]
        enum Subcmd {
            /// List things.
            List,
            /// Documented subcommand with a long text that will be wrapped around so that it fits
            /// in the space.
            ///
            /// With two paragraphs.
            Documented,
            Undocumented,
        }

        check::<MyCmd>(
            false,
            expect![[r#"
                Lousy ARgument PArser

                mycmd [--help] [--defaulted2=<DFL2>] [--defaulted=<DFL>] --long=<LONG> -s <SHORT> -m/--mixed <MIXED>
                      <POS> <POSITIONAL2> <POSITIONAL3> [list|documented|undocumented]

                ARGUMENTS:
                  POS          Documented positional with a long text that requires wrapping after the line runs on
                               for too long.

                               And some more text.
                  POSITIONAL2
                  POSITIONAL3  Another documented positional.

                OPTIONS:
                      --long=LONG        Long. Well, not *that* long.
                  -s SHORT               Short.
                  -m, --mixed=MIXED      Either.
                      --defaulted2=DFL2  This one has a default and is documented with a very long line that will be
                                         broken up to fit. [default: 123]

                                         Additional information for `--defaulted2`:
                                         - First item
                                         - Second item
                      --defaulted=DFL    [default: 123]
                      --help             Print help information.

                SUBCOMMANDS:
                  list          List things.
                  documented    Documented subcommand with a long text that will be wrapped around so that it fits
                                in the space.

                                With two paragraphs.
                  undocumented
            "#]],
        );
    }

    #[test]
    fn discovered_subcommands() {
        #[allow(dead_code)]
        #[derive(Command)]
        #[larpa(crate = "crate", no_repository, no_license, name = "subcmds")]
        enum MyCmd {
            Builtin,
            #[larpa(fallback, discover = "discoverer")]
            Fallback(Vec<OsString>),
        }

        fn discoverer(_: &CommandDesc) -> Vec<DiscoveredSubcommand> {
            vec![
                DiscoveredSubcommand::new("name"),
                DiscoveredSubcommand::new("path").with_path("some/path/to/file.exe"),
                DiscoveredSubcommand::new("desc").with_description("Description"),
                DiscoveredSubcommand::new("both")
                    .with_path("some/both/file.exe")
                    .with_description("Description (2)"),
            ]
        }

        check::<MyCmd>(
            true,
            expect![[r#"
                Lousy ARgument PArser

                <b>subcmds</b> <b>builtin</b>|<b>name</b>|<b>path</b>|<b>desc</b>|<b>both</b>|<b>...</b>

                <b>SUBCOMMANDS</b>:
                  <b>builtin</b>
                  <b>name</b>
                  <b>path</b>     some/path/to/file.exe
                  <b>desc</b>     Description
                  <b>both</b>     Description (2)
            "#]],
        );
    }
}
