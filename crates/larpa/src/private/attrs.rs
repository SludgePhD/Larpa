//! Documentation for attributes accepted by Larpa (only available on docs.rs).
//!
//! Different `#[larpa(…)]` attributes are accepted on different parts of the `#[derive(Command)]`
//! input.
//!
//! - [`top_level`] contains attributes that are valid on the top-level type.
//! - [`variant`] contains attributes that are valid on `enum` variants (which larpa treats as
//!   different subcommands).
//! - [`field`] contains attributes that are valid on fields of a `struct` or a `struct` variant.

#![allow(nonstandard_style)]

/// Attributes that are valid on the top-level type (`struct` or `enum`).
pub mod top_level {
    /// **`#[larpa(name = "my-command")]`** sets a custom default name for a command.
    ///
    /// If this attribute is not present, the canonical command name is taken from `CARGO_BIN_NAME`
    /// or (when not building a binary) from `CARGO_PKG_NAME`.
    ///
    /// At runtime, the actual command name is taken from the value of `argv[0]`.
    pub const name: () = ();

    /// **`#[larpa(crate = "::larpa_crate")]`** sets the path to the `larpa` runtime crate.
    ///
    /// This can be used when the crate has been renamed to something else in `Cargo.toml`.
    /// The default crate path is `::larpa`, which is correct for typical usage.
    pub const сrate: () = ();
    // (yes, that's a confusable and not the letter "c")

    /// **`#[larpa(no_generate_tests)]`** disables automatic unit test generation.
    ///
    /// Certain features cause the derive macro to output not just an implementation of [`Command`],
    /// but also a `#[test]` function that will ensure that any provided string values successfully
    /// parse into the target type.
    ///
    /// This attribute can be used to disable the generation of tests, in case this behavior is
    /// unwanted, or in case the derive macro is used in a context that does not allow unit tests.
    ///
    /// [`Command`]: crate::Command
    ///
    /// # Examples
    ///
    /// ```
    /// use larpa::Command;
    ///
    /// #[derive(Debug, PartialEq, Command)]
    /// #[larpa(no_generate_tests)]
    /// struct MyCmd {
    ///     #[larpa(default = "123")]
    ///     value: u8,
    /// }
    /// ```
    pub const no_generate_tests: () = ();

    /// **`#[larpa(version = "program version")]`** sets the program's version string.
    ///
    /// By default, the value of `CARGO_PKG_VERSION` is used for the version string.
    ///
    /// The argument of `version =` must be a `const` expression of type `&'static str`.
    ///
    /// # Examples
    ///
    /// ```
    /// use larpa::Command;
    ///
    /// #[derive(Command)]
    /// #[larpa(version = concat!("1.2.3", "-dirty"))]
    /// struct MyCmd {}
    ///
    /// assert_eq!(MyCmd::DESC.version(), Some("1.2.3-dirty"));
    /// ```
    pub const version: () = ();

    /// **`#[larpa(version_formatter = "custom_fmt")]`** sets a custom version formatter.
    ///
    /// This can be used for verbose `--version` output that includes the versions of important
    /// native libraries, or the commit hash.
    ///
    /// Note that the version formatter is different from the version string: The version string
    /// is intended to just be a short string that uniquely identifies a release of the program,
    /// while the job of the formatter is to display the `--version` output to the user (typically
    /// making use of the version string).
    ///
    /// The version *string* may also be used to generate a man page or external completion script,
    /// while the version *formatter* is typically only used to print the `--version` output on the
    /// command line.
    ///
    /// `"custom_fmt"` must be the path to a function with the following signature:
    ///
    /// ```
    /// fn custom_fmt(cx: &larpa::Context) -> String {
    ///     // ...
    /// #   String::new()
    /// }
    /// ```
    ///
    /// If this attribute is omitted, the default formatter is used, which will simply output the
    /// command name and version string.
    ///
    /// # Example
    ///
    /// Include additional version information when `--verbose` is passed:
    ///
    /// ```
    /// use larpa::{Context, Command};
    /// use larpa::types::{Verbosity, PrintVersion};
    ///
    /// #[derive(Debug, Command)]
    /// #[larpa(version_formatter = "version")]
    /// struct MyCmd {
    ///     #[larpa(flag, name = ["-V", "--version"])]
    ///     version: PrintVersion,
    ///
    ///     #[larpa(flag, name = ["-v", "--verbose"])]
    ///     verbosity: Verbosity,
    /// }
    ///
    /// fn version(cx: &Context) -> String {
    ///     if cx.verbosity() > 0 {
    ///         "verbose version".into()
    ///     } else {
    ///         "normal version".into()
    ///     }
    /// }
    ///
    /// let version = MyCmd::try_from_iter(["my-cmd", "--version"]).unwrap_err().to_string();
    /// assert_eq!(version, "normal version\n");
    ///
    /// let version = MyCmd::try_from_iter(["my-cmd", "-Vv"]).unwrap_err().to_string();
    /// assert_eq!(version, "verbose version\n");
    /// ```
    pub const version_formatter: () = ();

