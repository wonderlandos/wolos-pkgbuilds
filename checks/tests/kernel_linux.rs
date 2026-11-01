// SPDX-License-Identifier: GPL-2.0-or-later
//! linux-wolos (ADR 0016): the WolOS kernel fork at a pinned commit + Arch's config at the snapshot +
//! `linux-wolos/wolos.config` + extra patches from `$WOLOS_TREE/kernel/linux/patches` when present.
//! The kernel builds with makepkg; these tests pin what the recipe and its prepare step must do.
#![allow(dead_code, unused_imports)]
#[path = "support/siblings.rs"]
mod siblings;

use std::{fs, path::PathBuf, process::Command};
fn repo(p: &str) -> PathBuf {
    siblings::root()
        .join(p)
}

fn read(p: &str) -> String {
    fs::read_to_string(repo(p)).unwrap_or_else(|e| panic!("{p}: {e}"))
}

/// Sealed slots need fs-verity (btrfs supports it when the kernel does) and an overlay for /etc;
/// encrypted disks need dm-crypt and the TPM drivers.
#[test]
fn fragment_has_verity_overlay_and_disk_encryption() {
    let c = read("linux-wolos/wolos.config");
    for on in [
        "CONFIG_FS_VERITY=y",
        "CONFIG_OVERLAY_FS=y",
        "CONFIG_DM_CRYPT=y",
        "CONFIG_TCG_TPM=y",
        "CONFIG_TCG_CRB=y",
    ] {
        assert!(c.lines().any(|l| l == on), "wolos.config lacks {on}");
    }
    assert!(c.lines().any(|l| l == "CONFIG_BTRFS_FS=y"));
    assert!(!c.contains("# CONFIG_FS_VERITY is not set"));
}

#[test]
fn fragment_sets_the_desktop_options_each_with_a_reason() {
    let c = read("linux-wolos/wolos.config");
    for want in [
        "CONFIG_PREEMPT=y",
        "CONFIG_HZ_1000=y",
        "CONFIG_HZ=1000",
        "CONFIG_SCHED_CLASS_EXT=y",
        "CONFIG_BPF_SYSCALL=y",
        "CONFIG_DEBUG_INFO_BTF=y",
        "CONFIG_BTRFS_FS=y",
        "CONFIG_LOCALVERSION=\"-wolos\"",
    ] {
        assert!(c.lines().any(|l| l == want), "wolos.config lacks {want}");
    }
    // Every option has a reason: a comment line above it (or above the group it belongs to).
    let mut prev = "";
    for l in c.lines() {
        let option = l.starts_with("CONFIG_") || l.ends_with(" is not set");
        if option {
            assert!(
                prev.starts_with('#') || prev.starts_with("CONFIG_"),
                "{l}: no reason above it"
            );
        } else {
            assert!(l.is_empty() || l.starts_with('#'), "not an option: {l}");
        }
        prev = l;
    }
}

#[test]
fn freebsd_kernel_parts_are_gone() {
    for p in [
        "kernel/conf/WolKernel",
        "kernel/wolos_qos",
        "kernel/drm",
        "kernel/fetch-src.sh",
        "kernel/check.sh",
        "kernel/build.sh",
    ] {
        assert!(!repo(p).exists(), "{p} still present");
    }
}

