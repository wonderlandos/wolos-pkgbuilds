#!/usr/bin/env bash
# SPDX-License-Identifier: GPL-2.0-or-later
# Recipe rules (build.yml runs it before any build; no makepkg needed):
# - every package line names a directory with a PKGBUILD, and every LOCAL-DEP is an earlier line;
# - a recipe reads only its own directory and its sources (no $startdir/.., no sibling checkout);
# - the recipes built from wolos pin one and the same _wolos_commit;
# - linux-wolos-api-headers has linux-wolos's VERSION;
# - helium-browser keeps the paths wolos's Helium theme writes (crates/core/src/theme/helium.rs);
# - every PKGBUILD is shellcheck-clean as bash at error level (wolos/docs/STANDARDS.md §4; namcap
#   needs an Arch host, shellcheck runs anywhere; skipped with a note when shellcheck is missing);
# - sources are pinned (audit A4): a git source carries #commit=<40-hex sha> and SKIP; any other
#   source (tarball, URL, local file) has a real sha256sums entry, and a local file's entry matches
#   the file; a detached .asc/.sig may be SKIP when the recipe has validpgpkeys (gpg checks it).
#   The other exceptions, each with its reason, are in skip_ok below.
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
# Sources a recipe may leave at SKIP without being a pinned git source: "dir/file  reason".
skip_ok='
wolos-base/base.manifest  written by prebuild.sh from the built packages just before makepkg; no fixed content
'
for p in */PKGBUILD; do
  d=${p%/PKGBUILD}
  # One line per source: "<sum>	<source>" (the recipe evaluated in a subshell, as makepkg does).
  rows=$(cd "$d" && startdir=$PWD bash -c 'set +eu; source ./PKGBUILD >/dev/null 2>&1
    for i in "${!source[@]}"; do printf "%s	%s
" "${sha256sums[$i]:-MISSING}" "${source[$i]}"; done') ||
    { bad "$p: could not evaluate source/sha256sums"; continue; }
  while IFS=$'	' read -r sum src; do
    [ -n "$src" ] || continue
    url=${src#*::}; file=${src%%::*}; [ "$file" != "$src" ] || file=${url##*/}
    case $url in
      git+*)
        case $url in *'#commit='[0-9a-f]*) [ "$(printf '%s' "${url##*#commit=}" | grep -cE '^[0-9a-f]{40}$')" = 1 ] ||
            bad "$p: $file: #commit= is not a full 40-hex sha" ;;
          *) bad "$p: $file: git source without #commit=<sha>" ;; esac ;;
      *://*.asc | *://*.sig)
        # A detached signature is checked by makepkg against validpgpkeys, not hashed.
        [ "$sum" != MISSING ] || bad "$p: $file: no sha256sums entry"
        [ "$sum" != SKIP ] || grep -q '^validpgpkeys=' "$p" || bad "$p: $file: signature with SKIP but no validpgpkeys" ;;
      *://*)
        [ "$sum" != SKIP ] && [ "$sum" != MISSING ] || bad "$p: $file: remote source with sha256sums SKIP" ;;
      *)
        if [ "$sum" = SKIP ] || [ "$sum" = MISSING ]; then
          case $skip_ok in *"
$d/$file "*) ;; *) bad "$p: $file: local source with sha256sums SKIP" ;; esac
        elif [ -f "$d/$file" ] && [ "$(sha256sum < "$d/$file" | cut -d' ' -f1)" != "$sum" ]; then
          bad "$p: $file: sha256sums entry does not match the file (updpkgsums)"
        fi ;;
    esac
  done <<< "$rows"
done
if command -v shellcheck >/dev/null; then
  shellcheck -s bash -S error -- */PKGBUILD || bad "a PKGBUILD is not shellcheck-clean (see above)"
else
  echo "lint: shellcheck not installed, PKGBUILDs not checked" >&2
fi
[ $fail = 0 ] && echo "lint: ok"
exit $fail
