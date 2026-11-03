// SPDX-License-Identifier: GPL-2.0-or-later
//! What this file checks (wolos-pkgbuilds): wolos-base's prebuild (BF-7, audit-build-framework).
//! base-manifest.sh hashes every file in one sha256sum pass; its manifest of a fixed fixture
//! (spaces, a leading dash, a backslash, UTF-8, an empty file, a 4110 helper, relative, absolute,
//! dangling and directory symlinks, metadata files, enough files for xargs to split) is
//! byte-identical to the one the old one-process-per-file script wrote. prebuild.sh reuses the
//! host's pacman CacheDir and keeps its database instead of downloading the base on every run.
//! Needs bash, bsdtar and GNU find/sha256sum (the build host); skipped with a note without bsdtar.
use std::{
    fs,
    path::{Path, PathBuf},
    process::Command,
};

/// wolos-base/base-manifest.sh before BF-7 (one sha256sum and one readlink process per file).
const OLD: &str = r##"#!/usr/bin/env bash
# SPDX-License-Identifier: GPL-2.0-or-later
# base-manifest.sh PKG...: "sha256  /path" for every regular file and symlink in the given pacman
# packages, sorted by path (ADR 0017: `wol base verify` checks the installed base against it).
# Package metadata (.PKGINFO, .MTREE, .BUILDINFO, .INSTALL) is not a system file.
# Contents only: xattrs (file capabilities) need root to restore and are not hashed.
set -euo pipefail
T=$(mktemp -d); trap 'rm -rf "$T"' EXIT
for p in "$@"; do
  [ -f "$p" ] || continue
  bsdtar -xpf "$p" -C "$T" --no-xattrs --exclude '.PKGINFO' --exclude '.MTREE' --exclude '.BUILDINFO' --exclude '.INSTALL' 2>/dev/null ||
    { echo "base-manifest.sh: cannot read $p" >&2; exit 1; }
done
# Owner read on everything: some files are not readable even by their owner (dbus-daemon-launch-helper
# is 4110), and the build host extracts as a plain user; their sha256 came out empty.
chmod -R u+rX "$T"
cd "$T"
find . \( -type f -o -type l \) -printf '%P\n' | LC_ALL=C sort | while IFS= read -r f; do
  if [ -L "$f" ]; then
    printf '%s  /%s\n' "$(readlink "$f" | sha256sum | cut -d' ' -f1)" "$f"
  else
    h=$(sha256sum < "$f" | cut -d' ' -f1)
    [ -n "$h" ] || { echo "base-manifest.sh: cannot hash /$f" >&2; exit 1; }
    printf '%s  /%s\n' "$h" "$f"
  fi
done
"##;

fn repo() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .unwrap()
        .to_path_buf()
}

fn has(tool: &str) -> bool {
    Command::new(tool)
        .arg("--version")
        .output()
        .is_ok_and(|o| o.status.success())
}

fn sh(dir: &Path, script: &str) {
    let o = Command::new("bash")
        .arg("-c")
        .arg(script)
        .current_dir(dir)
        .output()
        .unwrap();
    assert!(
        o.status.success(),
        "{script}: {}",
        String::from_utf8_lossy(&o.stderr)
    );
}

/// Two packages (a .pkg.tar.zst each) with the awkward cases; returns their paths.
fn fixture(t: &Path) -> Vec<PathBuf> {
    sh(
        t,
        r#"set -e
mkdir -p a/usr/bin a/usr/lib/dbus-1.0 a/usr/share/doc/x 'a/usr/share/with space' b/usr/lib/many b/etc
printf '#!/bin/sh\necho hi\n' > a/usr/bin/hello; chmod 755 a/usr/bin/hello
: > a/usr/share/doc/x/empty
printf 'dash\n' > a/usr/share/doc/x/-dash
printf 'back\n' > 'a/usr/share/doc/x/back\slash'
printf 'utf\n' > a/usr/share/doc/x/ünïcødé
printf 'sp\n' > 'a/usr/share/with space/file name'
printf 'helper\n' > a/helper
ln -s hello a/usr/bin/hi
ln -s /usr/bin/hello a/usr/bin/abs
ln -s ../nowhere a/usr/bin/dangling
ln -s doc a/usr/share/doc-link
ln -s 'with space/file name' a/usr/share/sp-link
printf 'pkgname = a\n' > a/.PKGINFO; printf 'm\n' > a/.MTREE
for i in $(seq 1 4000); do printf '%s\n' "$i" > "b/usr/lib/many/file-number-$i.so"; done
printf 'conf\n' > b/etc/b.conf; printf 'pkgname = b\n' > b/.PKGINFO; printf 'x\n' > b/.INSTALL
printf '#mtree\n./usr/lib/dbus-1.0/dbus-daemon-launch-helper type=file mode=4110 contents=%s\n' "$PWD/a/helper" > helper.mtree
z=--zstd; bsdtar -cf /dev/null $z -T /dev/null 2>/dev/null || z=
(cd a && bsdtar -cf ../a-1-1-x86_64.pkg.tar.zst $z --exclude ./helper . @../helper.mtree)
(cd b && bsdtar -cf ../b-1-1-x86_64.pkg.tar.zst $z .)
"#,
    );
    vec![
        t.join("a-1-1-x86_64.pkg.tar.zst"),
        t.join("b-1-1-x86_64.pkg.tar.zst"),
    ]
}

