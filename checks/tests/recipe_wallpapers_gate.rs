// SPDX-License-Identifier: GPL-2.0-or-later
//! What this file checks (wolos-pkgbuilds): wolos-system ships the Moon Gate wallpaper the way the
//! desktop looks for it (shell kit/pickers.rs `gate_file`, kit/gate.rs `PARAMS_FILE`): the four
//! signature files, the gate in every built-in theme's dye generated at build from the theme files
//! (design brand/wallpapers/README.md, `walls.py themes siggate`), its parameter set
//! (`walls.py gate --params`), and Cachoeira Station with the earlier places when the design has them.
//! The run test sources the PKGBUILD and runs its two steps on the wolos sibling into a scratch pkgdir.
#[path = "support/siblings.rs"]
mod siblings;

use std::{fs, path::Path, process::Command};

fn read(p: &str) -> String {
    fs::read_to_string(siblings::root().join(p)).unwrap_or_else(|e| panic!("{p}: {e}"))
}

#[test]
fn the_recipe_generates_and_installs_the_gate() {
    let r = read("wolos-system/PKGBUILD");
    let makedeps = r
        .lines()
        .find(|l| l.starts_with("makedepends="))
        .expect("makedepends");
    assert!(
        makedeps.contains(" python"),
        "walls.py needs python at build: {makedeps}"
    );
    let build = &r[r.find("\nbuild() {").expect("build()")..];
    assert!(
        build.contains(r#"_brand_walls_build "$_repo" "$srcdir/brand-walls""#),
        "build() generates the theme dyes"
    );
    let package = &r[r.find("\npackage() {").expect("package()")..];
    assert!(
        package.contains(r#"_brand_walls_package "$_repo" "$srcdir/brand-walls" "$pkgdir""#),
        "package() installs them"
    );
    for want in [
        "themes siggate --out",
        "gate --params --out \"$e/wolos-gate.params.json\"",
        "WOLOS_THEMES=\"$1/crates/core/src/theme/builtin\"",
        "python3 -I",
        "usr/share/wolos/wallpapers",
    ] {
        assert!(r.contains(want), "{want}");
    }
}

/// A design tree without the generator must stop the build: the guard once returned 0 and the
/// package shipped no themed wallpapers without a word (the generator moved to tools/brand when the
/// design repo was unified and nothing noticed). The package step refuses a staging directory
/// that holds no gate too.
#[test]
fn a_design_tree_without_the_generator_fails_the_build_loudly() {
    let r = read("wolos-system/PKGBUILD");
    // Every generator path the recipe names under design/tools is tools/brand.
    for l in r.lines().filter(|l| !l.trim_start().starts_with('#')) {
        let mut rest = l;
        while let Some(i) = rest.find("design/tools/") {
            let tail = &rest[i + "design/tools/".len()..];
            assert!(
                tail.starts_with("brand/"),
                "the generator lives in tools/brand: {l}"
            );
            rest = tail;
        }
    }
    assert!(
        !r.contains("|| return 0"),
        "no silent skip in the recipe (RP-056)"
    );
    let tmp = Path::new(env!("CARGO_TARGET_TMPDIR")).join("brand-walls-missing");
    let _ = fs::remove_dir_all(&tmp);
    fs::create_dir_all(tmp.join("tree/design/tools")).unwrap();
    let script = format!(
        "set -e; source '{}'; _brand_walls_build '{}' '{}'",
        siblings::root().join("wolos-system/PKGBUILD").display(),
        tmp.join("tree").display(),
        tmp.join("stage").display(),
    );
    let out = Command::new("bash")
        .arg("-c")
        .arg(&script)
        .output()
        .expect("bash");
    let _ = fs::remove_dir_all(&tmp);
    assert!(
        !out.status.success(),
        "a missing walls.py must fail the build, not skip it"
    );
    let err = String::from_utf8_lossy(&out.stderr);
    assert!(
        err.contains("tools/brand/walls.py"),
        "the error names the generator: {err}"
    );
}

/// The package step must not pass over a staging directory without the gate (RP-056): it fails.
#[test]
fn the_package_step_refuses_an_empty_staging_directory() {
    let tmp = Path::new(env!("CARGO_TARGET_TMPDIR")).join("brand-walls-empty");
    let _ = fs::remove_dir_all(&tmp);
    fs::create_dir_all(tmp.join("stage")).unwrap();
    let script = format!(
        "set -e; source '{}'; _brand_walls_package '{}' '{}' '{}'",
        siblings::root().join("wolos-system/PKGBUILD").display(),
        tmp.join("tree").display(),
        tmp.join("stage").display(),
        tmp.join("pkg").display(),
    );
    let out = Command::new("bash")
        .arg("-c")
        .arg(&script)
        .output()
        .expect("bash");
    let shipped = tmp.join("pkg/usr/share/wolos/wallpapers").exists();
    let _ = fs::remove_dir_all(&tmp);
    assert!(
        !out.status.success() && !shipped,
        "an empty staging directory must fail the package"
    );
}

#[test]
fn the_steps_make_what_the_desktop_reads() {
    let root = siblings::root();
    let walls = root.join("design/tools/brand/walls.py");
    let themes = root.join("crates/core/src/theme/builtin");
    // No design checkout at all (a sibling without its submodule): nothing to run, said aloud. A
    // checkout that has the design but not the generator is the pin being behind: that fails.
    if !root.join("design/icons").is_dir() {
        eprintln!(
            "skip: the wolos sibling has no design checkout (WOLOS_SRC, git submodule update)"
        );
        return;
    }
    assert!(
        walls.is_file()
            && root
                .join("design/brand/wallpapers/wolos-gate-day-4k.svg")
                .is_file(),
        "the wolos sibling's design pin has no tools/brand/walls.py or brand/wallpapers: move the design \
         submodule to a commit that has them, or wolos-system ships no themed wallpapers"
    );
    let n_themes = fs::read_dir(&themes)
        .expect("theme files")
        .flatten()
        .filter(|e| e.path().extension().is_some_and(|x| x == "json"))
        .count();
    assert!(n_themes >= 20, "built-in themes: {n_themes}");
    let tmp = Path::new(env!("CARGO_TARGET_TMPDIR")).join("brand-walls-check");
    let _ = fs::remove_dir_all(&tmp);
    fs::create_dir_all(&tmp).unwrap();
    let (stage, pkg) = (tmp.join("stage"), tmp.join("pkg"));
    let script = format!(
        "set -e; source '{}'; _brand_walls_build '{}' '{}'; _brand_walls_package '{}' '{}' '{}'",
        root.join("wolos-system/PKGBUILD").display(),
        root.display(),
        stage.display(),
        root.display(),
        stage.display(),
        pkg.display()
    );
    let out = Command::new("bash")
        .arg("-c")
        .arg(&script)
        .output()
        .expect("bash");
    assert!(
        out.status.success(),
        "{}\n{}",
        String::from_utf8_lossy(&out.stdout),
        String::from_utf8_lossy(&out.stderr)
    );
    let wp = pkg.join("usr/share/wolos/wallpapers");
    for m in ["day", "night"] {
        for s in ["4k", "16x10"] {
            assert!(
                wp.join(format!("wolos-gate-{m}-{s}.svg")).is_file(),
                "gate {m} {s}"
            );
        }
    }
    // One file per theme, mode and size, each a whole SVG.
    let dyed: Vec<_> = fs::read_dir(wp.join("themes"))
        .unwrap()
        .flatten()
        .map(|e| e.path())
        .collect();
    assert_eq!(dyed.len(), n_themes * 4, "themes x day/night x 4k/16:10");
    for t in fs::read_dir(&themes).unwrap().flatten() {
        let id = t.path().file_stem().unwrap().to_string_lossy().into_owned();
        if t.path().extension().is_some_and(|x| x == "json") {
            let f = wp.join(format!("themes/gate-{id}-night-4k.svg"));
            let text = fs::read_to_string(&f).unwrap_or_else(|e| panic!("{}: {e}", f.display()));
            assert!(
                text.starts_with("<svg") && text.trim_end().ends_with("</svg>"),
                "{id}"
            );
        }
    }
    let params = fs::read_to_string(wp.join("wolos-gate.params.json")).expect("params");
    for want in [
        "\"ring\"",
        "\"sun\"",
        "\"squash\"",
        "\"ring_light\"",
        "\"ribbon_dye\"",
    ] {
        assert!(params.contains(want), "params {want}");
    }
    let places = root.join("design/brand/wallpapers");
    for p in ["cachoeira", "brasilia", "estaiada", "rioniteroi"] {
        if places.join(format!("{p}-day-4k.svg")).is_file() {
            assert!(
                wp.join(format!("{p}-night-16x10.svg")).is_file(),
                "place {p}"
            );
        }
    }
    let _ = fs::remove_dir_all(&tmp);
}
