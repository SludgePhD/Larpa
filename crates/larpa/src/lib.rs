//! The **L**ousy **Ar**gument **Pa**rser.
//!
//! Larpa is intended to be a simple `#[derive]`-based CLI option parser, in a similar vein to
//! [clap] and [argh].
//!
//! Larpa aims for the following goals:
//! - Stay relatively lightweight in terms of binary size impact, while still providing
//!   the bells and whistles users expect.
//! - Provide a vastly simpler and easier to understand interface than Clap.
//!
//! Larpa is still in development and is missing many features.
//! Feel free to contribute!
//!
//! # Current Features
//!
//! - Automatic generation of styled usage strings, `--help` and `--version` output.
//! - Introspectable command description ([`CommandDesc`]).
//! - Decent IDE support (completion and hover docs for attributes).
//! - About ~100 KB of binary size overhead (compared to ~500 KB for [clap] and ~30 KB for [argh]).
//!
//! # Usage
//!
//! The main entry point into the library is the [`Command`] trait.
//!
//! - [`Command::from_args`] parses [`env::args_os`] and exits the process when parsing fails.
//! - [`Command::try_from_args`] parses [`env::args_os`] and returns an [`Error`] on failure.
//! - [`Command::from_iter`] and [`Command::try_from_iter`] work the same way, except they accept
//!   a user-provided iterator to parse arguments from.
//!
//! The [`Command`] trait cannot be implemented by hand.
//! The only way to create an implementation is by using `#[derive(Command)]`.
//!
//! **The derive macro accepts several `#[larpa]` attributes, which are separately documented in
//! [`attrs`]. It is strongly recommended to read the documentation in that module for detailed
//! information about the attributes.**
//!
//! # Examples
//!
//! ## Arguments
//!
//! A simple CLI that accepts a few standard flags and a path to a configuration file.
//!
//! It demonstrates named and positional arguments, required and optional arguments,
//! arguments with custom default values, as well as [`Flag`]s and inverted flags.
//!
//! ```
//! use larpa::Command;
//! use larpa::types::{Verbosity, PrintHelp, PrintVersion};
//! use std::path::PathBuf;
//! use std::num::NonZeroU8;
//!
//! /// Data frobnication tool.
//! #[derive(Command)]
//! #[larpa(name = "frobnicator", version = "1.2.3")]
//! # #[larpa(no_license, no_repository, no_homepage)]
//! struct Args {
//!     /// Output more information.
//!     #[larpa(flag, name = ["-v", "--verbose"])]
//!     verbosity: Verbosity,
//!
//!     /// Output less information.
//!     #[larpa(flag, name = ["-q", "--quiet"], inverse_of = "verbosity")]
//!     quiet: (),
//!
//!     /// How many times to frobnicate the data (at least 1).
//!     #[larpa(name = "-n", default = "5")]
//!     iterations: NonZeroU8,
//!
//!     /// Path to the configuration file.
//!     #[larpa(name = "--config")]
//!     config: Option<PathBuf>,
//!
//!     /// The object to frobnicate (mandatory).
//!     object: String,
//! }
//! # larpa::private::snap::<Args>(["frobnicator", "--help"], "src/snaps/frob-help.txt");
//! # larpa::private::snap::<Args>(["frobnicator", "--version"], "src/snaps/frob-version.txt");
//! # larpa::private::snap::<Args>(["frobnicator"], "src/snaps/frob-missing-arg.txt");
//! # larpa::private::snap::<Args>(["frobnicator", "-n", "0"], "src/snaps/frob-0.txt");
//! ```
#![doc = include_str!("snaps/frob-help.txt")]
#![doc = include_str!("snaps/frob-version.txt")]
//!
//! An error will be returned when the mandatory `OBJECT` argument is missing:
//!
#![doc = include_str!("snaps/frob-missing-arg.txt")]
//!
//! Similarly, an error will be returned if a value cannot be parsed into the target type:
//!
#![doc = include_str!("snaps/frob-0.txt")]
//!
//! ## Subcommands
//!
//! Subcommands can be created by using `#[derive(Command)]` on an `enum`.
//!
//! [`#[larpa(fallback)]`][fallback] and [`#[larpa(discover)]`][discover] can be used on a variant
//! to implement external subcommands.
//!
//! An `enum` of subcommands can be embedded in a `struct` by marking a field with
//! [`#[larpa(subcommand)]`][subcommand].
//!
//! [fallback]: crate::attrs::variant::fallback
//! [discover]: crate::attrs::variant::discover
//!
//! ```
//! use larpa::Command;
//! use larpa::types::PrintVersion;
//! use std::ffi::OsString;
//! use std::path::PathBuf;
//!
//! #[derive(Command)]
//! enum Subcommand {
//!     // Variants can have fields that work just like they do in a `struct`.
//!
//!     /// Push changes.
//!     Push {
//!         /// Force-overwrite the remote ref.
//!         #[larpa(flag, name = ["-f", "--force"])]
//!         force: bool,
//!     },
//!
//!     // Alternatively, a tuple variant will use the inner type's `Command` implementation.
//!
//!     /// Pull changes.
//!     Pull(PullArgs),
//!
//!     // A variant without data is just a plain subcommand.
//!
//!     /// Show file status.
//!     Status,
//!
//!     // A catch-all variant can be created with `#[larpa(fallback)]`, to catch any subcommands
//!     // that don't match any of the other variants.
//!
//!     #[larpa(fallback)]
//!     Other(Vec<OsString>),
//! }
//!
//! #[derive(Command)]
//! struct PullArgs {
//!     #[larpa(flag, name = "--autostash")]
//!     autostash: bool,
//! }
//!
//! /// The gritty content tracker.
//! #[derive(Command)]
//! #[larpa(name = "grit", version = "1.0.0")]
//! # #[larpa(no_license, no_repository, no_homepage)]
//! struct Grit {
//!     /// Path to the configuration file.
//!     #[larpa(name = ["-c", "--config"])]
//!     config: Option<PathBuf>,
//!
//!     /// Print version information.
//!     #[larpa(name = "--version", flag)]
//!     version: PrintVersion,
//!
//!     #[larpa(subcommand)]
//!     subcommand: Subcommand,
//! }
//! # larpa::private::snap::<Grit>(["grit", "--help"], "src/snaps/grit-help.txt");
//! # larpa::private::snap::<Grit>(["grit", "push", "--help"], "src/snaps/grit-push-help.txt");
//! # larpa::private::snap::<Grit>(["grit", "pull", "--help"], "src/snaps/grit-pull-help.txt");
//! ```
#![doc = include_str!("snaps/grit-help.txt")]
#![doc = include_str!("snaps/grit-push-help.txt")]
#![doc = include_str!("snaps/grit-pull-help.txt")]
//!
//! [subcommand]: crate::attrs::field::subcommand
//! [clap]: https://github.com/clap-rs/clap
//! [argh]: https://github.com/google/argh