    /// **`#[larpa(license = "License String")]`** sets a custom license string.
    ///
    /// By default (if this attribute is omitted), the license specified in the Cargo.toml is used.
    /// To clear the license information, even if Cargo.toml does specify a license, use
    /// [`#[larpa(no_license)]`][no_license].
    ///
    /// The license string is conventionally an SPDX license identifier, but can be set to any
    /// string.
    ///
    /// At runtime, the license string can be retrieved via [`CommandDesc::license`].
    ///
    /// [`CommandDesc::license`]: crate::desc::CommandDesc::license
    pub const license: () = ();

    /// **`#[larpa(no_license)]`** clears the license string.
    ///
    /// By default (if this attribute is omitted), the license specified in the Cargo.toml is used.
    /// This attribute can be used to override that behavior and include *no* license string
    /// instead.
    pub const no_license: () = ();

    /// **`#[larpa(homepage = "https://example.com")]`** sets a custom homepage string.
    ///
    /// By default (if this attribute is omitted), the homepage specified in the Cargo.toml is used.
    /// To clear the homepage information, even if Cargo.toml does specify a homepage, use
    /// [`#[larpa(no_homepage)]`][no_homepage].
    ///
    /// The homepage string is conventionally a URI, but can be set to any string.
    ///
    /// At runtime, the homepage can be retrieved via [`CommandDesc::homepage`].
    ///
    /// [`CommandDesc::homepage`]: crate::desc::CommandDesc::homepage
    pub const homepage: () = ();

    /// **`#[larpa(no_homepage)]`** clears the homepage string.
    ///
    /// By default (if this attribute is omitted), the homepage specified in the Cargo.toml is used.
    /// This attribute can be used to override that behavior and include *no* homepage string
    /// instead.
    pub const no_homepage: () = ();

    /// **`#[larpa(repository = "https://example.com")]`** sets a custom repository string.
    ///
    /// By default (if this attribute is omitted), the repository specified in the Cargo.toml is used.
    /// To clear the repository information, even if Cargo.toml does specify a repository, use
    /// [`#[larpa(no_repository)]`][no_repository].
    ///
    /// The repository string is conventionally a URI, but can be set to any string.
    ///
    /// At runtime, the repository string can be retrieved via [`CommandDesc::repository`].
    ///
    /// [`CommandDesc::repository`]: crate::desc::CommandDesc::repository
    pub const repository: () = ();

    /// **`#[larpa(no_repository)]`** clears the repository string.
    ///
    /// By default (if this attribute is omitted), the repository specified in the Cargo.toml is used.
    /// This attribute can be used to override that behavior and include *no* repository string
    /// instead.
    pub const no_repository: () = ();
}

/// Attributes that are valid on `enum` variants (subcommands).
pub mod variant {
    /// **`#[larpa(name = "my-subcommand")]`** sets a custom name for a subcommand.
    ///
    /// When absent, the subcommand name is derived from the name of the variant by converting its
    /// name to `kebab-case`.
    pub const name: () = ();

