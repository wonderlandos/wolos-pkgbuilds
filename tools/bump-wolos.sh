#!/usr/bin/env bash
# SPDX-License-Identifier: GPL-2.0-or-later
# bump-wolos.sh SHA [WOLOS_CHECKOUT]: pin every recipe built from github.com/wonderlandos/wolos to SHA
# (_wolos_commit=), the way a distro bumps an upstream release. pkgver follows the source at SHA
# (CHANGELOG.md's newest entry; wopkg: wopkg/shard.yml; libwol and libwolkit: the ABI version in
# wol.h and wolkit.h; wolkit and cryswol: their shard.yml); pkgrel goes to 1 on a new pkgver, else +1.
# WOLOS_CHECKOUT: a wolos clone that has SHA (default: a fresh shallow fetch). bump.yml opens the PR.
# A recipe built from GitHub's archive of the commit (wolos-webapps: it builds --nodeps, no git in
# the chroot) pins its sha256 in _wolos_archive_sha256=: the archive of SHA is downloaded once and
# hashed (WOLOS_ARCHIVE_URL: the archive base, default GitHub's; tests use file://). A git+ source
# stays SKIP (its #commit= pins it).
set -euo pipefail
cd "$(dirname "$0")/.."
sha=${1:?SHA}; src=${2:-}
if [ -z "$src" ]; then
  src=$(mktemp -d); trap 'rm -rf "$src"' EXIT
  git -C "$src" init -q
  git -C "$src" fetch -q --depth 1 https://github.com/wonderlandos/wolos.git "$sha"
  git -C "$src" checkout -q FETCH_HEAD
fi
show() { git -C "$src" show "$sha:$1"; }
# The OS version is the NEWEST CHANGELOG section, and it must be a numbered release: skipping a
# `## Unreleased` heading to the first number below it shipped the whole 0.13 head as 0.12.1
# (QA-A9-IMAGE-01; checks/tests/rules_packaging.rs RP-058 holds the same line).
ver_os=$(show CHANGELOG.md | awk '/^## / && !v {v=$2} END {print v}')
[[ $ver_os =~ ^[0-9]+\.[0-9]+\.[0-9]+$ ]] ||
  { echo "bump-wolos: the newest CHANGELOG.md heading at $sha is not a numbered release (got '$ver_os'): write '## N.N.N (date)' first" >&2; exit 1; }
ver_wopkg=$(show wopkg/shard.yml | sed -n 's/^version: //p')
# libwol follows its ABI (WOL_VERSION_MAJOR.MINOR.PATCH), so apps can depend on libwol>=1.N.
ver_libwol=$(show crates/wol-abi/include/wol.h | awk '/^#define WOL_VERSION_(MAJOR|MINOR|PATCH) / {v = v (v == "" ? "" : ".") $3} END {print v}')
# libwolkit likewise (WOLKIT_VERSION_*: apps depend on libwolkit>=1.N); the wolkit and cryswol shards by
# their shard.yml.
ver_libwolkit=$(show crates/wolkit/include/wolkit.h | awk '/^#define WOLKIT_VERSION_(MAJOR|MINOR|PATCH) / {v = v (v == "" ? "" : ".") $3} END {print v}')
ver_wolkit=$(show wolos-crystal/shards/wolkit/shard.yml | sed -n 's/^version: //p')
ver_cryswol=$(show wolos-crystal/shards/cryswol/shard.yml | sed -n 's/^version: //p')
archive_sum=''
archive_sha256() {
  [ -n "$archive_sum" ] || archive_sum=$(curl -fsSL --retry 3 "${WOLOS_ARCHIVE_URL:-https://github.com/wonderlandos/wolos/archive}/$sha.tar.gz" | sha256sum | cut -d" " -f1)
  [[ $archive_sum =~ ^[0-9a-f]{64}$ ]] || { echo "bump-wolos: no archive of $sha to hash" >&2; exit 1; }
  echo "$archive_sum"
}
for p in $(grep -l '^_wolos_commit=' */PKGBUILD); do
  old=$(sed -n 's/^_wolos_commit=//p' "$p")
  [ "$old" != "$sha" ] || continue
  case $p in
    wopkg/*) ver=$ver_wopkg ;;
    libwol/*) ver=$ver_libwol ;;
    libwolkit/*) ver=$ver_libwolkit ;;
    wolkit/*) ver=$ver_wolkit ;;
    cryswol/*) ver=$ver_cryswol ;;
    wolos-browser-config/*) ver= ;;
    *) ver=$ver_os ;;
  esac
  cur=$(sed -n 's/^pkgver=\([^ ]*\).*/\1/p' "$p")
  rel=$(sed -n 's/^pkgrel=//p' "$p")
  if [ -n "$ver" ] && [ "$ver" != "$cur" ]; then
    sed -i -e "s/^pkgver=$cur/pkgver=$ver/" -e 's/^pkgrel=.*/pkgrel=1/' "$p"
  else
    sed -i "s/^pkgrel=.*/pkgrel=$((rel + 1))/" "$p"
  fi
  sed -i "s/^_wolos_commit=.*/_wolos_commit=$sha/" "$p"
  # Submodule sources pin the commits wolos records at SHA (audit A4: no unpinned git source).
  gitlink() { git -C "$src" ls-tree "$sha" "$1" | awk '$2 == "commit" {print $3}'; }
  if grep -q "^_brand_commit=" "$p"; then sed -i "s/^_brand_commit=.*/_brand_commit=$(gitlink design)/" "$p"; fi
  if grep -q "^_shell_commit=" "$p"; then sed -i "s/^_shell_commit=.*/_shell_commit=$(gitlink crates/shell)/" "$p"; fi
  # wolos-base takes os-release as one checksummed file of that commit.
  if grep -q "^_os_release_sha256=" "$p"; then
    sed -i "s/^_os_release_sha256=.*/_os_release_sha256=$(show files/usr/share/wolos/os-release | sha256sum | cut -d" " -f1)/" "$p"
  fi
  if grep -q "^_wolos_archive_sha256=" "$p"; then
    archive_sha256 >/dev/null   # here, not in $(...) below: the sum is kept for the next recipe
    sed -i "s/^_wolos_archive_sha256=.*/_wolos_archive_sha256=$archive_sum/" "$p"
  fi
  echo "$p: $old -> $sha ($(sed -n 's/^pkgver=\([^ ]*\).*/\1/p' "$p")-$(sed -n 's/^pkgrel=//p' "$p"))"
done
