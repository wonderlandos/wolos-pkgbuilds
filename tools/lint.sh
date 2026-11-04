#!/usr/bin/env bash
# SPDX-License-Identifier: GPL-2.0-or-later
# Recipe rules (build.yml runs it before any build; no makepkg needed):
# - every package line names a directory with a PKGBUILD, and every LOCAL-DEP is an earlier line;
# - a recipe reads only its own directory and its sources (no $startdir/.., no sibling checkout);
# - the recipes built from wolos pin one and the same _wolos_commit;
# - linux-wolos-api-headers and scx-wolos have linux-wolos's VERSION;
# - helium-browser keeps the paths wolos's Helium theme writes (crates/core/src/theme/helium.rs);
# - every PKGBUILD is shellcheck-clean as bash at error level (wolos/docs/STANDARDS.md §4; namcap
#   needs an Arch host, shellcheck runs anywhere; skipped with a note when shellcheck is missing);
# - sources are pinned (audit A4): a git source carries #commit=<40-hex sha> and SKIP; any other
#   source (tarball, URL, local file) has a real sha256sums entry, and a local file's entry matches
#   the file; a detached .asc/.sig may be SKIP when the recipe has validpgpkeys (gpg checks it).
#   The other exceptions, each with its reason, are in skip_ok below.
# - every Server line (also a commented one) in a shipped pacman.conf or mirrorlist carries the
#   date of ./snapshot (archive YYYY/MM/DD or release tag YYYY-MM-DD); the CachyOS /cachy/ redirect
#   rolls and has none (ADR 0030);
# - same version needs a pkgrel bump: a recipe whose directory differs from the last tag
#   (git describe --tags) still has that tag's epoch:pkgver-pkgrel -> fail (skipped without a tag).
# tools/lint.sh --only servers|pkgrel runs one of the last two (checks/tests/recipe_lint_rules.rs).
set -euo pipefail
cd "$(dirname "$0")/.."
fail=0
bad() { echo "lint: $*" >&2; fail=1; }
lint_servers() {
  local snap f n line url
  snap=$(tr -d '[:space:]' < snapshot)
  [[ $snap =~ ^[0-9]{4}-[0-9]{2}-[0-9]{2}$ ]] || { bad "snapshot: '$snap' is not YYYY-MM-DD"; return; }
  while IFS= read -r -d '' f; do
    while IFS=: read -r n line; do
      url=${line#*=}; url=${url//[[:space:]]/}
      case $url in
        */cachy/*) ;;
        *"${snap//-//}"* | *"$snap"*) ;;
        *) bad "${f#./}:$n: Server is not at the snapshot $snap: $url" ;;
      esac
    done < <(grep -nE '^[[:space:]]*#?[[:space:]]*Server[[:space:]]*=' "$f" || true)
  done < <(find . \( -path ./.git -o -path ./checks -o -path './*/src' -o -path './*/pkg' \) -prune -o \
    -type f \( -name pacman.conf -o -name '*mirrorlist*' \) ! -name PKGBUILD -print0)
}
# pkg_version DIR: epoch:pkgver-pkgrel of DIR/PKGBUILD, evaluated as makepkg does.
pkg_version() {
  (cd "$1" && startdir=$PWD bash -c 'set +eu; source ./PKGBUILD >/dev/null 2>&1; printf "%s:%s-%s" "${epoch:-0}" "$pkgver" "$pkgrel"')
}
lint_pkgrel() {
  local tag old p d
  if ! tag=$(git describe --tags --abbrev=0 2>/dev/null); then
    echo "lint: no git tag, pkgrel-bump rule skipped" >&2; return
  fi
  # The tag's tree (a symlink tar cannot make, e.g. on Windows, only loses that one file).
  old=$(mktemp -d); git archive "$tag" | tar -x -C "$old" 2>/dev/null || true
  for p in */PKGBUILD; do
    d=${p%/PKGBUILD}
    [ -f "$old/$p" ] || continue   # new since the tag
    git diff --quiet "$tag" -- "$d" && continue
    [ "$(pkg_version "$old/$d")" != "$(pkg_version "$d")" ] ||
      bad "$d: changed since $tag but still $(pkg_version "$d"): bump pkgrel (git diff $tag -- $d)"
  done
  rm -rf "$old"
}
case ${1:-} in
  --only)
    case ${2:-} in servers) lint_servers ;; pkgrel) lint_pkgrel ;; *) echo "lint: --only servers|pkgrel" >&2; exit 2 ;; esac
    [ $fail = 0 ] && echo "lint: ok"; exit $fail ;;
  '') ;;
  *) echo "usage: tools/lint.sh [--only servers|pkgrel]" >&2; exit 2 ;;
esac
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
for p in linux-wolos-api-headers scx-wolos; do
  grep -qx "_wk=$wk" $p/PKGBUILD && grep -qx "_linux=$lx" $p/PKGBUILD ||
    bad "$p: _wk/_linux differ from linux-wolos/VERSION ($wk, $lx)"
done
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
lint_servers
lint_pkgrel
if command -v shellcheck >/dev/null; then
  shellcheck -s bash -S error -- */PKGBUILD || bad "a PKGBUILD is not shellcheck-clean (see above)"
else
  echo "lint: shellcheck not installed, PKGBUILDs not checked" >&2
fi
[ $fail = 0 ] && echo "lint: ok"
exit $fail
