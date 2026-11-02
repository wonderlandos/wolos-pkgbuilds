// SPDX-License-Identifier: GPL-2.0-or-later
//! What this file checks (wolos-pkgbuilds): boot-setup recipes: the sealed system can not enable units at first start.
//! boot-setup with every tool faked: which chain each tier gets and what it asks of the firmware.
#![allow(dead_code, unused_imports)]
#[path = "support/siblings.rs"]
mod siblings;

use std::{
    fs,
    os::unix::fs::PermissionsExt,
    path::{Path, PathBuf},
    process::Command,
};
#[test]
fn the_sealed_system_can_not_enable_units_at_first_start() {
    let root = siblings::root_str();
    let i = std::fs::read_to_string(format!(
        "{root}/wolos-system/wolos-system.install"
    ))
    .unwrap();
    for u in [
        "wolos-verity-check.service",
        "wolos-tpm-bind.service",
        "wolos-boot-ok.service",
    ] {
        assert!(i.contains(u), "{u} is enabled when the package installs");
    }
}
