//! This is a more complicated example than `rosetta.rs`.
//!
//! It implements a git-like command with lots of options and subcommands, but with no
//! documentation whatsoever.

use std::{ffi::OsString, path::PathBuf};

use larpa::{
    Command,
    types::{PrintHelp, PrintVersion},
};

#[allow(dead_code)]
#[derive(Command)]
#[larpa(no_homepage, no_license, no_repository, name = "git")]
struct App {
    #[larpa(name = ["-v", "--version"], flag)]
    _version: PrintVersion,

    #[larpa(name = ["-h", "--help"], flag)]
    _help: PrintHelp,

    #[larpa(name = ["-p", "--paginate"], flag)]
    pager: Option<bool>,

    #[larpa(name = ["-P", "--no-pager"], flag, inverse_of = "pager")]
    no_pager: (),

    #[larpa(name = "--exec-path", flag)]
    exec_path: bool,

    #[larpa(name = "--html-path", flag)]
    html_path: bool,

    #[larpa(name = "--man-path", flag)]
    man_path: bool,

    #[larpa(name = "--info-path", flag)]
    info_path: bool,

    #[larpa(name = "--bare", flag)]
    bare: bool,

    #[larpa(name = "-c")]
    config: Vec<String>,

    #[larpa(name = "--config-env")]
    config_env: Vec<String>,

    #[larpa(name = "--git-dir")]
    git_dir: Option<PathBuf>,

    #[larpa(name = "--work-tree")]
    work_tree: Option<PathBuf>,

    #[larpa(name = "--namespace")]
    namespace: Option<PathBuf>,

    #[larpa(name = "--no-replace-objects", flag)]
    no_replace_objects: bool,

    #[larpa(name = "--no-lazy-fetch", flag)]
    no_lazy_fetch: bool,

    #[larpa(name = "--no-optional-locks", flag)]
    no_optional_locks: bool,

    #[larpa(name = "--no-advice", flag)]
    no_advice: bool,

    #[larpa(name = "--literal-pathspecs", flag)]
    literal_pathspecs: bool,

    #[larpa(name = "--glob-pathspecs", flag)]
    glob_pathspecs: bool,

    #[larpa(name = "--noglob-pathspecs", flag)]
    noglob_pathspecs: bool,

    #[larpa(name = "--icase-pathspecs", flag)]
    icase_pathspecs: bool,

    #[larpa(subcommand)]
    subcommand: Subcommand,
}

#[allow(dead_code)]
#[derive(Command)]
enum Subcommand {
    Add,
    Am,
    Archive,
    Backfill,
    Bisect,
    Branch,
    Bundle,
    Checkout,
    CherryPick,
    Citool,
    Clean,
    Clone,
    Commit,
    Describe,
    Diff,
    Fetch,
    FormatPatch,
    Gc,
    Grep,
    Gui,
    Init,
    Log,
    Maintenance,
    Merge,
    Mv,
    Notes,
    Pull,
    Push,
    RangeDiff,
    Rebase,
    Reset,
    Restore,
    Revert,
    Rm,
    Shortlog,
    Show,
    SparseCheckout,
    Stash,
    Status,
    Submodule(Submodule),
    Switch,
    Tag,
    Worktree,

    Config,
    FastExport,
    FastImport,
    FilterBranch,
    Mergetool,
    PackRefs,
    Prune,
    Reflog,
    Refs,
    Remote,
    Repack,
    Replace,

    Annotate,
    Blame,
    Bugreport,
    CountObjects,
    Diagnose,
    Difftool,
    Fsck,
    Help,
    Instaweb,
    MergeTree,
    Rerere,
    ShowBranch,
    VerifyCommit,
    VerifyTag,
    Version,
    Whatchanged,
    Gitweb,

    Archimport,
    Cvsexportcommit,
    Cvsimport,
    Cvsserver,
    ImapSend,
    P4,
    Quiltimport,
    RequestPull,
    SendEmail,
    Svn,

    #[larpa(fallback, discover)]
    Fallback(Vec<OsString>),
}

#[allow(dead_code)]
#[derive(Command)]
struct Submodule {
    #[larpa(name = "--quiet", flag)]
    quiet: bool,

    #[larpa(subcommand)]
    cmd: SubmoduleCmd,
}

#[derive(Command)]
enum SubmoduleCmd {
    Add,
    Status,
    Init,
    Deinit,
    Update,
    SetBranch,
    SetUrl,
    Summary,
    Foreach,
    Sync,
    Absorbgitdirs,
}

fn main() {
    if cfg!(feature = "from-args") {
        std::hint::black_box(App::from_args());
    }
    if cfg!(feature = "desc") {
        std::hint::black_box(App::DESC);
    }
}
