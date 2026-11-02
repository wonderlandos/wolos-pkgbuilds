// SPDX-License-Identifier: GPL-2.0-or-later
//! What this file checks (wolos-pkgbuilds): The wopkg recipe: library, header, completions and recipes in the package.
//! wopkg (ADR 0021) ships in the image: built in the `[wolos]` repo from `wopkg/` and
//! `crates/wopkg`, installed by the image, with libwopkg, its header, completions for zsh, bash and
//! fish, and the official recipes with their index.
#![allow(dead_code, unused_imports)]
#[path = "support/siblings.rs"]
mod siblings;

use std::{fs, path::PathBuf};
fn read(p: &str) -> String {
    let path = siblings::root()
        .join(p);
    fs::read_to_string(&path).unwrap_or_else(|e| panic!("{p}: {e}"))
}

#[test]
fn the_package_ships_library_header_completions_and_recipes() {
    let p = read("wopkg/PKGBUILD");
    for want in [
        "pkgname=wopkg",
        "/usr/bin/wopkg",
        "/usr/lib/libwopkg.so.1",
        "/usr/include/wopkg.h",
        "/usr/share/zsh/site-functions/_wopkg",
        "/usr/share/bash-completion/completions/wopkg",
        "/usr/share/fish/vendor_completions.d/wopkg.fish",
        "/usr/share/wopkg/recipes/",
        "recipe index recipes",
        "crates/wopkg/link.sh",
        "crystal spec",
    ] {
        assert!(p.contains(want), "PKGBUILD lacks {want}");
    }
    // Builds run in a sandbox: bubblewrap is a hard dependency.
    let deps = p.lines().find(|l| l.starts_with("depends=(")).unwrap();
    assert!(deps.contains("bubblewrap"), "{deps}");
    // Every recipe the package installs exists and pins its sources.
    let dir = siblings::root().join("wopkg/recipes");
    let mut n = 0;
    for e in fs::read_dir(&dir).unwrap().flatten() {
        let r = e.path().join("recipe.toml");
        let t = fs::read_to_string(&r).unwrap();
        assert!(
            t.contains("sha256 = \"") && t.contains("url = \"https://"),
            "{}",
            r.display()
        );
        n += 1;
    }
    assert!(n >= 3, "bend, hvm, zenc");
}
