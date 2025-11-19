use std::{ffi::OsString, path::PathBuf};

use larpa::{
    Command,
    types::{Color, PrintHelp, PrintVersion, Verbosity},
};

#[allow(dead_code)]
#[derive(Debug, Command)]
struct Args {
    #[larpa(name = ["-h", "--help"], flag)]
    help: PrintHelp,

    #[larpa(name = "-s")]
    required_short: i8,

    #[larpa(name = "--required-long")]
    required_long: i8,

    #[larpa(name = ["-m", "--mixed"])]
    required_mixed: i8,

    #[larpa(name = "--int", default = "127")]
    optional_long: i8,

    #[larpa(name = "-x", default = "-1")]
    optional_short: i8,

    #[larpa(name = ["-o", "--opt"], default)]
    optional_mixed: i8,

    #[larpa(flag, name = ["--version", "-V"])]
    version: PrintVersion,

    #[larpa(flag, name = ["--verbose", "-v"])]
    verbosity: Verbosity,

    #[larpa(flag, name = ["--quiet", "-q"], inverse_of = "verbosity")]
    quiet: (),

    #[larpa(flag, name = ["--pager"])]
    pager: Option<bool>,

    #[larpa(flag, name = ["--no-pager"], inverse_of = "pager")]
    no_pager: (),

    #[larpa(name = ["--color"])]
    color: Option<Color>,

    #[larpa(flag, name = ["-i"])]
    hidden: bool,

    required_positional: OsString,

    #[larpa(subcommand)]
    required_subcommand: Subcmd,
}

#[allow(dead_code)]
#[derive(Debug, Command)]
enum Subcmd {
    Path {
        /// The path.
        path: PathBuf,
    },
    #[larpa(name = "renamed")]
    Command,
    #[larpa(fallback)]
    Fallback(Vec<OsString>),
}

fn main() {
    let args = Args::from_args();
    println!("Successfully parsed arguments!");
    println!("{args:#?}");
}
