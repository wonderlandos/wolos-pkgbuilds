#!/bin/sh
# SPDX-License-Identifier: GPL-2.0-or-later
# /usr/bin/atlas: the launcher with WolOS defaults (plan-atlas section 1, patches-0 L-01..L-07).
#  L-01 Wayland when the session is one (Hyprland), else the engine decides.
#  L-02 hardware video decode and encode only when the machine has a GPU render node.
#  L-03 the login keyring (libsecret): the session's desktop name is not one the engine maps to it.
#  L-04 the desktop file name (the Dock running dot and the menu bar name follow the window class).
#  L-05 flags, whitespace separated, from /etc/atlas-flags.conf then ~/.config/atlas-flags.conf
#       come after the defaults and before the command line.
#  L-06 a profile from the previous browser package is moved once, atomically, to ~/.config/atlas
#       (cache too); default-browser entries follow; the keyring entry is never touched.
#  L-07 no default here turns off the sandbox, site isolation or web security, or opens a
#       debugging port (tools/lint.sh greps this file and every flags file of the repo).
export ATLAS_DESKTOP=atlas.desktop
export CHROME_DESKTOP=atlas.desktop

cfg=${XDG_CONFIG_HOME:-$HOME/.config}
cache=${XDG_CACHE_HOME:-$HOME/.cache}
data=${XDG_DATA_HOME:-$HOME/.local/share}
old_id=net.imput.helium
old_flags=helium-flags.conf
old_desktop=helium.desktop

# Rename DIR to a name that does not exist yet. mv -T is rename(2) on one file system: whole or
# not at all, and a second launch racing this one loses without damage.
move_once() {
  [ -d "$1" ] && [ ! -e "$2" ] || return 0
  mv -T -- "$1" "$2" 2>/dev/null || :
}

# The previous browser process holds the profile (SingletonLock -> host-pid): leave it alone.
profile_in_use() {
  lock=$(readlink "$1/SingletonLock" 2>/dev/null) || return 1
  pid=${lock##*-}
  case $pid in '' | *[!0-9]*) return 1 ;; esac
  kill -0 "$pid" 2>/dev/null
}

migrate() {
  [ -d "$cfg/$old_id" ] && [ ! -e "$cfg/atlas" ] || return 0
  profile_in_use "$cfg/$old_id" && return 0
  move_once "$cfg/$old_id" "$cfg/atlas"
  [ -d "$cfg/atlas" ] || return 0
  move_once "$cache/$old_id" "$cache/atlas"
  # Default browser: the user's choices, desktop id by desktop id, through a temporary file.
  for list in "$cfg/mimeapps.list" "$data/applications/mimeapps.list"; do
    [ -f "$list" ] || continue
    grep -q "$old_desktop" "$list" || continue
    if sed "s/$old_desktop/atlas.desktop/g" "$list" > "$list.atlas-new"; then
      mv -f -- "$list.atlas-new" "$list"
    else
      rm -f -- "$list.atlas-new"
    fi
  done
}
migrate
# The user's own flags file keeps its content under the new name (rename, never overwrite).
if [ -f "$cfg/$old_flags" ] && [ ! -e "$cfg/atlas-flags.conf" ]; then
  mv -T -- "$cfg/$old_flags" "$cfg/atlas-flags.conf" 2>/dev/null || :
fi

n=$#
system_flags=/etc/atlas-flags.conf
# A system file written for the previous package is read while the new one does not exist.
if [ ! -e "$system_flags" ] && [ -r "/etc/$old_flags" ]; then
  echo "atlas: $system_flags does not exist, reading the flags file of the previous package; move it there" >&2
  system_flags=/etc/$old_flags
fi
for conf in "$system_flags" "$cfg/atlas-flags.conf"; do
  [ -r "$conf" ] || continue
  while IFS= read -r line; do
    case $line in '' | '#'*) continue ;; esac
    # shellcheck disable=SC2086
    set -- "$@" $line
  done < "$conf"
done
# The command line goes last: move its $n words behind the flags just added.
i=0
while [ "$i" -lt "$n" ]; do
  set -- "$@" "$1"
  shift
  i=$((i + 1))
done

defaults="--password-store=gnome-libsecret"
if [ -n "${WAYLAND_DISPLAY:-}" ]; then
  defaults="$defaults --ozone-platform=wayland"
else
  defaults="$defaults --ozone-platform-hint=auto"
fi
# VA-API decode and encode on the GPU, only with a render node: a VM without one stays in software.
for node in /dev/dri/renderD*; do
  if [ -e "$node" ]; then
    defaults="$defaults --enable-features=AcceleratedVideoDecodeLinuxGL,AcceleratedVideoEncoder"
    break
  fi
done
# shellcheck disable=SC2086
set -- $defaults "$@"

exec /opt/atlas/atlas-wrapper "$@"
