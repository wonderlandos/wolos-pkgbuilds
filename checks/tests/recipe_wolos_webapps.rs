// SPDX-License-Identifier: GPL-2.0-or-later
//! What this file checks (wolos-pkgbuilds): The wolos-webapps recipe (ADR 0048, QA-WEB-4): the
//! system web apps and their kit in /usr/share/wolos/webapps with each app's launcher entry. The
//! recipe only runs the wolos source's `tools/install-webapps.sh` (which leaves out the development
//! files and reads `webapps/SHIPPED`); the host binary comes with wolos-system.
#![allow(dead_code, unused_imports)]
#[path = "support/siblings.rs"]
mod siblings;

use std::fs;
fn read(p: &str) -> String {
    let path = siblings::root().join(p);
    fs::read_to_string(&path).unwrap_or_else(|e| panic!("{p}: {e}"))
}

#[test]
fn the_recipe_installs_the_apps_through_the_source_script() {
    let p = read("wolos-webapps/PKGBUILD");
    for want in [
        "pkgname=wolos-webapps",
        "arch=(any)",
        "depends=(wolos-system webengine-wolos)",
        "\"$_repo/tools/install-webapps.sh\" \"$pkgdir\"",
        "LICENSES/BSD-2-Clause.txt",
        "^_wolos_commit=",
    ] {
        let found = if let Some(re) = want.strip_prefix('^') {
            p.lines().any(|l| l.starts_with(re))
        } else {
            p.contains(want)
        };
        assert!(found, "PKGBUILD lacks {want}");
    }
    assert!(
        p.contains("license=(GPL-2.0-or-later BSD-2-Clause)"),
        "the apps are GPL, the kit BSD"
    );
    assert!(
        !p.contains("cargo") && !p.contains("npm"),
        "text and icons only: no build, no bundler (ADR 0048)"
    );
}

#[test]
fn it_is_in_the_package_list_after_wolos_system() {
    let list = read("packages");
    let at = |name: &str| {
        list.lines()
            .position(|l| l.split_whitespace().next() == Some(name))
            .unwrap_or_else(|| panic!("{name} is not in packages"))
    };
    assert!(at("wolos-webapps") > at("wolos-system"));
    let line = list
        .lines()
        .find(|l| l.split_whitespace().next() == Some("wolos-webapps"))
        .unwrap();
    assert!(line.contains("wolos-system"), "{line}");
}

/// The source the recipe pins has what the recipe runs; checked against the wolos tree this repo
/// is tested with (the pin moves to a commit that has them with the next bump).
#[test]
fn the_wolos_tree_has_the_script_the_ship_list_and_the_entries() {
    let script = read("tools/install-webapps.sh");
    assert!(script.contains("usr/share/wolos/webapps"), "{script}");
    let shipped: Vec<String> = read("webapps/SHIPPED")
        .lines()
        .map(str::trim)
        .filter(|l| !l.is_empty() && !l.starts_with('#'))
        .map(str::to_owned)
        .collect();
    assert!(shipped.iter().any(|s| s == "doctor"), "{shipped:?}");
    for s in shipped {
        let e = read(&format!("webapps/launchers/org.wolos.webapp.{s}.desktop"));
        assert!(
            e.contains(&format!(
                "Exec=/usr/bin/wol-webhost /usr/share/wolos/webapps/{s}/app.toml"
            )),
            "{e}"
        );
    }
    assert!(
        read("tools/build-rust.sh").contains(" wol-webhost"),
        "wolos-system installs the host (wold only takes calls from /usr/bin/wol-webhost)"
    );
}

/// The commit the recipe pins has what `package()` runs. Asked of the wolos git repository
/// (`$WOLOS_SRC`, else `../wolos` next to this repo): the test fails while the pin is older than
/// the change that added the script, the ship list and the entries, and passes once `bump-wolos.sh`
/// has moved it (the fail-then-pass of the 0.13 integration).
#[test]
fn the_pinned_wolos_commit_has_the_script_the_ship_list_and_the_entries() {
    let p = read("wolos-webapps/PKGBUILD");
    let commit = p
        .lines()
        .find_map(|l| l.strip_prefix("_wolos_commit="))
        .expect("_wolos_commit=")
        .trim()
        .to_owned();
    assert_eq!(commit.len(), 40, "{commit}");
    let src = std::env::var_os("WOLOS_SRC")
        .map(std::path::PathBuf::from)
        .unwrap_or_else(|| std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../../wolos"));
    let has = |path: &str| {
        std::process::Command::new("git")
            .arg("-C")
            .arg(&src)
            .args(["cat-file", "-e", &format!("{commit}:{path}")])
            .output()
            .is_ok_and(|o| o.status.success())
    };
    for path in ["tools/install-webapps.sh", "webapps/SHIPPED"] {
        assert!(
            has(path),
            "the pinned wolos commit {commit} has no {path}: bump the pin (tools/bump-wolos.sh) \
             past the change that adds the web apps package"
        );
    }
}
