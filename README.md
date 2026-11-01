# wolos-pkgbuilds

Every package of the WolOS `[wolos]` repo, one directory per package, like Arch's and CachyOS's
packaging repos. How packages are built and released: [BUILD.md](BUILD.md).

| Path | What |
|---|---|
| `packages` | build order: name, directory, local deps, watched paths |
| `linux-wolos/` | WolKernel: the fork `wonderlandos/linux` at tag `wolkernel-<VERSION>`, `config.x86_64` (the WolOS fragment is the fork's `kernel/configs/wolos.config`), `VERSION`, `kmi/` ([README](linux-wolos/README.md)) |
| `linux-wolos-api-headers/` | UAPI headers from the same tag |
| `hyprland-wolos/` | Hyprland 0.56.2 + the WolOS patches (`0*.patch`) |
| `wolos-system/`, `wopkg/`, `wolos-base/`, `wolos-browser-config/` | built from `github.com/wonderlandos/wolos` at `_wolos_commit` (bumped by `tools/bump-wolos.sh`) |
| `flatpak-wolos/`, `helium-browser/`, `wolos-shim/`, `wolos-keyring/`, `wolos-mirrorlist/` | the rest of the repo |
| `pacman.conf`, `snapshot` | the build chroot: Arch at the WolOS snapshot only |
| `tools/lint.sh` | recipe rules (own directory only, one wolos pin, versions agree) |
| `checks/` | contract tests of these recipes (Rust, std only) |

Workflows: `build.yml` (changed directories -> ECS -> unsigned packages in S3), `bump.yml` (wolos
pushed: pin it, as a pull request), `ci.yml` (lint, shellcheck, checks). `publish.yml` (sign,
repo-add, `[wolos-testing]`, CDN) and `promote.yml` (testing -> `[wolos]`) are reusable workflows:
they run from the publish button in wolos, where the signing keys live (wolos `BUILD.md`).

License: GPL-2.0-or-later (`LICENSE`); each package states the license of what it ships, the kernel
is GPL-2.0-only.
