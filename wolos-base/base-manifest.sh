#!/usr/bin/env bash
# SPDX-License-Identifier: GPL-2.0-or-later
# base-manifest.sh PKG...: "sha256  /path" for every regular file and symlink in the given pacman
# packages, sorted by path (ADR 0017: `wol base verify` checks the installed base against it).
# Package metadata (.PKGINFO, .MTREE, .BUILDINFO, .INSTALL) is not a system file.
# Contents only: xattrs (file capabilities) need root to restore and are not hashed.
# A symlink's sum is the sha256 of its target and a newline (`readlink LINK | sha256sum`).
# One sha256sum pass over all paths (xargs), no process per file: thousands of files in the base.
set -euo pipefail
T=$(mktemp -d); trap 'rm -rf "$T"' EXIT
mkdir "$T/root" "$T/links"
for p in "$@"; do
  [ -f "$p" ] || continue
  bsdtar -xpf "$p" -C "$T/root" --no-xattrs --exclude '.PKGINFO' --exclude '.MTREE' --exclude '.BUILDINFO' --exclude '.INSTALL' 2>/dev/null ||
    { echo "base-manifest.sh: cannot read $p" >&2; exit 1; }
done
# Owner read on everything: some files are not readable even by their owner (dbus-daemon-launch-helper
# is 4110), and the build host extracts as a plain user; their sha256 came out empty.
chmod -R u+rX "$T/root"
cd "$T/root"
# Symlink targets without a fork each: find prints them (%l), the shell writes "target\n" to a file
# that the same sha256sum pass hashes in the link's place.
declare -A target=()
while IFS= read -r -d '' f && IFS= read -r -d '' l; do target[$f]=$l; done \
  < <(find . -type l -printf '%P\0%l\0')
paths=(); sums=(); i=0
while IFS= read -r -d '' f; do
  paths+=("$f")
  if [ -n "${target[$f]+x}" ]; then
    printf '%s\n' "${target[$f]}" > "$T/links/$i"; sums+=("$T/links/$i")
  else
    sums+=("./$f")
  fi
  i=$((i + 1))
done < <(find . \( -type f -o -type l \) -printf '%P\0' | LC_ALL=C sort -z)
[ "$i" -gt 0 ] || exit 0
# sha256sum keeps the argument order; a name with \ or a newline gets a leading \ before its sum.
printf '%s\0' "${sums[@]}" | xargs -0 sha256sum -- > "$T/sums" ||
  { echo "base-manifest.sh: cannot hash a file (see above)" >&2; exit 1; }
i=0
while IFS= read -r line; do
  case $line in '\'*) h=${line:1:64} ;; *) h=${line:0:64} ;; esac
  [[ $h =~ ^[0-9a-f]{64}$ ]] && [ "$i" -lt ${#paths[@]} ] || { echo "base-manifest.sh: cannot hash /${paths[$i]:-?}" >&2; exit 1; }
  printf '%s  /%s\n' "$h" "${paths[$i]}"
  i=$((i + 1))
done < "$T/sums"
[ "$i" = ${#paths[@]} ] || { echo "base-manifest.sh: $i sums for ${#paths[@]} files" >&2; exit 1; }
