// SPDX-License-Identifier: GPL-2.0-or-later
//! `march.skip` (IMAGE-F, wolos-build `pkg --march v3`): the recipes a CPU-level flavour must not
//! rebuild. A name that is no package, a missing reason, or a missing kernel or shim line fails.
//! Evidence and the decision: docs/modules/x86-64-v3.md (wolos).
#![allow(dead_code, unused_imports)]
#[path = "support/siblings.rs"]
mod siblings;

use std::{fs, path::PathBuf};

fn read(p: &str) -> String {
    let f = siblings::root().join(p);
    fs::read_to_string(&f).unwrap_or_else(|e| panic!("{p}: {e}"))
}

/// (name, reason) of every line of march.skip.
fn skipped() -> Vec<(String, String)> {
    read("march.skip")
        .lines()
        .filter(|l| !l.trim_start().starts_with('#') && !l.trim().is_empty())
        .map(|l| {
            let (n, why) = l.split_once('#').unwrap_or((l, ""));
            (n.trim().to_owned(), why.trim().to_owned())
        })
        .collect()
}

fn packages() -> Vec<String> {
    read("packages")
        .lines()
        .filter(|l| !l.trim_start().starts_with('#'))
        .filter_map(|l| l.split_whitespace().next().map(String::from))
        .collect()
}

#[test]
fn every_skipped_name_is_a_package_with_a_reason() {
    let pk = packages();
    for (n, why) in skipped() {
        assert!(pk.contains(&n), "march.skip names {n}, which is not in packages");
        assert!(why.len() > 10, "{n}: say why in the comment");
    }
}

#[test]
fn what_runs_before_the_os_or_has_its_own_flavour_is_never_rebuilt() {
    let s: Vec<String> = skipped().into_iter().map(|(n, _)| n).collect();
    for want in ["linux-wolos", "linux-wolos-v3", "wolos-shim", "scx-wolos", "atlas-browser"] {
        assert!(s.iter().any(|n| n == want), "march.skip lacks {want}");
    }
    // The shim is the first thing the firmware runs: it must stay on the baseline whatever CPU level the rest takes.
    assert!(read("wolos-shim/PKGBUILD").contains("arch=(x86_64)"));
    // The kernel flavour is a recipe of its own (the installer picks it by CPU).
    assert!(read("linux-wolos/build.conf").contains("linux-wolos-v3) makepkg_conf=(_march=v3)"));
}
