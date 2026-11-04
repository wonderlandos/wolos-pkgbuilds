// SPDX-License-Identifier: GPL-2.0-or-later
//! What this file checks (wolos-pkgbuilds): the image has NetworkManager's OpenVPN plugin. WolOS
//! imports `.ovpn` files with `nmcli connection import type openvpn` (wolos core/src/vpn.rs,
//! Settings > Network > Add VPN, `wol vpn import`); without the plugin
//! (/usr/lib/NetworkManager/VPN/nm-openvpn-service.name) the import is refused (NET-3, QA-NET-21).
#![allow(dead_code, unused_imports)]
#[path = "support/siblings.rs"]
mod siblings;

use std::process::Command;

#[test]
fn openvpn_import_has_its_plugin_in_the_image() {
    let out = Command::new("bash")
        .current_dir(siblings::root().join("wolos-system"))
        .arg("-c")
        .arg("source ./PKGBUILD; printf '%s\\n' \"${depends[@]}\"")
        .output()
        .unwrap();
    let depends = String::from_utf8_lossy(&out.stdout);
    for want in ["networkmanager-openvpn", "openvpn"] {
        assert!(
            depends.lines().any(|d| d.trim() == want),
            "{want} not in wolos-system's depends: {depends}"
        );
    }
}
