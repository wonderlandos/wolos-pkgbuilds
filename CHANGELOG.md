# Changelog

The user-facing changes of a WolOS release are in the wolos repository's `CHANGELOG.md`; this file
lists what changed in the recipes, their checks and their workflows. The four repositories are
tagged together (wolos ADR 0029).

## 0.12.0 (unreleased)

Recipes
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
