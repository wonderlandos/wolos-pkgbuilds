// SPDX-License-Identifier: GPL-2.0-or-later
//! What this file checks (wolos-pkgbuilds): the two tools/lint.sh rules of BF-8/W6, each on a small
//! fixture repo (`tools/lint.sh --only servers|pkgrel`):
//! - every Server line of a shipped pacman.conf or mirrorlist is at ./snapshot's date (the
//!   CachyOS /cachy/ redirect rolls and has none);
//! - same version needs a pkgrel bump: a recipe changed since the last tag with the tag's
//!   epoch:pkgver-pkgrel fails; a bump, a new recipe or no tag at all passes.
//!
//! Needs bash and git; skipped with a note without them.
use std::{
    fs,
    path::{Path, PathBuf},
    process::Command,
    sync::atomic::{AtomicUsize, Ordering},
};

fn repo() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .unwrap()
        .to_path_buf()
}

fn tools() -> bool {
    ["bash", "git"].iter().all(|t| {
        Command::new(t)
            .arg("--version")
            .output()
            .is_ok_and(|o| o.status.success())
    })
}

/// A fresh fixture repo with tools/lint.sh and a 2026-10-31 snapshot.
fn fixture() -> PathBuf {
    static N: AtomicUsize = AtomicUsize::new(0);
    let t = std::env::temp_dir().join(format!(
        "wolos-lint-{}-{}",
        std::process::id(),
        N.fetch_add(1, Ordering::SeqCst)
    ));
    let _ = fs::remove_dir_all(&t);
    fs::create_dir_all(t.join("tools")).unwrap();
    fs::copy(repo().join("tools/lint.sh"), t.join("tools/lint.sh")).unwrap();
    fs::write(t.join("snapshot"), "2026-10-31\n").unwrap();
    t
}

fn write(t: &Path, rel: &str, text: &str) {
    let p = t.join(rel);
    fs::create_dir_all(p.parent().unwrap()).unwrap();
    fs::write(p, text).unwrap();
}

fn git(t: &Path, args: &[&str]) {
    let o = Command::new("git")
        .args([
            "-c",
            "user.name=t",
            "-c",
            "user.email=t@t",
            "-c",
            "commit.gpgsign=false",
            "-c",
            "tag.gpgsign=false",
        ])
        .args(args)
        .current_dir(t)
        .output()
        .unwrap();
    assert!(
        o.status.success(),
        "git {args:?}: {}",
        String::from_utf8_lossy(&o.stderr)
    );
}

/// (passed, stderr) of `tools/lint.sh --only RULE` in T.
fn lint(t: &Path, rule: &str) -> (bool, String) {
    let o = Command::new("bash")
        .args(["tools/lint.sh", "--only", rule])
        .current_dir(t)
        .output()
        .unwrap();
    (
        o.status.success(),
        String::from_utf8_lossy(&o.stderr).into_owned(),
    )
}

const CONF: &str =
    "[core]\nServer = https://archive.archlinux.org/repos/2026/10/31/$repo/os/$arch\n";
const LIST: &str = "#Server = https://REPO_DOMAIN/$repo-2026-10-31\nServer = https://github.com/x/releases/download/$repo-2026-10-31\n";
const CACHY: &str = "# rolls\n#Server = https://REPO_DOMAIN/cachy/$repo\n";

