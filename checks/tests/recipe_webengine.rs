// SPDX-License-Identifier: GPL-2.0-or-later
//! What this file checks (wolos-pkgbuilds): The webengine-wolos recipe: the Rust browser engine at one pinned commit, built once into libwolwebengine.so.1.
//! ADR 0042 of wolos: the web-app runtime's engine is pinned by commit and built once per pin; apps
//! load the one library (crates/webengine of wolos), never a copy of their own. The recipe takes the
//! engine as a checksummed tarball at `_engine_rev` (never a git checkout: cargo cannot read the
//! engine's git index), its prebuilt JavaScript engine as a checksummed file, and the shim from wolos
//! at `_wolos_commit`; it builds the shim inside the engine's workspace so the engine's `Cargo.lock`
//! decides every version (prepare() refuses any other change to it), links with `link.sh` so only
//! `wolwebengine.map` is exported, runs the adaptor's engine tests in check(), and installs the
//! library, its link, the header and both licences.
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

fn is_sha1(s: &str) -> bool {
    s.len() == 40 && s.bytes().all(|b| b.is_ascii_hexdigit() && !b.is_ascii_uppercase())
}

#[test]
fn the_engine_is_pinned_by_commit_and_checksum() {
    let p = read("webengine-wolos/PKGBUILD");
    assert!(p.contains("pkgname=webengine-wolos"));
    let engine = scalar(&p, "_engine_rev");
    assert!(is_sha1(engine), "_engine_rev {engine} is a full commit id");
    assert!(is_sha1(scalar(&p, "_wolos_commit")), "_wolos_commit is a full commit id");
    // The engine as a tarball at the commit, never a git source (or a branch or tag).
    assert!(
        p.contains("servo-$_engine_rev.tar.gz::https://github.com/servo/servo/archive/$_engine_rev.tar.gz"),
        "the engine comes as the tarball of _engine_rev"
    );
    assert!(!p.contains("git+https://github.com/servo/"), "no git checkout of the engine");
    // Every source not from git has a real checksum: the engine and the JavaScript engine.
    let sums = p.split("sha256sums=(").nth(1).expect("sha256sums").split(')').next().unwrap();
    let hex: Vec<&str> = sums
        .split(|c: char| c.is_whitespace() || c == '\'')
        .filter(|w| w.len() == 64 && w.bytes().all(|b| b.is_ascii_hexdigit()))
        .collect();
    assert_eq!(hex.len(), 2, "two checksummed files: {sums}");
    assert!(p.contains("export MOZJS_ARCHIVE=$srcdir/libmozjs-$_mozjs"), "the pinned JS engine, not a download at build time");
}

#[test]
fn the_shim_builds_in_the_engines_workspace_under_its_lock() {
    let p = read("webengine-wolos/PKGBUILD");
    for want in [
        "cp -r \"$srcdir/wolos/crates/webengine/.\" ports/wolwebengine/",
        "\"ports/wolwebengine/engine\",",
        "cp Cargo.lock \"$srcdir/Cargo.lock.pin\"",
        "_lock_without_shim \"$srcdir/Cargo.lock.pin\"",
        "cargo build -p wolwebengine --release --frozen",
        "ports/wolwebengine/engine/link.sh \"$srcdir/target/release/libwolwebengine.a\"",
    ] {
        assert!(p.contains(want), "PKGBUILD lacks {want}");
    }
    // A cdylib would export the C symbols of every engine dependency: link.sh, from the map only.
    assert!(!p.contains("target/release/libwolwebengine.so"), "use link.sh, not a cdylib");
    // The shim's own sources say the same (wolos at the pin).
    let manifest = read("crates/webengine/engine/Cargo.toml");
    assert!(manifest.contains("crate-type = [\"staticlib\"]"), "{manifest}");
    let link = read("crates/webengine/engine/link.sh");
    assert!(link.contains("--version-script=\"$here/wolwebengine.map\""));
    assert!(link.contains("-soname,libwolwebengine.so.1"));
}

#[test]
fn check_runs_the_engine_tests_and_the_export_list() {
    let p = read("webengine-wolos/PKGBUILD");
    let check = p.split("check() {").nth(1).expect("check()").split("\n}\n").next().unwrap();
    for want in [
        "wolwebengine.map",
        "nm -D --defined-only",
        "SONAME.*\\[libwolwebengine.so.1\\]",
        "WOL_WEBENGINE_REQUIRED=1",
        "cargo test -p webengine --release --locked",
    ] {
        assert!(check.contains(want), "check() lacks {want}");
    }
}

#[test]
fn the_package_ships_library_link_header_and_both_licences() {
    let p = read("webengine-wolos/PKGBUILD");
    let pkg = p.split("package() {").nth(1).expect("package()");
    for want in [
        "\"$pkgdir/usr/lib/libwolwebengine.so.1\"",
        "ln -s libwolwebengine.so.1 \"$pkgdir/usr/lib/libwolwebengine.so\"",
        "crates/webengine/engine/include/wolwebengine.h\" \\\n    \"$pkgdir/usr/include/wolwebengine.h\"",
        "\"$_engine/LICENSE\" \"$pkgdir/usr/share/licenses/$pkgname/LICENSE-engine\"",
        "LICENSES/BSD-2-Clause.txt",
    ] {
        assert!(pkg.contains(want), "package() lacks {want}");
    }
    let provides = p.lines().find(|l| l.starts_with("provides=(")).unwrap();
    assert!(provides.contains("libwolwebengine.so"), "{provides}");
    let license = p.lines().find(|l| l.starts_with("license=(")).unwrap();
    for l in ["MPL-2.0", "BSD-2-Clause"] {
        assert!(license.contains(l), "{license} lacks {l}");
    }
    // What link.sh links, the package pulls in.
    let depends = p.lines().find(|l| l.starts_with("depends=(")).unwrap();
    let link = read("crates/webengine/engine/link.sh");
    for (flag, pkg) in [("-lfontconfig", "fontconfig"), ("-lz", "zlib"), ("-lstdc++", "gcc-libs")] {
        assert!(link.contains(flag), "link.sh lacks {flag}");
        assert!(depends.split(['(', ')', ' ']).any(|w| w == pkg), "{depends} lacks {pkg}");
    }
}
