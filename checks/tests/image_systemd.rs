// SPDX-License-Identifier: GPL-2.0-or-later
//! WolOS on systemd (ADR 0016): the rc.d scripts became units in files/usr/lib/systemd, devd
//! became udev, cron became a timer, dhclient hooks became a NetworkManager dispatcher script,
//! sysctl.conf.local became sysctl.d. Scripts run against fakes; units are checked key by key.
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

/// One picture from power-on to the desktop (ADR 0012): plymouth with the WolOS theme, quiet kernel.
#[test]
fn boot_shows_only_the_wolos_splash() {
    let p = read("files/usr/share/plymouth/themes/wolos/wolos.plymouth");
    assert!(
        p.contains("ModuleName=script")
            && p.contains("ScriptFile=/usr/share/plymouth/themes/wolos/wolos.script")
    );
    let s = read("files/usr/share/plymouth/themes/wolos/wolos.script");
    assert!(
        s.contains("Image(\"wolos.png\")") && s.contains("Window.SetBackgroundTopColor(0, 0, 0)")
    );
    let i = read("wolos-system/wolos-system.install");
    assert!(i.contains("plymouth-set-default-theme wolos"));
    // The theme is set by the scriptlet, so plymouth must be there first (pacstrap order), and
    // an upgrade sets it where it never took.
    let pb = read("wolos-system/PKGBUILD");
    let deps = pb.lines().find(|l| l.starts_with("depends=(")).unwrap();
    assert!(
        deps.split(['(', ' ', ')']).any(|d| d == "plymouth"),
        "{deps}"
    );
    let up = i.split("post_upgrade()").nth(1).unwrap();
    assert!(up.contains("plymouth-set-default-theme wolos"), "{up}");
}
