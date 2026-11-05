#!/bin/sh
# SPDX-License-Identifier: GPL-2.0-or-later
# nsswitch-mdns.sh [FILE]: .local names through avahi (nss-mdns) before systemd-resolved. avahi owns
# mDNS on WolOS (resolved.conf.d/wolos.conf: MulticastDNS=no); Arch's hosts line has `resolve
# [!UNAVAIL=return]`, which ends the lookup before any later module, so mdns4_minimal goes in front
# of it. IPv4 only (mdns4_minimal, as Debian and Ubuntu ship it): with mdns_minimal every lookup of
# a name that has no IPv6 address (most printers and speakers) waited 5 s for avahi's AAAA answer,
# getaddrinfo included (dry run 0.13.0, net-firewall-state: 5.0 s against 0.01 s). Idempotent; a
# line with mdns_minimal (WolOS 0.12 and 0.13 before this) becomes mdns4_minimal, any other mdns
# line is left alone. /etc/nsswitch.conf belongs to the filesystem package, so this edits it in
# place (pacman keeps it on upgrade: backup file).
f=${1:-/etc/nsswitch.conf}
[ -f "$f" ] || exit 0
if grep -Eq '^hosts:.*[[:space:]]mdns_minimal[[:space:]]' "$f"; then
  sed -i -E '/^hosts:/ s/([[:space:]])mdns_minimal([[:space:]])/\1mdns4_minimal\2/' "$f"
  exit 0
fi
grep -Eq '^hosts:.*[[:space:]]mdns' "$f" && exit 0
grep -Eq '^hosts:' "$f" || exit 0
if grep -Eq '^hosts:.*[[:space:]]resolve' "$f"; then
  sed -i -E '/^hosts:/ s/[[:space:]]resolve/ mdns4_minimal [NOTFOUND=return] resolve/' "$f"
else
  sed -i -E '/^hosts:/ s/[[:space:]]dns([[:space:]]|$)/ mdns4_minimal [NOTFOUND=return] dns\1/' "$f"
fi
