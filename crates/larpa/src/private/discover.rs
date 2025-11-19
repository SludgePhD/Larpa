use std::{env, fs};

use crate::desc::{CommandDesc, DiscoveredSubcommand};

/// The default subcommand discovery method, used by `#[larpa(discover)]`.
pub fn discover_subcommands(desc: &CommandDesc) -> Vec<DiscoveredSubcommand> {
    let start = format!("{}-", desc.canonical_name());

    let Some(path) = env::var_os("PATH") else {
        return Vec::new();
    };

    // Not using a `BTreeMap` here due to code size impact.
    let mut out: Vec<DiscoveredSubcommand> = Vec::new();
    for dir in env::split_paths(&path) {
        let Ok(iter) = fs::read_dir(dir) else {
            continue;
        };
        for res in iter {
            let Ok(entry) = res else {
                continue;
            };

            let Ok(name) = entry.file_name().into_string() else {
                continue;
            };
            if let Some(subcmd) = name.strip_prefix(&start) {
                let subcmd = DiscoveredSubcommand::new(subcmd).with_path(entry.path());
                match out.binary_search_by_key(&subcmd.name(), |a| a.name()) {
                    Ok(_) => {}
                    Err(i) => out.insert(i, subcmd),
                }
            }
        }
    }

    out
}
