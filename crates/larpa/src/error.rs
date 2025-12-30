use std::{
    ffi::OsString,
    fmt,
    io::{self, IsTerminal, Write, sink, stderr, stdout},
    process,
    str::Utf8Error,
};

use crate::{
    Context,
    desc::{ArgumentDesc, ArgumentName},
    types::Color,
    writer::{BOLD, DEFAULT_LINE_WIDTH, ITALIC, RED, RESET, Writer, YELLOW},
};

/// Indicates that a [`Command`] could not be parsed.
///
/// This error is returned from the methods in [`Command`] when the implementing type could not be
/// parsed from the command-line arguments.
/// This can happen when the user passes `--help` or `--version` (requesting a help or version
/// output instead of normal program operation), or when the provided arguments are invalid.
///
/// [`Command`]: crate::Command
#[derive(Debug)]
pub struct Error {
    pub(crate) kind: ErrorKind,
    pub(crate) context: Context,
}

impl Error {
    /// Reports the error to stdout or stderr, and exits the process with an appropriate status.
    pub fn report_and_exit(&self) -> ! {
        let mut reporter = self.reporter();
        if self.is_fatal() {
            reporter = reporter.output(stderr());
        } else {
            reporter = reporter.output(stdout());
        };
        reporter.report_and_exit()
    }

    /// Returns a [`Reporter`] that can write this error to a terminal and allows customization.
    pub fn reporter(&self) -> Reporter<'_> {
        Reporter {
            error: self,
            color: self.context.color(),
            wrap_width: DEFAULT_LINE_WIDTH,
            writer: Writer::io(sink()),
        }
    }

    fn report_impl(&self, f: &mut Writer) -> io::Result<()> {
        match &self.kind {
            ErrorKind::VersionRequested => {
                let fmt = self.context.root_desc.0.0.version_formatter;
                let version = fmt(&self.context);
                write!(f, "{version}")?;
                if !version.ends_with('\n') {
                    writeln!(f)?;
                }
            }
            ErrorKind::HelpRequested => {
                self.context
                    .desc
                    .help()
                    .with_invocation(&self.context.command_chain)
                    .write_to(f)?;
            }
            _ => {
                write!(f, "{RED}error{RESET}: ")?;
                f.set_indentation("error: ".len());
                self.write_error(f)?;
                write!(f, "\n\n")?;
                let mut usage = self
                    .context
                    .desc
                    .usage()
                    .with_invocation(&self.context.command_chain)
                    .with_prefix(format!("{BOLD}usage{RESET}: "));
                if let Some(arg) = self.kind.arg_index() {
                    usage = usage.highlight_arg(arg);
                }
                if self.kind.related_to_subcommand() {
                    usage = usage.highlight_subcommand();
                }
                usage.write_to(f)?;
                writeln!(f)?;
            }
        }

        f.flush()
    }

    fn arg(&self, index: usize) -> &ArgumentDesc {
        &self.context.desc.args()[index]
    }

    fn argname(&self, index: usize) -> impl fmt::Display {
        struct Disp<'a>(&'a ArgumentName);
        impl fmt::Display for Disp<'_> {
            fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
                write!(f, "`{YELLOW}{ITALIC}{}{RESET}`", self.0)
            }
        }
        Disp(self.arg(index).name())
    }

    fn write_error(&self, f: &mut Writer) -> io::Result<()> {
        match &self.kind {
            ErrorKind::HelpRequested => write!(f, "help requested"),
            ErrorKind::VersionRequested => write!(f, "version requested"),

            ErrorKind::Utf8Error(utf8_error) => write!(f, "{utf8_error}"),
            ErrorKind::ValueParseError { error, arg } => write!(
                f,
                "invalid value for argument {}: {error}",
                self.argname(*arg)
            ),
            ErrorKind::UnexpectedArgValue(arg) => {
                write!(f, "flag {} does not take a value", self.argname(*arg))
            }
            ErrorKind::UnexpectedArg { arg } => {
                write!(f, "unexpected {arg}")?;
                Ok(())
            }
            ErrorKind::DuplicateArg(arg) => {
                write!(f, "duplicate argument {}", self.argname(*arg))
            }
            ErrorKind::MissingArg(arg) => {
                write!(f, "missing argument {}", self.argname(*arg))
            }
            ErrorKind::MissingArgValue(arg) => {
                write!(f, "argument {} requires a value", self.argname(*arg))
            }
            ErrorKind::MissingSubcommand => write!(f, "a subcommand is required"),
            ErrorKind::UnknownSubcommand(cmd) => {
                write!(f, "unknown subcommand `{}`", cmd.display())
            }
        }
    }

    fn is_fatal(&self) -> bool {
        !matches!(
            &self.kind,
            ErrorKind::HelpRequested | ErrorKind::VersionRequested
        )
    }
}

/// This is the internal error type used throughout this library and by the macro-generated code.
///
/// It is only converted to a user-facing [`Error`] at the boundary, where [`Error`] can be enhanced
/// with command-specific context.
#[derive(Debug)]
pub enum ErrorKind {
    HelpRequested,
    VersionRequested,