    /// **`#[larpa(fallback)]`** marks a variant as the fallback subcommand.
    ///
    /// The fallback variant will be produced by the parser when no other subcommand matches.
    /// It must be a tuple variant that contains a `Vec<OsString>`.
    ///
    /// This can be used to implement external subcommands.
    ///
    /// # Examples
    ///
    /// ```
    /// use larpa::Command;
    /// use std::ffi::OsString;
    ///
    /// #[derive(Debug, PartialEq, Command)]
    /// enum MyCmd {
    ///     Subcommand,
    ///     #[larpa(fallback)]
    ///     Fallback(Vec<OsString>),
    /// }
    ///
    /// assert_eq!(MyCmd::from_iter(["my-cmd", "subcommand"]), MyCmd::Subcommand);
    /// assert_eq!(
    ///     MyCmd::from_iter(["my-cmd", "bla", "bla"]),
    ///     MyCmd::Fallback(vec!["bla".into(); 2]),
    /// );
    /// ```
    pub const fallback: () = ();

    /// **`#[larpa(discover = "…")]`** enables dynamic discovery of subcommands.
    ///
    /// This must be placed on the same variant as [`#[larpa(fallback)]`](fallback).
    ///
    /// The optional string argument specifies a path to a discovery function with the following
    /// signature:
    ///
    /// ```
    /// use larpa::desc::{CommandDesc, DiscoveredSubcommand};
    ///
    /// fn discover_subcommands(desc: &CommandDesc) -> Vec<DiscoveredSubcommand> {
    ///     // ...
    /// #   Vec::new()
    /// }
    /// ```
    ///
    /// If the argument is omitted, the default subcommand discovery function is used.
    /// It will search all directories in `$PATH` for executable files that start with `name-`,
    /// where `name` is [`CommandDesc::canonical_name`].
    ///
    /// The discovered subcommands will be displayed in the generated help and usage output.
    /// They do not affect which subcommands will be accepted by Larpa, so a non-existent external
    /// subcommand will still end up in the [`#[larpa(fallback)]`](fallback) variant (and then fail
    /// to execute).
    ///
    /// [`CommandDesc::canonical_name`]: crate::desc::CommandDesc::canonical_name
    ///
    /// # Examples
    ///
    /// Using the built-in discovery function:
    ///
    /// ```
    /// use larpa::Command;
    /// use std::ffi::OsString;
    ///
    /// #[derive(Debug, PartialEq, Command)]
    /// enum MyCmd {
    ///     Subcommand,
    ///     #[larpa(fallback, discover)]
    ///     Fallback(Vec<OsString>),
    /// }
    /// ```
    ///
    /// Using a custom discovery function:
    ///
    /// ```
    /// use larpa::Command;
    /// use larpa::desc::{CommandDesc, DiscoveredSubcommand};
    /// use std::ffi::OsString;
    ///
    /// #[derive(Debug, PartialEq, Command)]
    /// #[larpa(name = "cmd")]
    /// enum MyCmd {
    ///     Subcommand,
    ///     #[larpa(fallback, discover = "my_discoverer")]
    ///     Fallback(Vec<OsString>),
    /// }
    ///
    /// fn my_discoverer(desc: &CommandDesc) -> Vec<DiscoveredSubcommand> {
    ///     vec![
    ///         DiscoveredSubcommand::new("discovered1"),
    ///         DiscoveredSubcommand::new("discovered2"),
    ///     ]
    /// }
    ///
    /// assert_eq!(
    ///     MyCmd::DESC.usage().to_string(),
    ///     "cmd subcommand|discovered1|discovered2|...",
    /// );
    /// ```
    pub const discover: () = ();
}

/// Attributes that are valid on `struct` fields and fields of `enum` variants.
pub mod field {
    /// **`#[larpa(name = ["-s", "--long"])]`** sets the name(s) of an argument.
    ///
    /// When this attribute is absent, the argument is a *positional argument* instead of a
    /// *named argument*.
    ///
    /// The `name` attribute accepts either a single string, or an array of up to 2 strings.
    /// Only one short and one long argument name is permitted.
    ///
    /// # Examples
    ///
    /// ```
    /// use larpa::Command;
    ///
    /// #[derive(Debug, PartialEq, Command)]
    /// struct MyCmd {
    ///     #[larpa(name = ["--first-name", "-F"])]
    ///     first_name: String,
    ///     #[larpa(name = ["--last-name", "-L"])]
    ///     last_name: String,
    /// }
    ///
    /// assert_eq!(
    ///     MyCmd::from_iter(["my-cmd", "-FJohn", "--last-name", "Doe"]),
    ///     MyCmd { first_name: "John".into(), last_name: "Doe".into() },
    /// );
    /// ```
    pub const name: () = ();

