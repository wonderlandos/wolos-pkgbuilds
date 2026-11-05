// SPDX-License-Identifier: GPL-2.0-or-later
//! One Arch Linux Archive date per WolOS release (ADR 0016, ADR 0030; audit CA-6): this repo's
//! snapshot and pacman.conf (the build chroot), wolos-iso's profile/snapshot and its two
//! pacman.conf (the image, the installed system), and the wolos-mirrorlist tag all name it.
//! tools/bump-snapshot.sh rewrites every copy. Without the wolos-iso sibling ($WOLOS_ISO or
//! ../wolos-iso) the cross-repo half is skipped.
use std::{
    fs,
    path::{Path, PathBuf},
    process::Command,
};

fn here() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("..")
}

fn iso() -> Option<PathBuf> {
    let p =
        std::env::var_os("WOLOS_ISO").map_or_else(|| here().join("../wolos-iso"), PathBuf::from);
    p.join("profile/snapshot").is_file().then_some(p)
}

fn read(p: &Path) -> String {
    fs::read_to_string(p).unwrap_or_else(|e| panic!("{}: {e}", p.display()))
}

/// Archive dates (`YYYY/MM/DD`) of a pacman.conf's Server lines, `YYYY-MM-DD`.
fn conf_dates(conf: &str) -> Vec<String> {
    conf.lines()
        .filter_map(|l| l.strip_prefix("Server = https://archive.archlinux.org/repos/"))
        .map(|r| r.get(..10).unwrap_or(r).replace('/', "-"))
        .collect()
}

/// Every copy names one date: Ok(that date), Err(what differs).
fn agree(pb: &Path, iso: Option<&Path>) -> Result<String, String> {
    let snap = read(&pb.join("snapshot")).trim().to_owned();
    let mut bad = vec![];
    let mut confs = vec![pb.join("pacman.conf")];
    if let Some(i) = iso {
        let s = read(&i.join("profile/snapshot")).trim().to_owned();
        if s != snap {
            bad.push(format!("wolos-iso profile/snapshot {s} != {snap}"));
        }
        confs.push(i.join("profile/pacman.conf"));
        confs.push(i.join("profile/airootfs/etc/pacman.conf"));
    }
    for c in &confs {
        let d = conf_dates(&read(c));
        if d.len() < 3 || d.iter().any(|x| *x != snap) {
            bad.push(format!("{}: archive dates {d:?} != {snap}", c.display()));
        }
    }
    let m = read(&pb.join("wolos-mirrorlist/wolos-mirrorlist"));
    let tags: Vec<&str> = m
        .lines()
        .filter(|l| l.trim_start_matches('#').starts_with("Server ="))
        .collect();
    if tags.is_empty() || tags.iter().any(|l| !l.ends_with(&format!("/$repo-{snap}"))) {
        bad.push(format!("wolos-mirrorlist tags {tags:?} != $repo-{snap}"));
    }
    let pkg = read(&pb.join("wolos-mirrorlist/PKGBUILD"));
    if !pkg
        .lines()
        .any(|l| l == format!("pkgver={}", snap.replace('-', ".")))
    {
        bad.push(format!("wolos-mirrorlist pkgver != {snap}"));
    }
    if bad.is_empty() {
        Ok(snap)
    } else {
        Err(bad.join("\n"))
    }
}

#[test]
fn every_copy_names_one_snapshot() {
    let i = iso();
    if i.is_none() {
        eprintln!("skipped the wolos-iso half: no checkout at $WOLOS_ISO or ../wolos-iso");
    }
    let snap = agree(&here(), i.as_deref()).unwrap_or_else(|e| panic!("{e}"));
    assert_eq!(snap.len(), 10, "{snap}");
    // The chroot accepts what the image does (audit CA-10): the wolos-base manifest never
    // depends on the build host's CPU.
    let c = read(&here().join("pacman.conf"));
    assert!(c.contains("\nArchitecture = x86_64 x86_64_v3\n"), "{c}");
}

fn copy(from: &Path, to: &Path) {
    fs::create_dir_all(to.parent().unwrap()).unwrap();
    fs::copy(from, to).unwrap_or_else(|e| panic!("{}: {e}", from.display()));
}

fn sha256(p: &Path) -> String {
    let o = Command::new("sha256sum").arg(p).output().unwrap();
    String::from_utf8_lossy(&o.stdout)[..64].to_owned()
}

#[test]
fn bump_snapshot_moves_every_copy() {
    let t = std::env::temp_dir().join(format!("wol-bump-snapshot-{}", std::process::id()));
    let _ = fs::remove_dir_all(&t);
    let (pb, is) = (t.join("wolos-pkgbuilds"), t.join("wolos-iso"));
    for f in [
        "snapshot",
        "pacman.conf",
        "tools/bump-snapshot.sh",
        // bump-snapshot.sh also moves wolos-base's `_snapshot=` (the userland pin).
        "wolos-base/PKGBUILD",
        "wolos-mirrorlist/PKGBUILD",
        "wolos-mirrorlist/wolos-mirrorlist",
        "wolos-mirrorlist/wolos-cachy-mirrorlist",
    ] {
        copy(&here().join(f), &pb.join(f));
    }
    // The real wolos-iso files when the sibling is there, else stand-ins with this repo's date.
    let src = iso();
    for f in [
        "profile/snapshot",
        "profile/pacman.conf",
        "profile/airootfs/etc/pacman.conf",
    ] {
        match &src {
            Some(i) => copy(&i.join(f), &is.join(f)),
            None if f.ends_with("snapshot") => copy(&here().join("snapshot"), &is.join(f)),
            None => copy(&here().join("pacman.conf"), &is.join(f)),
        }
    }
    let before = agree(&pb, Some(&is)).unwrap_or_else(|e| panic!("{e}"));
    let bump = |d: &str| {
        Command::new("bash")
            .arg(pb.join("tools/bump-snapshot.sh"))
            .arg(d)
            .arg(&is)
            .output()
            .unwrap()
    };
    // Not a date: refused, nothing written.
    for bad in ["2027-02-30", "2027/01/15", "latest", "2027-1-5"] {
        let o = bump(bad);
        assert!(!o.status.success(), "{bad} accepted");
        assert_eq!(
            agree(&pb, Some(&is)),
            Ok(before.clone()),
            "{bad} changed a file"
        );
    }
    let o = bump("2027-01-15");
    assert!(o.status.success(), "{}", String::from_utf8_lossy(&o.stderr));
    assert_eq!(agree(&pb, Some(&is)), Ok("2027-01-15".to_owned()));
    let pkg = read(&pb.join("wolos-mirrorlist/PKGBUILD"));
    assert!(pkg.contains("\npkgver=2027.01.15\npkgrel=1\n"), "{pkg}");
    for f in ["wolos-mirrorlist", "wolos-cachy-mirrorlist"] {
        let s = sha256(&pb.join("wolos-mirrorlist").join(f));
        assert!(
            pkg.contains(&format!("'{s}'")),
            "{f}: sha256sums not updated: {pkg}"
        );
    }
    // The same date again: nothing to bump, pkgrel stays.
    let o = bump("2027-01-15");
    assert!(o.status.success());
    assert!(read(&pb.join("wolos-mirrorlist/PKGBUILD")).contains("\npkgrel=1\n"));
    let _ = fs::remove_dir_all(&t);
}
