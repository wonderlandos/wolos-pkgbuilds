#!/usr/bin/env bash
# SPDX-License-Identifier: GPL-2.0-or-later
# bump-kernel.sh TAG: point linux-wolos and linux-wolos-api-headers at the fork's release tag
# wolkernel-<wolkernel>-<linux> (github.com/wonderlandos/linux): linux-wolos/VERSION, the headers'
# _wk/_linux, and the sha256 of GitHub's tag tarball in both recipes (their first sha256sums entry).
# scx-wolos reads the same tarball (sched_ext headers): its _wk/_linux and checksum move too, and its
# pkgrel goes +1 (its pkgver is the WolOS release, not the kernel's).
# pkgrel goes to 1 on a new version, else +1 (the same version re-tagged). PKGREL=N sets it instead
# (both recipes), e.g. to supersede an already published release of the same version.
set -euo pipefail
cd "$(dirname "$0")/.."
tag=${1:?TAG (wolkernel-<wolkernel>-<linux>)}
[[ $tag =~ ^wolkernel-([0-9][0-9.]*)-([0-9][0-9.]*)$ ]] ||
  { echo "bump-kernel: $tag is not wolkernel-<wolkernel>-<linux>" >&2; exit 2; }
wk=${BASH_REMATCH[1]} lx=${BASH_REMATCH[2]}
url=https://github.com/wonderlandos/linux/archive/refs/tags/$tag.tar.gz
tmp=$(mktemp); trap 'rm -f "$tmp"' EXIT
curl -fsSL --retry 3 -o "$tmp" "$url" || { echo "bump-kernel: cannot download $url" >&2; exit 1; }
# A WolKernel tag carries the fork's config fragment (linux-wolos/prepare.sh needs it).
tar -tzf "$tmp" "linux-$tag/kernel/configs/wolos.config" >/dev/null 2>&1 ||
  { echo "bump-kernel: $tag has no kernel/configs/wolos.config" >&2; exit 1; }
sum=$(sha256sum "$tmp" | cut -d' ' -f1)

old=$(sed -n 's/^wolkernel=//p' linux-wolos/VERSION)+$(sed -n 's/^linux=//p' linux-wolos/VERSION)
sed -i -e "s/^wolkernel=.*/wolkernel=$wk/" -e "s/^linux=.*/linux=$lx/" linux-wolos/VERSION
sed -i -e "s/^_wk=.*/_wk=$wk/" -e "s/^_linux=.*/_linux=$lx/" linux-wolos-api-headers/PKGBUILD
for p in linux-wolos/PKGBUILD linux-wolos-api-headers/PKGBUILD; do
  rel=$(sed -n 's/^pkgrel=//p' "$p")
  if [ -n "${PKGREL:-}" ]; then rel=$PKGREL
  elif [ "$old" != "$wk+$lx" ]; then rel=1
  else rel=$((rel + 1)); fi
  sed -i -E -e "s/^pkgrel=.*/pkgrel=$rel/" -e "s/^sha256sums=\('[^']*'/sha256sums=('$sum'/" "$p"
  grep -q "^sha256sums=('$sum'" "$p" || { echo "bump-kernel: $p: no sha256sums=('...' line to fill" >&2; exit 1; }
  echo "$p: $wk+$lx-$rel ($tag, sha256 $sum)"
done
p=scx-wolos/PKGBUILD rel=$(sed -n 's/^pkgrel=//p' scx-wolos/PKGBUILD)
sed -i -E -e "s/^_wk=.*/_wk=$wk/" -e "s/^_linux=.*/_linux=$lx/" -e "s/^pkgrel=.*/pkgrel=$((rel + 1))/" \
  -e "s/^sha256sums=\('[^']*'/sha256sums=('$sum'/" "$p"
grep -q "^sha256sums=('$sum'" "$p" || { echo "bump-kernel: $p: no sha256sums=('...' line to fill" >&2; exit 1; }
echo "$p: pkgrel $((rel + 1)) ($tag, sha256 $sum)"
