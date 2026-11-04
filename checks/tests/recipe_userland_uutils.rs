// SPDX-License-Identifier: GPL-2.0-or-later
//! What this file checks (wolos-pkgbuilds): which GNU tools wolos-base replaces with uutils/sudo-rs and why the rest stays GNU (README "Userland and GNU" of wolos-iso).
//! Replaced: coreutils (every GNU 9.12 name is a symlink to the uutils `uu-NAME`, no other name) and
//! sudo. Kept GNU, each with the evidence that decided it (2026-11-04): findutils and diffutils are
//! not in Arch `extra` (only uutils-coreutils is), uutils-diffutils 0.6.0 lacks flags devtools, dkms
//! and libtool use, procps has no uutils release (its `ps` prints nothing for the updater's
//! `ps -ax -o pid= -o ppid=`), util-linux is 0.0.1. wolos-base/userland-matrix.sh is the gate for
//! the next swap: every tool WolOS calls has a case in it, with the GNU answer in
//! wolos-base/userland-matrix/*.out. Static checks run everywhere; WOLOS_USERLAND_MATRIX=1 also runs
//! the matrix against the tools in $PATH (the image, a VM, or a directory of candidates first in
//! $PATH) and requires every tool to exist and answer --version.
#![allow(dead_code, unused_imports)]
#[path = "support/siblings.rs"]
mod siblings;

use std::{collections::BTreeSet, fs, path::PathBuf, process::Command};

fn repo(p: &str) -> PathBuf {
    siblings::root().join(p)
}

fn read(p: &str) -> String {
    fs::read_to_string(repo(p)).unwrap_or_else(|e| panic!("{p}: {e}"))
}

/// The program names of Arch coreutils 9.12-2 (`pacman -Ql coreutils`, /usr/bin).
const GNU_COREUTILS: &[&str] = &[
    "[",
    "b2sum",
    "base32",
    "base64",
    "basename",
    "basenc",
    "cat",
    "chgrp",
    "chmod",
    "chown",
    "chroot",
    "cksum",
    "comm",
    "cp",
    "csplit",
    "cut",
    "date",
    "dd",
    "df",
    "dir",
    "dircolors",
    "dirname",
    "du",
    "echo",
    "env",
    "expand",
    "expr",
    "factor",
    "false",
    "fmt",
    "fold",
    "groups",
    "head",
    "hostid",
    "id",
    "install",
    "join",
    "link",
    "ln",
    "logname",
    "ls",
    "md5sum",
    "mkdir",
    "mkfifo",
    "mknod",
    "mktemp",
    "mv",
    "nice",
    "nl",
    "nohup",
    "nproc",
    "numfmt",
    "od",
    "paste",
    "pathchk",
    "pinky",
    "pr",
    "printenv",
    "printf",
    "ptx",
    "pwd",
    "readlink",
    "realpath",
    "rm",
    "rmdir",
    "seq",
    "sha1sum",
    "sha224sum",
    "sha256sum",
    "sha384sum",
    "sha512sum",
    "shred",
    "shuf",
    "sleep",
    "sort",
    "split",
    "stat",
    "stdbuf",
    "stty",
    "sum",
    "sync",
    "tac",
    "tail",
    "tee",
    "test",
    "timeout",
    "touch",
    "tr",
    "true",
    "truncate",
    "tsort",
    "tty",
    "uname",
    "unexpand",
    "uniq",
    "unlink",
    "users",
    "vdir",
    "wc",
    "who",
    "whoami",
    "yes",
];

/// One row per GNU tool package and what it was decided: (package, tools WolOS calls, decision).
/// `uutils` rows are in wolos-base; `gnu` rows are not, and the evidence says why (see the header).
const DECISIONS: &[(&str, &[&str], &str)] = &[
    (
        "coreutils",
        &[
            "cp",
            "mv",
            "rm",
            "install",
            "sha256sum",
            "sort",
            "stat",
            "timeout",
        ],
        "uutils",
    ),
    ("sudo", &["sudo", "sudoedit", "visudo"], "uutils"),
    ("findutils", &["find", "xargs"], "gnu"),
    ("diffutils", &["diff", "cmp"], "gnu"),
    ("procps-ng", &["ps", "pgrep", "pkill", "free"], "gnu"),
    (
        "util-linux",
        &[
            "mount", "umount", "lsblk", "blkid", "findmnt", "sfdisk", "partx", "mkswap", "swapoff",
            "runuser",
        ],
        "gnu",
    ),
];

