# Changelog

The user-facing changes of a WolOS release are in the wolos repository's `CHANGELOG.md`; this file
lists what changed in the recipes, their checks and their workflows. The four repositories are
tagged together (wolos ADR 0029).

## 0.13.0 (unreleased)

Recipes
- `rsh` 0.1.3: Ruby with a shell layer (wolos ADR 0041), built from the tagged release tarball with `make test` (unit, golden, contract, interactive and sanitizer runs) in `check()`. Against the upstream package text: `ruby-rake` is a make dependency (mruby's `minirake` runs `rake`), `procps-ng` a check dependency (a golden test counts zombies with `ps`), and the interactive completion check uses `/etc/os-release` instead of `/etc/hostname`, which a clean build chroot does not have.
- `scx-wolos`: the `scx_wolos` QoS scheduler on sched_ext (wolos `kernel/scx_wolos`) with `scx_wolos.service`, on by default through its preset. Built against the kernel it runs on: sched_ext headers from the WolKernel tag tarball `wolkernel-1.0-7.3.1`, the same file and checksum as `linux-wolos` (`tools/bump-kernel.sh` moves all three), BTF from the `linux-wolos` package's `vmlinuz`. pkgrel 2: built for `wolkernel-1.0-7.3.1`. The hardened `scx_wolos.service` (QA-PRIV-14) and the per-CPU background stamp (QA-SYS-13) are in wolos `kernel/scx_wolos` and reach this package only once `bump-wolos.sh` moves `_wolos_commit` to a wolos commit that has them.
- `wolos-system` installs `/usr/share/wolos/plugins.schema.json` (ADR 0031), loads `ntsync` at boot, adds the CJK, Arabic and Hebrew font fallbacks (`/etc/fonts/conf.d/70-wolos-cjk-arabic.conf`), turns on power-profiles-daemon and keeps the fwupd metadata download off (`51-wolos-image.preset`), and puts `mdns_minimal` before `resolve` in `/etc/nsswitch.conf` on install and upgrade, so `.local` names resolve through avahi.
- `wolos-module-dev` depends on `rubocop` (the linter `wol script check` runs on Ruby scripts).
- New optional modules `wolos-module-ime` (fcitx5 with Mozc, Chinese and Hangul) and `wolos-module-scanner` (SANE, sane-airscan, ipp-usb, Document Scanner); their registry records come with wolos.

Checks
- `checks/tests/image_promises.rs`: the packages and image defaults the code runs (ruby, rsh, scx-wolos, power-profiles-daemon, fwupd, the CJK fonts, nss-mdns, the gvfs backends) are in the image lists and their recipes, and the nsswitch edit is idempotent.

## 0.12.0

Recipes
- `linux-wolos` and `linux-wolos-api-headers` 1:1.0+7.3.1-1: WolKernel 1.0 on Linux 7.3.1 (stable, 24 CVE fixes). The fork series (branch `wolos-7.3.1`, tag `wolkernel-1.0-7.3.1`) is rebased from its former base, a 7.4 merge-window snapshot that was labelled 7.3.0, onto the released 7.3 line; options only 7.4 has (for example the steal governor) are gone. `tools/bump-kernel.sh wolkernel-1.0-7.3.1` fills the tarball checksum once the tag is on GitHub.
- Optional modules as meta packages: `wolos-module-ai-claude`, `-ai-local`, `-ai-sessions`, `-console`, `-creative`, `-dev` and `-gaming` (depends only, `arch=any`). `wolos-module-ai-local` turns on its local AI service; `wolos-module-ai-claude` installs the WolOS managed settings for Claude Code, so they no longer ship with every system.
- `wolos-hue`: the `wol-hue` theme tool, with its third-party notice.
- `wolos-system` installs `THIRD-PARTY.md` under `/usr/share/licenses/wolos-system/` and keeps more of its `/etc` files on upgrade (`backup=`).
- The compositor fork: six patches regenerated after the window manager audit (frame ticks, cursor hiding, occluded windows, frame timing, lock replacement, the zone under the pointer); all patches apply on the pinned base, their checksums are updated and the series is listed in `hyprland-wolos/SERIES.md`.
- `wolos-base`: the prebuild keeps its package database and cache between runs (a warm prebuild takes about 20 s instead of 420 s), with the same manifest.
- `wolos-mirrorlist`: the CDN lines are commented out until the CDN exists, so a sync does not first fail on a name that does not resolve; a testing image installs a system on its own channel.

Snapshot and workflows
- `tools/bump-snapshot.sh YYYY-MM-DD` moves the whole system to one Arch Linux snapshot date in every file that names it; a check keeps them in step.
- `publish.yml` and `promote.yml` check out this repository at the commit the caller pinned, not at a branch, and every publish uploads the snapshot date.
- The builder image is pinned by digest in `.github/builder-image`, the same digest as wolos-iso.

Checks
- `tools/lint.sh` and new checks: packaging rules (docs/RULES-packaging.md), recipe lint rules, the base manifest, the snapshot, the modules and their shipped files, the compositor patch series and the executable bits of shipped scripts.
