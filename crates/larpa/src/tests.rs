mod basic;
mod subcommands;
mod version;

use std::ffi::OsString;

use crate::{
    Command,
    markup::markup,
    types::Color,
    writer::{BOLD, RESET},
};
use expect_test::{Expect, expect};

#[track_caller]
fn check_err<C: Command>(args: impl IntoIterator<Item: Into<OsString>>, expect: Expect) {
    match C::try_from_iter(args) {
        Ok(_) => panic!("parsing succeeded when it should fail"),
        Err(e) => {
            let mut out = Vec::new();
            let color = if e.context.color() != Color::Auto {
                e.context.color()
            } else {
                Color::Always
            };
            e.reporter().color(color).raw_output(&mut out).report().ok();
            let out = String::from_utf8_lossy(&out);
            expect.assert_eq(&markup(&out));
        }
    }
}

fn check(input: String, expect: Expect) {
    expect.assert_eq(&markup(&input));
}

#[test]
fn test_markup() {
    check(format!("{RESET}"), expect![[r#""#]]);
    check(format!("c{RESET}"), expect![[r#"c"#]]);
    check(format!("{RESET}c"), expect![[r#"c"#]]);

    check(format!("{BOLD}c"), expect![[r#"<b>c</b>"#]]);
    check(format!("{BOLD}c{RESET}d"), expect![[r#"<b>c</b>d"#]]);
    check(
        format!("{BOLD}c{RESET}{RESET}d{RESET}"),
        expect![[r#"<b>c</b>d"#]],
    );
}
