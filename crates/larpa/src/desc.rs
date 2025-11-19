//! Introspectable command description.
//!
//! This module contains [`CommandDesc`], which is automatically generated for every
//! [`Command`][crate::Command] and allows generating `--help` output, a command usage string,
//! man pages, shell completions, etc. by simply inspecting its contents.

use std::{
    ffi::{OsStr, OsString},
    fmt,
    path::{Path, PathBuf},
};

use crate::private::{ArgumentDescImpl, ArgumentNameImpl, CommandDescImpl, CommandDescInner};

/// Introspectable command description.
#[derive(Debug, Clone)]
pub struct CommandDesc(pub(crate) CommandDescImpl);

impl CommandDesc {
    fn inner(&self) -> &'static CommandDescInner {
        self.0.0
    }

    /// Returns the canonical name of the command.
    ///
    /// This is derived from the name of the crate that defines the command.
    /// The *invoked name* of a command is `argv[0]`, which is typically the name of the executable,
    /// and may differ from the canonical name.
    #[inline]
    pub fn canonical_name(&self) -> &str {
        self.inner().canonical_name
    }

    #[inline]
    pub fn description(&self) -> Option<&str> {
        self.inner().description
    }

    /// Returns the version string of the command.
    ///
    /// This defaults to the Cargo package version (via `CARGO_PKG_VERSION`), but can be customized
    /// via [`#[larpa(version = "custom-version")]`][version].
    ///
    /// Returns [`None`] when no version was specified manually and `CARGO_PKG_VERSION` was not set
    /// (likely because the package wasn't built by Cargo).
    ///
    /// [version]: crate::attrs::top_level::version
    #[inline]
    pub fn version(&self) -> Option<&str> {
        self.inner().version
    }

    #[inline]
    pub fn authors(&self) -> Option<&str> {
        self.inner().authors
    }

    /// Returns the program license identifier.
    ///
    /// By default, this is taken from the package's `Cargo.toml`, but it can be overridden with
    /// [`#[larpa(license)]`][license].
    ///
    /// [license]: crate::attrs::top_level::license
    #[inline]
    pub fn license(&self) -> Option<&str> {
        self.inner().license
    }

    #[inline]
    pub fn homepage(&self) -> Option<&str> {
        self.inner().homepage
    }

    #[inline]
    pub fn repository(&self) -> Option<&str> {
        self.inner().repository
    }

    #[inline]
    pub fn args(&self) -> &[ArgumentDesc] {
        self.inner().args
    }

    #[inline]
    pub fn is_subcommand_optional(&self) -> bool {
        self.inner().subcommand_optional
    }

    /// Returns `true` if there may be invokable subcommands beyond the ones in
    /// [`CommandDesc::subcommands`].
    ///
    /// This is `true` when the command is an `enum` with a variant annotated with
    /// [`#[larpa(fallback)]`][fallback].
    ///
    /// [fallback]: crate::attrs::variant::fallback
    #[inline]
    pub fn has_subcommand_fallback(&self) -> bool {
        self.inner()
            .subcommands
            .as_ref()
            .is_some_and(|d| d.has_fallback)
    }

    /// Invokes the dynamic subcommand discovery function.
    ///
    /// For `enum`s with a [`#[larpa(fallback)]`][fallback] variant, a dynamic subcommand discovery
    /// function may be provided by the user that finds additional applicable subcommands at
    /// runtime.
    ///
    /// If [`#[larpa(discover)]`][discover] was not used, this will return an empty [`Vec`].
    ///
    /// [fallback]: crate::attrs::variant::fallback
    /// [discover]: crate::attrs::variant::discover
    pub fn discover_subcommands(&self) -> Vec<DiscoveredSubcommand> {
        match self
            .inner()
            .subcommands
            .as_ref()
            .and_then(|d| d.discover_subcommands)
        {
            Some(desc) => desc(self),
            None => Vec::new(),
        }
    }

    /// Returns the list of built-in subcommands, if this command has any.
    #[inline]
    pub fn subcommands(&self) -> &[SubcommandDesc] {
        match self.inner().subcommands.as_ref() {
            Some(desc) => desc.subcommands,
            None => &[],
        }
    }
}

/// Description of a built-in subcommand.
///
/// Returned by [`CommandDesc::subcommands`].
#[derive(Debug)]
pub struct SubcommandDesc(pub(crate) CommandDesc);

impl SubcommandDesc {
    #[inline]
    pub fn canonical_name(&self) -> &str {
        self.0.canonical_name()
    }

    #[inline]
    pub fn description(&self) -> Option<&str> {
        self.0.description()
    }

    #[inline]
    pub fn args(&self) -> &[ArgumentDesc] {
        self.0.args()
    }

