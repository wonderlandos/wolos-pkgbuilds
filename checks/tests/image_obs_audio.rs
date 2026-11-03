// SPDX-License-Identifier: GPL-2.0-or-later
//! obs-pipewire-audio-capture: the OBS sources for app/desktop/input audio over PipeWire, from the
//! AUR recipe, built at a pinned upstream commit and listed in the [wolos] repo. OBS itself is an
//! online install, so this is a repo package, not an image package (wolos docs/creators-audio.md).
#![allow(dead_code, unused_imports)]
#[path = "support/siblings.rs"]
mod siblings;

use std::{fs, path::PathBuf};
fn repo(p: &str) -> PathBuf {
    siblings::root().join(p)
}

fn read(p: &str) -> String {
    fs::read_to_string(repo(p)).unwrap_or_else(|e| panic!("{p}: {e}"))
}

#[test]
fn recipe_pins_one_upstream_commit_for_obs_33() {
    let p = read("obs-pipewire-audio-capture/PKGBUILD");
    let commit = p
        .lines()
        .find_map(|l| l.strip_prefix("_commit="))
        .map(|v| v.split_whitespace().next().unwrap_or(""))
        .expect("_commit=");
    assert!(
        commit.len() == 40 && commit.chars().all(|c| c.is_ascii_hexdigit()),
        "a full commit id: {commit:?}"
    );
    for want in [
        "pkgname=obs-pipewire-audio-capture",
        "url='https://github.com/dimtpap/obs-pipewire-audio-capture'",
        "source=(\"$pkgname::git+$url.git#commit=$_commit\")",
        "depends=('obs-studio>=33' pipewire)",
        "DESTDIR=\"$pkgdir\" cmake --install build",
        "license=(GPL-2.0-or-later)",
    ] {
        assert!(
            p.lines().any(|l| l.trim() == want),
            "PKGBUILD lacks `{want}`"
        );
    }
    assert!(
        p.contains("-DCMAKE_INSTALL_LIBDIR=lib"),
        "Arch's lib, not lib64"
    );
    assert!(
        !p.contains("archive/refs/tags"),
        "a tag tarball can move; the commit cannot"
    );
    assert!(
        !p.contains("$pkgdir/usr\""),
        "install through DESTDIR, not a prefix inside pkgdir"
    );
}

#[test]
fn the_repo_builds_it_and_the_image_does_not_ship_it() {
    let t = read("packages");
    let line = t
        .lines()
        .find(|l| l.starts_with("obs-pipewire-audio-capture "))
        .expect("obs-pipewire-audio-capture in packages");
    let f: Vec<&str> = line.split_whitespace().collect();
    assert_eq!(f[1], "obs-pipewire-audio-capture");
    assert_eq!(f[2], "-", "no local deps");
    assert!(f[3..].contains(&"obs-pipewire-audio-capture/"), "{line}");
    let img = read("profile/packages.desktop");
    assert!(
        !img.lines().any(|l| {
            let n = l.split_whitespace().next();
            n == Some("obs-studio") || n == Some("obs-pipewire-audio-capture")
        }),
        "OBS and its plugin are online installs"
    );
}
