// SPDX-License-Identifier: GPL-2.0-or-later
//! What this file checks (wolos-pkgbuilds): the packages and image defaults the WolOS code and manual
//! promise (QA100 RC-33, "build what the manual promises"): rsh and ruby for `wol script` (ADR 0041),
//! scx_wolos on sched_ext (QoS), power-profiles-daemon (Energy Mode), fwupd, the CJK fonts with their
//! fontconfig fallbacks, ntsync at boot (Games Doctor), .local names through nss-mdns, the plugin
//! schema at the path ADR 0031 names. The recipes here, the image lists in wolos-iso (sibling).
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

/// The package names of the two image lists (comments cut).
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

/// The first field of every `packages` line.
fn wolos_packages() -> Vec<String> {
    read("packages")
        .lines()
        .filter(|l| !l.trim_start().starts_with('#'))
        .filter_map(|l| l.split_whitespace().next().map(String::from))
        .collect()
}

fn var(p: &str, name: &str) -> String {
    p.lines()
        .find_map(|l| l.strip_prefix(&format!("{name}=")))
        .map(|v| v.split_whitespace().next().unwrap_or("").to_string())
        .unwrap_or_else(|| panic!("no {name}="))
}

#[test]
fn the_image_ships_what_the_code_runs() {
    let img = image();
    for want in [
        "ruby",                  // script.rs: /usr/bin/ruby
        "ruby-stdlib",           // require "minitest", "rake", irb: Ruby as Ruby people know it
        "rsh",                   // .rsh scripts, terminal shell
        "scx-wolos",             // QoS scheduler
        "power-profiles-daemon", // power/policy.rs PROFILE_ARGV powerprofilesctl
        "fwupd",
        "noto-fonts-cjk",
        "nss-mdns",
        "gvfs-mtp",
        "gvfs-gphoto2",
        "gvfs-smb",
    ] {
        assert!(img.iter().any(|p| p == want), "image lacks {want}");
    }
    // The [wolos] ones are built here.
    let ours = wolos_packages();
    for p in ["rsh", "scx-wolos"] {
        assert!(ours.iter().any(|o| o == p), "packages lacks {p}");
    }
}

#[test]
fn rsh_builds_the_tagged_release_by_its_checksum() {
    let p = read("rsh/PKGBUILD");
    assert_eq!(var(&p, "pkgver"), "0.1.3");
    assert!(p.contains("$url/archive/refs/tags/v$pkgver.tar.gz"));
    assert!(p.contains("'6ccb6597707d6d174f0210eb87d7db5b1ab2c97bf75e7f31a0cb5463f1cb1433'"));
    assert!(
        p.contains("check() {") && p.contains(" test\n"),
        "make test runs"
    );
}

#[test]
fn scx_wolos_is_built_against_the_wolkernel_and_on_by_default() {
    let p = read("scx-wolos/PKGBUILD");
    let k = read("linux-wolos/PKGBUILD");
    let v = read("linux-wolos/VERSION");
    let wk = v
        .lines()
        .find_map(|l| l.strip_prefix("wolkernel="))
        .unwrap();
    let lx = v.lines().find_map(|l| l.strip_prefix("linux=")).unwrap();
    assert_eq!(var(&p, "_wk"), wk);
    assert_eq!(var(&p, "_linux"), lx);
    // The same tag tarball as the kernel, by the same checksum (bump-kernel.sh moves it).
    let sum = |t: &str| {
        t.lines()
            .find_map(|l| l.strip_prefix("sha256sums=('"))
            .map(|s| s[..64].to_string())
            .unwrap()
    };
    assert_eq!(
        sum(&p),
        sum(&k),
        "scx-wolos and linux-wolos read one tarball"
    );
    assert!(read("tools/bump-kernel.sh").contains("scx-wolos/PKGBUILD"));
    // The kernel the BTF comes from is a local build dependency, and sched_ext is built in.
    assert!(p.contains("makedepends=(") && p.contains(" linux-wolos "));
    let line = read("packages")
        .lines()
        .find(|l| l.starts_with("scx-wolos "))
        .map(String::from)
        .unwrap();
    assert_eq!(
        line.split_whitespace().nth(2),
        Some("linux-wolos"),
        "{line}"
    );
    assert!(
        read("linux-wolos/config.x86_64")
            .lines()
            .any(|l| l == "CONFIG_SCHED_CLASS_EXT=y")
    );
    assert!(p.contains("kernel/scx_wolos") && p.contains("extract-vmlinux"));
    assert_eq!(
        read("scx-wolos/60-scx-wolos.preset")
            .lines()
            .filter(|l| !l.starts_with('#'))
            .collect::<Vec<_>>(),
        ["enable scx_wolos.service"]
    );
    let i = read("scx-wolos/scx-wolos.install");
    assert!(i.contains("systemctl preset scx_wolos.service"));
    assert!(
        i.contains("[ -d /run/systemd/system ]"),
        "no start in a chroot"
    );
    assert!(i.contains("pre_remove()") && i.contains("disable --now scx_wolos.service"));
}

