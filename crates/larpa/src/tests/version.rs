use expect_test::expect;

use crate::{
    Command, Context,
    tests::check_err,
    types::{PrintVersion, Verbosity},
};

#[test]
fn custom_version() {
    #[derive(Command)]
    #[larpa(crate = "crate", version = concat!("custom-version", "-1.0"))]
    struct MyCmd {}

    assert_eq!(MyCmd::DESC.version(), Some("custom-version-1.0"));
}

#[test]
fn print_version() {
    #[derive(Command)]
    #[larpa(crate = "crate", name = "mycmd", version = concat!("custom-version", "-1.0"))]
    struct MyCmd {
        #[larpa(flag, name = "--version")]
        _version: PrintVersion,
    }

    let error = MyCmd::try_from_iter(["mycmd", "--version"]).err().unwrap();
    expect![[r#"
        mycmd custom-version-1.0
    "#]]
    .assert_eq(&error.to_string());

    check_err::<MyCmd>(
        ["mycmd", "--invalid"],
        expect![[r#"
            <red>error</red>: unexpected argument `--invalid`

            <b>usage</b>: <b>mycmd</b> [--version] [--help]
        "#]],
    );
}

#[test]
fn custom_formatter() {
    #[allow(dead_code)]
    #[derive(Debug, Command)]
    #[larpa(crate = "crate", version = "1.2.3", version_formatter = "my_fmt")]
    struct MyCmd {
        #[larpa(flag, name = "--version")]
        version: PrintVersion,

        #[larpa(flag, name = ["-v", "--verbose"])]
        verbosity: Verbosity,
    }

    fn my_fmt(cx: &Context) -> String {
        let version = cx.command_desc().version().unwrap();
        if cx.verbosity() > 0 {
            format!("MY-CMD {version}\nverbose info\n")
        } else {
            format!("MY-CMD {version}")
        }
    }

    check_err::<MyCmd>(
        ["my-cmd", "--version"],
        expect![[r#"
            MY-CMD 1.2.3
        "#]],
    );

    check_err::<MyCmd>(
        ["my-cmd", "--version", "-v"],
        expect![[r#"
            MY-CMD 1.2.3
            verbose info
        "#]],
    );
}
