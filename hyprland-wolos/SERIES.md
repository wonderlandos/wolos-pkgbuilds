# hyprland-wolos series

Base: upstream Hyprland `v$pkgver` of `PKGBUILD` (0.57.1; v0.12.x keeps the 0.56.2 series in its
release tag). The PKGBUILD applies every `0*.patch` beside it in name order (`patch -Np1`). Map of
what each one is for: `wolos/docs/modules/hyprland-fork.md`.
Policy math lives in `src/helpers/Wolos*.hpp` (pure), tested by `tests/helpers/Wolos*.cpp` (gtest).
`checks/tests/hyprland_series.rs` keeps this table, the patch files and the PKGBUILD `source=` in step.
Dropped on 0.57: the old 0003 (no `.conf` deprecation banner): 0.57 has no `.conf` loader left. The other
file names keep their numbers so references to them stay valid.

Check that the series still applies (forge, no Hyprland headers needed):

    v=$(bash -c 'source PKGBUILD; echo $pkgver')
    git clone -q --depth 1 -b v$v https://github.com/hyprwm/Hyprland /tmp/hl-check
    cd /tmp/hl-check && for p in <this dir>/0*.patch; do git apply --check "$p" && git apply "$p"; done

| patch | area | what | test |
|---|---|---|---|
| 0001-animation-tick-once-per-monitor-frame-instead-of-eve.patch | animation | animation timer ticks twice per frame of the fastest monitor instead of every 1 ms (a 1x timer beat against vblank: judder, audit4 S5-7); the file name predates the 2x | `WolosPerf.animationTickIsTwicePerFrameOfFastestMonitor` |
| 0002-renderer-arm-cursor-hide-ticker-for-the-timeout-5-s-.patch | cursor | cursor-hide ticker sleeps until `cursor:inactive_timeout` can fire (5 s heartbeat otherwise); pointer motion that unhides re-arms it (S5-10) | `WolosPerf.cursorTickerSleepsUntilTimeout` |
| 0004-xdg-shell-advertise-minimise-client-minimise-moves-t.patch | xdg-shell | `wm_capabilities` minimise; client minimise moves the window to `special:hidden` | - |
| 0005-session-lock-a-takeover-lock-hides-workspaces-from-i.patch | session lock | a takeover lock hides workspaces from its first frame; crash restart relaunches the locker (ADR 0012) | `WolosLock.*` |
| 0006-sendshortcut-keys-resolve-without-held-modifiers-the.patch | input | `send_shortcut`/`send_key_state` find the key in any layout (keycode resolver), tap only the asked modifiers, restore the real ones | `WolosKeys.*` |
| 0007-layer-shell-content-commits-damage-only-what-changed.patch | render | layer content commits damage only what changed; quiet backdrop commits keep the cached blur | `WolosPerf.layerContentCommitIsDamagedPrecisely` |
| 0008-focus-restores-a-minimised-window-to-the-current-des.patch | focus | focusing a minimised window restores it to the current desktop | `WolosWindow.focusRestoresOnlyMinimisedWindows` |
| 0009-direct-scanout-auto-mode-also-covers-fullscreen-vide.patch | scanout | `direct_scanout = 2` also covers fullscreen video and tearing-hint games | `WolosPerf.autoScanoutCoversGamesAndVideo` |
| 0010-render-covered-windows-get-frame-callbacks-at-most-e.patch | render | covered windows get frame callbacks every 33 ms or 2 updates past 10 windows; occluders by opaque region (rounded/CSD windows count), windows allowed over a fullscreen window keep their frames (S5-9) | `WolosPerf.fullscreenCoversOnlyWhatHyprlandHidesUnderIt`, `roundedAndShadowedWindowsStillOcclude` |
| 0011-output-after-a-mode-change-rendering-waits-at-most-2.patch | output | after a mode change rendering waits at most 250 ms for clients to ack | `WolosPerf.reconfigureWaitsForAcksButNeverPast250ms` |
| 0012-render-frame-callbacks-carry-the-output-s-predicted-.patch | render | frame callbacks carry the predicted next present time, one monotonic floor across outputs (S5-12); late render = `misc:frame_deadline` | `WolosPerf.frameTimesNeverGoBackAcrossOutputs` |
| 0013-xdg-shell-own-frame-windows-see-zoom-change-state.patch | xdg-shell | toplevels with an xdg-decoration object see zoom as a maximized state change (in the Wayland window backend) | - |
| 0014-session-lock-a-dead-locker-leaves-black-while-one-can.patch | session lock | a dead locker leaves black while wol-lock restores it; only `/usr/bin/wol-lock` may replace a held or pending lock (S5-15) | `WolosLock.onlyWolLockTakesOverALockedSession` |
| 0015-layout-a-dragged-window-never-goes-under-the-menu-bar.patch | layout | a dragged floating window never goes under the menu bar and keeps 40 px on screen, clamped to the output under the pointer (S5-8) | `WolosWindow.dragReachesTheOutputAbove` |
| 0016-monitor-max_bpc-only-when-built-against-aquamarine-0.patch | output | the 0.57.1 max-bpc call is compiled only against aquamarine 0.16 or newer (the snapshot has 0.15.1); retires itself when the snapshot moves | - |
| 0017-ipc-keyword-and-the-0.56-dispatch-form-on-the-lua-co.patch | ipc | `hyprctl keyword NAME VALUE` runs as the Lua call it means (options with typed literals, `bind*`/`unbind`, `animation` with a `spring:NAME` curve as `spring`, named rules; a rule update without `enabled` keeps the rule's state); `hyprctl dispatch NAME ARGS` with a 0.56 dispatcher name runs it (the 0.56 translator, on `Config::Actions`); keyword binds call it via `hl.legacy_dispatch`. Lua forms unchanged | `WolosCompat.*` |
| 0018-start-hyprland-a-crash-restarts-with-the-config-never-safe.patch | start | a crash restarts Hyprland with the same arguments and config and the crash lock, never `--safe-mode` (which dropped the WolOS config); a second crash within 30 s ends `start-hyprland` with 1 so the login window comes back (QA-LIVE-22) | `WolosRestart.aFirstCrashRestartsWithTheWolosConfig`, `WolosRestart.aSecondCrashWithinThirtySecondsGivesUp` |
