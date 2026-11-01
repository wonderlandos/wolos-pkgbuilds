#!/bin/sh
# SPDX-License-Identifier: GPL-2.0-or-later
# linux-wolos prepare step (PKGBUILD prepare()): SRC BASECONFIG.
# BASECONFIG (Arch config.x86_64) + the fork's kernel/configs/wolos.config (the only WolOS fragment),
# then make olddefconfig. The PKGBUILD runs olddefconfig a second time after its build options; the
# published kernels came from these two passes, and a single pass differs: the transitional
# GOOGLE_FRAMEBUFFER_COREBOOT keeps DRM_COREBOOTDRM=m after one pass, the second drops it.
# Patches are commits on the fork (github.com/wonderlandos/linux, tag wolkernel-*): nothing is
# patched here.
set -eu
SRC=${1:?SRC}; BASE=${2:?BASECONFIG}
BASE=$(cd "$(dirname "$BASE")" && pwd)/$(basename "$BASE")
cd "$SRC"
[ -f kernel/configs/wolos.config ] || { echo "prepare.sh: $SRC has no kernel/configs/wolos.config (not a WolKernel tag?)" >&2; exit 1; }
cp "$BASE" .config
sh scripts/kconfig/merge_config.sh -m .config kernel/configs/wolos.config
make olddefconfig
