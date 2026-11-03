#!/usr/bin/env bash
# SPDX-License-Identifier: GPL-2.0-or-later
# wolos-base prebuild (wolos-build pkg runs it on the build host before the chroot build):
#   prebuild.sh DEPS_DIR WORK_DIR  ->  ./base.manifest
# The sha256 of every file of the base packages at the snapshot (resolved with an own database,
# never the host's: ../pacman.conf) plus the WolOS packages built before it (DEPS_DIR: linux-wolos*,
# wolos-system). ADR 0017: `wol base verify` checks the installed base against it.
# Warm runs download nothing: the packages come from the host's pacman CacheDir (shared with the
# chroot and `wolos-build cache`; new downloads land there when it is writable, else in STATE) and
# the sync database is kept in STATE, keyed by ../pacman.conf (its Server lines carry the snapshot).
# STATE: $WOLOS_PREBUILD_STATE, default WORK_DIR/../.state-wolos-base (wolos-build empties WORK_DIR
# on every run, not its parent).
set -euo pipefail
HERE=$(cd "$(dirname "$0")" && pwd); DEPS=${1:?DEPS_DIR}; WORK=${2:?WORK_DIR}
conf=$HERE/../pacman.conf
mkdir -p "$WORK"
STATE=${WOLOS_PREBUILD_STATE:-$(cd "$WORK/.." && pwd)/.state-wolos-base}
key=$(sha256sum < "$conf" | cut -c1-16)
mkdir -p "$STATE/pacdb-$key" "$STATE/pkgcache"
# Databases of another snapshot are never read again.
find "$STATE" -mindepth 1 -maxdepth 1 -name 'pacdb-*' ! -name "pacdb-$key" -exec rm -rf {} +
db=$STATE/pacdb-$key
cachedirs=()
for c in $(pacman-conf CacheDir 2>/dev/null); do [ -d "$c" ] && cachedirs+=(--cachedir "${c%/}"); done
cachedirs+=(--cachedir "$STATE/pkgcache")   # pacman downloads into the first writable one
base_parts="base uutils-coreutils sudo-rs libarchive zsh systemd"
pm() { fakeroot -- pacman --noconfirm --config "$conf" --dbpath "$db" "${cachedirs[@]}" "$@"; }
# The snapshot's databases never change: refresh only when they are missing.
rm -f "$db/db.lck"   # wolos-build runs one prebuild at a time; a killed run leaves its lock
y=(); ls "$db"/sync/*.db >/dev/null 2>&1 || y=(-y)
# shellcheck disable=SC2086 # one package name per word
pm -S "${y[@]}" -w $base_parts
# The exact files of this resolution (an empty local db: every dependency), found in the cache dirs,
# in file-name order (the order the old fresh-cache glob extracted them in).
pkgs=()
# shellcheck disable=SC2086
for f in $(pm -Sp --print-format '%f' $base_parts | LC_ALL=C sort); do
  p=
  for c in $(pacman-conf CacheDir 2>/dev/null) "$STATE/pkgcache"; do
    [ -f "${c%/}/$f" ] && { p=${c%/}/$f; break; }
  done
  [ -n "$p" ] || { echo "prebuild.sh: $f was not downloaded" >&2; exit 1; }
  pkgs+=("$p")
done
"$HERE/base-manifest.sh" "${pkgs[@]}" "$DEPS"/linux-wolos-*.pkg.tar.zst \
  "$DEPS"/wolos-system-*.pkg.tar.zst > "$HERE/base.manifest"
