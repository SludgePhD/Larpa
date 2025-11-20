# `sizebench`

This is a binary size overhead measurement tool for Larpa.

It is run in CI to measure the approximate binary size overhead of Larpa for the test cases in
`examples`.
To reproduce the CI measurements, run `SIZEBENCH=1 cargo test -p sizebench`.

During normal (non-CI) testing, the tests are only build in debug mode to reduce the build times.
`SIZEBENCH=1` makes sizebench build them with the release profile defined in the workspace
`Cargo.toml`, which is intended to be identical to that used by [`argparse-rosetta-rs`].

[`argparse-rosetta-rs`]: https://github.com/rosetta-rs/argparse-rosetta-rs