/// linux-wolos/VERSION is the one place the product version lives: "WolKernel 1.0 (7.3)" =
/// WolKernel release + the Linux base in parentheses. PKGBUILDs, uname -v and the README follow it.
#[test]
fn version_has_one_source_and_shows_the_linux_base_in_parentheses() {
    let v = read("linux-wolos/VERSION");
    let get = |k: &str| {
        v.lines()
            .find_map(|l| l.strip_prefix(&format!("{k}=")))
            .unwrap_or_else(|| panic!("VERSION lacks {k}="))
            .to_string()
    };
    let (wk, linux) = (get("wolkernel"), get("linux"));
    assert_eq!((wk.as_str(), linux.as_str()), ("1.0", "7.3"));
    let shown = format!("WolKernel {wk} ({linux})");
    assert!(
        read("kernel/README.md").contains(&shown),
        "README lacks {shown}"
    );
    for pkg in [
        "linux-wolos/PKGBUILD",
        "linux-wolos-api-headers/PKGBUILD",
    ] {
        let p = read(pkg);
        assert!(p.contains("VERSION"), "{pkg} does not read linux-wolos/VERSION");
        assert!(!p.contains("pkgver=7."), "{pkg} hard-codes the version");
        let dir = repo(pkg).parent().unwrap().to_path_buf();
        let out = Command::new("bash")
            .arg("-c")
            .arg("startdir=$PWD; source ./PKGBUILD >/dev/null; echo \"$pkgver|${KBUILD_BUILD_VERSION:-}\"")
            .current_dir(&dir)
            .output()
            .unwrap();
        let out = String::from_utf8_lossy(&out.stdout);
        let (pv, uv) = out.trim().split_once('|').unwrap();
        assert_eq!(pv, format!("{wk}+{linux}"), "{pkg} pkgver");
        if pkg.ends_with("linux-wolos/PKGBUILD") {
            // kbuild pastes KBUILD_BUILD_VERSION unquoted into a shell line (the bzImage "is
            // ready" message): parentheses or quotes there stop the build.
            let plain = format!("WolKernel {wk} Linux {linux}");
            assert!(uv.contains(&plain), "uname -v lacks {plain}: {uv:?}");
            assert!(
                !uv.contains(['(', ')', '\'', '"']),
                "uname -v breaks kbuild: {uv:?}"
            );
        }
    }
}

fn option_set(c: &str, opt: &str) -> bool {
    c.lines().any(|l| l == opt)
}

/// Desktop tuning and debloat: what the fragment must set, and what it must never switch off
/// (Steam/Proton, Flatpak, Docker, KVM, DKMS, laptops).
#[test]
fn fragment_tunes_for_desktops_and_keeps_compatibility() {
    let c = read("linux-wolos/wolos.config");
    for want in [
        "CONFIG_NTSYNC=m",
        "CONFIG_TRANSPARENT_HUGEPAGE_MADVISE=y",
        "CONFIG_LRU_GEN_ENABLED=y",
        "CONFIG_TCP_CONG_BBR=y",
        "CONFIG_DEFAULT_BBR=y",
        "CONFIG_LEGACY_VSYSCALL_NONE=y",
        "# CONFIG_PCMCIA is not set",
        "# CONFIG_BLK_DEV_FD is not set",
        "# CONFIG_PARPORT is not set",
        "# CONFIG_FB_VESA is not set",
        "CONFIG_DRM_SIMPLEDRM=y",
        "CONFIG_ZRAM=m",
        "CONFIG_NR_CPUS=512",
        // Fork code: lazy audit syscall work, int $0x80 only for 32-bit tasks, retbleed stuffing
        // on Intel without eIBRS.
        "CONFIG_WOLOS_AUDIT_LAZY=y",
        "CONFIG_WOLOS_NO_INT80_64=y",
        "CONFIG_WOLOS_RETBLEED_STUFF=y",
        // Extra series: CPU floor right after an input event.
        "CONFIG_WOLOS_INPUT_BOOST=y",
    ] {
        assert!(option_set(&c, want), "wolos.config lacks {want}");
    }
    for keep in [
        "IA32_EMULATION",
        "MODIFY_LDT_SYSCALL",
        "BINFMT_MISC",
        "OVERLAY_FS",
        "USER_NS",
        "KVM",
        "KVM_INTEL",
        "KVM_AMD",
        "MODULES",
        "MODVERSIONS",
        "VETH",
        "BRIDGE",
        "FUTEX",
        "IO_URING",
        "SECCOMP",
        "DRM_I915",
        "DRM_AMDGPU",
        "DRM_XE",
        "IWLWIFI",
        "BT",
        "THINKPAD_ACPI",
        "ACPI_BATTERY",
    ] {
        let off = format!("# CONFIG_{keep} is not set");
        assert!(!option_set(&c, &off), "wolos.config turns off {keep}");
    }
    assert!(!option_set(&c, "CONFIG_IA32_EMULATION_DEFAULT_DISABLED=y"));
    // Every removed userspace interface is listed for `wol doctor`.
    let abi = read("linux-wolos/abi-removed.txt");
    for name in [
        "X86_X32_ABI",
        "LEGACY_VSYSCALL",
        "WOLOS_NO_INT80_64",
        "LOOP_CHANGE_FD",
        "IPV6_ADDRFORM",
    ] {
        assert!(
            abi.lines()
                .any(|l| l.split_whitespace().nth(1) == Some(name)),
            "abi-removed.txt lacks {name}"
        );
    }
}