    Utf8Error(Utf8Error),
    /// Failed to parse an argument value.
    ///
    /// The boxed error can be [`Utf8Error`], when the type expects a valid UTF-8 input, or any
    /// type-specific parsing error, such as [`ParseIntError`][std::num::ParseIntError].
    ///
    /// The [`ErrorKind::Utf8Error`] variant indicates that invalid UTF-8 was provided as part of
    /// the *name* of an argument instead its value.
    ValueParseError {
        arg: usize,
        error: Box<dyn std::error::Error>,
    },
    /// Argument doesn't take a value, but `--arg=...` was passed.
    UnexpectedArgValue(usize),
    UnexpectedArg {
        arg: String,
    },
    /// A non-repeating argument was provided multiple times. The [`usize`] is the argument index in
    /// the `CommandDesc`.
    DuplicateArg(usize),
    /// A required argument was not provided. The [`usize`] is the argument index in the
    /// `CommandDesc`.
    MissingArg(usize),
    MissingArgValue(usize),
    /// A subcommand is required, but none was provided.
    MissingSubcommand,
    /// A subcommand was provided, and a subcommand is accepted by this command, but the name
    /// doesn't match any of the supported subcommands.
    UnknownSubcommand(OsString),
}

impl ErrorKind {
    fn arg_index(&self) -> Option<usize> {
        match self {
            ErrorKind::MissingArg(arg)
            | ErrorKind::MissingArgValue(arg)
            | ErrorKind::DuplicateArg(arg)
            | ErrorKind::UnexpectedArgValue(arg)
            | ErrorKind::ValueParseError { arg, .. } => Some(*arg),

            ErrorKind::HelpRequested
            | ErrorKind::VersionRequested
            | ErrorKind::Utf8Error(..)
            | ErrorKind::UnexpectedArg { .. }
            | ErrorKind::MissingSubcommand
            | ErrorKind::UnknownSubcommand(..) => None,
        }
    }

    fn related_to_subcommand(&self) -> bool {
        match self {
            ErrorKind::MissingSubcommand | ErrorKind::UnknownSubcommand(..) => true,

            ErrorKind::MissingArg(..)
            | ErrorKind::MissingArgValue(..)
            | ErrorKind::DuplicateArg(..)
            | ErrorKind::HelpRequested
            | ErrorKind::VersionRequested
            | ErrorKind::Utf8Error(..)
            | ErrorKind::ValueParseError { .. }
            | ErrorKind::UnexpectedArgValue { .. }
            | ErrorKind::UnexpectedArg { .. } => false,
        }
    }
}

impl fmt::Display for Error {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        self.report_impl(&mut Writer::display(f))
            .map_err(|_| fmt::Error)
    }
}
impl std::error::Error for Error {}

/// Success status code.
const EX_OK: i32 = 0;

/// This is the exit code we return with when parsing the command line fails.
///
/// It is taken from the Linux/BSD/OS X `sysexits.h` and described as
///
/// > The command was used incorrectly, e.g., with the wrong number of arguments,
/// > a bad flag, bad syntax in a parameter, or whatever.
const EX_USAGE: i32 = 64;

/// Writes an [`Error`] to a destination.
///
/// Returned by [`Error::reporter`].
pub struct Reporter<'a> {
    error: &'a Error,
    color: Color,
    wrap_width: usize,
    writer: Writer<'a>,
}

impl<'a> Reporter<'a> {
    /// Configures whether to use ANSI colors.
    ///
    /// By default (if this function isn't called), [`Reporter`] will respect any
    /// a `--color=<always|auto|never>` argument that was encountered on the command line
    /// (this mechanism requires the [`Command`][crate::Command] implementor to have an argument
    /// of type [`Color`]).
    ///
    /// If no `--color` argument was passed, and this method *isn't* called, the default is
    /// [`Color::Auto`].
    pub fn color(self, color: Color) -> Self {
        Self { color, ..self }
    }

    /// Sets the maximum line width.
    ///
    /// Lines will be soft-wrapped if they exceed this width.
    ///
    /// By default, lines will get wrapped after an unspecified, but typically reasonable width.
    pub fn wrap_width(self, width: usize) -> Self {
        Self {
            wrap_width: width,
            ..self
        }
    }

    /// Writes output to the given destination.
    ///
    /// The provided type has to implement [`IsTerminal`] for automatic colorization to work.
    /// To output to something that does not implement [`IsTerminal`], use [`Reporter::raw_output`].
    pub fn output<W: io::Write + IsTerminal + 'a>(self, w: W) -> Self {
        Self {
            writer: Writer::fd(w),
            ..self
        }
    }

    /// Writes the error to an [`io::Write`] implementor.
    ///
    /// This will not use ANSI colors unless colors are forcibly enabled by passing `--color=always`
    /// or calling [`Reporter::color`] with [`Color::Always`].
    pub fn raw_output<W: io::Write + 'a>(self, w: W) -> Self {
        Self {
            writer: Writer::io(w),
            ..self
        }
    }

    /// Reports the error to the configured destination.
    pub fn report(mut self) -> io::Result<()> {
        match self.color {
            Color::Never => self.writer.force_color(false),
            Color::Always => self.writer.force_color(true),
            _ => {}
        }
        self.writer.set_max_line_width(self.wrap_width);
        self.error.report_impl(&mut self.writer)
    }

    /// Reports the error to the configured destination, and exits the process with an appropriate
    /// status.
    ///
    /// If the [`Error`] is fatal (ie. the result of an incorrect command invocation), this will
    /// exit with code `EX_USAGE` (64).
    /// Otherwise, the [`Error`] indicates that the user requested `--help` or `--version` output,
    /// and this method will exit the process with code 0.
    pub fn report_and_exit(self) -> ! {
        let code = if self.error.is_fatal() {
            EX_USAGE
        } else {
            EX_OK
        };
        self.report().ok();
        process::exit(code);
    }
}
