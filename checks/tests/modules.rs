// SPDX-License-Identifier: GPL-2.0-or-later
//! What this file checks (wolos-pkgbuilds): the optional module meta packages (ADR 0032,
//! wolos docs/modules/modules.md). One `wolos-module-<id>/` per registry record in wolos
//! (`files/usr/share/wolos/modules/<id>.json`, shipped by wolos-system), depends-only: `depends=` is
//! wolos-system plus the record's `packages`, nothing is built, nothing outside Arch or this repo's
//! [wolos] packages is a dependency, every one is a line of `packages`. The developer module carries
//! the Crystal userland runtime (crystal, libwol, the cryswol shard). The ignored test checks every
//! other dependency exists at the snapshot (network: `cargo test -- --ignored`).
#![allow(dead_code, unused_imports)]
#[path = "support/siblings.rs"]
mod siblings;

use std::{fs, path::PathBuf, process::Command};

fn repo(p: &str) -> PathBuf {
    siblings::root().join(p)
}

fn read(p: &str) -> String {
    fs::read_to_string(repo(p)).unwrap_or_else(|e| panic!("{p}: {e}"))
}

/// The strings of one JSON array field (`"packages": ["a", "b"]`); the records are flat.
fn json_list(text: &str, key: &str) -> Vec<String> {
    let k = format!("\"{key}\":");
    let Some(at) = text.find(&k) else {
        return Vec::new();
    };
    let rest = &text[at + k.len()..];
    let (Some(open), Some(close)) = (rest.find('['), rest.find(']')) else {
        return Vec::new();
    };
    rest[open + 1..close]
        .split(',')
        .map(|s| s.trim().trim_matches('"').to_string())
        .filter(|s| !s.is_empty())
        .collect()
}

/// Module ids: the record file stems.
fn ids() -> Vec<String> {
    let mut v: Vec<String> = fs::read_dir(repo("files/usr/share/wolos/modules"))
        .expect("wolos registry (sibling checkout)")
        .flatten()
        .filter_map(|e| {
            e.file_name()
                .to_str()
                .and_then(|n| n.strip_suffix(".json"))
                .map(String::from)
        })
        .collect();
    v.sort();
    v
}

/// One bash array or scalar of a PKGBUILD, evaluated as makepkg does.
/// The package name of a dependency (`crystal>=1.22.0` -> `crystal`).
fn dep_name(d: &str) -> &str {
    d.split(['<', '>', '=']).next().unwrap_or(d)
}

fn pkgvar(dir: &str, var: &str) -> Vec<String> {
    let out = Command::new("bash")
        .current_dir(repo(dir))
        .arg("-c")
        .arg(format!(
            "set -eu; source ./PKGBUILD; printf '%s\\n' \"${{{var}[@]}}\""
        ))
        .output()
        .unwrap();
    assert!(
        out.status.success(),
        "{dir}: {}",
        String::from_utf8_lossy(&out.stderr)
    );
    String::from_utf8_lossy(&out.stdout)
        .lines()
        .filter(|l| !l.is_empty())
        .map(String::from)
        .collect()
}