/// The words of one `name=( ... )` array of a PKGBUILD, quotes removed.
fn array(pkgbuild: &str, name: &str) -> Vec<String> {
    let start = pkgbuild
        .find(&format!("\n{name}=("))
        .unwrap_or_else(|| panic!("no {name}=( in the PKGBUILD"))
        + name.len()
        + 3;
    let body = &pkgbuild[start..];
    let end = body.find(')').expect("unterminated array");
    body[..end]
        .lines()
        .map(|l| l.split('#').next().unwrap_or(""))
        .flat_map(|l| l.split_whitespace())
        .map(|w| w.trim_matches('\'').to_string())
        .filter(|w| !w.is_empty())
        .collect()
}

#[test]
fn coreutils_links_are_exactly_the_gnu_names() {
    let pkg = read("wolos-base/PKGBUILD");
    let linked: BTreeSet<String> = array(&pkg, "_coreutils").into_iter().collect();
    let gnu: BTreeSet<String> = GNU_COREUTILS.iter().map(|s| s.to_string()).collect();
    assert_eq!(
        linked.difference(&gnu).collect::<Vec<_>>(),
        Vec::<&String>::new(),
        "linked as a GNU name that GNU coreutils 9.12 does not ship"
    );
    assert_eq!(
        gnu.difference(&linked).collect::<Vec<_>>(),
        Vec::<&String>::new(),
        "a GNU coreutils name that `coreutils` provides but wolos-base does not link"
    );
    assert!(pkg.contains(r#"ln -s "uu-$b" "$pkgdir/usr/bin/$b""#));
    assert!(pkg.contains(r#"ln -s "$b-rs" "$pkgdir/usr/bin/$b""#));
    // uutils ships more programs (hostname, kill, uptime, more, arch): those names belong to
    // inetutils, util-linux and procps, which stay, so they are not linked.
    for not_ours in ["hostname", "kill", "uptime", "more", "arch"] {
        assert!(!linked.contains(not_ours), "{not_ours} must not be linked");
    }
}

#[test]
fn only_coreutils_and_sudo_are_replaced() {
    let pkg = read("wolos-base/PKGBUILD");
    assert_eq!(
        array(&pkg, "provides"),
        ["coreutils=9.12", "sudo", "VIRTUALBOX-GUEST-MODULES"]
    );
    assert_eq!(array(&pkg, "conflicts"), ["coreutils", "sudo"]);
    let depends = array(&pkg, "depends");
    for (package, _, decision) in DECISIONS {
        let swapped = depends.iter().any(|d| d == &format!("uutils-{package}"));
        match *decision {
            "uutils" if *package == "coreutils" => assert!(swapped, "uutils-coreutils"),
            "uutils" => assert!(depends.iter().any(|d| d == "sudo-rs"), "sudo-rs"),
            _ => {
                assert!(!swapped, "{package} is GNU until its matrix passes");
                for field in ["provides", "conflicts"] {
                    assert!(
                        !array(&pkg, field).iter().any(|w| w.starts_with(package)),
                        "{package} in {field}"
                    );
                }
            }
        }
    }
    // The kept-GNU packages come with `base`, which stays a dependency.
    assert!(depends.iter().any(|d| d == "base"));
}

#[test]
fn matrix_covers_every_tool_wolos_calls() {
    let script = read("wolos-base/userland-matrix.sh");
    let cases: Vec<&str> = script
        .lines()
        .filter(|l| l.trim_start().starts_with("case_ "))
        .collect();
    assert!(cases.len() > 140, "{} cases", cases.len());
    for (package, tools, decision) in DECISIONS {
        for tool in *tools {
            if *decision == "uutils" || ["util-linux"].contains(package) {
                continue; // coreutils is in its own section below; util-linux is not swappable yet
            }
            assert!(
                cases
                    .iter()
                    .filter(|l| l.contains("ours-"))
                    .any(|l| l.contains(&format!("{tool} "))),
                "no ours-* case runs `{tool}` ({package})"
            );
        }
    }
    for tool in [
        "cp",
        "mv",
        "rm",
        "install",
        "sha256sum",
        "sort",
        "stat",
        "timeout",
        "touch",
        "readlink",
    ] {
        assert!(
            cases
                .iter()
                .filter(|l| l.contains("ours-"))
                .any(|l| l.contains(&format!("{tool} "))),
            "no ours-* case runs `{tool}`"
        );
    }
}

#[test]
fn golden_answers_match_the_cases() {
    let script = read("wolos-base/userland-matrix.sh");
    for section in ["coreutils", "findutils", "diffutils", "procps"] {
        let golden = read(&format!("wolos-base/userland-matrix/{section}.out"));
        let in_golden: BTreeSet<&str> = golden
            .lines()
            .filter_map(|l| l.strip_prefix("== "))
            .collect();
        // The section function's own case_ lines.
        let body = script
            .split(&format!("sec_{section}() {{"))
            .nth(1)
            .and_then(|s| s.split("\n}\n").next())
            .unwrap_or_else(|| panic!("no sec_{section}"));
        let in_script: BTreeSet<&str> = body
            .lines()
            .filter_map(|l| l.trim_start().strip_prefix("case_ "))
            .filter_map(|l| l.split_whitespace().next())
            .collect();
        assert_eq!(
            in_golden, in_script,
            "{section}: cases and golden answers differ (rewrite with `userland-matrix.sh write` on GNU tools)"
        );
        assert!(
            in_script.iter().any(|n| n.starts_with("ours-")),
            "{section}: no ours-* case"
        );
    }
    // The answers were printed by GNU: the exit code lines the updater depends on are in them.
    let find = read("wolos-base/userland-matrix/findutils.out");
    assert!(
        find.contains("== ours-update-exec-plus-fails\nrc=1\n"),
        "find -exec {{}} + failing is exit 1"
    );
    assert!(
        find.contains("== ours-update-exec-semicolon\nrc=0\n"),
        "find -exec ; failing stays exit 0"
    );
}

/// Every tool WolOS calls exists and answers; the matrix passes. Needs the tools to test in $PATH
/// (the image, a VM, or a directory of candidates first), so it only runs with WOLOS_USERLAND_MATRIX=1.
#[test]
fn tools_in_path_answer_and_pass_the_matrix() {
    if std::env::var_os("WOLOS_USERLAND_MATRIX").is_none() {
        eprintln!("skipped: set WOLOS_USERLAND_MATRIX=1 to run it against the tools in $PATH");
        return;
    }
    let mut all: BTreeSet<&str> = GNU_COREUTILS.iter().copied().collect();
    for (_, tools, _) in DECISIONS {
        all.extend(tools.iter().copied());
    }
    for tool in all {
        if tool == "["
            || tool == "test"
            || tool == "echo"
            || tool == "true"
            || tool == "false"
            || tool == "printf"
            || tool == "pwd"
        {
            continue; // shell builtins also exist as programs; `--version` is not their contract
        }
        let ok = ["--version", "-V"].iter().any(|flag| {
            Command::new(tool)
                .arg(flag)
                .output()
                .is_ok_and(|o| o.status.success())
        });
        assert!(
            ok,
            "{tool}: not found in $PATH or answers neither --version nor -V"
        );
    }
    let out = Command::new("bash")
        .arg(repo("wolos-base/userland-matrix.sh"))
        .arg("check")
        .output()
        .expect("bash");
    let text = String::from_utf8_lossy(&out.stdout);
    assert!(
        out.status.success(),
        "{}",
        text.lines()
            .filter(|l| !l.starts_with("ok "))
            .collect::<Vec<_>>()
            .join("\n")
    );
}
