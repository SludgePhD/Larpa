//! Measures the executable file size overhead of Larpa.
//!
//! The measurement result is printed to stdout and made available as a GitHub Actions run summary.
//! <https://github.blog/news-insights/product-news/supercharging-github-actions-with-job-summaries/>

use std::{
    env,
    ffi::OsStr,
    fs::File,
    io::{self, Seek, SeekFrom, Write},
    process::Command,
};

/// The list of examples we're measuring.
const EXAMPLES: &[&str] = &["rosetta", "grit"];

fn cargo(args: &[impl AsRef<OsStr>]) {
    let status = Command::new(env::var_os("CARGO").expect("`$CARGO` is not set"))
        .args(args)
        .status()
        .unwrap();
    assert!(status.success());
}

fn size(example: &str, features: &str) -> io::Result<u64> {
    cargo(&[
        "build",
        "--release",
        "-p",
        env!("CARGO_PKG_NAME"),
        "--example",
        example,
        "--features",
        features,
    ]);

    let mut f = File::open(format!("target/release/examples/{example}"))?;
    f.seek(SeekFrom::End(0))?;
    f.stream_position()
}

fn main() -> io::Result<()> {
    let mut summary = None;
    if let Some(step) = env::var_os("GITHUB_STEP_SUMMARY") {
        summary = Some(File::options().append(true).create(true).open(step)?);
    }

    let mut append = |s: &str| -> io::Result<()> {
        if let Some(f) = &mut summary {
            writeln!(f, "{s}")?;
        }

        println!("{s}");
        Ok(())
    };

    append("# Binary Size Overhead")?;
    append("")?;
    append("| Name | `Command::from_args` | `Command::DESC` |")?;
    append("|------|----------------------|-----------------|")?;
    for example in EXAMPLES {
        let empty = size(example, "")?;
        let from_args = (size(example, "from-args")? - empty) / 1024;
        let desc = (size(example, "desc")? - empty) / 1024;

        append(&format!("| {example} | {from_args} KiB | {desc} KiB |"))?;
    }

    Ok(())
}
