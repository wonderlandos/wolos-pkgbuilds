// SPDX-License-Identifier: GPL-2.0-or-later
//! IMAGE-F (lead order 2026-11-04, Flathub and Flatpak curation): what a Flatpak app finds around its
//! sandbox on the image. Portals: which backend answers each interface (a sandboxed app has no other
//! way to open a file, read the colour scheme, ask for a secret or open a document); Fonts: the host
//! fonts Flatpak mounts at /run/host/fonts are on the image; Flathub: the remote file the live
//! session and the Store add for an account. Recipe `flatpak-wolos` is in image_flatpak.rs.
//! docs/modules/flathub.md (wolos).
#![allow(dead_code, unused_imports)]
#[path = "support/siblings.rs"]
mod siblings;

use std::{collections::HashMap, fs, path::PathBuf};

fn read(p: &str) -> String {
    let f = siblings::root().join(p);
    fs::read_to_string(&f).unwrap_or_else(|e| panic!("{p}: {e}"))
}

fn image() -> Vec<String> {
    let mut v = Vec::new();
    for f in ["profile/packages.x86_64", "profile/packages.desktop"] {
        for l in read(f).lines() {
            let l = l.split('#').next().unwrap().trim();
            v.extend(l.split_whitespace().map(String::from));
        }
    }
    v
}

/// `[preferred]` of the portals configuration: key -> value (comments cut).
fn portals() -> HashMap<String, String> {
    let t = read("desktop/overlay/etc/xdg/xdg-desktop-portal/hyprland-portals.conf");
    assert!(t.lines().any(|l| l.trim() == "[preferred]"));
    t.lines()
        .map(|l| l.split('#').next().unwrap().trim())
        .filter_map(|l| l.split_once('='))
        .map(|(k, v)| (k.trim().to_owned(), v.trim().to_owned()))
        .collect()
}

#[test]
fn every_portal_interface_a_sandboxed_app_uses_has_a_named_backend() {
    let p = portals();
    assert_eq!(p["default"], "hyprland;gtk", "screen casting and shots from the compositor, the rest from gtk");
    // File dialogs, the colour scheme and accent (libadwaita and Flatpak apps follow `wol theme`
    // through gsettings), the app chooser behind "Open with" and printing: the gtk backend.
    for i in ["FileChooser", "Settings", "AppChooser", "Print"] {
        assert_eq!(p[&format!("org.freedesktop.impl.portal.{i}")], "gtk", "{i}");
    }
    // Passwords of browsers and chat apps: gnome-keyring implements the Secret portal; neither
    // hyprland nor gtk does, so without a line an app asking gets no answer.
    assert_eq!(p["org.freedesktop.impl.portal.Secret"], "gnome-keyring");
}

#[test]
fn the_backends_and_the_fonts_are_on_the_image() {
    let img = image();
    for want in [
        "flatpak",
        "xdg-desktop-portal",
        "xdg-desktop-portal-gtk",
        "xdg-desktop-portal-hyprland",
        "gnome-keyring",
        // /usr/share/fonts is mounted into every sandbox (/run/host/fonts): these are what Flatpak
        // apps get for the interface, code, emoji and CJK text.
        "inter-font",
        "ttf-jetbrains-mono",
        "noto-fonts",
        "noto-fonts-emoji",
        "noto-fonts-cjk",
    ] {
        assert!(img.iter().any(|p| p == want), "image lacks {want}");
    }
}

#[test]
fn the_flathub_remote_is_the_one_the_code_adds() {
    let f = read("desktop/overlay/etc/flatpak/remotes.d/flathub.flatpakrepo");
    assert!(f.contains("Url=https://dl.flathub.org/repo/"), "{f}");
    assert!(f.contains("GPGKey="), "the remote is signed: {f}");
    let code = read("crates/core/src/flatpak.rs");
    assert!(code.contains("https://dl.flathub.org/repo/flathub.flatpakrepo"));
    assert!(code.contains("pub const REMOTE: &str = \"flathub\";"));
}

/// The image lists `flatpak`; the [wolos] repo's fork replaces it (same name through `provides`),
/// so the portals and the consent helper above talk to the fork's sandbox.
#[test]
fn the_fork_stands_in_for_the_flatpak_the_image_lists() {
    let p = read("flatpak-wolos/PKGBUILD");
    assert!(p.contains("conflicts=(flatpak)"), "{p}");
    assert!(p.contains("provides=(flatpak="), "the image's `flatpak` line resolves to the fork");
    assert!(image().iter().any(|i| i == "flatpak"));
}
