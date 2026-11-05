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

/// The recipe builds `--nodeps` (build.conf: installing wolos-system in the chroot clashes with its
/// sudo), so no git in the chroot: the source is GitHub's archive of `_wolos_commit`, pinned by
/// `_wolos_archive_sha256`, and `tools/bump-wolos.sh` moves both. Run on a copy of the recipe with
/// a stand-in wolos commit and archive (`WOLOS_ARCHIVE_URL=file://...`): the commit and the
/// archive's sha256 are the new ones, the source line still follows `$_wolos_commit`.
#[test]
fn bump_wolos_moves_the_archive_pin_and_its_sha256() {
    use std::process::Command;
    let p = read("wolos-webapps/PKGBUILD");
    assert!(read("wolos-webapps/build.conf").contains("makepkg_args=(--nodeps)"));
    assert!(
        !p.contains("git+") && !p.contains("makedepends=(git)"),
        "--nodeps: no git in the chroot"
    );
    assert!(
        p.contains("depends=(wolos-system webengine-wolos)"),
        "depends stay the installed system's"
    );
    assert!(p.contains(
        "source=(\"wolos-$_wolos_commit.tar.gz::https://github.com/wonderlandos/wolos/archive/$_wolos_commit.tar.gz\")"
    ));
    assert!(p.contains("sha256sums=(\"$_wolos_archive_sha256\")"));
    assert!(p.contains("local _repo=$srcdir/wolos-$_wolos_commit"));

    let t = std::env::temp_dir().join(format!("bump-webapps-{}", std::process::id()));
    let _ = fs::remove_dir_all(&t);
    let (pb, src, arc) = (t.join("pb"), t.join("wolos"), t.join("archive"));
    for d in [pb.join("tools"), pb.join("wolos-webapps"), arc.clone()] {
        fs::create_dir_all(d).unwrap();
    }
    fs::write(pb.join("tools/bump-wolos.sh"), read("tools/bump-wolos.sh")).unwrap();
    fs::write(pb.join("wolos-webapps/PKGBUILD"), &p).unwrap();
    // A wolos tree with what bump-wolos.sh reads for the versions.
    for (f, c) in [
        ("CHANGELOG.md", "# Changelog\n\n## 0.13.9 (2026-11-05)\n"),
        ("wopkg/shard.yml", "version: 0.1.0\n"),
        (
            "crates/wol-abi/include/wol.h",
            "#define WOL_VERSION_MAJOR 1\n#define WOL_VERSION_MINOR 0\n#define WOL_VERSION_PATCH 0\n",
        ),
        (
            "crates/wolkit/include/wolkit.h",
            "#define WOLKIT_VERSION_MAJOR 1\n#define WOLKIT_VERSION_MINOR 0\n#define WOLKIT_VERSION_PATCH 0\n",
        ),
        ("wolos-crystal/shards/wolkit/shard.yml", "version: 0.1.0\n"),
        ("wolos-crystal/shards/cryswol/shard.yml", "version: 0.1.0\n"),
    ] {
        let f = src.join(f);
        fs::create_dir_all(f.parent().unwrap()).unwrap();
        fs::write(f, c).unwrap();
    }
    let git = |args: &[&str]| {
        let o = Command::new("git")
            .arg("-C")
            .arg(&src)
            .args(args)
            .output()
            .unwrap();
        assert!(
            o.status.success(),
            "git {args:?}: {}",
            String::from_utf8_lossy(&o.stderr)
        );
        String::from_utf8(o.stdout).unwrap().trim().to_owned()
    };
    git(&["init", "-q"]);
    git(&["add", "."]);
    git(&[
        "-c",
        "user.name=t",
        "-c",
        "user.email=t@t",
        "commit",
        "-qm",
        "t",
    ]);
    let sha = git(&["rev-parse", "HEAD"]);
    fs::write(
        arc.join(format!("{sha}.tar.gz")),
        format!("archive of {sha}\n"),
    )
    .unwrap();
    let sum = Command::new("sha256sum")
        .arg(arc.join(format!("{sha}.tar.gz")))
        .output()
        .unwrap();
    let sum = String::from_utf8(sum.stdout).unwrap();
    let sum = sum.split_whitespace().next().unwrap();

    let o = Command::new("bash")
        .arg(pb.join("tools/bump-wolos.sh"))
        .arg(&sha)
        .arg(&src)
        .env("WOLOS_ARCHIVE_URL", format!("file://{}", arc.display()))
        .output()
        .unwrap();
    let after = fs::read_to_string(pb.join("wolos-webapps/PKGBUILD")).unwrap();
    let _ = fs::remove_dir_all(&t);
    assert!(
        o.status.success(),
        "bump-wolos.sh: {}",
        String::from_utf8_lossy(&o.stderr)
    );
    assert!(
        after.contains(&format!("\n_wolos_commit={sha}\n")),
        "{after}"
    );
    assert!(
        after.contains(&format!("\n_wolos_archive_sha256={sum}\n")),
        "bump-wolos.sh did not re-pin the archive's sha256 to {sum}:\n{after}"
    );
    assert!(
        after.contains("archive/$_wolos_commit.tar.gz\")"),
        "{after}"
    );
}
