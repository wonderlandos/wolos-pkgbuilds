// SPDX-License-Identifier: GPL-2.0-or-later
//! Everyday userland from the real-GPU run: sudo for administrators, zsh for every account, user
//! folders, unlocked keyring, printing, removable media and default apps (Helium the browser).
//! Scripts run on the host with fakes.
#![allow(dead_code, unused_imports)]
#[path = "support/siblings.rs"]
mod siblings;

use std::{fs, os::unix::fs::PermissionsExt, path::Path, path::PathBuf, process::Command};
fn repo(p: &str) -> PathBuf {
    siblings::root()
        .join(p)
}

fn read(p: &str) -> String {
    fs::read_to_string(repo(p)).unwrap_or_else(|e| panic!("{p}: {e}"))
}

#[test]
fn administrators_can_sudo() {
    // Setup made an admin (group wheel) but the stock sudoers has no %wheel rule.
    let p = "desktop/overlay/etc/sudoers.d/wolos";
    let rules: Vec<String> = read(p)
        .lines()
        .filter(|l| !l.starts_with('#') && !l.trim().is_empty())
        .map(String::from)
        .collect();
    assert_eq!(rules, ["%wheel ALL=(ALL:ALL) ALL"]);
    assert!(
        read("wolos-system/PKGBUILD")
            .contains(r#"chmod 440 "$pkgdir/etc/sudoers.d/wolos""#),
        "sudoers drop-in must be 0440"
    );
    // sudo-rs reads /etc/sudoers, ours (sudo_works_on_every_system).
    // visudo validates the syntax where the host has it.
    if let Ok(out) = Command::new("visudo").arg("-csf").arg(repo(p)).output() {
        assert!(
            out.status.success(),
            "{}",
            String::from_utf8_lossy(&out.stderr)
        );
    }
}

/// sudo-rs ships no /etc/sudoers and no PAM service: without ours every sudo fails, and with it
/// the disk page of the live installer ("No disk found") and an administrator's first command.
#[test]
fn sudo_works_on_every_system() {
    let sudoers = read("desktop/overlay/etc/sudoers");
    assert!(sudoers.lines().any(|l| l == "@includedir /etc/sudoers.d"));
    for service in ["sudo", "sudo-i"] {
        let pam = read(&format!("desktop/overlay/etc/pam.d/{service}"));
        for kind in ["auth", "account", "session"] {
            assert!(
                pam.lines()
                    .any(|l| l.starts_with(kind) && l.ends_with("system-auth")),
                "{service}: {kind}"
            );
        }
    }
    let pkg = read("wolos-system/PKGBUILD");
    assert!(pkg.contains(r#"chmod 440 "$pkgdir/etc/sudoers""#));
    assert!(pkg.contains("backup=(etc/sudoers "));
}

#[test]
fn files_offers_clamav_and_the_action_can_only_start_wol_analyze() {
    let root = std::path::siblings::root();
    let read = |p: &str| std::fs::read_to_string(root.join(p)).unwrap();
    let c = read("desktop/nautilus/wolos-analyze.c");
    assert!(c.contains("\"Analyze with ClamAV\""));
    assert!(c.contains("\"wol-analyze\""));
    let rust = read("tools/build-rust.sh");
    assert!(rust.contains(" wol-analyze"));
    assert!(rust.contains("nautilus/wolos-analyze.c"));
    let pkg = read("wolos-system/PKGBUILD");
    assert!(
        pkg.lines()
            .any(|l| l.starts_with("makedepends=") && l.contains("nautilus"))
    );
    // Off stays off: nothing here may reach the switch.
    let window = read("crates/shell/src/ui/analyze.rs");
    assert!(!window.contains("malware-signatures"));
    assert!(!c.contains("malware-signatures"));
}