/// Lean core and Apple hardware: options the fragment turns off or on.
#[test]
fn fragment_trims_unused_code_and_supports_apple_devices() {
    let c = read("linux-wolos/wolos.config");
    for off in [
        "ACPI_DEBUG",
        "SECURITY_SMACK",
        "SECURITY_TOMOYO",
        "SECURITY_IPE",
        "SECURITY_SAFESETID",
        "SECURITY_LOADPIN",
        "FUNCTION_ERROR_INJECTION",
        "MMIOTRACE",
        "HWLAT_TRACER",
        "OSNOISE_TRACER",
        "TIMERLAT_TRACER",
        "SCHED_STACK_END_CHECK",
    ] {
        let line = format!("# CONFIG_{off} is not set");
        assert!(c.lines().any(|l| l == line), "wolos.config keeps {off}");
    }
    for on in [
        "CONFIG_USB_IPHETH=m",
        "CONFIG_HID_APPLE=m",
        "CONFIG_HID_MAGICMOUSE=m",
        "CONFIG_HID_BATTERY_STRENGTH=y",
        "CONFIG_HIDRAW=y",
        "CONFIG_FUSE_FS=m",
        "CONFIG_APFS_FS=m",
        "CONFIG_TUN=m",
    ] {
        assert!(c.lines().any(|l| l == on), "wolos.config lacks {on}");
    }
}

/// Build options for developers and CI: CPU level (generic default, x86-64-v3 flavour), Clang ThinLTO,
/// a fast build of only the loaded modules, ccache and parallel make.
#[test]
fn recipe_has_build_flavours_and_fast_developer_builds() {
    let p = read("linux-wolos/PKGBUILD");
    for want in [
        "_march=${_march:-generic}",
        "x86-64-v3",
        "_use_llvm_lto=${_use_llvm_lto:-none}",
        "LTO_CLANG_THIN",
        "LLVM=1",
        "_localmodcfg",
        "localmodconfig",
        "modprobed.db",
        "ccache",
        "MAKEFLAGS",
        "nproc",
    ] {
        assert!(p.contains(want), "PKGBUILD lacks {want}");
    }
}

/// Scheduler patches on, four 8250 ports probed at boot instead of 32.
#[test]
fn config_turns_on_the_scheduler_patches() {
    let c = read("linux-wolos/wolos.config");
    for on in [
        "CONFIG_SCHED_PROXY_EXEC=y",
        "CONFIG_WOLOS_SHIELD=y",
        "CONFIG_SERIAL_8250_RUNTIME_UARTS=4",
        "CONFIG_ZSWAP_DEFAULT_ON=y",
    ] {
        assert!(c.lines().any(|l| l == on), "wolos.config lacks {on}");
    }
}

/// A branch profile feeds the next build: AutoFDO and Propeller stay off without one.
#[test]
fn recipe_takes_autofdo_and_propeller_profiles() {
    let p = read("linux-wolos/PKGBUILD");
    for want in [
        "_autofdo_profile=${_autofdo_profile:-}",
        "_propeller_prefix=${_propeller_prefix:-}",
        "-e AUTOFDO_CLANG",
        "-e PROPELLER_CLANG",
        "CLANG_AUTOFDO_PROFILE=",
        "CLANG_PROPELLER_PROFILE_PREFIX=",
    ] {
        assert!(p.contains(want), "PKGBUILD lacks {want}");
    }
}
