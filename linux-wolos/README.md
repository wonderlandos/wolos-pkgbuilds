# linux-wolos (WolKernel)

**WolKernel 1.0 (7.3.1)**: WolKernel release 1.0 on Linux 7.3.1 (stable). The product version comes first, the Linux base is always shown in parentheses.
`VERSION` is the single source (`wolkernel=`, `linux=`): the linux-wolos PKGBUILD reads it (pkgver `1.0+7.3.1`, epoch 1), `tools/lint.sh` holds linux-wolos-api-headers to it, and `uname -v` shows it.

WolKernel (package `linux-wolos`, ADR 0016/0017/0029) = the tag `wolkernel-<wolkernel>-<linux>` of github.com/wonderlandos/linux (upstream stable + one commit per WolKernel patch on branch `wolos-7.3.1`, see `README.wolos.md` there), over Arch's x86_64 config. Patches live only on the fork.
Config: Arch's `config.x86_64` + the fork's `kernel/configs/wolos.config` at the tag (`prepare.sh`), the only WolOS fragment; `make olddefconfig` runs twice (prepare.sh, then the PKGBUILD), as for the published kernels. Source: GitHub's tag tarball, its sha256 pinned (`tools/bump-kernel.sh TAG`).

| path | what |
|---|---|
| `VERSION` | WolKernel and Linux base versions (bump `wolkernel=` per release, `linux=` when the fork moves to a new upstream stable release or line, e.g. 7.3 to 7.3.1; the tag follows) |
| `abi-removed.txt` | userspace interfaces removed versus Arch's kernel, one per line (`wol doctor` reads it) |
| `../linux-wolos-api-headers/` | UAPI headers from the same tag; replaces `linux-api-headers` |
| `kmi/` | module ABI freeze per release line (`kmi.sh extract|check`) |

What users notice versus Arch's `linux`:
- Anything that runs on stock Arch runs here: 32-bit apps and Steam, Wine/Proton, Flatpak/AppImage/Snap, Docker/Podman, KVM/VirtualBox, browser sandboxes, DKMS modules (NVIDIA, ZFS, v4l2loopback).
- The app in front goes first. wold applies a QoS class per thread (nice and uclamp through `sched_setattr`, 2 ms EEVDF slice for the boosted classes) and writes the frontmost pid to `/sys/kernel/wolos/foreground_pid` on every switch; the kernel uses that pid for the i915 waitboost (only the app in front raises GPU clocks) and to protect it in OOM victim choice. A key, click or pointer move also lifts the CPU floor for 100 ms (input boost).
- Removed (`abi-removed.txt`): the x32 ABI, legacy PTYs and the vsyscall page (boot with `vsyscall=xonly` for pre-2011 static binaries), plus pre-2010 PC hardware (PCMCIA/CardBus, floppy, parallel port, gameport, 1990s PCI NICs, pre-ACPI MP tables), the VESA/EFI framebuffers (simpledrm covers boot), cluster sockets (RDS, TIPC, RxRPC/AFS) and raw-flash filesystems. Datacenter-only drivers are not built.
- Module ABI is stable within a release line (CONFIG_MODVERSIONS): vendor modules built once keep loading until the next line.
- `uname -r` ends in `-wolos`; `uname -v` reads `#1 WolKernel 1.0 Linux 7.3.1 SMP PREEMPT_DYNAMIC <date>`.

What is in 1.0 (each has an off switch; table with sources in `README.wolos.md` on the fork):

| area | change |
|---|---|
| scheduler | stock EEVDF with the QoS classes above. The `scx_wolos` sched_ext scheduler (per-CPU queues, QoS slices, frontmost boost) is built and tested in `kernel/scx_wolos/` of the wolos repo but is **not shipped in v0.12**: no package installs it and no service starts it. To try it by hand: `make -C kernel/scx_wolos KSRC=<tree of the wolkernel tag> VMLINUX_BTF=/sys/kernel/btf/vmlinux`, then `sudo kernel/scx_wolos/build/scx_wolos` (the running kernel must be this WolKernel; details in `kernel/README.md`) |
| frontmost app | i915 waitboost only for the app in front; OOM victim choice favours it; input boost (CPU floor for 100 ms after input) |
| locks | futex spins briefly before sleeping (`futex.spin_ns`); `selftests/wolos/futex_pingpong` measures it |
| games | NTSYNC (module) and `futex_waitv` (always built); split-lock mitigation off (sysctl `kernel.split_lock_mitigate=0`); i915 frame-deadline boost |
| memory | zswap in front of zram (zstd, zram writeback built), MGLRU on, `page-cluster` 0 (sysctl), THP on request |
| disk | I/O scheduler by udev rule: none on NVMe, mq-deadline on SATA SSDs and SD cards, BFQ on spinning disks |
| network | BBR with fq by default, MPTCP built in, WireGuard as a module |
| graphics | i915 deadline boost (above); AMD color pipeline and parallel CPU bring-up are the base kernel's own |
| build | `-O3`, x86-64-v3/v4 flavours, Clang ThinLTO, zstd modules, ccache (options below) |

Measured (7.3.0, `make olddefconfig`, before the 1.0 trim): Arch 6332 modules / 3247 built-in options; WolKernel 6154 / 3164 (-178 modules, -83 built-in).
WolKernel 1.0 (7.3.1): not re-measured.


## Build

| path | what |
|---|---|
| `PKGBUILD` | `wolos-build pkg linux-wolos` (clean chroot): `linux-wolos` + `linux-wolos-headers` (DKMS, e.g. `nvidia-open-dkms`) |
| `prepare.sh` | fork fragment then ours, `make olddefconfig` (no patches here) |
| `config.x86_64` | Arch `linux` config at the snapshot (`7.2.11.arch1-1`) |

Build options (environment for `makepkg`, see the PKGBUILD header):

| variable | effect |
|---|---|
| `_march=generic\|v3\|v4\|native` | CPU level; `v3`/`v4` build `linux-wolos-v3`/`-v4` (generic stays the default package for every x86-64 PC) |
| `_cc_o3=y` | `-O3` instead of `-O2` |
| `_use_llvm_lto=thin\|full` | Clang/LLVM build with ThinLTO or full LTO |
| `_autofdo_profile=FILE` | AutoFDO: `perf record -a -b` on a desktop or game session, `llvm-profgen --kernel --binary=vmlinux --perfdata=perf.data -o FILE`. Needs LBR or AMD BRS and `_use_llvm_lto` |
| `_propeller_prefix=PATH` | Propeller on top of AutoFDO: `create_llvm_prof --format=propeller` writes `PATH_cc_profile.txt` and `PATH_ld_profile.txt` |
| `_localmodcfg=y` | developer build of only the modules in `~/.config/modprobed.db` (or the loaded ones) |

ccache is used when installed; `MAKEFLAGS` defaults to `-j$(nproc)`.

Published only in `[wolos-testing]` and `[wolos]` (build.yml, publish.yml, promote.yml); the ISO never ships Arch `linux`. Other kernels installed with pacman get their own boot entry; `linux-wolos` stays the default.
