// SPDX-License-Identifier: GPL-2.0-or-later
//! What this file checks (wolos-pkgbuilds): The libwol recipe: the shared library, its link and the header in the package.
//! libwol (ADR 0020) is the app API every language links: C, the Cryswol shard (ADR 0036), Swift and
//! .NET. The recipe builds `crates/wol-abi` and links `libwol.so.1` with `link.sh`, so the exports
//! and symbol versions come only from `abi/wol.map`; it installs the library, the `libwol.so` link
//! for `-lwol` and `wol.h`. pkgver is the ABI version (apps depend on `libwol>=1.N`), kept in step by
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

#[test]
fn the_package_ships_library_link_and_header() {
    let p = read("libwol/PKGBUILD");
    for want in [
        "pkgname=libwol",
        "cargo build -p wol-abi --release --locked",
        "crates/wol-abi/link.sh",
        "/usr/lib/libwol.so.1",
        "ln -s libwol.so.1 \"$pkgdir/usr/lib/libwol.so\"",
        "crates/wol-abi/include/wol.h\" \"$pkgdir/usr/include/wol.h\"",
        "LICENSES/BSD-2-Clause.txt",
        "--test abi",
    ] {
        assert!(p.contains(want), "PKGBUILD lacks {want}");
    }
    // A cdylib would export rustc's own symbol list too: the library comes from link.sh only.
    assert!(
        !p.contains("target/release/libwol.so"),
        "use link.sh, not a cdylib"
    );
    // Packages of apps depend on the soname.
    let provides = p.lines().find(|l| l.starts_with("provides=(")).unwrap();
    assert!(provides.contains("libwol.so"), "{provides}");
    let license = p.lines().find(|l| l.starts_with("license=(")).unwrap();
    assert!(license.contains("BSD-2-Clause"), "{license}");
    assert!(
        read("packages")
            .lines()
            .any(|l| l.split_whitespace().take(2).eq(["libwol", "libwol"])),
        "libwol is a line of packages"
    );
}

#[test]
fn pkgver_is_the_abi_version_and_bump_wolos_keeps_it() {
    let p = read("libwol/PKGBUILD");
    let ver = scalar(&p, "pkgver");
    let parts: Vec<u32> = ver
        .split('.')
        .map(|n| n.parse().unwrap_or_else(|_| panic!("pkgver {ver}")))
        .collect();
    assert_eq!(parts.len(), 3, "pkgver {ver} is MAJOR.MINOR.PATCH");
    // The soname is the ABI major (link.sh): libwol.so.1 is ABI 1.x.
    assert_eq!(parts[0], 1, "pkgver {ver} and libwol.so.1");
    // The recipe never claims an ABI newer than the source tree's header.
    let h = read("crates/wol-abi/include/wol.h");
    let def = |n: &str| -> u32 {
        h.lines()
            .find_map(|l| l.strip_prefix(&format!("#define WOL_VERSION_{n} ")))
            .unwrap_or_else(|| panic!("wol.h lacks WOL_VERSION_{n}"))
            .trim()
            .parse()
            .unwrap()
    };
    let head = [def("MAJOR"), def("MINOR"), def("PATCH")];
    assert!(
        parts.as_slice() <= head.as_slice(),
        "pkgver {ver} > wol.h {head:?}"
    );
    let bump = read("tools/bump-wolos.sh");
    assert!(
        bump.contains("libwol/*) ver=$ver_libwol")
            && bump.contains("WOL_VERSION_(MAJOR|MINOR|PATCH)"),
        "bump-wolos.sh sets libwol's pkgver from wol.h"
    );
}
