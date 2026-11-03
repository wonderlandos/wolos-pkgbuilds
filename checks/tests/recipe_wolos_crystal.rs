// SPDX-License-Identifier: GPL-2.0-or-later
//! What this file checks (wolos-pkgbuilds): The wolos-crystal recipe: the Crystal tools as one package, plus the cli, tool and ai templates, the app skill and CRYSTAL_PATH.
//! wolos-crystal is what `wol app new` (wolos crates/controller/src/app.rs) needs on a machine: it
//! depends on the toolchain and both shards (the app template comes with wolkit) and installs the
//! other three templates where the verb reads them, the skill the WolOS agent skill points at, and
//! a login-shell CRYSTAL_PATH so `require "wolkit"` works in a fresh project. Its check() renders
//! the templates with the same placeholders as the verb (I7) and runs their specs.
#![allow(dead_code, unused_imports)]
#[path = "support/siblings.rs"]
mod siblings;

use std::fs;
fn read(p: &str) -> String {
    let path = siblings::root().join(p);
    fs::read_to_string(&path).unwrap_or_else(|e| panic!("{p}: {e}"))
}

fn line<'a>(text: &'a str, start: &str) -> &'a str {
    text.lines()
        .find(|l| l.starts_with(start))
        .unwrap_or_else(|| panic!("PKGBUILD lacks {start}"))
}

fn words(l: &str) -> Vec<&str> {
    l.split(['(', ')', ' ']).filter(|w| !w.is_empty()).collect()
}

#[test]
fn depends_on_the_toolchain_and_both_shards() {
    let p = read("wolos-crystal/PKGBUILD");
    assert!(p.contains("pkgname=wolos-crystal\n"));
    assert!(line(&p, "arch=(").contains("any"));
    let depends = words(line(&p, "depends=("));
    for d in ["crystal", "shards", "make", "pkgconf", "libwol", "libwolkit", "cryswol", "wolkit"] {
        assert!(depends.contains(&d), "depends lacks {d}: {depends:?}");
    }
    // The shards and libraries are [wolos] packages built before this one.
    assert!(
        read("packages").lines().any(|l| {
            let w: Vec<&str> = l.split_whitespace().collect();
            w.len() >= 4
                && w[..2] == ["wolos-crystal", "wolos-crystal"]
                && ["libwol", "libwolkit", "cryswol", "wolkit"]
                    .iter()
                    .all(|d| w[2].split(',').any(|x| x == *d))
        }),
        "wolos-crystal is a line of packages, after libwol, libwolkit, cryswol and wolkit"
    );
}

#[test]
fn installs_the_three_templates_the_skill_and_crystal_path() {
    let p = read("wolos-crystal/PKGBUILD");
    assert!(p.contains("_templates=(cli tool ai)"), "the app template is wolkit's");
    for want in [
        "\"$pkgdir/usr/share/wolos/templates/$t\"",
        "install -Dm644 wolos-crystal/skill/SKILL.md \"$pkgdir/usr/share/wolos/crystal/skill/SKILL.md\"",
        "\"$pkgdir/etc/profile.d/wolos-crystal.sh\"",
        "export CRYSTAL_PATH=lib:/usr/lib/wolos/crystal:/usr/lib/crystal",
        "set -gx CRYSTAL_PATH lib:/usr/lib/wolos/crystal:/usr/lib/crystal",
        "backup=(etc/profile.d/wolos-crystal.sh)",
        "usr/share/licenses/$pkgname/LICENSE",
    ] {
        assert!(p.contains(want), "PKGBUILD lacks {want}");
    }
    // What it copies exists in the wolos tree, and nothing else installs those three templates.
    for t in ["cli", "tool", "ai"] {
        for f in ["shard.yml.tmpl", "Makefile.tmpl", "src/main.cr.tmpl"] {
            let path = format!("wolos-crystal/templates/{t}/{f}");
            assert!(siblings::root().join(&path).is_file(), "{path} is missing");
        }
    }
    assert!(siblings::root().join("wolos-crystal/skill/SKILL.md").is_file());
    let wolkit = read("wolos-crystal/shards/wolkit/Makefile");
    assert!(wolkit.contains("cp -r ../../templates/app $(TEMPLATE_DIR)/"));
    assert!(!wolkit.contains("templates/cli"), "two packages would own the cli template");
    // The agent skill points at the path this package installs.
    let agent = read("files/usr/share/wolos/agents/skills/wolos/SKILL.md");
    assert!(agent.contains("/usr/share/wolos/crystal/skill/SKILL.md"));
}

#[test]
fn check_renders_like_wol_app_new_and_runs_the_specs() {
    let p = read("wolos-crystal/PKGBUILD");
    for want in [
        "s/{{id}}/org.wolos.demo/g",
        "s/{{name}}/demo/g",
        "s/{{module}}/Demo/g",
        "${rel//\\{\\{name\\}\\}/demo}",
        "crystal spec --error-on-warnings",
        "shards build --frozen --local",
        "crystal tool format --check src spec",
    ] {
        assert!(p.contains(want), "check() lacks {want}");
    }
    // Every placeholder the three templates use is one check() fills (I7, wolos RCR-050).
    for t in ["cli", "tool", "ai"] {
        let mut stack = vec![siblings::root().join("wolos-crystal/templates").join(t)];
        while let Some(d) = stack.pop() {
            for e in fs::read_dir(&d).unwrap().flatten() {
                let path = e.path();
                if path.is_dir() {
                    stack.push(path);
                    continue;
                }
                let text = fs::read_to_string(&path).unwrap_or_default();
                for piece in text.split("{{").skip(1) {
                    let word = piece.split("}}").next().unwrap_or("");
                    assert!(
                        ["id", "name", "module"].contains(&word),
                        "{}: {{{{{word}}}}} is not filled by check()",
                        path.display()
                    );
                }
            }
        }
    }
}

#[test]
fn pkgver_follows_the_os_through_bump_wolos() {
    let p = read("wolos-crystal/PKGBUILD");
    assert!(p.contains("_wolos_commit="), "bump-wolos.sh pins it with the other wolos recipes");
    let bump = read("tools/bump-wolos.sh");
    assert!(
        !bump.contains("wolos-crystal/*)"),
        "wolos-crystal takes the OS version (bump-wolos.sh's default)"
    );
    let changelog = read("CHANGELOG.md");
    let os = changelog
        .lines()
        .find_map(|l| l.strip_prefix("## ").and_then(|r| r.split_whitespace().next()))
        .filter(|v| v.starts_with(|c: char| c.is_ascii_digit()));
    if let Some(os) = os {
        let pkgver = line(&p, "pkgver=").trim_start_matches("pkgver=");
        assert!(
            pkgver.split_whitespace().next().unwrap() <= os,
            "pkgver {pkgver} is newer than the OS {os}"
        );
    }
}