    /// **`#[larpa(default = "string value")]`** sets the default value of an argument.
    ///
    /// If `"string value"` is omitted, the field type's [`Default`] implementation is used.
    ///
    /// If `"string value"` is provided, it will be parsed into the target type using [`FromStr`].
    /// In that case, the derive macro will also emit a `#[test]` function that will test that the
    /// provided string successfully parses into the target type, unless
    /// [`#[larpa(no_generate_tests)`][no_generate_tests] is used.
    ///
    /// [`FromStr`]: std::str::FromStr
    /// [no_generate_tests]: super::top_level::no_generate_tests
    ///
    /// # Examples
    ///
    /// ```
    /// use larpa::Command;
    ///
    /// #[derive(Debug, PartialEq, Command)]
    /// struct MyCmd {
    ///     #[larpa(default)]
    ///     val1: u32,
    ///     #[larpa(default = "123")]
    ///     val2: u32,
    /// }
    ///
    /// assert_eq!(MyCmd::from_iter(["my-cmd"]), MyCmd { val1: 0, val2: 123 });
    /// assert_eq!(MyCmd::from_iter(["my-cmd", "987"]), MyCmd { val1: 987, val2: 123 });
    /// assert_eq!(MyCmd::from_iter(["my-cmd", "1", "2"]), MyCmd { val1: 1, val2: 2 });
    /// ```
    pub const default: () = ();

    /// **`#[larpa(flag)]`** marks a field as a "flag" option that does not take a value.
    ///
    /// The type of the field must implement the [`Flag`] trait.
    /// Multiple flags with "short" names can be specified together:
    /// `-vdvadv` specifies the `-v` flag 3 times, `-d` twice, and `-a` once.
    ///
    /// **Must** be combined with [`#[larpa(name)]`][name] (flags cannot be positional).
    ///
    /// [`Flag`]: crate::Flag
    ///
    /// # Examples
    ///
    /// ```
    /// use larpa::Command;
    ///
    /// #[derive(Debug, PartialEq, Command)]
    /// struct MyCmd {
    ///     #[larpa(name = ["--verbose", "-v"], flag)]
    ///     verbosity: usize,
    ///
    ///     #[larpa(name = ["--skip"], flag)]
    ///     skip: bool,
    /// }
    ///
    /// assert_eq!(MyCmd::from_iter(["my-cmd"]), MyCmd { verbosity: 0, skip: false });
    /// assert_eq!(MyCmd::from_iter(["my-cmd", "--skip"]), MyCmd { verbosity: 0, skip: true });
    /// assert_eq!(MyCmd::from_iter(["my-cmd", "-v"]), MyCmd { verbosity: 1, skip: false });
    /// assert_eq!(MyCmd::from_iter(["my-cmd", "-vvv"]), MyCmd { verbosity: 3, skip: false });
    /// assert_eq!(MyCmd::from_iter(["my-cmd", "-vvv", "--skip"]), MyCmd { verbosity: 3, skip: true });
    /// ```
    pub const flag: () = ();

