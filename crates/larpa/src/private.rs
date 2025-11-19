//! Internal functionality used by the derive macro.
//!
//! Not part of the public API. Semver-exempt.

pub mod attrs;
pub mod discover;
mod snap;
pub mod version;

use crate::value::BoxedError;

use std::ffi::OsString;

use crate::desc::{ArgumentName, DiscoveredSubcommand};

pub use crate::desc::CommandDesc;
pub use crate::error::ErrorKind;
pub use crate::flag::{Flag, FlagInternal, InvertibleFlag};
pub use crate::parser::{ParseState, RawArg};
pub use crate::value::Converter;
pub use crate::{
    Command, Context, Error,
    desc::{ArgumentDesc, SubcommandDesc},
};

pub use snap::snap;

pub use core::default::Default;
pub use core::option::Option::{self, None, Some};
pub use core::result::Result::{Err, Ok};
pub use core::str::FromStr;
pub use std::ffi::OsStr;
pub use std::vec::Vec;

pub mod prelude {
    pub use super::ContextExt as _;
    pub use crate::value::{
        ViaArgValue as _, ViaFromStr as _, ViaTryFromOsStr as _, ViaTryFromStr as _,
    };
}

pub type Result<T, E = Error> = std::result::Result<T, E>;

/// Methods on [`Context`] that are used by macro-generated code.
pub trait ContextExt {
    fn for_subcommand(&self, invoked_name: &str, desc: CommandDesc) -> Context;
    fn err(&self, kind: ErrorKind) -> Error;
    fn chunks(&self) -> &[OsString];
    fn version_requested(&self) -> bool;
    fn help_requested(&self) -> bool;

    fn value(&self, p: &mut ParseState, argidx: usize) -> Result<&OsStr> {
        match p.peek_value(self.chunks()) {
            Some((val, state)) => {
                *p = state;
                Ok(val)
            }
            None => Err(self.err(ErrorKind::MissingArgValue(argidx))),
        }
    }
    fn no_value(&self, p: &ParseState, argidx: usize) -> Result<()> {
        if p.after_eq() {
            return Err(self.err(ErrorKind::UnexpectedArgValue(argidx)));
        }
        Ok(())
    }
    fn next(&self, p: &mut ParseState) -> Result<RawArg<'_>> {
        match p.peek_next(self.chunks()) {
            Ok((arg, state)) => {
                *p = state;
                Ok(arg)
            }
            Err(e) => Err(self.err(e)),
        }
    }
    fn peek_next(&self, p: &ParseState) -> Result<RawArg<'_>> {
        match p.peek_next(self.chunks()) {
            Ok((arg, _)) => Ok(arg),
            Err(e) => Err(self.err(e)),
        }
    }
    fn drain(&self, cmd: &OsStr, p: &mut ParseState) -> Vec<OsString> {
        let chunks = &self.chunks()[p.chunk_index()..];
        let mut out = Vec::with_capacity(chunks.len() + 1);
        out.push(cmd.to_os_string());
        out.extend_from_slice(chunks);
        out
    }
    fn err_missing_subcommand(&self) -> Error {
        self.err(ErrorKind::MissingSubcommand)
    }
    fn err_unexpected(&self, arg: RawArg<'_>) -> Error {
        self.err(ErrorKind::UnexpectedArg {
            arg: arg.to_string(),
        })
    }
    fn err_missing_arg(&self, index: usize) -> Error {
        self.err(ErrorKind::MissingArg(index))
    }
    fn err_unknown_subcommand(&self, cmd: &OsStr) -> Error {
        self.err(ErrorKind::UnknownSubcommand(cmd.to_os_string()))
    }
    fn err_value(&self, e: BoxedError, argidx: usize) -> Error {
        self.err(ErrorKind::ValueParseError {
            error: e,
            arg: argidx,
        })
    }
    fn version_or_help_request(&self) -> Result<()> {
        if self.version_requested() {
            return Err(self.err(ErrorKind::VersionRequested));
        }
        if self.help_requested() {
            return Err(self.err(ErrorKind::HelpRequested));
        }

        Ok(())
    }
    fn try_insert<S: Slot<T>, T>(&self, slot: &mut S, value: T, arg: usize) -> Result<()> {
        if slot.try_insert(value).is_none() {
            return Err(self.err(ErrorKind::DuplicateArg(arg)));
        }

        Ok(())
    }
}

