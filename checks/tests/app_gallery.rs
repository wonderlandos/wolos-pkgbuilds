// SPDX-License-Identifier: GPL-2.0-or-later
//! What this file checks (wolos-pkgbuilds): The wolos-app-gallery recipe: Gallery (org.wolos.gallery), the Crystal picture library and viewer, built, checked and installed from wolos-crystal/apps/gallery.
//! The recipe builds with the app's own Makefile against the system libwolkit and libwol (the
//! shards are relative requires inside the wolos tree, so nothing is fetched), runs the app's
//! whole `make spec` in check() (pure specs, the headless engine against the golden screenshots,
//! the look against its sheet) and installs the program as `wol-gallery` with its launcher entry.
//! These tests hold the recipe, the `packages` line and the app files the recipe relies on together.
#![allow(dead_code, unused_imports)]
#[path = "support/siblings.rs"]
mod siblings;

use std::fs;
fn read(p: &str) -> String {
    let path = siblings::root().join(p);
    fs::read_to_string(&path).unwrap_or_else(|e| panic!("{p}: {e}"))
}

const APP: &str = "wolos-crystal/apps/gallery";
const RECIPE: &str = "wolos-app-gallery/PKGBUILD";

fn words(line: &str) -> Vec<&str> {
    line.split(['(', ')', ' ', '\'', '"'])
        .filter(|w| !w.is_empty())
        .collect()
}

#[test]
fn the_recipe_builds_checks_and_installs_the_app() {
    let p = read(RECIPE);
    for want in [
        "pkgname=wolos-app-gallery",
        "cd \"$srcdir/wolos/wolos-crystal/apps/gallery\"",
        "make bin/gallery WOLKIT_BUILD=/usr/lib RELEASE='--release --no-debug'",
        "make spec WOLKIT_BUILD=/usr/lib WOLKIT_STYLEC=/usr/bin/wolkit-stylec",
        "\"$pkgdir/usr/bin/wol-gallery\"",
        "\"$pkgdir/usr/share/applications/org.wolos.gallery.desktop\"",
        "\"$_app/org.wolos.gallery.consent.json\" \"$pkgdir/usr/share/wolos/consent.d/org.wolos.gallery.json\"",
        "\"$pkgdir/usr/share/licenses/$pkgname/LICENSE\"",
        "pkgver=0.13.0  # the newest CHANGELOG.md entry at _wolos_commit",
    ] {
        assert!(p.contains(want), "{RECIPE} lacks {want}");
    }
    let depends = p.lines().find(|l| l.starts_with("depends=(")).unwrap();
    for d in ["libwolkit", "libwol", "gc", "pcre2", "openssl"] {
        assert!(words(depends).contains(&d), "{depends} lacks {d}");
    }
    let make = p.lines().find(|l| l.starts_with("makedepends=(")).unwrap();
    // 'crystal>=1.22.0': the recipe pins the compiler's floor (RCR-046), the package is crystal
    assert!(
        words(make)
            .iter()
            .any(|w| w.trim_matches('\'').split(['<', '>', '=']).next() == Some("crystal")),
        "{make} lacks crystal"
    );
    let license = p.lines().find(|l| l.starts_with("license=(")).unwrap();
    assert_eq!(words(license), ["license=", "BSD-2-Clause"], "{license}");
    // Never fetch shards: the app has no dependencies and builds with crystal itself.
    assert!(
        !p.contains("shards install") && !p.contains("shards build"),
        "{RECIPE} runs shards"
    );
}

#[test]
fn gallery_is_a_line_of_packages_built_after_the_engine() {
    let line = read("packages")
        .lines()
        .find(|l| l.split_whitespace().next() == Some("wolos-app-gallery"))
        .map(str::to_owned)
        .expect("wolos-app-gallery is a line of packages");
    let w: Vec<&str> = line.split_whitespace().collect();
    assert_eq!(w[1], "wolos-app-gallery", "{line}");
    let deps: Vec<&str> = w[2].split(',').collect();
    assert!(
        deps.contains(&"libwolkit") && deps.contains(&"libwol"),
        "{line}"
    );
    assert_eq!(&w[3..], ["wolos-app-gallery/"], "{line}");
}

#[test]
fn the_app_has_what_the_recipe_installs_and_runs() {
    let mk = read(&format!("{APP}/Makefile"));
    for want in [
        "bin/gallery:",
        "spec: spec-core spec-engine",
        "BUILD := $(abspath $(or $(WOLKIT_BUILD),$(SHARD)/build))",
        "STYLEC := $(or $(WOLKIT_STYLEC),",
        "$(RELEASE)",
        "FONTS := $(ROOT)/crates/wolkit/tests/golden/fonts",
    ] {
        assert!(mk.contains(want), "{APP}/Makefile lacks {want}");
    }
    let desktop = read(&format!("{APP}/org.wolos.gallery.desktop"));
    for want in [
        "Exec=wol-gallery %F",
        "Icon=wolos-photos",
        "StartupWMClass=org.wolos.gallery",
        "MimeType=image/jpeg;image/png;",
    ] {
        assert!(
            desktop.lines().any(|l| l == want),
            "the desktop entry lacks {want}"
        );
    }
    // The default grants the package installs: exactly the two verbs the app uses, for its program.
    let consent = read(&format!("{APP}/org.wolos.gallery.consent.json"));
    for want in [
        r#"{ "app": "/usr/bin/wol-gallery", "capability": "trash" }"#,
        r#"{ "app": "/usr/bin/wol-gallery", "capability": "share" }"#,
    ] {
        assert!(consent.contains(want), "the consent defaults lack {want}");
    }
    assert_eq!(consent.matches("capability").count(), 2, "{consent}");
    let main = read(&format!("{APP}/src/main.cr"));
    assert!(
        main.contains("app_id: Gallery::APP_ID"),
        "{APP}/src/main.cr"
    );
    assert!(read(&format!("{APP}/src/gallery/dirs.cr")).contains("APP_ID = \"org.wolos.gallery\""));
    let shard = read(&format!("{APP}/shard.yml"));
    assert!(shard.contains("name: gallery") && shard.contains("license: BSD-2-Clause"));
    assert!(
        !shard.contains("dependencies:"),
        "the app takes no shard dependency"
    );
    for f in [
        "spec/golden/library-date.png",
        "spec/golden/viewer.png",
        "spec/golden/empty.png",
        "spec/golden/loading.png",
        "spec/golden/error.png",
        "DESIGN.md",
        "tools/styles.sh",
    ] {
        assert!(
            siblings::root().join(APP).join(f).exists(),
            "{APP}/{f} is missing"
        );
    }
    assert!(siblings::root().join("LICENSES/BSD-2-Clause.txt").exists());
}