    /// **`#[larpa(inverse_of = "field")]`** marks a flag as an inverse of another flag.
    ///
    /// This attribute *must* be combined with [`#[larpa(flag)]`][flag], and the type of the field
    /// this attribute is used on must be `()`.
    ///
    /// The `"field"` argument must be the name of a neighboring field in the same struct
    /// (or variant), which also has to use [`#[larpa(flag)]`][flag].
    ///
    /// When this flag is encountered in the command-line arguments, the target flag will be
    /// decremented.
    ///
    /// # Examples
    ///
    /// Combine a `--verbose` and a `--quiet` flag:
    ///
    /// ```
    /// use larpa::Command;
    ///
    /// #[derive(Debug, PartialEq, Command)]
    /// struct MyCmd {
    ///     #[larpa(name = ["--verbose", "-v"], flag)]
    ///     verbosity: i8,
    ///
    ///     #[larpa(name = ["--quiet", "-q"], flag, inverse_of = "verbosity")]
    ///     quiet: (),
    /// }
    ///
    /// assert_eq!(MyCmd::from_iter(["my-cmd"]), MyCmd { verbosity: 0, quiet: () });
    /// assert_eq!(MyCmd::from_iter(["my-cmd", "-v"]), MyCmd { verbosity: 1, quiet: () });
    /// assert_eq!(MyCmd::from_iter(["my-cmd", "-q"]), MyCmd { verbosity: -1, quiet: () });
    /// assert_eq!(MyCmd::from_iter(["my-cmd", "-vvv"]), MyCmd { verbosity: 3, quiet: () });
    /// assert_eq!(MyCmd::from_iter(["my-cmd", "-vqv"]), MyCmd { verbosity: 1, quiet: () });
    /// assert_eq!(MyCmd::from_iter(["my-cmd", "-qqqv"]), MyCmd { verbosity: -2, quiet: () });
    /// ```
    ///
    /// Combine a `--pager` and `--no-pager` flag, and also detect absence of both (for automatic
    /// selection):
    ///
    /// ```
    /// use larpa::Command;
    ///
    /// #[derive(Debug, PartialEq, Command)]
    /// struct MyCmd {
    ///     #[larpa(name = ["--pager"], flag)]
    ///     pager: Option<bool>,
    ///
    ///     #[larpa(name = ["--no-pager"], flag, inverse_of = "pager")]
    ///     no_pager: (),
    /// }
    ///
    /// assert_eq!(MyCmd::from_iter(["my-cmd"]), MyCmd { pager: None, no_pager: () });
    /// assert_eq!(MyCmd::from_iter(["my-cmd", "--pager"]), MyCmd { pager: Some(true), no_pager: () });
    /// assert_eq!(MyCmd::from_iter(["my-cmd", "--no-pager"]), MyCmd { pager: Some(false), no_pager: () });
    /// assert_eq!(MyCmd::from_iter(["my-cmd", "--pager", "--no-pager"]), MyCmd { pager: Some(false), no_pager: () });
    /// assert_eq!(MyCmd::from_iter(["my-cmd", "--no-pager", "--pager"]), MyCmd { pager: Some(true), no_pager: () });
    /// ```
    pub const inverse_of: () = ();

    /// **`#[larpa(subcommand)]`** marks a field as a nested subcommand.
    ///
    /// The field type has to implement [`Command`], and it has to be an `enum`.
    ///
    /// The field can we wrapped in [`Option`] to indicate an optional subcommand.
    ///
    /// Cannot be combined with any other attribute.
    ///
    /// [`Command`]: crate::Command
    ///
    /// # Examples
    ///
    /// ```
    /// use larpa::Command;
    ///
    /// #[derive(Debug, PartialEq, Command)]
    /// struct MyCmd {
    ///     #[larpa(subcommand)]
    ///     subcommand: Subcommand,
    /// }
    ///
    /// #[derive(Debug, PartialEq, Command)]
    /// enum Subcommand {
    ///     Print,
    ///     Dec,
    ///     Inc,
    /// }
    ///
    /// assert!(MyCmd::try_from_iter(["my-cmd"]).is_err()); // Subcommand is required.
    /// assert_eq!(MyCmd::from_iter(["my-cmd", "print"]), MyCmd { subcommand: Subcommand::Print });
    /// assert_eq!(MyCmd::from_iter(["my-cmd", "dec"]), MyCmd { subcommand: Subcommand::Dec });
    /// ```
    pub const subcommand: () = ();

    /// **`#[larpa(required)]`** marks a repeated argument as required.
    ///
    /// A required repeated argument **must** be specified at least once, and may be specified any
    /// number of times.
    ///
    /// Only accepted on fields with type [`Vec<T>`].
    ///
    /// # Examples
    ///
    /// ```
    /// use larpa::Command;
    /// use std::path::PathBuf;
    ///
    /// #[derive(Debug, PartialEq, Command)]
    /// struct MyCmd {
    ///     #[larpa(required)]
    ///     files: Vec<PathBuf>,
    /// }
    ///
    /// assert!(MyCmd::try_from_iter(["my-cmd"]).is_err()); // At least one argument is required.
    /// assert_eq!(MyCmd::from_iter(["my-cmd", "file1"]), MyCmd { files: vec!["file1".into()] });
    /// assert_eq!(
    ///     MyCmd::from_iter(["my-cmd", "file1", "file2"]),
    ///     MyCmd { files: vec!["file1".into(), "file2".into()] },
    /// );
    /// ```
    pub const required: () = ();
}
