#!/bin/sh
# SPDX-License-Identifier: GPL-2.0-or-later
# nsswitch-mdns.sh [FILE]: .local names through avahi (nss-mdns) before systemd-resolved. avahi owns
# mDNS on WolOS (resolved.conf.d/wolos.conf: MulticastDNS=no); Arch's hosts line has `resolve
# [!UNAVAIL=return]`, which ends the lookup before any later module, so mdns_minimal goes in front
# of it. Idempotent; leaves a hosts line that already names mdns alone. /etc/nsswitch.conf belongs to
# the filesystem package, so this edits it in place (pacman keeps it on upgrade: backup file).
f=${1:-/etc/nsswitch.conf}
[ -f "$f" ] || exit 0
grep -Eq '^hosts:.*[[:space:]]mdns' "$f" && exit 0
grep -Eq '^hosts:' "$f" || exit 0
if grep -Eq '^hosts:.*[[:space:]]resolve' "$f"; then
  sed -i -E '/^hosts:/ s/[[:space:]]resolve/ mdns_minimal [NOTFOUND=return] resolve/' "$f"
else
  sed -i -E '/^hosts:/ s/[[:space:]]dns([[:space:]]|$)/ mdns_minimal [NOTFOUND=return] dns\1/' "$f"
fi
