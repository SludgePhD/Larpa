# Larpa: The **L**ousy **Ar**gument **Pa**rser.

A simple `#[derive]`-based command line argument parsing library.

## Goals

- Stay more lightweight than Clap, while providing many of its bells and whistles in an
  easier-to-digest package with a simpler API.
- Provide built-in generation of `--help`/`--version` output.
- Build a well-designed, stable, intuitive API surface that I can proudly publish as 1.0.0.

## Non-Goals

- Handle all possible command-line interfaces.
  - *Larpa* aims to handle all reasonable command-line interfaces that roughly adhere to the
    [GNU CLI argument conventions], but not more than that.
  - In particular, order-dependent named arguments or argument groups will probably never be
    supported, as the `#[derive]`-based design isn't a good fit. Use something like [lexopt] instead.
  - Dynamically constructing and modifying the accepted syntax of the command-line interface will
    also likely never be supported. Use [Clap] instead.
- Handle weird/niche use cases like `#![no_std]` usage, or operating systems that are significantly
  different from Unix or Windows.

[GNU CLI argument conventions]: https://sourceware.org/glibc/manual/2.42/html_mono/libc.html#Argument-Syntax
[lexopt]: https://github.com/blyxxyz/lexopt
[Clap]: https://github.com/clap-rs/clap

## Example

```rust
use larpa::Command;
use larpa::types::{Verbosity, PrintVersion, PrintHelp};
use std::path::PathBuf;

#[derive(Command)]
struct Shredder {
    #[larpa(name = ["-v", "--verbose"], flag)]
    verbosity: Verbosity,

    #[larpa(name = ["-q", "--quiet"], flag, inverse_of = "verbosity")]
    quiet: (),

    #[larpa(name = "--version", flag)]
    version: PrintVersion,

    #[larpa(name = "--help", flag)]
    help: PrintHelp,

    /// The configuration file to use.
    #[larpa(name = "--config")]
    config: Option<PathBuf>,

    /// The speed to run the shredder at (in RPM).
    #[larpa(name = ["-s", "--speed"], default = "8000.0")]
    speed: f64,

    /// Path to the file to shred (or `-` to shred data from stdin).
    file: PathBuf,
}
```
