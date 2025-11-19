use std::{
    env,
    ffi::OsString,
    fs,
    io::{self, Write},
};

use crate::{Command, Error, markup::markup_html, types::Color};

/// Snapshot tool for doc tests.
///
/// This tries to parse `C` from `args`, expecting it to fail.
/// The resulting error (which may be the `--help` output) is then formatted and written to `path`.
pub fn snap<C: Command>(args: impl IntoIterator<Item: Into<OsString>>, path: &str) {
    let args = args.into_iter().map(Into::into).collect::<Vec<_>>();
    match C::try_from_iter(args.iter().cloned()) {
        Ok(_) => panic!("parsing succeeded, but was expected to fail"),
        Err(e) => {
            fmterr(e, args, path).unwrap();
        }
    }
}

fn fmterr(e: Error, args: Vec<OsString>, path: &str) -> io::Result<()> {
    let mut error = Vec::new();
    e.reporter()
        .color(Color::Always)
        .wrap_width(90) // rustdoc code blocks fit around 95 characters before scroll bars appear
        .raw_output(&mut error)
        .report()?;

    let error = markup_html(
        &String::from_utf8(error)
            .unwrap()
            .replace('>', "&gt;")
            .replace('<', "&lt;"),
    );

    let mut invocation = Vec::new();
    for (i, arg) in args.iter().enumerate() {
        if i > 0 {
            invocation.push(b' ');
        }
        invocation.extend_from_slice(arg.as_encoded_bytes());
    }

    let mut file = Vec::new();
    write!(file, "<pre><code>$ ")?;
    file.write_all(&invocation)?;
    writeln!(file)?;
    file.write_all(error.as_bytes())?;
    writeln!(file, "</code></pre>")?;

    let changed = match fs::read(path) {
        Ok(old) => old != file,
        Err(_) => true,
    };
    match fs::write(path, file) {
        Ok(()) => {
            if changed {
                panic!("contents of '{path}' have changed; run tests again to pass");
            } else {
                Ok(())
            }
        }
        Err(e) => {
            panic!(
                "failed to create '{path}': {e} (pwd: {})",
                env::current_dir().unwrap().display()
            );
        }
    }
}