impl ContextExt for Context {
    fn for_subcommand(&self, invoked_name: &str, desc: CommandDesc) -> Context {
        self.for_subcommand_impl(invoked_name, desc)
    }

    fn err(&self, kind: ErrorKind) -> Error {
        Error {
            kind,
            context: self.clone(),
        }
    }

    fn chunks(&self) -> &[OsString] {
        &self.chunks
    }

    fn version_requested(&self) -> bool {
        self.version_requested.get()
    }

    fn help_requested(&self) -> bool {
        self.help_requested.get()
    }
}

pub trait Slot<T> {
    fn try_insert(&mut self, value: T) -> Option<()>;
}
impl<T> Slot<T> for Option<T> {
    fn try_insert(&mut self, value: T) -> Option<()> {
        if self.is_some() {
            return None;
        }

        *self = Some(value);
        Some(())
    }
}
impl<T> Slot<T> for Vec<T> {
    fn try_insert(&mut self, value: T) -> Option<()> {
        self.push(value);
        Some(())
    }
}

pub trait CommandInternal: Sized {
    fn parse(cx: &Context, p: &mut ParseState) -> Result<Self>;
}

#[diagnostic::on_unimplemented(
    message = "`{Self}` is not an `enum` of subcommands",
    label = "types used with `#[larpa(subcommand)]` must be `enum`s that derive `Command`"
)]
pub trait Subcommands: Sized {
    const ENUM_DESC: SubcommandEnumDesc;

    /// Parses the subcommand `cmd`.
    fn dispatch(cmd: &OsStr, cx: &Context, p: &mut ParseState) -> Result<Self>;
}

#[derive(Debug, Clone)]
pub struct CommandDescImpl(pub &'static CommandDescInner);

#[derive(Debug)]
pub struct CommandDescInner {
    pub canonical_name: &'static str,
    pub description: Option<&'static str>,
    pub version: Option<&'static str>,
    pub authors: Option<&'static str>,
    pub license: Option<&'static str>,
    pub homepage: Option<&'static str>,
    pub repository: Option<&'static str>,
    pub version_formatter: fn(&Context) -> String,
    pub args: &'static [ArgumentDesc],
    pub subcommand_optional: bool,
    pub subcommands: Option<&'static SubcommandEnumDesc>,
}

#[derive(Debug)]
pub struct SubcommandEnumDesc {
    pub has_fallback: bool,
    pub discover_subcommands: Option<fn(&CommandDesc) -> Vec<DiscoveredSubcommand>>,
    pub subcommands: &'static [SubcommandDesc],
}

#[derive(Debug)]
pub struct ArgumentDescImpl {
    pub description: Option<&'static str>,
    pub name: ArgumentName,
    pub custom_default: Option<&'static str>,
    pub optional: bool,
    pub repeating: bool,
}

#[derive(Debug)]
pub struct ArgumentNameImpl {
    pub short: Option<char>,
    pub long: Option<&'static str>,
    /// `None` for flags (which are required to have either a short or long form).
    pub value_name: Option<&'static str>,
}

pub const fn command_desc(imp: CommandDescImpl) -> CommandDesc {
    CommandDesc(imp)
}
pub const fn command_desc_impl(desc: CommandDesc) -> CommandDescImpl {
    desc.0
}
pub const fn subcommand_desc(imp: CommandDescImpl) -> SubcommandDesc {
    SubcommandDesc(CommandDesc(imp))
}
pub const fn argument_desc(imp: ArgumentDescImpl) -> ArgumentDesc {
    ArgumentDesc(imp)
}
pub const fn argument_name(imp: ArgumentNameImpl) -> ArgumentName {
    ArgumentName(imp)
}
