// SPDX-License-Identifier: GPL-2.0-or-later
//! What this file checks (wolos-pkgbuilds): Recipes of the Hyprland fork, wolos-system and the one versioned wolos-base.
//! The Arch Linux image (ADR 0016): mkarchiso profile `profile`, pinned to one Arch Linux Archive
//! date, `[wolos]` before the Arch repos, linux-wolos only, the `[wolos]` repo build and the release
//! manifest. Scripts run against fakes; nothing here needs root or the network.
#![allow(dead_code, unused_imports)]
#[path = "support/siblings.rs"]
mod siblings;

use std::{
    fs,
    path::{Path, PathBuf},
    process::Command,
};
fn repo(p: &str) -> PathBuf {
    siblings::root().join(p)
}

fn read(p: &str) -> String {
    fs::read_to_string(repo(p)).unwrap_or_else(|e| panic!("{p}: {e}"))
}

#[test]
fn hyprland_fork_and_system_package_recipes() {
    let h = read("hyprland-wolos/PKGBUILD");
    for want in [
        "pkgname=hyprland-wolos",
        "provides=(hyprland",
        "conflicts=(hyprland",
        "pkgver=0.57.1",
        "patch -Np1",
    ] {
        assert!(h.contains(want), "hyprland-wolos lacks {want}");
    }
    // The patches are local sources beside the PKGBUILD (a clean chroot sees nothing else): every
    // 0*.patch beside it is listed, and the recipe reaches no sibling checkout.
    let dir = repo("hyprland-wolos");
    let mut n = 0;
    for e in fs::read_dir(&dir).unwrap() {
        let name = e.unwrap().file_name().into_string().unwrap();
        if name.ends_with(".patch") {
            n += 1;
            assert!(
                h.lines().any(|l| l.trim() == name),
                "hyprland-wolos source lacks {name}"
            );
        }
    }
    assert!(n > 0, "no patches in {}", dir.display());
    assert!(
        !h.contains("$startdir\"/.."),
        "hyprland-wolos reaches outside its directory"
    );
    let w = read("wolos-system/PKGBUILD");
    for want in [
        "pkgname=wolos-system",
        "build-rust.sh",
        "desktop/overlay",
        "files",
        "build-plugins.sh",
        "hyprland-wolos",
        // The notices of the third-party code the shell carries travel with the package.
        "install -Dm644 \"$_repo/docs/THIRD-PARTY.md\" \"$pkgdir/usr/share/licenses/$pkgname/THIRD-PARTY.md\"",
    ] {
        assert!(w.contains(want), "wolos-system lacks {want}");
    }
    for gone in [
        "image/hyprland/build-fork.sh",
        "image/hyprland/port-files",
        "image/fetch-theme.sh",
        "image/iso.sh",
        "image/installerconfig",
        "image/rcconf.sh",
    ] {
        assert!(!repo(gone).exists(), "{gone} still present");
    }
}

/// wolos-base (ADR 0017): kernel + userland at the release version; non-GNU tools stand in for the
/// GNU ones pacman knows (`provides`/`conflicts`), so `base` still resolves.
#[test]
fn wolos_base_is_one_versioned_base() {
    let b = read("wolos-base/PKGBUILD");
    for want in [
        "pkgname=wolos-base",
        "CHANGELOG.md",
        "'base'",
        "'linux-wolos'",
        "'linux-wolos-headers'",
        "'linux-wolos-api-headers'",
        "'uutils-coreutils'",
        "'sudo-rs'",
        "'libarchive'",
        "'zsh'",
        "'systemd'",
        "'wolos-system'",
        "provides=('coreutils=9.12' 'sudo' 'VIRTUALBOX-GUEST-MODULES')",
        "conflicts=('coreutils' 'sudo')",
        "usr/share/wolos/base.manifest",
        "os-release",
    ] {
        assert!(b.contains(want), "wolos-base lacks {want}");
    }
    for keep in ["glibc", "gcc-libs", "bash"] {
        assert!(!b.contains(&format!("conflicts=('{keep}")), "{keep} stays");
    }
    // makepkg resolves no `../` source: the tree's os-release is read from $startdir.
    assert!(
        !b.lines()
            .any(|l| l.starts_with("source=") && l.contains("../"))
    );
    // From the pinned wolos source, not a sibling checkout.
    assert!(
        b.contains("files/usr/share/wolos/os-release\"")
            && b.contains("\"$srcdir/os-release-$_wolos_commit\"")
    );
    assert!(
        b.contains("_os_release_sha256="),
        "os-release has a checksum"
    );
    let os = read("files/usr/share/wolos/os-release");
    assert!(os.contains("ID=wolos") && os.contains("ID_LIKE=arch"));
    assert!(
        os.contains("VERSION_ID=@VERSION@"),
        "filled by the base build"
    );
}
