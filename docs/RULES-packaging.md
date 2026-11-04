# Packaging rules (RP)

Consistency rules for the recipes (PKGBUILDs), the `packages` file, the `.install` scripts, the
pins that `tools/bump-wolos.sh` rewrites, and the contract tests in `checks/`. Every rule is one
test in `checks/tests/rules_packaging.rs` (`cargo test --manifest-path checks/Cargo.toml --test
rules_packaging`); the tests read files, they never run makepkg. Standards: wolos/docs/STANDARDS.md
sections 4 (PKGBUILDs), 5 and 7. `tools/lint.sh` (run by build.yml) stays the gate for what needs a
shell: it evaluates `source`/`sha256sums` and checks the local file sums, the single
`_wolos_commit`, the api-headers version, paths that leave the directory, and shellcheck. These
rules add the metadata, ordering, pin and test-hygiene checks around it.

Baselines are growth-only: `BASELINE` in the test file lists today's known violations. A new
violation fails; so does a baseline entry that no longer violates (delete it). A rule is never
relaxed by adding an entry.

| Id | Rule | Group |
|----|------|-------|
| RP-001 | every PKGBUILD has a non-empty pkgdesc | metadata |
| RP-002 | every PKGBUILD has url | metadata |
| RP-003 | every PKGBUILD has a non-empty license array | metadata |
| RP-004 | every PKGBUILD has a non-empty arch array | metadata |
| RP-005 | pkgver is set and has no hyphen; pkgrel is an integer | metadata |
| RP-006 | every PKGBUILD has pkgname or pkgbase | metadata |
| RP-007 | the first line of every PKGBUILD is the SPDX header | metadata |
| RP-008 | a PKGBUILD with a source array has sha256sums | sources |
| RP-009 | sha256sums has one entry per source | sources |
| RP-010 | SKIP only for git sources with #commit=, signatures with validpgpkeys, and the documented base.manifest | sources |
| RP-011 | sha256sums entries are 64 lowercase hex, SKIP, or a variable expansion | sources |
| RP-012 | every git+ source carries #commit= | sources |
| RP-013 | no #branch=, #tag= or #revision= source (a moving or retaggable ref) | sources |
| RP-014 | no "or true" fallback (a double-bar true) in package() | package() |
| RP-015 | every `install -D` in package() gives an explicit mode (-Dm644, -m755, ...) | package() |
| RP-016 | no chmod 777, 666, a+w, o+w or ugo+w anywhere | package() |
| RP-017 | package(), build() and prepare() call no curl, wget, sudo or pacman -S | package() |
| RP-018 | every PKGBUILD defines package() or package_* | package() |
| RP-019 | install, cp, mkdir, ln, mv in package() never target an absolute system path without $pkgdir | package() |
| RP-020 | licenses are SPDX identifiers, not the legacy GPL, GPL2, BSD, custom: names | metadata |
| RP-021 | arch is x86_64 or any | metadata |
| RP-022 | url and sources use https | sources |
| RP-023 | backup entries are relative (no leading slash) | package() |
| RP-024 | every recipe directory is a line of `packages` | packages file |
| RP-025 | every `packages` DIR has a PKGBUILD | packages file |
| RP-026 | package names in `packages` are unique | packages file |
| RP-027 | every `packages` line has NAME DIR DEPS and at least one PATH | packages file |
| RP-028 | every LOCAL-DEP is an earlier line (build order) | packages file |
| RP-029 | the dependency graph resolves into waves without a cycle (lib/plan.py of wolos-build) | packages file |
| RP-030 | a package's PATHS include its own directory | packages file |
| RP-031 | every PATH exists in this repo | packages file |
| RP-032 | packages sharing a directory (linux-wolos, linux-wolos-v3) have the same deps and paths | packages file |
| RP-033 | a package named like its directory is declared by that PKGBUILD (pkgname or pkgbase) | packages file |
| RP-034 | each LOCAL-DEP is named in the recipe, its prebuild.sh or build.conf | packages file |
| RP-035 | no pkgname is declared by two recipes | packages file |
| RP-036 | a provides of an Arch package name also has the matching conflicts (libraries and virtual capabilities excepted) | provides |
| RP-037 | a package never conflicts with itself | provides |
| RP-038 | every package named in lib/plan.py SIZES is a line of `packages` (when wolos-build is next to this repo) | packages file |
| RP-039 | _wolos_commit is a full 40-hex sha and one value in all recipes | pins |
| RP-040 | _brand_commit and _shell_commit are full 40-hex shas | pins |
| RP-041 | every wolos-sourced pin variable in a recipe is one tools/bump-wolos.sh rewrites | pins |
| RP-042 | every variable tools/bump-wolos.sh rewrites exists in some recipe | pins |
| RP-043 | pkgdesc has no final period and at most 160 characters | metadata |
| RP-044 | the directories tools/bump-wolos.sh names exist | pins |
| RP-045 | a recipe pinned by _wolos_commit has a plain `pkgrel=N` line (the form bump-wolos.sh edits) | pins |
| RP-046 | an .install script holds only pre_/post_ install, upgrade, remove functions at top level | install |
| RP-047 | every .install file is named by `install=` in its recipe | install |
| RP-048 | every `install=` names an existing file | install |
| RP-049 | .install scripts never `exit 1` or `set -e` (a failing hook must not abort the transaction) | install |
| RP-050 | .install scripts end with a newline | install |
| RP-051 | every checks test names at least one directory of this repo (siblings_copy and this file excepted) | checks |
| RP-052 | no checks test reads `<dir>/PKGBUILD` for a directory that does not exist | checks |
| RP-053 | every checks test starts with the SPDX line and has a `//!` description | checks |
| RP-054 | every checks test file contains a #[test] | checks |
| RP-055 | every file wolos-system ships under `etc/` (the wolos tree's `desktop/overlay/etc`) is in backup=, or a baseline row says why an upgrade may put it back | origins (0.13) |
| RP-056 | no `|| return 0` or `|| exit 0` in a recipe: a step whose input is absent fails loudly, it does not ship a package without it | origins (0.13) |
| RP-057 | every `_wolos_commit`, `_brand_commit` and `_shell_commit` is a commit a remote branch or tag of the wolos checkout (`$WOLOS_SRC`, else `../wolos`) still contains: the builder fetches a pin by sha and a host serves only what a ref reaches (not checked without a checkout) | origins (0.13) |

## Known violations (baseline)

- RP-024: `wolos-browser-config` has a PKGBUILD but no line in `packages`; it is pinned by
  bump-wolos.sh but never built by `wolos-build pkg`. Add a line, or retire the recipe.
- RP-034: `wolos-base` lists hyprland-wolos as a local dep but reaches it through wolos-system.
- RP-051: boot_setup_sealed_units.rs, hypr_conf.rs, image_repo_sums.rs, modules.rs and modules_files.rs test files of
  the sibling repos or the image (or a recipe directory not yet here), not a directory of this repo.
- RP-051: recipe_firewall_files.rs and recipe_network_vpn.rs (NET-3) test files that read wolos-system through the
  overlaid tree and name no package directory of their own.

- RP-055: the `etc/` files of the overlay that no person or Settings page writes (NetworkManager dispatcher, Chromium
  and Flatpak policy, fonts, the PAM stacks, the polkit-like job files, the GTK, Qt and portal defaults): an upgrade
  putting the shipped file back is the intent. A new file under `desktop/overlay/etc` that a person or a Settings
  page writes (the firewall rule files and the sudoers drop-in were the two that 0f05dc2 and 870c239 fixed) goes into
  backup=; one that does not needs a row here with the reason.
- RP-056: wolos-system's `_brand_walls_package` skips an absent staging directory (`[ -d "$e" ] || return 0`);
  `_brand_walls_build` already fails when the generator is missing (3c1044e), so the directory exists in a good build.

## Not automated

- That `pkgver` follows the source (bump-wolos.sh sets it from CHANGELOG.md; the lint cannot see wolos).
- That the sha256 of a remote tarball is right (lint.sh checks local files only; `updpkgsums` fixes).
- namcap results (needs an Arch host); shellcheck of each PKGBUILD is in `tools/lint.sh`.
- That `depends` are complete (a clean-chroot build in build.yml proves it).
- `pkgrel` resets on a new `pkgver` (bump-wolos.sh does it; a hand edit is a review item).
