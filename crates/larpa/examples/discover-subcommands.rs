//! Shows how to configure automatic discovery of external subcommand binaries.
//!
//! The generated help and usage string will contain all discovered external subcommands.

use std::{ffi::OsString, process};

use larpa::Command;

#[derive(Debug, Command)]
#[larpa(name = "git")]
enum Git {
    Commit,
    Push,
    Pull,
    #[larpa(fallback, discover)]
    External(Vec<OsString>),
}

fn main() {
    let git = Git::from_args();

    // If a non-builtin subcommand is passed, try to invoke `git-{subcommand}`.
    if let Git::External(args) = git {
        let mut name = OsString::from("git-");
        name.push(&args[0]);

        match process::Command::new(&name)
            .args(args.into_iter().skip(1))
            .status()
        {
            Ok(status) => {
                if !status.success() {
                    eprintln!("subcommand exited with error code");
                    process::exit(1);
                }
            }
            Err(e) => {
                eprintln!("failed to run `{}`: {e}", name.display());
                process::exit(1);
            }
        }
    }
}
