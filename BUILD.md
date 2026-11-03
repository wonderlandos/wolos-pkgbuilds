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

## Release

One path, and `wolos-build` (private repo, `lib/publish.sh`) is its only writer: of the GitHub
releases, the CDN bucket `s3://wolos-repo/<tag>/` and its CloudFront invalidation. The workflows
only check out, assume the OIDC role, pass the keys (explicit secrets, `environment: release`) and
call it. The signing keys live only in wolos, so every step is a button of its `publish.yml`
(it calls this repo's `publish.yml` and `promote.yml`):

```sh
# 1. merge to wolos-pkgbuilds main -> build.yml builds the changed packages on ECS (unsigned, S3)
gh workflow run publish.yml -R wonderlandos/wolos -f what=packages -f run=<build run id>   # -> [wolos-testing]
# 2. tag the release on all four repos at their tested commits (dry run first; --do tags and pushes)
wolos-build release v0.12.0 && wolos-build release --do v0.12.0
# 3. on the forge: build and VM-test the ISO into s3://$BUCKET/runs/release-v0.12.0/iso/ (below)
gh workflow run publish.yml -R wonderlandos/wolos -f what=iso -f run=v0.12.0             # release.json, latest release
gh workflow run publish.yml -R wonderlandos/wolos -f what=promote                         # [wolos-testing] -> [wolos]
```

- `what=packages` (wolos-pkgbuilds `publish.yml`) and `what=promote` (`promote.yml`) run
  `wolos-build publish testing|stable`: a package file name is published once (the same bytes
  again: nothing uploaded; other bytes: the run stops, bump pkgrel), then the database is signed
  and the release gets the new packages, their `*.inputs.json`, then `inputs.json` (what
  `wolos-build pkg` compares to skip unchanged packages), `snapshot`, `packages.json` (name,
  version, desc, size, native: for the site), the database and `revision`. The CDN bucket gets the
  same files in the same order (packages immutable, the database last), then the invalidation. A
  missing bucket stops the run before any write (`--no-cdn` skips the CDN).
- The ISO is built and VM-tested where it is built, on the forge, from `[wolos-testing]`:
  `wolos-build iso --out DIR && WOLOS_VM=V sh ../wolos-iso/tests/test-install.sh --release DIR`,
  then `cp V/test/checks.txt DIR/ && aws s3 sync DIR/ s3://$BUCKET/runs/release-vX.Y.Z/iso/`.
  `what=iso` (wolos-iso `release.yml`) takes it from there (no `BOOT PASS` in its `checks.txt`, or a
  `FAIL`: no release), Secure-Boot-signs and boots it when the keys are set, then
  `wolos-build release --do --iso` writes and signs `release.json` and publishes the latest
  release. `what=iso-s3` publishes any tested ISO of a run the same way, as a pre-release.
- `wolos-build release vX.Y.Z` puts the same `vX.Y.Z` tag on `wolos`, `wolos-pkgbuilds`,
  `wolos-iso` and `wolos-build` (signed tags, clean work trees; a tag already at HEAD is kept) and
  writes the release manifest (each repo's commit, the snapshot). Without `--do` it only prints the
  git commands. No repo tags on its own (ADR 0029).
- Hand mode: without `WOLOS_RELEASES_TOKEN` every release write is recorded
  (`WOLOS_GH_RECORD`) into `s3://$BUCKET/runs/<run>/gh-release.sh` with the signed files, and the
  run ends with a `::notice::` holding the command for main's gh login; after the upload,
  `what=cdn -f run=<run>` (`wolos-build publish --cdn-only <run>`) fills the CDN.