fn manifest(script: &Path, pkgs: &[PathBuf]) -> Vec<u8> {
    let o = Command::new("bash")
        .arg(script)
        .args(pkgs)
        .output()
        .unwrap();
    assert!(
        o.status.success(),
        "{}: {}",
        script.display(),
        String::from_utf8_lossy(&o.stderr)
    );
    o.stdout
}

#[test]
fn the_one_pass_manifest_is_byte_identical_to_the_old_one() {
    if !has("bsdtar") {
        eprintln!("bsdtar not installed: base-manifest.sh not run (the build host has it)");
        return;
    }
    let t = std::env::temp_dir().join(format!("wolos-base-manifest-{}", std::process::id()));
    let _ = fs::remove_dir_all(&t);
    fs::create_dir_all(&t).unwrap();
    let pkgs = fixture(&t);
    let old = t.join("old-base-manifest.sh");
    fs::write(&old, OLD).unwrap();
    let new = manifest(&repo().join("wolos-base/base-manifest.sh"), &pkgs);
    let want = manifest(&old, &pkgs);
    let text = String::from_utf8_lossy(&new);
    assert!(
        text.lines().count() > 4000,
        "{} lines",
        text.lines().count()
    );
    for path in [
        "/usr/lib/dbus-1.0/dbus-daemon-launch-helper",
        "/usr/share/doc/x/back\\slash",
        "/usr/share/with space/file name",
        "/usr/bin/dangling",
        "/usr/share/doc-link",
    ] {
        assert!(
            text.lines().any(|l| l.ends_with(&format!("  {path}"))),
            "no {path}"
        );
    }
    assert!(
        !text.contains(".PKGINFO") && !text.contains(".INSTALL"),
        "metadata hashed"
    );
    assert!(new == want, "manifests differ:\n{}", diff(&want, &new));
    // No package: an empty manifest, as before.
    assert_eq!(
        manifest(&repo().join("wolos-base/base-manifest.sh"), &[]),
        manifest(&old, &[])
    );
    let _ = fs::remove_dir_all(&t);
}

fn diff(a: &[u8], b: &[u8]) -> String {
    let (a, b) = (String::from_utf8_lossy(a), String::from_utf8_lossy(b));
    a.lines()
        .zip(b.lines())
        .filter(|(x, y)| x != y)
        .take(5)
        .map(|(x, y)| format!("- {x}\n+ {y}\n"))
        .collect::<String>()
        + &format!("({} vs {} lines)", a.lines().count(), b.lines().count())
}

#[test]
fn base_manifest_forks_no_process_per_file() {
    let s = fs::read_to_string(repo().join("wolos-base/base-manifest.sh")).unwrap();
    assert!(s.contains("xargs -0 sha256sum --"), "one sha256sum pass");
    assert!(
        !s.contains("$(readlink"),
        "symlink targets come from find -printf %l"
    );
    assert!(!s.contains("sha256sum <"), "no per-file sha256sum");
}

#[test]
fn prebuild_reuses_the_host_cache_and_keeps_its_database() {
    let s = fs::read_to_string(repo().join("wolos-base/prebuild.sh")).unwrap();
    assert!(
        s.contains("pacman-conf CacheDir"),
        "the host's pacman cache"
    );
    assert!(s.contains("pacdb-$key"), "database kept per pacman.conf");
    assert!(
        s.contains("WOLOS_PREBUILD_STATE"),
        "state outside WORK_DIR (wolos-build empties it)"
    );
    assert!(
        s.contains("--print-format '%f'"),
        "the manifest takes the resolved files only"
    );
    assert!(!s.contains("rm -rf \"$WORK"), "never empties its own cache");
    assert!(
        !s.contains("pkgcache/*.pkg.tar.zst"),
        "not every file of a shared cache"
    );
}
