// SPDX-License-Identifier: GPL-2.0-or-later
//! Memory-bandwidth #2: fullscreen buffers go straight to a KMS plane and browser video is decoded
//! by VA-API into dmabufs (NV12 1.5 B/px, no CPU copy) instead of software RGBA.
#![allow(dead_code, unused_imports)]
#[path = "support/siblings.rs"]
mod siblings;

use std::path::PathBuf;
fn read(p: &str) -> String {
    std::fs::read_to_string(siblings::root().join(p)).unwrap()
}

#[test]
fn direct_scanout_and_vaapi_decode_are_on() {
    let perf = read("desktop/overlay/usr/share/wolos/hypr/wolos-perf.conf");
    assert!(perf.contains("direct_scanout = 2"));
    let pol = read("desktop/browser-config/policies.json");
    for k in [
        r#""media.ffmpeg.vaapi.enabled": { "Value": true"#,
        r#""media.hardware-video-decoding.force-enabled": { "Value": true"#,
    ] {
        assert!(pol.contains(k), "policies.json lacks {k}");
    }
}

/// Upstream `direct_scanout = 2` scans out only content-type game: browser video kept compositing.
#[test]
fn fork_scans_out_fullscreen_video_and_tearing_games() {
    let dir = siblings::root().join("hyprland-wolos");
    let mut names: Vec<String> = std::fs::read_dir(&dir)
        .unwrap()
        .map(|e| e.unwrap().file_name().into_string().unwrap())
        .collect();
    names.sort();
    let want = [
        "0007-layer-shell-content-commits-damage-only-what-changed",
        "0008-focus-restores-a-minimised-window-to-the-current-des",
        "0009-direct-scanout-auto-mode-also-covers-fullscreen-vide",
    ];
    for w in want {
        assert!(names.iter().any(|n| n.starts_with(w)), "missing patch {w}");
    }
    let scanout = names.iter().find(|n| n.starts_with(want[2])).unwrap();
    let p = read(&format!("hyprland-wolos/{scanout}"));
    assert!(
        p.lines()
            .any(|l| l.starts_with('+') && l.contains("CONTENT_TYPE_VIDEO"))
    );
}