#[test]
fn every_record_has_its_meta_package_and_the_same_dependencies() {
    let ids = ids();
    assert!(ids.len() >= 6, "{ids:?}");
    let list = read("packages");
    for id in &ids {
        let dir = format!("wolos-module-{id}");
        let rec = read(&format!("files/usr/share/wolos/modules/{id}.json"));
        assert_eq!(pkgvar(&dir, "pkgname"), std::slice::from_ref(&dir));
        assert_eq!(pkgvar(&dir, "arch"), ["any"], "{dir}: a meta package");
        let mut want = vec!["wolos-system".to_string()];
        want.extend(json_list(&rec, "packages"));
        // by name: a recipe may add a version floor (`crystal>=1.22.0`) the record leaves out
        let mut got: Vec<String> = pkgvar(&dir, "depends")
            .iter()
            .map(|d| dep_name(d).to_string())
            .collect();
        assert_eq!(got.len(), {
            let mut u = got.clone();
            u.sort();
            u.dedup();
            u.len()
        });
        got.sort();
        want.sort();
        assert_eq!(
            got, want,
            "{dir}: depends= is wolos-system + {id}.json packages"
        );
        // Nothing to fetch, nothing to build: no sources, no build step, --nodeps.
        let p = read(&format!("{dir}/PKGBUILD"));
        assert!(pkgvar(&dir, "source").is_empty(), "{dir} has sources");
        assert!(
            !p.contains("build()") && !p.contains("_wolos_commit"),
            "{dir}"
        );
        assert!(p.starts_with("# SPDX-License-Identifier: GPL-2.0-or-later\n"));
        assert!(p.contains("ADR 0032"), "{dir} names its ADR");
        assert!(
            read(&format!("{dir}/build.conf")).contains("makepkg_args=(--nodeps)"),
            "{dir}: the chroot never installs the module's gigabytes"
        );
        // pkgver is the release the records belong to.
        assert_eq!(pkgvar(&dir, "pkgver"), ["0.12.0"], "{dir}");
        // In the [wolos] repo, no local build deps, rebuilt when its directory changes.
        let line = list
            .lines()
            .find(|l| l.split_whitespace().next() == Some(dir.as_str()))
            .unwrap_or_else(|| panic!("packages lacks {dir}"));
        let f: Vec<&str> = line.split_whitespace().collect();
        assert_eq!(f[1..], [dir.as_str(), "-", &format!("{dir}/")], "{line}");
        // Anything outside Arch (Claude Code, PowerShell) is a user-side recipe, never a depends.
        for r in json_list(&rec, "recipes") {
            assert!(
                !got.contains(&r),
                "{dir}: recipe {r} must not be a pacman dependency"
            );
        }
    }
    // No meta package without a record.
    for e in fs::read_dir(siblings::root()).unwrap().flatten() {
        let n = e.file_name().to_string_lossy().into_owned();
        if let Some(id) = n.strip_prefix("wolos-module-") {
            assert!(ids.iter().any(|i| i == id), "{n} has no registry record");
        }
    }
}

/// The [wolos] repo's package names (first field of each `packages` line).
fn wolos_packages() -> Vec<String> {
    read("packages")
        .lines()
        .filter(|l| !l.trim_start().starts_with('#'))
        .filter_map(|l| l.split_whitespace().next().map(String::from))
        .collect()
}

/// Crystal is the userland language and Cryswol its framework (ADR 0036): the developer module
/// brings the compiler (with pkgconf), libwol.so.1 and the shard under
/// /usr/lib/wolos/crystal/cryswol, so a `require "cryswol"` program builds and runs on an installed
/// system (the image does not ship them). libwol and cryswol come from [wolos]; no other module depends on a [wolos] package.
#[test]
fn dev_module_brings_the_crystal_runtime() {
    let deps = pkgvar("wolos-module-dev", "depends");
    // pkgconf: `crystal build` asks pkg-config for the libraries a shard links.
    for p in ["crystal", "pkgconf", "libwol", "cryswol"] {
        assert!(
            deps.iter().any(|d| dep_name(d) == p),
            "wolos-module-dev lacks {p}"
        );
    }
    // the compiler at the pinned release or newer (wolos tools/crystal/toolchain.json, RCR-046)
    assert!(
        deps.iter().any(|d| d.starts_with("crystal>=")),
        "wolos-module-dev: crystal without the pinned floor"
    );
    let ours = wolos_packages();
    for p in ["libwol", "cryswol"] {
        assert!(ours.iter().any(|o| o == p), "{p} is not a [wolos] package");
    }
    // The shard's recipe still pulls the runtime it links.
    let shard = pkgvar("cryswol", "depends");
    assert!(
        shard.iter().any(|d| d.starts_with("crystal>=")) && shard.iter().any(|d| d == "libwol")
    );
    for id in ids() {
        let dir = format!("wolos-module-{id}");
        for d in pkgvar(&dir, "depends") {
            if d != "wolos-system" && ours.contains(&d) {
                assert!(
                    id == "dev" && (d == "libwol" || d == "cryswol"),
                    "{dir}: [wolos] dependency {d}"
                );
            }
        }
    }
}

