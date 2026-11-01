#!/bin/sh
# SPDX-License-Identifier: GPL-2.0-or-later
# linux-wolos prepare step (PKGBUILD prepare()): SRC BASECONFIG.
# BASECONFIG (Arch config.x86_64) + the fork's kernel/configs/wolos.config, the only WolOS fragment.
# No olddefconfig here: the PKGBUILD runs it once, after its build options. A second run drops
# options a first one kept only as transitional (DRM_COREBOOTDRM=m via GOOGLE_FRAMEBUFFER_COREBOOT).
# Patches are commits on the fork (github.com/wonderlandos/linux, tag wolkernel-*): nothing is
# patched here.
set -eu
SRC=${1:?SRC}; BASE=${2:?BASECONFIG}
BASE=$(cd "$(dirname "$BASE")" && pwd)/$(basename "$BASE")
cd "$SRC"
[ -f kernel/configs/wolos.config ] || { echo "prepare.sh: $SRC has no kernel/configs/wolos.config (not a WolKernel tag?)" >&2; exit 1; }
cp "$BASE" .config
sh scripts/kconfig/merge_config.sh -m .config kernel/configs/wolos.config