#[test]
fn server_lines_are_at_the_snapshot_date() {
    if !tools() {
        eprintln!("bash/git missing: lint rules not run");
        return;
    }
    let t = fixture();
    write(&t, "pacman.conf", CONF);
    write(&t, "wolos-mirrorlist/wolos-mirrorlist", LIST);
    write(&t, "wolos-mirrorlist/wolos-cachy-mirrorlist", CACHY);
    write(
        &t,
        "wolos-mirrorlist/PKGBUILD",
        "Server = not a shipped file\n",
    );
    let (ok, err) = lint(&t, "servers");
    assert!(ok, "{err}");

    // An Arch archive at another date.
    write(&t, "pacman.conf", &CONF.replace("2026/10/31", "2026/10/20"));
    let (ok, err) = lint(&t, "servers");
    assert!(!ok && err.contains("pacman.conf:2:"), "{err}");
    write(&t, "pacman.conf", CONF);

    // A commented-out server is shipped too: uncommenting it must not move the date.
    write(
        &t,
        "wolos-mirrorlist/wolos-mirrorlist",
        &LIST.replacen("2026-10-31", "2026-10-28", 1),
    );
    let (ok, err) = lint(&t, "servers");
    assert!(
        !ok && err.contains("wolos-mirrorlist/wolos-mirrorlist:1:"),
        "{err}"
    );

    // A recipe's own pacman.conf counts as well.
    write(&t, "wolos-mirrorlist/wolos-mirrorlist", LIST);
    write(&t, "other/pacman.conf", "Server = https://mirror/$repo\n");
    let (ok, err) = lint(&t, "servers");
    assert!(!ok && err.contains("other/pacman.conf:1:"), "{err}");
    let _ = fs::remove_dir_all(&t);
}

fn pkgbuild(rel: u32) -> String {
    format!("pkgname=foo\npkgver=1.0\npkgrel={rel}\narch=(any)\n")
}

#[test]
fn a_changed_recipe_needs_a_new_version() {
    if !tools() {
        eprintln!("bash/git missing: lint rules not run");
        return;
    }
    let t = fixture();
    write(&t, "foo/PKGBUILD", &pkgbuild(1));
    write(&t, "foo/fix.patch", "a\n");
    write(&t, "bar/PKGBUILD", "pkgname=bar\npkgver=2\npkgrel=1\n");
    git(&t, &["init", "-q"]);
    git(&t, &["add", "-A"]);
    git(&t, &["commit", "-qm", "one"]);

    // No tag yet: skipped, with a note.
    let (ok, err) = lint(&t, "pkgrel");
    assert!(ok && err.contains("skipped"), "{err}");

    git(&t, &["tag", "v1"]);
    let (ok, err) = lint(&t, "pkgrel");
    assert!(ok, "unchanged: {err}");

    // A patch changes, the version does not: fail, naming the recipe (working tree vs tag).
    write(&t, "foo/fix.patch", "b\n");
    let (ok, err) = lint(&t, "pkgrel");
    assert!(
        !ok && err.contains("foo: changed since v1") && !err.contains("bar"),
        "{err}"
    );

    // Committed but still the same version: still a failure.
    git(&t, &["commit", "-qam", "two"]);
    assert!(!lint(&t, "pkgrel").0);

    // pkgrel bump: ok. A recipe new since the tag needs nothing.
    write(&t, "foo/PKGBUILD", &pkgbuild(2));
    write(&t, "baz/PKGBUILD", "pkgname=baz\npkgver=1\npkgrel=1\n");
    let (ok, err) = lint(&t, "pkgrel");
    assert!(ok, "{err}");

    // pkgver or epoch moves count too; a comment-only change does not escape the rule.
    write(
        &t,
        "bar/PKGBUILD",
        "pkgname=bar\npkgver=2\npkgrel=1\n# note\n",
    );
    assert!(!lint(&t, "pkgrel").0);
    write(
        &t,
        "bar/PKGBUILD",
        "pkgname=bar\nepoch=1\npkgver=2\npkgrel=1\n# note\n",
    );
    let (ok, err) = lint(&t, "pkgrel");
    assert!(ok, "{err}");
    let _ = fs::remove_dir_all(&t);
}

#[test]
fn the_real_lint_runs_both_rules() {
    let s = fs::read_to_string(repo().join("tools/lint.sh")).unwrap();
    let main = s.split("\nlint_servers\nlint_pkgrel\n").count();
    assert_eq!(
        main, 2,
        "tools/lint.sh calls lint_servers and lint_pkgrel in its main run"
    );
    assert!(s.contains("git describe --tags --abbrev=0"));
}
