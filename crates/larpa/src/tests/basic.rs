use expect_test::expect;

use crate::{
    Command,
    tests::check_err,
    types::{Color, PrintHelp, PrintVersion},
};

#[test]
fn empty() {
    #[derive(Debug, PartialEq, Command)]
    #[larpa(crate = "crate")]
    struct MyCmd {}

    // Passes an empty argument to the program. This should fail, since it is parsed as a positional
    // argument, but none are expected.
    assert_eq!(MyCmd::from_iter(["my-cmd"]), MyCmd {});
    check_err::<MyCmd>(
        ["my-cmd", ""],
        expect![[r#"
            <red>error</red>: unexpected argument ``

            <b>usage</b>: <b>my-cmd</b> [--help]
        "#]],
    );
}

#[test]
fn simple_flags() {
    #[derive(Debug, PartialEq, Command)]
    #[larpa(crate = "crate")]
    struct MyCmd {
        #[larpa(flag, name = "-f")]
        flag: bool,
    }

    assert_eq!(MyCmd::from_iter(["my-cmd"]), MyCmd { flag: false });
    assert_eq!(MyCmd::from_iter(["my-cmd", "-f"]), MyCmd { flag: true });
}

#[test]
fn inverse_of() {
    #[derive(Debug, PartialEq, Command)]
    #[larpa(crate = "crate")]
    struct MyCmd {
        #[larpa(flag, name = "-f")]
        flag: bool,

        #[larpa(flag, name = "-i", inverse_of = "flag")]
        inv: (),
    }

    assert_eq!(
        MyCmd::from_iter(["my-cmd"]),
        MyCmd {
            flag: false,
            inv: ()
        }
    );
    assert_eq!(
        MyCmd::from_iter(["my-cmd", "-f"]),
        MyCmd {
            flag: true,
            inv: ()
        }
    );
    assert_eq!(
        MyCmd::from_iter(["my-cmd", "-i"]),
        MyCmd {
            flag: false,
            inv: ()
        }
    );
    assert_eq!(
        MyCmd::from_iter(["my-cmd", "-fi"]),
        MyCmd {
            flag: false,
            inv: ()
        }
    );
    assert_eq!(
        MyCmd::from_iter(["my-cmd", "-if"]),
        MyCmd {
            flag: true,
            inv: ()
        }
    );
}

#[test]
fn version_flag() {
    #[derive(Debug, PartialEq, Command)]
    #[larpa(crate = "crate", name = "mycmd", version = "1.2.3")]
    struct MyCmd {
        #[larpa(name = "--VERSION", flag)]
        version: PrintVersion,

        mandatory: u8,
    }

    assert_eq!(
        MyCmd::from_iter(["my-cmd", "1"]),
        MyCmd {
            version: PrintVersion,
            mandatory: 1,
        }
    );

    let error = MyCmd::try_from_iter(["my-cmd", "--VERSION"])
        .unwrap_err()
        .to_string();
    assert_eq!(error, "mycmd 1.2.3\n");
}

#[test]
fn help_flag() {
    /// Test command.
    #[derive(Debug, PartialEq, Command)]
    #[larpa(crate = "crate", name = "mycmd", version = "1.2.3", no_generate_help)]
    struct MyCmd {
        /// Print help information.
        #[larpa(name = "--HELP", flag)]
        help: PrintHelp,

        mandatory: u8,
    }

    assert_eq!(
        MyCmd::from_iter(["my-cmd", "1"]),
        MyCmd {
            help: PrintHelp,
            mandatory: 1,
        }
    );

    let error = MyCmd::try_from_iter(["my-cmd", "--HELP"])
        .unwrap_err()
        .to_string();
    expect![[r#"
        Test command.

        my-cmd [--HELP] <MANDATORY>

        OPTIONS:
              --HELP  Print help information.

        Repository: https://github.com/SludgePhD/Larpa
        License: 0BSD
    "#]]
    .assert_eq(&error);
}

#[test]
fn simple_named_int() {
    #[derive(Debug, PartialEq, Command)]
    #[larpa(crate = "crate")]
    struct MyCmd {
        #[larpa(name = "-i")]
        int: u8,
    }

    assert_eq!(MyCmd::from_iter(["my-cmd", "-i0"]), MyCmd { int: 0 });
    assert_eq!(MyCmd::from_iter(["my-cmd", "-i1"]), MyCmd { int: 1 });
    assert_eq!(MyCmd::from_iter(["my-cmd", "-i", "2"]), MyCmd { int: 2 });
}

#[test]
fn simple_positional_int() {
    #[derive(Debug, PartialEq, Command)]
    #[larpa(crate = "crate")]
    struct MyCmd {
        int: u8,
    }

    assert_eq!(MyCmd::from_iter(["my-cmd", "0"]), MyCmd { int: 0 });
    assert_eq!(MyCmd::from_iter(["my-cmd", "1"]), MyCmd { int: 1 });
}

#[test]
fn defaulted_arg() {
    #[derive(Debug, PartialEq, Command)]
    #[larpa(crate = "crate")]
    struct MyCmd {
        #[larpa(default, name = "-i")]
        int: u8,
    }

    assert_eq!(MyCmd::from_iter(["my-cmd"]), MyCmd { int: 0 });
    assert_eq!(MyCmd::from_iter(["my-cmd", "-i0"]), MyCmd { int: 0 });
    assert_eq!(MyCmd::from_iter(["my-cmd", "-i1"]), MyCmd { int: 1 });

    #[derive(Debug, PartialEq, Command)]
    #[larpa(crate = "crate", no_generate_tests)]
    struct MyCmd2 {
        #[larpa(default = "123", name = "-i")]
        int: u8,
    }

    assert_eq!(MyCmd2::from_iter(["my-cmd"]), MyCmd2 { int: 123 });
    assert_eq!(MyCmd2::from_iter(["my-cmd", "-i0"]), MyCmd2 { int: 0 });
    assert_eq!(MyCmd2::from_iter(["my-cmd", "-i1"]), MyCmd2 { int: 1 });
}

#[test]
fn positional_default() {
    #[derive(Debug, PartialEq, Command)]
    #[larpa(crate = "crate")]
    struct MyCmd {
        #[larpa(default)]
        int: u8,
    }

    assert_eq!(MyCmd::from_iter(["my-cmd"]), MyCmd { int: 0 });
    assert_eq!(MyCmd::from_iter(["my-cmd", "0"]), MyCmd { int: 0 });
    assert_eq!(MyCmd::from_iter(["my-cmd", "1"]), MyCmd { int: 1 });

    #[derive(Debug, PartialEq, Command)]
    #[larpa(crate = "crate", no_generate_tests)]
    struct MyCmd2 {
        #[larpa(default = "123")]
        value: u8,
    }

    assert_eq!(MyCmd2::from_iter(["my-cmd"]), MyCmd2 { value: 123 });
    assert_eq!(MyCmd2::from_iter(["my-cmd", "0"]), MyCmd2 { value: 0 });
    assert_eq!(MyCmd2::from_iter(["my-cmd", "1"]), MyCmd2 { value: 1 });
}

// Inner items cannot be tested, so make this a module item.
#[derive(Debug, PartialEq, Command)]
#[larpa(crate = "crate")]
struct CustomDefault {
    #[larpa(default = "5", name = "-i")]
    int: u8,
}

// We should not emit a unit test for this one.
#[derive(Debug, PartialEq, Command)]
#[larpa(crate = "crate", no_generate_tests)]
struct _CustomDefault2 {
    #[larpa(name = "-i", default = "does not look like an integer")]
    int: u8,
}

#[test]
fn custom_default() {
    assert_eq!(
        CustomDefault::from_iter(["my-cmd"]),
        CustomDefault { int: 5 }
    );
    assert_eq!(
        CustomDefault::from_iter(["my-cmd", "-i0"]),
        CustomDefault { int: 0 }
    );
    assert_eq!(
        CustomDefault::from_iter(["my-cmd", "-i1"]),
        CustomDefault { int: 1 }
    );
}

#[test]
fn optional_arg() {
    #[derive(Debug, PartialEq, Command)]
    #[larpa(crate = "crate")]
    struct MyCmd {
        #[larpa(name = "-i")]
        int: Option<u8>,
    }

    assert_eq!(MyCmd::from_iter(["my-cmd",]), MyCmd { int: None });
    assert_eq!(MyCmd::from_iter(["my-cmd", "-i0"]), MyCmd { int: Some(0) });
    assert_eq!(MyCmd::from_iter(["my-cmd", "-i1"]), MyCmd { int: Some(1) });

    check_err::<MyCmd>(
        ["my-cmd", "-i0", "-i1"],
        expect![[r#"
            <red>error</red>: duplicate argument `<yellow><i>-i</i></yellow>`

            <b>usage</b>: <b>my-cmd</b> [--help] [<u>-i <INT></u>]
        "#]],
    );
}

#[test]
fn repeated_named_arg() {
    #[derive(Debug, PartialEq, Command)]
    #[larpa(crate = "crate")]
    struct MyCmd {
        #[larpa(name = "-i")]
        int: Vec<u8>,
    }

    assert_eq!(MyCmd::from_iter(["my-cmd"]), MyCmd { int: vec![] });
    assert_eq!(
        MyCmd::from_iter(["my-cmd", "-i", "7"]),
        MyCmd { int: vec![7] }
    );
    assert_eq!(
        MyCmd::from_iter(["my-cmd", "-i3", "-i2"]),
        MyCmd { int: vec![3, 2] }
    );
}

#[test]
fn repeated_positional() {
    #[derive(Debug, PartialEq, Command)]
    #[larpa(crate = "crate")]
    struct MyCmd {
        int: Vec<u8>,
    }

    assert_eq!(MyCmd::from_iter(["my-cmd"]), MyCmd { int: vec![] });
    assert_eq!(MyCmd::from_iter(["my-cmd", "1"]), MyCmd { int: vec![1] });
    assert_eq!(
        MyCmd::from_iter(["my-cmd", "9", "5"]),
        MyCmd { int: vec![9, 5] }
    );
}

#[test]
fn required_repeated() {
    #[derive(Debug, PartialEq, Command)]
    #[larpa(crate = "crate")]
    struct MyCmd {
        #[larpa(required)]
        int: Vec<u8>,
    }

    check_err::<MyCmd>(
        ["my-cmd"],
        expect![[r#"
            <red>error</red>: missing argument `<yellow><i>INT</i></yellow>`

            <b>usage</b>: <b>my-cmd</b> [--help] <u><b><INT>...</b></u>
        "#]],
    );

    assert_eq!(MyCmd::from_iter(["my-cmd", "1"]), MyCmd { int: vec![1] });
}

#[test]
fn multiple_positional() {
    #[derive(Debug, PartialEq, Command)]
    #[larpa(crate = "crate")]
    struct MyCmd {
        first: String,
        rest: Vec<u8>,
    }

    assert_eq!(
        MyCmd::from_iter(["my-cmd", "first"]),
        MyCmd {
            first: "first".into(),
            rest: vec![]
        }
    );
    assert_eq!(
        MyCmd::from_iter(["my-cmd", "first", "3"]),
        MyCmd {
            first: "first".into(),
            rest: vec![3]
        }
    );
    assert_eq!(
        MyCmd::from_iter(["my-cmd", "first", "3", "7"]),
        MyCmd {
            first: "first".into(),
            rest: vec![3, 7]
        }
    );
}

#[test]
fn reserved_field_names() {
    #[derive(Debug, PartialEq, Command)]
    #[larpa(crate = "crate")]
    struct MyCmd {
        #[larpa(name = "--raw")]
        raw: String,

        #[larpa(name = "--cx")]
        cx: u8,

        #[larpa(name = "--value")]
        value: u8,
    }

    assert_eq!(
        MyCmd::from_iter(["my-cmd", "--raw", "raw", "--cx", "6", "--value=2"]),
        MyCmd {
            raw: "raw".into(),
            cx: 6,
            value: 2,
        }
    );
}

#[test]
fn invalid_value() {
    #[derive(Debug, PartialEq, Command)]
    #[larpa(crate = "crate")]
    struct Pos {
        int: i32,
    }

    check_err::<Pos>(
        ["my-cmd", "a"],
        expect![[r#"
            <red>error</red>: invalid value for argument `<yellow><i>INT</i></yellow>`: invalid digit found in string

            <b>usage</b>: <b>my-cmd</b> [--help] <u><b><INT></b></u>
        "#]],
    );

    #[derive(Debug, PartialEq, Command)]
    #[larpa(crate = "crate")]
    struct Named {
        #[larpa(name = ["-i", "--int"])]
        int: i32,
    }

    check_err::<Named>(
        ["named", "-iX"],
        expect![[r#"
            <red>error</red>: invalid value for argument `<yellow><i>-i/--int</i></yellow>`: invalid digit found in string

            <b>usage</b>: <b>named</b> [--help] <b><u>-i/--int <INT></b></u>
        "#]],
    );
    check_err::<Named>(
        ["named", "-i"],
        expect![[r#"
            <red>error</red>: argument `<yellow><i>-i/--int</i></yellow>` requires a value

            <b>usage</b>: <b>named</b> [--help] <b><u>-i/--int <INT></b></u>
        "#]],
    );
}

#[test]
fn does_not_take_value() {
    #[derive(Debug, PartialEq, Command)]
    #[larpa(crate = "crate")]
    struct Flag {
        #[larpa(flag, name = ["--verbose"])]
        verbose: i32,
    }

    check_err::<Flag>(
        ["flag", "--verbose="],
        expect![[r#"
            <red>error</red>: flag `<yellow><i>--verbose</i></yellow>` does not take a value

            <b>usage</b>: <b>flag</b> [<u>--verbose</u>] [--help]
        "#]],
    );
}

#[test]
fn color() {
    #[derive(Debug, PartialEq, Command)]
    #[larpa(crate = "crate")]
    struct MyCmd {
        #[larpa(name = "--color", default)]
        color: Color,
    }

    check_err::<MyCmd>(
        ["my-cmd", "--color=always", "--wrong"],
        expect![[r#"
            <red>error</red>: unexpected argument `--wrong`

            <b>usage</b>: <b>my-cmd</b> [--help] [--color=<COLOR>]
        "#]],
    );
    check_err::<MyCmd>(
        ["my-cmd", "--color=never", "--wrong"],
        expect![[r#"
            error: unexpected argument `--wrong`

            usage: my-cmd [--help] [--color=<COLOR>]
        "#]],
    );
}

#[test]
fn cfgs() {
    #[derive(Debug, PartialEq, Command)]
    #[larpa(crate = "crate")]
    struct MyCmd {
        #[larpa(name = "--arg")]
        #[cfg(false)]
        arg: u8,

        #[larpa(name = "--arg2", default)]
        #[cfg(true)]
        arg: u8,

        #[larpa(subcommand)]
        subcommand: Subcommand,
    }

    #[derive(Debug, PartialEq, Command)]
    #[larpa(crate = "crate")]
    enum Subcommand {
        #[cfg(false)]
        Nope,
        Yep,
    }

    assert_eq!(
        MyCmd::from_iter(["my-cmd", "yep"]),
        MyCmd {
            arg: 0,
            subcommand: Subcommand::Yep
        }
    );

    check_err::<MyCmd>(
        ["my-cmd", "nope"],
        expect![[r#"
            <red>error</red>: unknown subcommand `nope`

            <b>usage</b>: <b>my-cmd</b> [--help] [--arg2=<ARG>] <u><b>yep</b></u>
        "#]],
    );
}
