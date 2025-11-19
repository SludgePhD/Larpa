use std::ffi::OsString;

use expect_test::expect;

use crate::desc::{CommandDesc, DiscoveredSubcommand};
use crate::{Command, tests::check_err};

#[test]
fn basic_subcommands() {
    #[derive(Debug, PartialEq, Command)]
    #[larpa(crate = "crate")]
    enum MyCmd {
        Sub1,
        #[larpa(name = "sub2")]
        Other,
    }

    assert_eq!(MyCmd::from_iter(["my-cmd", "sub1"]), MyCmd::Sub1);
    assert_eq!(MyCmd::from_iter(["my-cmd", "sub2"]), MyCmd::Other);

    check_err::<MyCmd>(
        ["my-cmd"],
        expect![[r#"
            <red>error</red>: a subcommand is required

            <b>usage</b>: <b>my-cmd</b> <u><b>sub1</b></u>|<u><b>sub2</b></u>
        "#]],
    );
    check_err::<MyCmd>(
        ["my-cmd", "other"],
        expect![[r#"
            <red>error</red>: unknown subcommand `other`

            <b>usage</b>: <b>my-cmd</b> <u><b>sub1</b></u>|<u><b>sub2</b></u>
        "#]],
    );
}

#[test]
fn subcommand_args() {
    #[derive(Debug, PartialEq, Command)]
    #[larpa(crate = "crate")]
    enum MyCmd {
        Sub1 {
            #[larpa(flag, name = "-f")]
            first: u8,
        },
        Sub2 {
            #[larpa(flag, name = "-s")]
            second: u8,
        },
    }

    check_err::<MyCmd>(
        ["my-cmd"],
        expect![[r#"
            <red>error</red>: a subcommand is required

            <b>usage</b>: <b>my-cmd</b> <u><b>sub1</b></u>|<u><b>sub2</b></u>
        "#]],
    );
    check_err::<MyCmd>(
        ["my-cmd", "weirdcmd"],
        expect![[r#"
            <red>error</red>: unknown subcommand `weirdcmd`

            <b>usage</b>: <b>my-cmd</b> <u><b>sub1</b></u>|<u><b>sub2</b></u>
        "#]],
    );
    assert_eq!(
        MyCmd::from_iter(["my-cmd", "sub1"]),
        MyCmd::Sub1 { first: 0 }
    );
    assert_eq!(
        MyCmd::from_iter(["my-cmd", "sub1", "-f"]),
        MyCmd::Sub1 { first: 1 }
    );
    assert_eq!(
        MyCmd::from_iter(["my-cmd", "sub1", "-ff"]),
        MyCmd::Sub1 { first: 2 }
    );
    check_err::<MyCmd>(
        ["my-cmd", "sub1", "-s"],
        expect![[r#"
            <red>error</red>: unexpected argument `-s`

            <b>usage</b>: <b>my-cmd sub1</b> [-f] [--help]
        "#]],
    );

    assert_eq!(
        MyCmd::from_iter(["my-cmd", "sub2"]),
        MyCmd::Sub2 { second: 0 }
    );
    assert_eq!(
        MyCmd::from_iter(["my-cmd", "sub2", "-s"]),
        MyCmd::Sub2 { second: 1 }
    );
    assert_eq!(
        MyCmd::from_iter(["my-cmd", "sub2", "-ss"]),
        MyCmd::Sub2 { second: 2 }
    );
    check_err::<MyCmd>(
        ["my-cmd", "sub2", "-f"],
        expect![[r#"
            <red>error</red>: unexpected argument `-f`

            <b>usage</b>: <b>my-cmd sub2</b> [-s] [--help]
        "#]],
    );
    check_err::<MyCmd>(
        ["my-cmd", "sub2", "bla"],
        expect![[r#"
            <red>error</red>: unexpected argument `bla`

            <b>usage</b>: <b>my-cmd sub2</b> [-s] [--help]
        "#]],
    );
}

#[test]
fn wrapped() {
    #[derive(Debug, PartialEq, Command)]
    #[larpa(crate = "crate")]
    enum MyCmd {
        Sub1 {
            #[larpa(flag, name = "-f")]
            first: u8,
        },
        Sub2(Sub2),
    }

    #[derive(Debug, PartialEq, Command)]
    #[larpa(crate = "crate")]
    struct Sub2 {
        #[larpa(flag, name = "-s")]
        second: u8,
    }

    assert_eq!(
        MyCmd::from_iter(["my-cmd", "sub2"]),
        MyCmd::Sub2(Sub2 { second: 0 })
    );
    assert_eq!(
        MyCmd::from_iter(["my-cmd", "sub2", "-ss"]),
        MyCmd::Sub2(Sub2 { second: 2 })
    );
}

#[test]
fn subcommand_takes_arg() {
    #[derive(Debug, PartialEq, Command)]
    #[larpa(crate = "crate")]
    enum MyCmd {
        Sub1 {
            #[larpa(name = "-i")]
            int: u8,
        },
    }

    assert_eq!(
        MyCmd::from_iter(["my-cmd", "sub1", "-i9"]),
        MyCmd::Sub1 { int: 9 },
    );
}

#[test]
fn fallback() {
    #[derive(Debug, PartialEq, Command)]
    #[larpa(crate = "crate")]
    enum MyCmd {
        Builtin {
            #[larpa(flag, name = "-f")]
            first: u8,
        },
        #[larpa(fallback)]
        External(Vec<OsString>),
    }

    assert_eq!(
        MyCmd::from_iter(["my-cmd", "builtin"]),
        MyCmd::Builtin { first: 0 },
    );
    assert_eq!(
        MyCmd::from_iter(["my-cmd", "builtin", "-f"]),
        MyCmd::Builtin { first: 1 },
    );
    assert_eq!(
        MyCmd::from_iter(["my-cmd", "dunno"]),
        MyCmd::External(vec!["dunno".into()]),
    );
    assert_eq!(
        MyCmd::from_iter(["my-cmd", "dunno", "--bla"]),
        MyCmd::External(vec!["dunno".into(), "--bla".into()]),
    );
}

#[test]
fn subcommand_field() {
    #[derive(Debug, PartialEq, Command)]
    #[larpa(crate = "crate")]
    struct Cmd {
        #[larpa(flag, name = "-a")]
        arg: u8,

        #[larpa(subcommand)]
        subcmd: Subcmd,
    }

    #[derive(Debug, PartialEq, Command)]
    #[larpa(crate = "crate")]
    enum Subcmd {
        List,
        Status,
    }

    assert_eq!(
        Cmd::from_iter(["cmd", "list"]),
        Cmd {
            arg: 0,
            subcmd: Subcmd::List
        }
    );
    assert_eq!(
        Cmd::from_iter(["cmd", "-a", "list"]),
        Cmd {
            arg: 1,
            subcmd: Subcmd::List
        }
    );
    assert_eq!(
        Cmd::from_iter(["cmd", "-a", "status"]),
        Cmd {
            arg: 1,
            subcmd: Subcmd::Status
        }
    );

    check_err::<Cmd>(
        ["cmd"],
        expect![[r#"
            <red>error</red>: a subcommand is required

            <b>usage</b>: <b>cmd</b> [-a] [--help] <u><b>list</b></u>|<u><b>status</b></u>
        "#]],
    );
}

#[test]
fn opt_subcommand_field() {
    #[derive(Debug, PartialEq, Command)]
    #[larpa(crate = "crate")]
    struct Cmd {
        #[larpa(name = "-n")]
        num: u8,

        #[larpa(subcommand)]
        subcmd: Option<Subcmd>,
    }

    #[derive(Debug, PartialEq, Command)]
    #[larpa(crate = "crate")]
    enum Subcmd {
        List,
        Status,
    }

    assert_eq!(
        Cmd::from_iter(["cmd", "-n1", "list"]),
        Cmd {
            num: 1,
            subcmd: Some(Subcmd::List),
        }
    );
    assert_eq!(
        Cmd::from_iter(["cmd", "-n1"]),
        Cmd {
            num: 1,
            subcmd: None,
        }
    );
}

#[test]
fn with_required_arg() {
    #[derive(Debug, PartialEq, Command)]
    #[larpa(crate = "crate")]
    struct Cmd {
        #[larpa(name = "-a")]
        arg: u8,

        #[larpa(subcommand)]
        subcmd: Subcmd,
    }

    #[derive(Debug, PartialEq, Command)]
    #[larpa(crate = "crate")]
    enum Subcmd {
        List,
        Status,
    }

    check_err::<Cmd>(
        ["cmd"],
        expect![[r#"
            <red>error</red>: missing argument `<yellow><i>-a</i></yellow>`

            <b>usage</b>: <b>cmd</b> [--help] <b><u>-a <ARG></b></u> <b>list</b>|<b>status</b>
        "#]],
    );
    check_err::<Cmd>(
        ["cmd", "list"],
        expect![[r#"
            <red>error</red>: missing argument `<yellow><i>-a</i></yellow>`

            <b>usage</b>: <b>cmd</b> [--help] <b><u>-a <ARG></b></u> <b>list</b>|<b>status</b>
        "#]],
    );

    check_err::<Cmd>(
        ["cmd", "-a"],
        expect![[r#"
            <red>error</red>: argument `<yellow><i>-a</i></yellow>` requires a value

            <b>usage</b>: <b>cmd</b> [--help] <b><u>-a <ARG></b></u> <b>list</b>|<b>status</b>
        "#]],
    );

    check_err::<Cmd>(
        ["cmd", "-a", "3"],
        expect![[r#"
            <red>error</red>: a subcommand is required

            <b>usage</b>: <b>cmd</b> [--help] <b>-a <ARG></b> <u><b>list</b></u>|<u><b>status</b></u>
        "#]],
    );
}

#[test]
fn default_discovery() {
    #[derive(Debug, PartialEq, Command)]
    #[larpa(crate = "crate", name = "larpa-test-cmd-without-subcommands")]
    enum Cmd {
        #[larpa(fallback, discover)]
        Fallback(Vec<OsString>),
    }

    check_err::<Cmd>(
        ["cmd"],
        expect![[r#"
            <red>error</red>: a subcommand is required

            <b>usage</b>: <b>cmd</b> <u><b>...</b></u>
        "#]],
    );
}

#[test]
fn custom_discovery() {
    #[derive(Debug, PartialEq, Command)]
    #[larpa(crate = "crate", name = "cmd")]
    enum Cmd {
        #[larpa(fallback, discover = "custom_discovery")]
        Fallback(Vec<OsString>),
    }

    fn custom_discovery(_desc: &CommandDesc) -> Vec<DiscoveredSubcommand> {
        vec![DiscoveredSubcommand::new("dynamic")]
    }

    check_err::<Cmd>(
        ["cmd"],
        expect![[r#"
            <red>error</red>: a subcommand is required

            <b>usage</b>: <b>cmd</b> <u><b>dynamic</b></u>|<u><b>...</b></u>
        "#]],
    );
}

#[test]
fn nested() {
    #[derive(Debug, PartialEq, Command)]
    #[larpa(crate = "crate")]
    enum Cmd {
        Sub1(Sub1),
    }

    #[derive(Debug, PartialEq, Command)]
    #[larpa(crate = "crate")]
    enum Sub1 {
        Sub2(Sub2),
    }

    #[derive(Debug, PartialEq, Command)]
    #[larpa(crate = "crate")]
    enum Sub2 {
        Done,
    }

    Cmd::from_iter(["top-level", "sub1", "sub2", "done"]);

    check_err::<Cmd>(
        ["cmd", "sub1", "sub2", "wrong"],
        expect![[r#"
            <red>error</red>: unknown subcommand `wrong`

            <b>usage</b>: <b>cmd sub1 sub2</b> <u><b>done</b></u>
        "#]],
    );
    check_err::<Cmd>(
        ["cmd", "sub1", "sub2"],
        expect![[r#"
            <red>error</red>: a subcommand is required

            <b>usage</b>: <b>cmd sub1 sub2</b> <u><b>done</b></u>
        "#]],
    );
    check_err::<Cmd>(
        ["cmd", "sub1"],
        expect![[r#"
            <red>error</red>: a subcommand is required

            <b>usage</b>: <b>cmd sub1</b> <u><b>sub2</b></u>
        "#]],
    );
    check_err::<Cmd>(
        ["cmd"],
        expect![[r#"
            <red>error</red>: a subcommand is required

            <b>usage</b>: <b>cmd</b> <u><b>sub1</b></u>
        "#]],
    );
}
