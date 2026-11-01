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
-> wolos-iso tags a release from testing -> `promote.yml` -> `[wolos]`.