#[test]
fn local_ai_takes_its_gpu_backend_on_the_root_side() {
    let deps = pkgvar("wolos-module-ai-local", "depends");
    assert!(deps.iter().any(|d| d == "ollama"));
    assert!(
        deps.iter().all(|d| !d.starts_with("ollama-")),
        "the backend is per machine: modules-install adds it, not depends="
    );
    assert_eq!(
        pkgvar("wolos-module-ai-local", "install"),
        ["wolos-module-ai-local.install"]
    );
    let i = read("wolos-module-ai-local/wolos-module-ai-local.install");
    assert!(i.contains("systemctl enable ollama.service"));
    // No running systemd in the installer's chroot: start only when it runs.
    assert!(i.contains("[ -d /run/systemd/system ]"));
    assert!(i.contains("pre_remove()") && i.contains("disable --now ollama.service"));
    let o = Command::new("bash")
        .arg("-n")
        .arg(repo("wolos-module-ai-local/wolos-module-ai-local.install"))
        .output()
        .unwrap();
    assert!(o.status.success(), "{}", String::from_utf8_lossy(&o.stderr));
    // The root helper adds the backend `wol hw compute` names; the user step installs nothing.
    let mi = read("files/usr/lib/wolos/modules-install");
    assert!(mi.contains("wol hw compute --json") && mi.contains("\"$stem-$b\""));
    // The record names the stem; the backend is never a depends= (checked above).
    let rec = read("files/usr/share/wolos/modules/ai-local.json");
    assert!(rec.contains("\"backend\": \"ollama\""), "{rec}");
    let post = read("files/usr/lib/wolos/ai-local");
    assert!(!post.contains("wol install") && !post.contains("pacman -S"));
}

/// Every dependency of every module is a package of the snapshot's core, extra or multilib repo (ADR 0032:
/// "dependencies exist at the snapshot"), or one of this repo's [wolos] packages (libwol, cryswol for
/// the developer module, checked above). Needs the network and bsdtar.
#[test]
#[ignore = "network: downloads the snapshot's core, extra and multilib databases"]
fn dependencies_exist_at_the_snapshot() {
    let snap = read("snapshot").trim().replace('-', "/");
    let mut names: Vec<String> = Vec::new();
    for r in ["core", "extra", "multilib"] {
        let url = format!("https://archive.archlinux.org/repos/{snap}/{r}/os/x86_64/{r}.db");
        let out = Command::new("bash")
            .arg("-c")
            .arg(format!(
                "set -o pipefail; curl -fsSL --max-time 300 '{url}' | bsdtar -tf - | sed -n 's|/$||p'"
            ))
            .output()
            .unwrap();
        assert!(
            out.status.success(),
            "{url}: {}",
            String::from_utf8_lossy(&out.stderr)
        );
        // name-pkgver-pkgrel: the name is everything before the last two dashes.
        for l in String::from_utf8_lossy(&out.stdout).lines() {
            let mut p: Vec<&str> = l.rsplitn(3, '-').collect();
            if p.len() == 3 {
                names.push(p.pop().unwrap().to_string());
            }
        }
    }
    assert!(names.len() > 1000, "{} packages read", names.len());
    let mut missing = Vec::new();
    let ours = wolos_packages();
    for id in ids() {
        for d in pkgvar(&format!("wolos-module-{id}"), "depends") {
            if d != "wolos-system" && !ours.contains(&d) && !names.contains(&d) {
                missing.push(format!("{id}: {d}"));
            }
        }
    }
    // The GPU backends modules-install may add for ai-local.
    for b in ["ollama-cuda", "ollama-rocm", "ollama-vulkan"] {
        if !names.iter().any(|n| n == b) {
            missing.push(format!("ai-local backend: {b}"));
        }
    }
    assert!(missing.is_empty(), "not at the snapshot: {missing:?}");
}
