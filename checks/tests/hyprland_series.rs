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