#![cfg_attr(docsrs, feature(doc_cfg))]
// FIXME: these lints cannot be scoped to a single item due to a compiler bug
#![allow(confusable_idents, mixed_script_confusables)]

macro_rules! hit {
    ($lbl:ident) => {
        #[cfg(test)]
        cov_mark::hit!($lbl);
    };
}

#[doc = include_str!("../../../README.md")]
mod readme {}

#[doc(hidden)]
pub mod private;

#[cfg(test)]
mod tests;

pub mod desc;
mod error;
mod flag;
mod generate;
mod markup;
mod parser;
mod text;
pub mod types;
mod value;
mod writer;

use std::{
    cell::Cell,
    env,
    ffi::{OsStr, OsString},
    path::PathBuf,
    rc::Rc,
};

#[cfg(docsrs)]
#[doc(cfg(docsrs))]
pub use private::attrs;

pub use larpa_derive::Command;

pub use error::{Error, Reporter};
pub use flag::Flag;
pub use generate::usage::Usage;

use crate::{
    desc::CommandDesc,
    parser::ParseState,
    private::Result,
    types::{Color, Verbosity},
};

/// A CLI command.
///
/// This trait cannot be implemented by hand, it can only be `#[derive]`d.
pub trait Command: private::CommandInternal + Sized {
    /// The command description.
    const DESC: CommandDesc;

    /// Tries to parse an instance of `Self` from the arguments passed to the program
    /// ([`env::args_os`]).
    ///
    /// If parsing fails, an error will be printed and the process will exit with an appropriate
    /// error code.
    fn from_args() -> Self {
        match Self::try_from_args() {
            Ok(v) => v,
            Err(e) => e.report_and_exit(),
        }
    }

    /// Tries to parse an instance of `Self` from the given arguments.
    ///
    /// The first element yielded by `args` is treated as the program name (`argv[0]`).
    ///
    /// If parsing fails, an error will be printed and the process will exit with an appropriate
    /// error code.
    fn from_iter<I>(args: I) -> Self
    where
        I: IntoIterator<Item: Into<OsString>>,
    {
        match Self::try_from_iter(args) {
            Ok(v) => v,
            Err(e) => e.report_and_exit(),
        }
    }

