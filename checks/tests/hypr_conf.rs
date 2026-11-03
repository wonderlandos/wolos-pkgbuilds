// SPDX-License-Identifier: GPL-2.0-or-later
//! The desktop compositor config drives the title-bar plugin and window animations; it must agree
//! with the realtime geometry/curves and the kit: 12 px window controls on the right, close
//! rightmost (centre 22.5 px in from the right edge), then zoom and minimise 22 px apart leftward.
//! Live 0.56.2: the controls were green-yellow-red, 20 px apart, CSD apps got a second bar, and the
//! open animation used a 400 ms overshooting spring instead of our ease-out. The 0.57 series reads
//! only hyprland.lua; hyprland.conf ships beside it until every writer has switched.
#![allow(dead_code, unused_imports)]
#[path = "support/siblings.rs"]
mod siblings;

/// wolos desktop/overlay/usr/share/wolos/hypr/hyprland.conf (a sibling repo: read at run time).
fn conf() -> String {
    let p = format!(
        "{}/desktop/overlay/usr/share/wolos/hypr/hyprland.conf",
        siblings::root_str()
    );
    std::fs::read_to_string(&p).unwrap_or_else(|e| panic!("{p}: {e}"))
}

fn value(key: &str) -> String {
    conf()
        .lines()
        .map(str::trim)
        .find_map(|l| {
            l.strip_prefix(key)?
                .trim()
                .strip_prefix('=')
                .map(|v| v.trim().to_owned())
        })
        .unwrap_or_else(|| panic!("{key} missing"))
}

/// Live: two Hyprland banners on a fresh session ("started without start-hyprland",
/// ".conf ... removed in 0.57"). Customers must never see compositor nags.
#[test]
fn session_starts_hyprland_without_nag_banners() {
    let session = std::fs::read_to_string(format!(
        "{}/desktop/overlay/usr/bin/wol-session",
        siblings::root_str()
    ))
    .expect("wolos desktop/overlay/usr/bin/wol-session");
    let launch = session
        .lines()
        .rev()
        .find(|l| l.starts_with("exec "))
        .unwrap();
    // On the user bus inside a logind session, else a private bus: always through start-hyprland.
    assert_eq!(launch, "exec ${bus:+$bus --} start-hyprland");
    // hyprland-guiutils is not in the image: its "missing" banner must be off (both forms).
    assert_eq!(value("disable_hyprland_guiutils_check"), "true");
    assert!(
        lua()
            .lines()
            .any(|l| l.trim() == "disable_hyprland_guiutils_check = true,"),
        "hyprland.lua keeps the guiutils banner off"
    );
    // The 0.57 series reads only Lua, so there is no `.conf` banner to drop (the 0.56 series'
    // patch 0003 is gone): the session's user file loads the shipped hyprland.lua.
    let pkgver =
        std::fs::read_to_string(format!("{}/hyprland-wolos/PKGBUILD", siblings::root_str()))
            .unwrap()
            .lines()
            .find_map(|l| l.strip_prefix("pkgver=").map(str::to_owned))
            .unwrap();
    assert!(pkgver.starts_with("0.57."), "{pkgver}");
    assert!(
        session.contains("require(\"$share/hypr/hyprland.lua\")"),
        "wol-session writes the Lua user file"
    );
}

/// wolos desktop/overlay/usr/share/wolos/hypr/hyprland.lua: the compositor 0.57 configuration.
fn lua() -> String {
    let p = format!(
        "{}/desktop/overlay/usr/share/wolos/hypr/hyprland.lua",
        siblings::root_str()
    );
    std::fs::read_to_string(&p).unwrap_or_else(|e| panic!("{p}: {e}"))
}

/// Live: libadwaita drew Text Editor's minimise light disabled; Hyprland 0.56 never advertised
/// the minimise capability and ignored set_minimized.
#[test]
fn fork_makes_client_minimise_real() {
    let dir = format!("{}/hyprland-wolos", siblings::root_str());
    let patches: String = std::fs::read_dir(dir)
        .unwrap()
        .map(|e| e.unwrap().path())
        .filter(|p| p.extension().is_some_and(|x| x == "patch"))
        .map(|p| std::fs::read_to_string(p).unwrap())
        .collect();
    let added = |s: &str| patches.lines().any(|l| l.starts_with('+') && l.contains(s));
    assert!(added("XDG_TOPLEVEL_WM_CAPABILITIES_MINIMIZE"));
    // 0.57: the window backend hands the client's request over as `minimized`.
    assert!(added("request.minimized") && added("special:hidden"));
}

/// Live: `focuswindow` (switcher, menu actions) focused a floating window but left it behind
/// another; wol-hypr raises the window that becomes active.
#[test]
fn own_frame_windows_see_zoom_change_state() {
    let dir = format!("{}/hyprland-wolos", siblings::root_str());
    let patches: String = std::fs::read_dir(dir)
        .unwrap()
        .map(|e| e.unwrap().path())
        .filter(|p| p.extension().is_some_and(|x| x == "patch"))
        .map(|p| std::fs::read_to_string(p).unwrap())
        .collect();
    let added = |s: &str| patches.lines().any(|l| l.starts_with('+') && l.contains(s));
    // Live on Helium 0.20.2.1: the zoom button zoomed once and then did nothing, because every
    // window stays "maximized" from its first map and so zooming changed no state. Every toplevel
    // with an xdg-decoration object (client- or server-side frame: Chromium, Qt, SDL) gets the
    // state dropped for one configure; GTK creates none and keeps it (docs/modules/hyprland-fork.md).
    assert!(added("seesZoomState(TOPLEVEL->wlToplevel())"));
    assert!(added("return m_decorations.contains(toplevel);"));
    assert!(added("TOPLEVEL->setMaximized(false);"));
    assert!(added("->setMaximized(true);"));
    // Dropping the map-time state instead gives clients shadow margins: windows land 10 to 20 px
    // off and clipped (measured on Helium and libadwaita).
    assert!(
        !patches
            .lines()
            .any(|l| l.starts_with('-') && l.contains("m_toplevel->setMaximized(true);"))
    );
}

/// The keys must exist in the fork, or Hyprland reports a config error at login.
#[test]
fn fork_registers_the_frame_policy_keys() {
    let dir = format!("{}/hyprland-wolos", siblings::root_str());
    let patches: String = std::fs::read_dir(dir)
        .unwrap()
        .map(|e| e.unwrap().path())
        .filter(|p| p.extension().is_some_and(|x| x == "patch"))
        .map(|p| std::fs::read_to_string(p).unwrap())
        .collect();
    for k in [
        "misc:throttle_occluded",
        "misc:throttle_window_count",
        "misc:throttle_occlusion_cutoff",
        "misc:throttle_freeze_ms",
        "misc:throttle_freeze_updates",
        "misc:reconfigure_ack_timeout_ms",
        "misc:frame_deadline",
    ] {
        assert!(
            patches.contains("+        MS<") && patches.contains(&format!("\"{k}\"")),
            "{k}"
        );
    }
}
