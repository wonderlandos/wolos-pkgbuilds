// SPDX-License-Identifier: GPL-2.0-or-later
//! One picture from power-on to the desktop (ADR 0012): loader splash, black console, locked start,
//! compositor colours. Scripts run against fakes; configs are checked line by line.
#![allow(dead_code, unused_imports)]
#[path = "support/siblings.rs"]
mod siblings;

use std::fs;
use std::os::unix::fs::PermissionsExt;
use std::path::{Path, PathBuf};
use std::process::Command;
fn repo(p: &str) -> PathBuf {
    siblings::root().join(p)
}

fn read(p: &str) -> String {
    fs::read_to_string(repo(p)).unwrap_or_else(|e| panic!("{p}: {e}"))
}

/// Boot splash: the WolOS mark (plymouth theme image) is a small 8-bit RGBA PNG.
#[test]
fn splash_is_the_wolos_mark() {
    let w = read("wolos-system/PKGBUILD");
    assert!(w.contains("design/brand/wolos-splash.png") || w.contains("$d/brand/wolos-splash.png"));
    let png = fs::read(repo("design/brand/wolos-splash.png")).unwrap();
    assert_eq!(&png[..8], b"\x89PNG\r\n\x1a\n");
    let be = |i: usize| u32::from_be_bytes(png[i..i + 4].try_into().unwrap());
    let (w, h) = (be(16), be(20));
    assert!(w <= 480 && h <= 480, "{w}x{h}");
    assert_eq!((png[24], png[25]), (8, 6), "8-bit RGBA");
}
