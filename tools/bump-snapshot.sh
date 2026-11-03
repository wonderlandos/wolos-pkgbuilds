#!/usr/bin/env bash
# SPDX-License-Identifier: GPL-2.0-or-later
# bump-snapshot.sh YYYY-MM-DD [WOLOS_ISO]: move WolOS to one Arch Linux Archive date in every file
# that writes it (audit CA-6; checks/tests/snapshot.rs keeps them in step): snapshot and
# pacman.conf here, profile/snapshot, profile/pacman.conf and profile/airootfs/etc/pacman.conf in
# wolos-iso (WOLOS_ISO, default ../wolos-iso), and the wolos-mirrorlist tag (pkgver = the date,
# pkgrel 1, sha256sums). Run it in both repos' working trees, then commit each.
set -euo pipefail
cd "$(dirname "$0")/.."
die() { echo "bump-snapshot: $*" >&2; exit 1; }
d=${1:?usage: bump-snapshot.sh YYYY-MM-DD [WOLOS_ISO]}
[[ $d =~ ^[0-9]{4}-[0-9]{2}-[0-9]{2}$ ]] && [ "$(date -u -d "$d" +%F 2>/dev/null)" = "$d" ] ||
  die "$d is not a date (YYYY-MM-DD)"
iso=${2:-${WOLOS_ISO:-../wolos-iso}}
[ -f "$iso/profile/snapshot" ] || die "no wolos-iso at $iso (profile/snapshot)"
confs=(pacman.conf "$iso/profile/pacman.conf" "$iso/profile/airootfs/etc/pacman.conf")
for f in "${confs[@]}"; do
  grep -qE '/repos/[0-9]{4}/[0-9]{2}/[0-9]{2}/' "$f" || die "$f names no archive date"
done
old=$(tr -d '[:space:]' < snapshot)
for f in snapshot "$iso/profile/snapshot"; do echo "$d" > "$f"; done
for f in "${confs[@]}"; do sed -i -E "s#/repos/[0-9]{4}/[0-9]{2}/[0-9]{2}/#/repos/${d//-//}/#g" "$f"; done
m=wolos-mirrorlist
sed -i -E "s/[0-9]{4}-[0-9]{2}-[0-9]{2}/$d/g" "$m/wolos-mirrorlist"
cur=$(sed -n 's/^pkgver=//p' "$m/PKGBUILD")
if [ "$cur" != "${d//-/.}" ]; then
  sed -i -e "s/^pkgver=.*/pkgver=${d//-/.}/" -e 's/^pkgrel=.*/pkgrel=1/' "$m/PKGBUILD"
fi
# sha256sums: one line per source, in source order (the PKGBUILD's two-line layout).
a=$(sha256sum "$m/wolos-mirrorlist" | cut -c1-64); b=$(sha256sum "$m/wolos-cachy-mirrorlist" | cut -c1-64)
sed -i -E -e "s/^sha256sums=\('[0-9a-f]{64}'/sha256sums=('$a'/" -e "s/^( +)'[0-9a-f]{64}'\)/\1'$b')/" "$m/PKGBUILD"
grep -q "'$a'" "$m/PKGBUILD" && grep -q "'$b'" "$m/PKGBUILD" || die "$m/PKGBUILD: sha256sums not updated"
echo "snapshot: $old -> $d (${#confs[@]} pacman.conf, 2 snapshot files, $m $(sed -n 's/^pkgver=//p' "$m/PKGBUILD")-$(sed -n 's/^pkgrel=//p' "$m/PKGBUILD"))"