    #[inline]
    pub fn subcommands(&self) -> &[SubcommandDesc] {
        self.0.subcommands()
    }
}

/// A dynamically discovered subcommand (for example, from searching `$PATH`).
///
/// This type can be returned by a user-defined external subcommand discovery function.
/// For more information on how to set this up, please refer to the documentation for the
/// [`#[larpa(discover)]`][discover] attribute.
///
/// [discover]: crate::attrs::variant::discover
#[derive(Debug)]
pub struct DiscoveredSubcommand {
    name: OsString,
    description: Option<String>,
    path: Option<PathBuf>,
}

impl DiscoveredSubcommand {
    /// Creates a [`DiscoveredSubcommand`] with the given subcommand name.
    ///
    /// It is assumed that `$cmd $name` (where `$cmd` is the canonical name of the command,
    /// and `$name` is the `name` argument of this function) will invoke this subcommand.
    pub fn new(name: impl Into<OsString>) -> Self {
        Self {
            name: name.into(),
            description: None,
            path: None,
        }
    }

    /// Sets the description of this subcommand.
    pub fn with_description(mut self, desc: impl Into<String>) -> Self {
        self.description = Some(desc.into());
        self
    }

    /// Sets the path to the external subcommand.
    pub fn with_path(mut self, path: impl Into<PathBuf>) -> Self {
        self.path = Some(path.into());
        self
    }

    #[inline]
    pub fn name(&self) -> &OsStr {
        &self.name
    }

    #[inline]
    pub fn description(&self) -> Option<&str> {
        self.description.as_deref()
    }

    #[inline]
    pub fn path(&self) -> Option<&Path> {
        self.path.as_deref()
    }
}

/// Description of a command-line argument.
///
/// Returned by [`CommandDesc::args`] and [`SubcommandDesc::args`].
#[derive(Debug)]
pub struct ArgumentDesc(pub(crate) ArgumentDescImpl);

impl ArgumentDesc {
    /// Returns whether the argument is positional.
    ///
    /// Positional arguments have neither a short nor a long flag associated with them, and are
    /// matched purely by their position in the command line.
    #[inline]
    pub fn is_positional(&self) -> bool {
        self.short().is_none() && self.long().is_none()
    }

    /// Returns whether the argument is optional.
    #[inline]
    pub fn is_optional(&self) -> bool {
        self.0.optional
    }

    /// Returns whether the argument is required.
    ///
    /// This is the inverse of [`is_optional`][Self::is_optional].
    ///
    /// If a required argument is not provided, parsing will fail.
    #[inline]
    pub fn is_required(&self) -> bool {
        !self.is_optional()
    }

    /// Returns whether the argument can be provided multiple times.
    #[inline]
    pub fn is_repeating(&self) -> bool {
        self.0.repeating
    }

    /// Returns whether this argument takes a value.
    ///
    /// If `false`, the argument is a flag that only cares about whether (or how many times) it is
    /// present.
    #[inline]
    pub fn takes_value(&self) -> bool {
        self.value_name().is_some()
    }

    #[inline]
    pub fn value_name(&self) -> Option<&str> {
        self.0.name.0.value_name
    }

    /// Returns the description of this argument.
    #[inline]
    pub fn description(&self) -> Option<&str> {
        self.0.description
    }

    /// Returns the custom default value (if any).
    ///
    /// Note that this will only return [`Some`] when using
    /// [`#[larpa(default = "custom value")]`][dfl].
    /// Using [`#[larpa(default)]`][dfl] alone will use the field type's [`Default`] implementation
    /// and this method will return [`None`].
    ///
    /// [dfl]: crate::attrs::field::default
    #[inline]
    pub fn custom_default(&self) -> Option<&str> {
        self.0.custom_default
    }

    /// Returns the argument's short form (eg. `-s`), without the leading `-`.
    #[inline]
    pub fn short(&self) -> Option<char> {
        self.0.name.0.short
    }

    /// Returns the argument's long form (eg. `--long`), without the leading `--`.
    #[inline]
    pub fn long(&self) -> Option<&str> {
        self.0.name.0.long
    }

    pub fn name(&self) -> &ArgumentName {
        &self.0.name
    }
}

/// A [`Display`][fmt::Display]able name of a command-line argument.
#[derive(Debug)]
pub struct ArgumentName(pub(crate) ArgumentNameImpl);

impl fmt::Display for ArgumentName {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match (self.0.short, self.0.long) {
            (Some(short), None) => write!(f, "-{short}"),
            (None, Some(long)) => write!(f, "--{long}"),
            (Some(short), Some(long)) => write!(f, "-{short}/--{long}"),
            (None, None) => f.write_str(self.0.value_name.unwrap()),
        }
    }
}
