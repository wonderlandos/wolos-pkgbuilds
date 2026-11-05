// SPDX-License-Identifier: GPL-2.0-or-later
//! What this file checks (wolos-pkgbuilds): The wolkit recipe: the Crystal UI kit shard, its default look, tools and app template in the package.
//! The wolkit shard (ADR 0039) is the Crystal side of libwolkit. The recipe installs it with the
//! shard's own `make install` (the shard under `/usr/lib/wolos/crystal/wolkit`, the template under
//! `/usr/share/wolos/templates/app`), so the layout lives in one place; it depends on the engine
//! and the app API it links. pkgver is the shard's `shard.yml` version, kept in step by
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

const SHARD: &str = "wolos-crystal/shards/wolkit";

#[test]
fn the_package_installs_the_shard_with_its_own_make_install() {
    let p = read("wolkit/PKGBUILD");
    for want in [
        "pkgname=wolkit",
        "wolos-crystal/shards/wolkit\"",
        "make install DESTDIR=\"$pkgdir\" PREFIX=/usr",
        "make spec-core",
        "usr/share/licenses/$pkgname/LICENSE",
    ] {
        assert!(p.contains(want), "PKGBUILD lacks {want}");
    }
    let depends = p.lines().find(|l| l.starts_with("depends=(")).unwrap();
    for d in ["crystal", "cryswol", "libwolkit", "libwol"] {
        assert!(
            // 'crystal>=1.22.0' is the package crystal (RCR-046 pins the compiler's floor)
            depends.split(['(', ')', ' ']).any(|w| w
                .trim_matches('\'')
                .split(['<', '>', '='])
                .next()
                == Some(d)),
            "{depends} lacks {d}"
        );
    }
    let license = p.lines().find(|l| l.starts_with("license=(")).unwrap();
    assert!(license.contains("BSD-2-Clause"), "{license}");
    assert!(
        read("packages").lines().any(|l| {
            let w: Vec<&str> = l.split_whitespace().collect();
            w.len() >= 4
                && w[..2] == ["wolkit", "wolkit"]
                && w[2].split(',').any(|d| d == "libwolkit")
        }),
        "wolkit is a line of packages, built after libwolkit"
    );
}

#[test]
fn make_install_puts_the_shard_styles_tools_and_template_in_place() {
    // What the recipe relies on: the shard's install target copies these, to these places.
    let mk = read(&format!("{SHARD}/Makefile"));
    for want in [
        "SHARD_DIR := $(DESTDIR)$(PREFIX)/lib/wolos/crystal/wolkit",
        "TEMPLATE_DIR := $(DESTDIR)$(PREFIX)/share/wolos/templates",
        "cp -r src bin styles shard.yml LICENSE README.md $(SHARD_DIR)/",
        // QA-PLUG-29: every template `wol app new --template app|cli|tool|ai` copies is installed.
        "TEMPLATES := app cli tool ai",
        "for t in $(TEMPLATES); do cp -r ../../templates/$$t $(TEMPLATE_DIR)/; done",
    ] {
        assert!(mk.contains(want), "{SHARD}/Makefile lacks {want}");
    }
    for f in ["bin/wolkit-new.cr", "styles", "src/wolkit.cr", "LICENSE"] {
        assert!(
            siblings::root().join(SHARD).join(f).exists(),
            "{SHARD}/{f} is missing"
        );
    }
}

#[test]
fn pkgver_is_the_shard_version_and_bump_wolos_keeps_it() {
    let p = read("wolkit/PKGBUILD");
    let shard = read(&format!("{SHARD}/shard.yml"));
    let want = shard
        .lines()
        .find_map(|l| l.strip_prefix("version: "))
        .expect("shard.yml has a version");
    assert_eq!(scalar(&p, "pkgver"), want.trim());
    let bump = read("tools/bump-wolos.sh");
    assert!(
        bump.contains("wolkit/*) ver=$ver_wolkit")
            && bump.contains("wolos-crystal/shards/wolkit/shard.yml"),
        "bump-wolos.sh sets wolkit's pkgver from shard.yml"
    );
}