    /// Tries to parse an instance of `Self` from the arguments passed to the program
    /// ([`env::args_os`]).
    ///
    /// If parsing fails, an [`Error`] is returned to the caller.
    fn try_from_args() -> Result<Self> {
        Self::try_from_iter(env::args_os())
    }

    /// Tries to parse an instance of `Self` from the given arguments.
    ///
    /// The first element yielded by `args` is treated as the program name (`argv[0]`).
    ///
    /// If parsing fails, an [`Error`] is returned to the caller.
    fn try_from_iter<I>(args: I) -> Result<Self>
    where
        I: IntoIterator<Item: Into<OsString>>,
    {
        let mut args = args.into_iter().map(Into::into);
        let argv0 = PathBuf::from(
            args.next()
                .unwrap_or_else(|| Self::DESC.canonical_name().into()),
        );
        let invocation = match argv0.file_name() {
            Some(basename) => basename.to_os_string(),
            None => argv0.into_os_string(),
        };
        let cx = Context::new(args.collect(), invocation, Self::DESC);
        let mut state = ParseState::default();
        Self::parse(&cx, &mut state)
    }
}

/// A parsing context.
///
/// This is created by Larpa when one of the entry points of the [`Command`] trait is invoked by
/// the user, and updated as arguments are parsed.
///
/// Some built-in types like [`Color`] and [`Verbosity`] will be stored in the [`Context`] when
/// they are encountered on the command line, and can be fetched from user-defined callbacks when
/// desired.
#[derive(Debug, Clone)]
pub struct Context {
    /// The list of input arguments (`argv`).
    chunks: Rc<[OsString]>,
    /// Chain of invoked subcommands, including the original executable.
    ///
    /// Initialized to `argv[0]`, appended to when encountering a subcommand.
    pub(crate) command_chain: OsString,
    pub(crate) desc: CommandDesc,
    pub(crate) root_desc: CommandDesc,
    pub(crate) color: Cell<Color>,
    pub(crate) verbosity: Cell<Verbosity>,
    pub(crate) version_requested: Cell<bool>,
    pub(crate) help_requested: Cell<bool>,
}

impl Context {
    pub(crate) fn new(chunks: Rc<[OsString]>, invocation: OsString, desc: CommandDesc) -> Self {
        Self {
            chunks,
            command_chain: invocation,
            desc: desc.clone(),
            root_desc: desc,
            color: Cell::new(Color::Auto),
            verbosity: Cell::new(Verbosity::ZERO),
            version_requested: Cell::new(false),
            help_requested: Cell::new(false),
        }
    }

    #[cfg(test)]
    pub(crate) fn dummy() -> Self {
        use crate::private::{CommandDescImpl, CommandDescInner};

        Self::new(
            Default::default(),
            OsString::new(),
            CommandDesc(CommandDescImpl(&CommandDescInner {
                canonical_name: "",
                description: None,
                version: None,
                authors: None,
                license: None,
                homepage: None,
                repository: None,
                version_formatter: private::version::default_formatter,
                args: &[],
                subcommand_optional: false,
                subcommands: None,
            })),
        )
    }

    /// Creates a derived context for a subcommand.
    pub(crate) fn for_subcommand_impl(&self, invoked_name: &str, desc: CommandDesc) -> Context {
        let command_chain = OsString::from_iter([
            self.command_chain.as_os_str(),
            OsStr::new(" "),
            OsStr::new(invoked_name),
        ]);
        Context {
            chunks: self.chunks.clone(),
            command_chain,
            desc,
            root_desc: self.root_desc.clone(),
            color: self.color.clone(),
            verbosity: self.verbosity.clone(),
            version_requested: self.version_requested.clone(),
            help_requested: self.help_requested.clone(),
        }
    }

    /// Returns the current [`Color`] preference.
    ///
    /// If an argument is declared with type [`Color`], any time it is encountered on the command
    /// line, this value is set to the value on the command line.
    #[inline]
    pub fn color(&self) -> Color {
        self.color.get()
    }

    /// Returns the current [`Verbosity`] level.
    ///
    /// If a flag is declared with type [`Verbosity`], and time it (or its [`inverse_of`]) is
    /// encountered on the command line, the field value is incremented (or decremented) and stored
    /// in the [`Context`].
    ///
    /// [`inverse_of`]: crate::attrs::field::inverse_of
    #[inline]
    pub fn verbosity(&self) -> Verbosity {
        self.verbosity.get()
    }

    /// Returns the [`CommandDesc`] of the subcommand currently being parsed.
    #[inline]
    pub fn command_desc(&self) -> &CommandDesc {
        &self.desc
    }
}
