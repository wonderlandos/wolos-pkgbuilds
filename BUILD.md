# Building WolOS packages

**Concept.** A package is a PKGBUILD in its own directory plus its sources (a tarball, a git tag,
or a pinned commit of wolos). `wolos-build pkg NAME` builds it with devtools `makechrootpkg` in a
clean chroot of the Arch snapshot (`pacman.conf`); `[wolos]` packages it needs go in with `-I`.

**Why.** Arch's rule: one recipe per package, one clean way to build it, the same on every machine.
A recipe never reads a sibling checkout, so what was built is exactly what its sources name, and
builders never hold a signing key (ADR 0029 in wolos).

**How.**

```sh
git clone https://github.com/wonderlandos/wolos-build ../wolos-build    # private: the tool
../wolos-build/wolos-build pkg wolos-mirrorlist                          # one package, here
../wolos-build/wolos-build pkg --changed origin/main~1                   # what a commit changed
../wolos-build/wolos-build pkg --ecs linux-wolos                         # the same on ECS
tools/lint.sh && cargo test --manifest-path checks/Cargo.toml            # before a pull request
```

Per-package hooks, both optional: `prebuild.sh DEPS_DIR WORK_DIR` runs on the build host before
the chroot build (wolos-base writes its `base.manifest`), `build.conf` sets `makepkg_args`
(wolos-base: `--nodeps`).

Release: merge to `main` -> `build.yml` (ECS, unsigned) -> `publish.yml` (signed, `[wolos-testing]`)
-> the release is tagged (below) and the wolos-iso tag builds the ISO from testing -> `promote.yml` -> `[wolos]`.

Who tags: a release is one version tagged together on all four repos (`wolos`, `wolos-pkgbuilds`,
`wolos-iso`, `wolos-build`) from wolos-build, after the ISO built from `[wolos-testing]` passed its VM
tests (ADR 0029). The same `v<version>` tag in every repo names the code, the recipes, the image and
the tool that built it. No repo tags on its own. (wolos-build has no `tag` command yet; until it
does, the release manager runs `git tag -s v<version>` in each of the four repos at the tested commits.)
