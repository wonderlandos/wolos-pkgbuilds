#!/usr/bin/env bash
# SPDX-License-Identifier: GPL-2.0-or-later
# Recipe rules (build.yml runs it before any build; no makepkg needed):
# - every package line names a directory with a PKGBUILD, and every LOCAL-DEP is an earlier line;
# - a recipe reads only its own directory and its sources (no $startdir/.., no sibling checkout);
# - the recipes built from wolos pin one and the same _wolos_commit;
# - linux-wolos-api-headers has linux-wolos's VERSION;
# - helium-browser keeps the paths wolos's Helium theme writes (crates/core/src/theme/helium.rs).
set -euo pipefail
cd "$(dirname "$0")/.."
fail=0
bad() { echo "lint: $*" >&2; fail=1; }
seen=" "
while read -r name dir deps _; do
  case $name in '' | '#'*) continue ;; esac
  [ -f "$dir/PKGBUILD" ] || bad "$name: no $dir/PKGBUILD"
  for d in ${deps//,/ }; do
    [ "$d" = - ] || case $seen in *" $d "*) ;; *) bad "$name: dep $d is not built before it" ;; esac
  done
  seen="$seen$name "
done < packages
for p in */PKGBUILD; do
  if grep -nE '\$startdir/\.\.|\.\./\.\./|WOLOS_TREE|WOLOS_OVERLAY' "$p"; then bad "$p reaches outside its directory"; fi
done
commits=$(grep -h '^_wolos_commit=' */PKGBUILD | sort -u)
[ "$(printf '%s\n' "$commits" | grep -c .)" -le 1 ] || bad "different _wolos_commit pins: $commits"
wk=$(sed -n 's/^wolkernel=//p' linux-wolos/VERSION); lx=$(sed -n 's/^linux=//p' linux-wolos/VERSION)
grep -qx "_wk=$wk" linux-wolos-api-headers/PKGBUILD && grep -qx "_linux=$lx" linux-wolos-api-headers/PKGBUILD ||
  bad "linux-wolos-api-headers: _wk/_linux differ from linux-wolos/VERSION ($wk, $lx)"
for want in /opt/helium/initial_preferences /usr/bin/helium; do
  grep -qF "$want" helium-browser/PKGBUILD || bad "helium-browser lacks $want"
done
[ $fail = 0 ] && echo "lint: ok"
exit $fail
