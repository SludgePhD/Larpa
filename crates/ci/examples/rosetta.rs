//! This is a port of the test case used by <https://github.com/rosetta-rs/argparse-rosetta-rs>.

use std::{num::NonZeroU32, path::PathBuf};

use larpa::{Command, types::PrintHelp};

#[allow(dead_code)]
#[derive(Command)]
#[larpa(no_homepage, no_license, no_repository)] // the rosetta example omits this information
struct App {
    /// Sets a number.
    #[larpa(name = "--number")]
    number: u32,

    /// Sets an optional number.
    #[larpa(name = "--opt-number")]
    opt_number: Option<u32>,

    /// Sets width.
    #[larpa(name = "--width", default = "10")]
    width: NonZeroU32,

    input: Vec<PathBuf>,

    /// Print help information.
    #[larpa(name = "--help", flag)]
    help: PrintHelp,
}

fn main() {
    if !cfg!(feature = "empty") {
        let args = App::from_args();
        std::hint::black_box(args);
    }
}
