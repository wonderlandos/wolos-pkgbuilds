// SPDX-License-Identifier: GPL-2.0-or-later
//! hyprland-wolos/SERIES.md is the map of the Hyprland patch series: every `0*.patch` beside the
//! PKGBUILD has its row, in name order (the order the PKGBUILD applies them), and the PKGBUILD's
//! `source=` names each one (audit4 S5: a patch missing from the table or the sources is a patch
//! nobody reviews or nobody builds).

fn dir() -> std::path::PathBuf {
    std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../hyprland-wolos")
}

fn patches() -> Vec<String> {
    let mut v: Vec<String> = std::fs::read_dir(dir())
        .expect("hyprland-wolos/")
        .filter_map(|e| e.ok()?.file_name().into_string().ok())
        .filter(|n| n.starts_with('0') && n.ends_with(".patch"))
        .collect();
    v.sort();
    v
}

/// Patch names in the first column of SERIES.md's table rows, in table order.
fn series_rows(md: &str) -> Vec<String> {
    md.lines()
        .filter_map(|l| l.strip_prefix("| "))
        .filter_map(|l| l.split(" |").next())
        .map(str::trim)
        .filter(|c| c.ends_with(".patch"))
        .map(str::to_owned)
        .collect()
}

#[test]
fn series_lists_every_patch_in_apply_order() {
    let md = std::fs::read_to_string(dir().join("SERIES.md")).expect("hyprland-wolos/SERIES.md");
    let files = patches();
    assert!(!files.is_empty(), "no patches found in {:?}", dir());
    assert_eq!(
        series_rows(&md),
        files,
        "SERIES.md rows must be exactly the 0*.patch files, in name order"
    );
}

#[test]
fn pkgbuild_sources_every_patch() {
    let pkgbuild =
        std::fs::read_to_string(dir().join("PKGBUILD")).expect("hyprland-wolos/PKGBUILD");
    for p in patches() {
        assert!(
            pkgbuild.lines().any(|l| l.trim() == p),
            "{p} is not in the PKGBUILD source= array"
        );
    }
}

#[test]
fn row_parser_reads_only_patch_rows() {
    let md = "| patch | area |\n|---|---|\n| 0001-a.patch | x |\n| 0002-b.patch | y |\nnot a row 0003-c.patch\n";
    assert_eq!(series_rows(md), ["0001-a.patch", "0002-b.patch"]);
}

/// The base SERIES.md names is the PKGBUILD's `pkgver` (a rebase onto a new tag changes both).
#[test]
fn series_names_the_pkgbuild_base() {
    let pkgbuild =
        std::fs::read_to_string(dir().join("PKGBUILD")).expect("hyprland-wolos/PKGBUILD");
    let pkgver = pkgbuild
        .lines()
        .find_map(|l| l.strip_prefix("pkgver="))
        .expect("pkgver=");
    let md = std::fs::read_to_string(dir().join("SERIES.md")).expect("hyprland-wolos/SERIES.md");
    let base = md
        .lines()
        .find(|l| l.starts_with("Base: "))
        .expect("SERIES.md Base: line");
    assert!(
        base.contains(&format!("({pkgver};")) || base.contains(&format!("({pkgver})")),
        "{base}"
    );
}

/// `source=(...)` entries of the PKGBUILD (one per line) and `sha256sums=(...)` entries.
fn array_items(pkgbuild: &str, name: &str) -> Vec<String> {
    let start = pkgbuild
        .find(&format!("\n{name}=("))
        .unwrap_or_else(|| panic!("{name}=( in the PKGBUILD"))
        + name.len()
        + 3;
    let body = &pkgbuild[start..];
    body[..body.find(')').expect("closing )")]
        .split_whitespace()
        .map(|w| w.trim_matches('\'').trim_matches('"').to_owned())
        .collect()
}

/// A checksum per source, none merged into another line (a spliced `sha256sums` entry passes
/// `makepkg --geninteg` nowhere and fails the build on the forge, long after the edit).
#[test]
fn pkgbuild_has_one_checksum_per_source_and_one_name_per_line() {
    let pkgbuild =
        std::fs::read_to_string(dir().join("PKGBUILD")).expect("hyprland-wolos/PKGBUILD");
    let sources = array_items(&pkgbuild, "source");
    let sums = array_items(&pkgbuild, "sha256sums");
    assert_eq!(sources.len(), sums.len(), "source= and sha256sums= differ in length");
    for s in &sums {
        assert!(
            s.len() == 64 && s.bytes().all(|b| b.is_ascii_hexdigit()),
            "not a sha256: {s}"
        );
    }
    for p in patches() {
        let n = sources.iter().filter(|s| **s == p).count();
        assert_eq!(n, 1, "{p} must be in source= exactly once");
    }
    // One patch per number: two 0018 files would apply twice or fight over the same hunks.
    let all = patches();
    let mut numbers: Vec<&str> = patches_numbers(&all);
    let before = numbers.len();
    numbers.dedup();
    assert_eq!(before, numbers.len(), "two patch files share a number");
}

fn patches_numbers(names: &[String]) -> Vec<&str> {
    names.iter().map(|n| &n[..4]).collect()
}

/// QA-LIVE-22: after a Hyprland crash `start-hyprland` restarts with the person's config, never
/// with `--safe-mode` (a bare desktop without the shell), and gives up after a second crash
/// within 30 s so the login window comes back. Patch 0018 carries it, with its gtest.
#[test]
fn start_hyprland_restarts_with_the_config_not_safe_mode() {
    let patch = patches()
        .into_iter()
        .find(|n| n.starts_with("0018-start-hyprland-"))
        .expect("patch 0018 (start-hyprland restart policy)");
    let text = std::fs::read_to_string(dir().join(&patch)).unwrap();
    // The upstream line that turned a crash into safe mode is removed, and the policy is used.
    assert!(
        text.contains("-            safeMode = true;"),
        "0018 removes `safeMode = true` after an unclean exit"
    );
    assert!(text.contains("safeMode = WolosRestart::safeModeAfterCrash();"));
    assert!(text.contains("WolosRestart::afterCrash(lastCrash, NOW)"));
    assert!(text.contains("return 1;"), "a second crash exits with 1");
    assert!(text.contains("REPEAT_WINDOW{30}"), "the repeat window is 30 s");
    // Its tests are in the patch and named in SERIES.md.
    assert!(text.contains("tests/helpers/WolosRestart.cpp"));
    let md = std::fs::read_to_string(dir().join("SERIES.md")).unwrap();
    for t in [
        "WolosRestart.aFirstCrashRestartsWithTheWolosConfig",
        "WolosRestart.aSecondCrashWithinThirtySecondsGivesUp",
    ] {
        assert!(text.contains(&t["WolosRestart.".len()..]), "{t} is in the patch");
        assert!(md.contains(t), "{t} is named in SERIES.md");
    }
}
