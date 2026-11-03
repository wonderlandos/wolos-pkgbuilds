// SPDX-License-Identifier: GPL-2.0-or-later
//! What this file checks (wolos-pkgbuilds): The cryswol recipe: the Crystal app framework shard over libwol in the package.
//! The cryswol shard (ADR 0036) is the Crystal side of libwol and the base the wolkit shard is built
//! on. The recipe installs it with the shard's own `make install` (the shard under
//! `/usr/lib/wolos/crystal/cryswol`), so the layout lives in one place; it depends on the app API
//! it links, and wolkit depends on it. pkgver is the shard's `shard.yml` version, kept in step by
//! `tools/bump-wolos.sh`.
#![allow(dead_code, unused_imports)]
#[path = "support/siblings.rs"]
mod siblings;

use std::fs;
fn read(p: &str) -> String {
    let path = siblings::root().join(p);
    fs::read_to_string(&path).unwrap_or_else(|e| panic!("{p}: {e}"))
}

fn scalar<'a>(text: &'a str, name: &str) -> &'a str {
    let pre = format!("{name}=");
    let v = text
        .lines()
        .find_map(|l| l.strip_prefix(&pre))
        .unwrap_or_else(|| panic!("PKGBUILD lacks {name}="));
    v.split_whitespace().next().unwrap_or("")
}

fn array_has(text: &str, name: &str, want: &str) -> bool {
    let pre = format!("{name}=(");
    text.lines()
        .find(|l| l.starts_with(&pre))
        .unwrap_or_else(|| panic!("PKGBUILD lacks {name}=("))
        .split(['(', ')', ' '])
        .any(|w| w == want)
}

/// The `packages` line of NAME, split on whitespace.
fn package_line(name: &str) -> Vec<String> {
    read("packages")
        .lines()
        .map(|l| l.split_whitespace().map(String::from).collect::<Vec<_>>())
        .find(|w| w.len() >= 4 && w[0] == name)
        .unwrap_or_else(|| panic!("{name} is not a line of packages"))
}

const SHARD: &str = "wolos-crystal/shards/cryswol";

#[test]
fn the_package_installs_the_shard_with_its_own_make_install() {
    let p = read("cryswol/PKGBUILD");
    for want in [
        "pkgname=cryswol",
        "wolos-crystal/shards/cryswol\"",
        "make install DESTDIR=\"$pkgdir\" PREFIX=/usr",
        "make fmt-check",
        "usr/share/licenses/$pkgname/LICENSE",
    ] {
        assert!(p.contains(want), "PKGBUILD lacks {want}");
    }
    assert_eq!(scalar(&p, "arch"), "(any)");
    for d in ["crystal", "libwol"] {
        assert!(array_has(&p, "depends", d), "cryswol depends lacks {d}");
    }
    // The default shard is BSD-2-Clause, the opt-in pkg/realtime parts are GPL-2.0-or-later.
    for l in ["BSD-2-Clause", "GPL-2.0-or-later"] {
        assert!(array_has(&p, "license", l), "cryswol license lacks {l}");
    }
    let line = package_line("cryswol");
    assert_eq!(line[1], "cryswol");
    assert!(
        line[2].split(',').any(|d| d == "libwol"),
        "cryswol is built after libwol: {line:?}"
    );
}

#[test]
fn wolkit_depends_on_cryswol_and_is_built_after_it() {
    assert!(
        array_has(&read("wolkit/PKGBUILD"), "depends", "cryswol"),
        "wolkit depends lacks cryswol"
    );
    let line = package_line("wolkit");
    assert!(
        line[2].split(',').any(|d| d == "cryswol"),
        "wolkit lists cryswol as a local dep: {line:?}"
    );
    let names: Vec<String> = read("packages")
        .lines()
        .filter(|l| !l.trim().is_empty() && !l.starts_with('#'))
        .filter_map(|l| l.split_whitespace().next().map(String::from))
        .collect();
    let at = |n: &str| names.iter().position(|x| x == n).unwrap();
    assert!(at("libwol") < at("cryswol") && at("cryswol") < at("wolkit"));
}

#[test]
fn make_install_puts_the_shard_in_place() {
    // What the recipe relies on: the shard's install and check targets, to these places.
    let mk = read(&format!("{SHARD}/Makefile"));
    for want in [
        "SHARD_DIR := $(DESTDIR)$(PREFIX)/lib/wolos/crystal/cryswol",
        "cp -r src shard.yml LICENSE README.md $(SHARD_DIR)/",
        "fmt-check:",
    ] {
        assert!(mk.contains(want), "{SHARD}/Makefile lacks {want}");
    }
    for f in ["src/cryswol.cr", "src/wol.cr", "LICENSE", "README.md"] {
        assert!(
            siblings::root().join(SHARD).join(f).exists(),
            "{SHARD}/{f} is missing"
        );
    }
}

#[test]
fn pkgver_is_the_shard_version_and_bump_wolos_keeps_it() {
    let p = read("cryswol/PKGBUILD");
    let shard = read(&format!("{SHARD}/shard.yml"));
    let want = shard
        .lines()
        .find_map(|l| l.strip_prefix("version: "))
        .expect("shard.yml has a version");
    assert_eq!(scalar(&p, "pkgver"), want.trim());
    let bump = read("tools/bump-wolos.sh");
    assert!(
        bump.contains("cryswol/*) ver=$ver_cryswol")
            && bump.contains("wolos-crystal/shards/cryswol/shard.yml"),
        "bump-wolos.sh sets cryswol's pkgver from shard.yml"
    );
}
