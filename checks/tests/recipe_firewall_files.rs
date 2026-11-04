// SPDX-License-Identifier: GPL-2.0-or-later
//! What this file checks (wolos-pkgbuilds): the firewall rule files settings write are the
//! person's after install. Settings that open a port (Remote Login, AirPlay Receiver) write their
//! file in /etc/wolos/nftables.d; wolos-system ships each one closed, so each must be in its
//! backup=, or an upgrade puts the closed file back while the service stays on (QA-NET-5, PRIV-5).
#![allow(dead_code, unused_imports)]
#[path = "support/siblings.rs"]
mod siblings;

use std::{fs, path::PathBuf, process::Command};

fn repo(p: &str) -> PathBuf {
    siblings::root().join(p)
}

#[test]
fn every_shipped_firewall_rule_file_is_kept_across_upgrades() {
    let out = Command::new("bash")
        .current_dir(repo("wolos-system"))
        .arg("-c")
        .arg("source ./PKGBUILD; printf '%s\\n' \"${backup[@]}\"")
        .output()
        .unwrap();
    let backup = String::from_utf8_lossy(&out.stdout);
    let dir = repo("desktop/overlay/etc/wolos/nftables.d");
    let mut names: Vec<String> = fs::read_dir(&dir)
        .unwrap_or_else(|e| panic!("{}: {e}", dir.display()))
        .flatten()
        .map(|e| e.file_name().to_string_lossy().into_owned())
        .filter(|n| n.ends_with(".conf"))
        .collect();
    names.sort();
    for want in ["airplay.conf", "ssh.conf"] {
        assert!(
            names.iter().any(|n| n == want),
            "{want} not shipped: {names:?}"
        );
    }
    for n in names {
        let want = format!("etc/wolos/nftables.d/{n}");
        assert!(backup.lines().any(|l| l == want), "{want} not in backup=");
    }
}
