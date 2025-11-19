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

fn size(file: &str) -> io::Result<u64> {
    let mut f = File::open(file)?;
    f.seek(SeekFrom::End(0))?;
    f.stream_position()
}

fn measure(example: &str) -> io::Result<u64> {
    cargo(&[
        "build",
        "--release",
        "-p",
        "ci",
        "--example",
        example,
        "--features=empty",
    ]);

    let empty = size(&format!("target/release/examples/{example}"))?;

    cargo(&["build", "--release", "-p", "ci", "--example", example]);

    let nonempty = size(&format!("target/release/examples/{example}"))?;

    let overhead = nonempty - empty;

    Ok(overhead)
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
    append("| Name | Overhead |")?;
    append("|------|----------|")?;
    for example in EXAMPLES {
        let overhead = measure(example)? / 1024;
        append(&format!("| {example} | {overhead} KiB |"))?;
    }

    Ok(())
}
