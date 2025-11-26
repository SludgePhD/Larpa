//! Measures the executable file size overhead of Larpa.
//!
//! The measurement result is printed to stdout and made available as a GitHub Actions run summary.
//! <https://github.blog/news-insights/product-news/supercharging-github-actions-with-job-summaries/>

#![cfg(test)]

use std::{
    env::{self, consts::EXE_SUFFIX},
    error::Error,
    ffi::OsStr,
    fs::File,
    io::{self, Seek, SeekFrom, Write},
    process::Command,
};

/// The list of examples we're measuring.
const EXAMPLES: &[&str] = &["rosetta", "grit"];

/// If this env var is set, will build in `--release` mode and output the result to
/// `GITHUB_STEP_SUMMARY`.
const ENV_VAR: &str = "SIZEBENCH";

const TARGET_TRIPLE: &str = include_str!(concat!(env!("OUT_DIR"), "/target.txt"));

type Result<T> = std::result::Result<T, Box<dyn Error>>;

fn cargo_build(release: bool, args: &[impl AsRef<OsStr>]) {
    let mut command = Command::new(env::var_os("CARGO").expect("`$CARGO` is not set"));
    command.arg("build").args(args);
    if release {
        command.arg("--release");
    }
    let status = command.status().unwrap();
    assert!(status.success());
}

fn size(example: &str, features: &str) -> Result<u64> {
    let release = env::var_os(ENV_VAR).is_some();
    cargo_build(
        release,
        &[
            "--target",
            TARGET_TRIPLE,
            "-p",
            env!("CARGO_PKG_NAME"),
            "--example",
            example,
            "--features",
            features,
        ],
    );

    let profile = if release { "release" } else { "debug" };
    let target_dir = format!("../../target/{TARGET_TRIPLE}/{profile}");
    let mut f = File::open(format!("{target_dir}/examples/{example}{EXE_SUFFIX}"))
        .map_err(|e| format!("{e} (pwd: {})", env::current_dir().unwrap().display()))?;
    f.seek(SeekFrom::End(0))?;
    Ok(f.stream_position()?)
}

fn main() -> Result<()> {
    let mut summary = None;
    if let Some(step) = env::var_os("GITHUB_STEP_SUMMARY")
        && env::var_os(ENV_VAR).is_some()
    {
        summary = Some(File::options().append(true).create(true).open(step)?);
    }

    let mut append = |s: &str| -> io::Result<()> {
        if let Some(f) = &mut summary {
            writeln!(f, "{s}")?;
        }

        println!("{s}");
        Ok(())
    };

    let profile = if env::var_os(ENV_VAR).is_some() {
        "release"
    } else {
        "debug"
    };
    append("# Binary Size Overhead")?;
    append("")?;
    append(&format!("Build profile: **`{profile}`**"))?;
    append(&format!("Build target: **`{TARGET_TRIPLE}`**"))?;
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