#[test]
fn wolos_system_ships_the_image_defaults() {
    let p = read("wolos-system/PKGBUILD");
    for want in [
        "\"$pkgdir/usr/share/wolos/plugins.schema.json\"",
        "\"$pkgdir/usr/lib/modules-load.d/ntsync.conf\"",
        "\"$pkgdir/etc/fonts/conf.d/70-wolos-cjk-arabic.conf\"",
        "\"$pkgdir/usr/lib/systemd/system-preset/51-wolos-image.preset\"",
        "\"$pkgdir/usr/lib/wolos/nsswitch-mdns\"",
    ] {
        assert!(p.contains(want), "wolos-system/PKGBUILD lacks {want}");
    }
    // ADR 0031 / crates/shared/src/plugins.rs SCHEMA_PATH.
    assert!(
        read("crates/shared/src/plugins.rs").contains("\"/usr/share/wolos/plugins.schema.json\"")
    );
    assert!(
        read("wolos-system/ntsync.conf")
            .lines()
            .any(|l| l == "ntsync")
    );
    let f = read("wolos-system/70-wolos-cjk-arabic.conf");
    for fam in [
        "Noto Sans CJK SC",
        "Noto Sans CJK JP",
        "Noto Sans CJK KR",
        "Noto Sans CJK TC",
        "Noto Sans Arabic",
        "Noto Sans Hebrew",
        "Noto Color Emoji",
        "Noto Sans Mono CJK JP",
    ] {
        assert!(
            f.contains(&format!("<string>{fam}</string>"))
                || f.contains(&format!("<family>{fam}</family>")),
            "{fam}"
        );
    }
    let pr = read("wolos-system/51-wolos-image.preset");
    assert!(
        pr.lines()
            .any(|l| l == "enable power-profiles-daemon.service")
    );
    assert!(
        pr.lines().any(|l| l == "disable fwupd-refresh.timer"),
        "no third-party download by default"
    );
    // Every install path runs the nsswitch edit.
    let i = read("wolos-system/wolos-system.install");
    assert_eq!(
        i.matches("/usr/lib/wolos/nsswitch-mdns").count(),
        2,
        "post_install and post_upgrade"
    );
}

#[test]
fn nsswitch_edit_puts_mdns_before_resolve_once() {
    let d = std::env::temp_dir().join(format!("nsswitch-{}", std::process::id()));
    fs::create_dir_all(&d).unwrap();
    let f = d.join("nsswitch.conf");
    // Arch's filesystem package line at the snapshot.
    fs::write(
        &f,
        "passwd: files systemd\nhosts: mymachines resolve [!UNAVAIL=return] files myhostname dns\n",
    )
    .unwrap();
    for _ in 0..2 {
        let o = Command::new("sh")
            .arg(repo("wolos-system/nsswitch-mdns.sh"))
            .arg(&f)
            .output()
            .unwrap();
        assert!(o.status.success(), "{o:?}");
    }
    assert_eq!(
        fs::read_to_string(&f).unwrap(),
        "passwd: files systemd\nhosts: mymachines mdns_minimal [NOTFOUND=return] resolve [!UNAVAIL=return] files myhostname dns\n"
    );
    fs::remove_dir_all(&d).ok();
}
