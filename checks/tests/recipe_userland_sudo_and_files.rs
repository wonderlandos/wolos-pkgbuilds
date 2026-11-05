// SPDX-License-Identifier: GPL-2.0-or-later
//! What this file checks (wolos-pkgbuilds): Userland recipes: sudo for administrators on every system, the Files ClamAV action that can only start wol analyze.
//! Everyday userland from the real-GPU run: sudo for administrators, zsh for every account, user
//! folders, unlocked keyring, printing, removable media and default apps (Helium the browser).
//! Scripts run on the host with fakes.
#![allow(dead_code, unused_imports)]
#[path = "support/siblings.rs"]
mod siblings;

use std::{fs, os::unix::fs::PermissionsExt, path::Path, path::PathBuf, process::Command};
fn repo(p: &str) -> PathBuf {
    siblings::root().join(p)
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
    // A refused sudo says why, never with an insult (wolos QA-LIVE3-24).
    assert_eq!(rules, ["%wheel ALL=(ALL:ALL) ALL", "Defaults !insults"]);
    assert!(
        read("wolos-system/PKGBUILD").contains(r#"chmod 440 "$pkgdir/etc/sudoers.d/wolos""#),
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
        // auth is WolOS's own stack (wolos QA-PRIV-13: pam_faillock around pam_unix, the lock
        // screen's limits); account and session are Arch's.
        for module in [
            "pam_faillock.so preauth",
            "pam_unix.so",
            "pam_faillock.so authfail",
        ] {
            assert!(
                pam.lines()
                    .any(|l| l.starts_with("auth") && l.contains(module)),
                "{service}: auth {module}"
            );
        }
        for kind in ["account", "session"] {
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
fn files_offers_the_full_check_and_the_action_can_only_start_wol_analyze() {
    let root = siblings::root();
    let read = |p: &str| std::fs::read_to_string(root.join(p)).unwrap();
    let c = read("desktop/nautilus/wolos-analyze.c");
    // The label names what it does, not whose engine (Wonderland Doctrine).
    assert!(c.contains("#define MENU_LABEL \"Check for Malware\""));
    assert!(
        !c.to_lowercase().contains("clamav"),
        "no engine name in the action"
    );
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

/// E2E-SHARE (ADR 0059, S01): Files gets a "Share…" item from a second extension that starts
/// `wol-share` with the file's path; the same makedepends (nautilus) build it, and the program is
/// one of the binaries the build installs.
#[test]
fn files_offers_share_and_the_extension_is_built_with_wol_share() {
    let root = siblings::root();
    let read = |p: &str| std::fs::read_to_string(root.join(p)).unwrap();
    let c = read("desktop/nautilus/wolos-share.c");
    assert!(c.contains("#define MENU_LABEL \"Share…\""));
    assert!(c.contains("\"wol-share\""));
    let rust = read("tools/build-rust.sh");
    assert!(rust.contains(" wol-share"));
    assert!(rust.contains("nautilus/wolos-share.c"));
    assert!(rust.contains("libwolos-share.so"));
}

/// A9 follow-up: every login-shell profile wolos-system ships in /etc/profile.d (the console
/// prompt, the IME session exports) is in its backup=, so a local edit survives an upgrade and
/// pacman leaves a .pacnew instead of overwriting it.
#[test]
fn every_profile_d_script_is_kept_across_upgrades() {
    let backup = Command::new("bash")
        .current_dir(repo("wolos-system"))
        .arg("-c")
        .arg("source ./PKGBUILD; printf '%s\\n' \"${backup[@]}\"")
        .output()
        .unwrap();
    let backup = String::from_utf8_lossy(&backup.stdout);
    let dir = repo("desktop/overlay/etc/profile.d");
    let mut names: Vec<String> = fs::read_dir(&dir)
        .unwrap_or_else(|e| panic!("{}: {e}", dir.display()))
        .flatten()
        .map(|e| e.file_name().to_string_lossy().into_owned())
        .collect();
    names.sort();
    assert!(names.contains(&"wolos-ime.sh".to_string()), "{names:?}");
    for n in names {
        let want = format!("etc/profile.d/{n}");
        assert!(backup.lines().any(|l| l == want), "{want} not in backup=");
    }
}
