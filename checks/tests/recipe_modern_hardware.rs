// SPDX-License-Identifier: GPL-2.0-or-later
//! IMAGE-F (lead order 2026-11-04, "use advanced drivers of Intel etc."): the recipe side of the
//! modern-laptop hardware set. wolos-system's preset turns on thermald (Intel's cooling helper);
//! fprintd (fingerprint) and bolt (Thunderbolt) are D-Bus activated and stay out of the preset, so
//! nothing runs until a reader or a dock is there. The image lists (wolos-iso, sibling) carry the
//! packages; `wol doctor hardware` (wolos, sibling) reads these services.
#![allow(dead_code, unused_imports)]
#[path = "support/siblings.rs"]
mod siblings;

use std::{fs, path::PathBuf, process::Command};

fn repo(p: &str) -> PathBuf {
    siblings::root().join(p)
}

fn read(p: &str) -> String {
    fs::read_to_string(repo(p)).unwrap_or_else(|e| panic!("{p}: {e}"))
}

fn preset_lines() -> Vec<String> {
    read("wolos-system/51-wolos-image.preset")
        .lines()
        .filter(|l| !l.starts_with('#') && !l.trim().is_empty())
        .map(String::from)
        .collect()
}

#[test]
fn thermald_is_on_and_the_dbus_activated_services_are_not_in_the_preset() {
    let p = preset_lines();
    assert!(p.iter().any(|l| l == "enable thermald.service"), "{p:?}");
    assert!(p.iter().any(|l| l == "enable power-profiles-daemon.service"), "{p:?}");
    for svc in ["fprintd", "boltd", "bolt"] {
        assert!(
            !p.iter().any(|l| l.contains(svc)),
            "{svc} is D-Bus activated: a preset line would start it with no reader or dock"
        );
    }
    // One power daemon: nothing in the preset turns on a second one.
    for other in ["tlp", "auto-cpufreq", "system76-power"] {
        assert!(!p.iter().any(|l| l.contains(other)), "{other}");
    }
}

#[test]
fn the_recipe_pins_the_preset_it_ships() {
    let pkg = read("wolos-system/PKGBUILD");
    let sums = Command::new("sha256sum")
        .arg(repo("wolos-system/51-wolos-image.preset"))
        .output()
        .unwrap();
    let want = String::from_utf8_lossy(&sums.stdout);
    let want = want.split_whitespace().next().unwrap();
    assert!(
        pkg.contains(&format!("'{want}'")),
        "wolos-system/PKGBUILD sha256sums does not carry the preset's {want}"
    );
}

#[test]
fn the_services_come_from_packages_on_the_image() {
    let mut img = String::new();
    for f in ["profile/packages.x86_64", "profile/packages.desktop"] {
        for l in read(f).lines() {
            img.push_str(l.split('#').next().unwrap());
            img.push('\n');
        }
    }
    for pkg in ["thermald", "fprintd", "bolt", "power-profiles-daemon"] {
        assert!(img.split_whitespace().any(|p| p == pkg), "wolos-iso profile lacks {pkg}");
    }
}

/// The kernel has what the `wol doctor hardware` rows read: the modern speed controls
/// (`cpu-power`: intel_pstate, amd-pstate), Thunderbolt and USB4 (`thunderbolt`), Intel's thermal
/// control (`cooling`), and the DRM drivers whose firmware `firmware` looks for.
#[test]
fn the_kernel_has_the_drivers_the_doctor_rows_read() {
    let cfg = read("linux-wolos/config.x86_64");
    let on = |k: &str| {
        cfg.lines()
            .any(|l| l == format!("{k}=y") || l == format!("{k}=m"))
    };
    for k in [
        "CONFIG_X86_INTEL_PSTATE",
        "CONFIG_X86_AMD_PSTATE",
        "CONFIG_USB4",
        "CONFIG_THERMAL",
        "CONFIG_INTEL_TCC",
        "CONFIG_DRM_I915",
        "CONFIG_DRM_XE",
        "CONFIG_DRM_AMDGPU",
        "CONFIG_BT_INTEL",
        "CONFIG_IWLWIFI",
        "CONFIG_SND_SOC_SOF_TOPLEVEL",
    ] {
        assert!(on(k), "linux-wolos/config.x86_64 lacks {k}");
    }
}
