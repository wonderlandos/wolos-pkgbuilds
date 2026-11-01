#!/bin/sh
# SPDX-License-Identifier: GPL-2.0-or-later
# linux-wolos prepare step (PKGBUILD prepare()): SRC BASECONFIG FRAGMENT.
# BASECONFIG (Arch config.x86_64) + the fork's kernel/configs/wolos.config when present + FRAGMENT
# (./wolos.config, merged last so it wins), then make olddefconfig.
# Patches are commits on the fork (github.com/wonderlandos/linux, tag wolkernel-*): nothing is
# patched here (the old kernel/linux/patches series and $WOLOS_OVERLAY hook are gone).
set -eu
SRC=${1:?SRC}; BASE=${2:?BASECONFIG}; FRAG=${3:?FRAGMENT}
BASE=$(cd "$(dirname "$BASE")" && pwd)/$(basename "$BASE")
FRAG=$(cd "$(dirname "$FRAG")" && pwd)/$(basename "$FRAG")
cd "$SRC"
cp "$BASE" .config
if [ -f kernel/configs/wolos.config ]; then
  sh scripts/kconfig/merge_config.sh -m .config kernel/configs/wolos.config
fi
sh scripts/kconfig/merge_config.sh -m .config "$FRAG"
make olddefconfig
