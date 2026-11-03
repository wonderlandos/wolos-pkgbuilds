// SPDX-License-Identifier: GPL-2.0-or-later
//! What this file checks (wolos-pkgbuilds): The libwolkit recipe: the shared library, its link, the header and the style compiler in the package.
//! libwolkit (ADR 0039) is the WolOS UI engine the wolkit shard binds and any language can link. The
//! recipe builds `crates/wolkit` and links `libwolkit.so.1` with `link.sh`, so the exports and
//! symbol versions come only from `abi/wolkit.map`; it installs the library, the `libwolkit.so` link
//! for `-lwolkit`, `wolkit.h` and `wolkit-stylec`. pkgver is the ABI version (apps depend on
//! `libwolkit>=1.N`), kept in step by `tools/bump-wolos.sh`.
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

#[test]
fn the_package_ships_library_link_header_and_style_compiler() {
    let p = read("libwolkit/PKGBUILD");
    for want in [
        "pkgname=libwolkit",
        "cargo build -p wolkit --release --locked",
        "crates/wolkit/link.sh",
        "/usr/lib/libwolkit.so.1",
        "ln -s libwolkit.so.1 \"$pkgdir/usr/lib/libwolkit.so\"",
        "crates/wolkit/include/wolkit.h\" \"$pkgdir/usr/include/wolkit.h\"",
        "target/release/wolkit-stylec\" \"$pkgdir/usr/bin/wolkit-stylec\"",
        "LICENSES/BSD-2-Clause.txt",
        "--test abi",
        "--test conformance",
    ] {
        assert!(p.contains(want), "PKGBUILD lacks {want}");
    }
    // A cdylib would export rustc's own symbol list too: the library comes from link.sh only.
    assert!(
        !p.contains("target/release/libwolkit.so"),
        "use link.sh, not a cdylib"
    );
    // Packages of apps depend on the soname.
    let provides = p.lines().find(|l| l.starts_with("provides=(")).unwrap();
    assert!(provides.contains("libwolkit.so"), "{provides}");
    // What link.sh links, the package must pull in.
    let depends = p.lines().find(|l| l.starts_with("depends=(")).unwrap();
    let link = read("crates/wolkit/link.sh");
    for (flag, pkg) in [
        ("-lwayland-client", "wayland"),
        ("-lxkbcommon", "libxkbcommon"),
    ] {
        assert!(link.contains(flag), "link.sh lacks {flag}");
        assert!(
            depends.split(['(', ')', ' ']).any(|w| w == pkg),
            "{depends} lacks {pkg}"
        );
    }
    let license = p.lines().find(|l| l.starts_with("license=(")).unwrap();
    assert!(license.contains("BSD-2-Clause"), "{license}");
    assert!(
        read("packages")
            .lines()
            .any(|l| l.split_whitespace().take(2).eq(["libwolkit", "libwolkit"])),
        "libwolkit is a line of packages"
    );
}

#[test]
fn pkgver_is_the_abi_version_and_bump_wolos_keeps_it() {
    let p = read("libwolkit/PKGBUILD");
    let ver = scalar(&p, "pkgver");
    let parts: Vec<u32> = ver
        .split('.')
        .map(|n| n.parse().unwrap_or_else(|_| panic!("pkgver {ver}")))
        .collect();
    assert_eq!(parts.len(), 3, "pkgver {ver} is MAJOR.MINOR.PATCH");
    // The soname is the ABI major (link.sh): libwolkit.so.1 is ABI 1.x.
    assert_eq!(parts[0], 1, "pkgver {ver} and libwolkit.so.1");
    // The recipe never claims an ABI newer than the source tree's header.
    let h = read("crates/wolkit/include/wolkit.h");
    let def = |n: &str| -> u32 {
        h.lines()
            .find_map(|l| l.strip_prefix(&format!("#define WOLKIT_VERSION_{n} ")))
            .unwrap_or_else(|| panic!("wolkit.h lacks WOLKIT_VERSION_{n}"))
            .trim()
            .parse()
            .unwrap()
    };
    let head = [def("MAJOR"), def("MINOR"), def("PATCH")];
    assert!(
        parts.as_slice() <= head.as_slice(),
        "pkgver {ver} > wolkit.h {head:?}"
    );
    let bump = read("tools/bump-wolos.sh");
    assert!(
        bump.contains("libwolkit/*) ver=$ver_libwolkit")
            && bump.contains("WOLKIT_VERSION_(MAJOR|MINOR|PATCH)"),
        "bump-wolos.sh sets libwolkit's pkgver from wolkit.h"
    );
}
