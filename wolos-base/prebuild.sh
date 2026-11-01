#!/usr/bin/env bash
# SPDX-License-Identifier: GPL-2.0-or-later
# wolos-base prebuild (wolos-build pkg runs it on the build host before the chroot build):
#   prebuild.sh DEPS_DIR WORK_DIR  ->  ./base.manifest
# The sha256 of every file of the base packages at the snapshot (downloaded with an own database,
# never the host's: ../pacman.conf) plus the WolOS packages built before it (DEPS_DIR: linux-wolos*,
# wolos-system). ADR 0017: `wol base verify` checks the installed base against it.
set -euo pipefail
HERE=$(cd "$(dirname "$0")" && pwd); DEPS=${1:?DEPS_DIR}; WORK=${2:?WORK_DIR}
mkdir -p "$WORK/pacdb" "$WORK/pkgcache"
base_parts="base uutils-coreutils sudo-rs libarchive zsh systemd"
# shellcheck disable=SC2086 # one package name per word
fakeroot -- pacman -Syw --noconfirm --config "$HERE/../pacman.conf" --dbpath "$WORK/pacdb" \
  --cachedir "$WORK/pkgcache" $base_parts
"$HERE/base-manifest.sh" "$WORK"/pkgcache/*.pkg.tar.zst "$DEPS"/linux-wolos-*.pkg.tar.zst \
  "$DEPS"/wolos-system-*.pkg.tar.zst > "$HERE/base.manifest"
