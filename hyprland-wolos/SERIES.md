# hyprland-wolos series

Base: upstream Hyprland `v$pkgver` of `PKGBUILD` (0.56.2). The PKGBUILD applies every `0*.patch` beside it
in name order (`patch -Np1`). Map of what each one is for: `wolos/docs/modules/hyprland-fork.md`.
Policy math lives in `src/helpers/Wolos*.hpp` (pure), tested by `tests/helpers/Wolos*.cpp` (gtest).
`checks/tests/hyprland_series.rs` keeps this table, the patch files and the PKGBUILD `source=` in step.

Check that the series still applies (forge, no Hyprland headers needed):

    v=$(bash -c 'source PKGBUILD; echo $pkgver')
    git clone -q --depth 1 -b v$v https://github.com/hyprwm/Hyprland /tmp/hl-check
    cd /tmp/hl-check && for p in <this dir>/0*.patch; do git apply --check "$p" && git apply "$p"; done

| patch | area | what | test |
|---|---|---|---|
| 0001-animation-tick-once-per-monitor-frame-instead-of-eve.patch | animation | animation timer ticks twice per frame of the fastest monitor instead of every 1 ms (a 1x timer beat against vblank: judder, audit4 S5-7); the file name predates the 2x | `WolosPerf.animationTickIsTwicePerFrameOfFastestMonitor` |
| 0002-renderer-arm-cursor-hide-ticker-for-the-timeout-5-s-.patch | cursor | cursor-hide ticker sleeps until `cursor:inactive_timeout` can fire (5 s heartbeat otherwise); pointer motion that unhides re-arms it (S5-10) | `WolosPerf.cursorTickerSleepsUntilTimeout` |
| 0003-config-no-.conf-deprecation-banner-WolOS-pins-0.56-w.patch | config | no `.conf` deprecation banner (WolOS pins 0.56 with a `.conf`) | - |
| 0004-xdg-shell-advertise-minimise-client-minimise-moves-t.patch | xdg-shell | `wm_capabilities` minimise; client minimise moves the window to `special:hidden` | - |
| 0005-session-lock-a-takeover-lock-hides-workspaces-from-i.patch | session lock | a takeover lock hides workspaces from its first frame; crash restart relaunches the locker (ADR 0012) | `WolosLock.*` |
| 0006-sendshortcut-keys-resolve-without-held-modifiers-the.patch | input | `sendshortcut` finds the key in any layout, taps only the asked modifiers, restores the real ones | `WolosKeys.*` |
| 0007-layer-shell-content-commits-damage-only-what-changed.patch | render | layer content commits damage only what changed; quiet backdrop commits keep the cached blur | `WolosPerf.layerContentCommitIsDamagedPrecisely` |
| 0008-focus-restores-a-minimised-window-to-the-current-des.patch | focus | focusing a minimised window restores it to the current desktop | `WolosWindow.focusRestoresOnlyMinimisedWindows` |
| 0009-direct-scanout-auto-mode-also-covers-fullscreen-vide.patch | scanout | `direct_scanout = 2` also covers fullscreen video and tearing-hint games | `WolosPerf.autoScanoutCoversGamesAndVideo` |
| 0010-render-covered-windows-get-frame-callbacks-at-most-e.patch | render | covered windows get frame callbacks every 33 ms or 2 updates past 10 windows; occluders by opaque region (rounded/CSD windows count), windows allowed over a fullscreen window keep their frames (S5-9) | `WolosPerf.fullscreenCoversOnlyWhatHyprlandHidesUnderIt`, `roundedAndShadowedWindowsStillOcclude` |
| 0011-output-after-a-mode-change-rendering-waits-at-most-2.patch | output | after a mode change rendering waits at most 250 ms for clients to ack | `WolosPerf.reconfigureWaitsForAcksButNeverPast250ms` |
| 0012-render-frame-callbacks-carry-the-output-s-predicted-.patch | render | frame callbacks carry the predicted next present time, one monotonic floor across outputs (S5-12); late render = `misc:frame_deadline` | `WolosPerf.frameTimesNeverGoBackAcrossOutputs` |
| 0013-xdg-shell-own-frame-windows-see-zoom-change-state.patch | xdg-shell | toplevels with an xdg-decoration object see zoom as a maximized state change | - |
| 0014-session-lock-a-dead-locker-leaves-black-while-one-can.patch | session lock | a dead locker leaves black while wol-lock restores it; only `/usr/bin/wol-lock` may replace a held or pending lock (S5-15) | `WolosLock.onlyWolLockTakesOverALockedSession` |
| 0015-layout-a-dragged-window-never-goes-under-the-menu-bar.patch | layout | a dragged floating window never goes under the menu bar and keeps 40 px on screen, clamped to the output under the pointer (S5-8) | `WolosWindow.dragReachesTheOutputAbove` |
