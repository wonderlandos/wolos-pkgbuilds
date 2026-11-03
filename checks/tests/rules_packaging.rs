// SPDX-License-Identifier: GPL-2.0-or-later
//! What this file checks (wolos-pkgbuilds): the packaging rules RP-nnn of docs/RULES-packaging.md,
//! one test per rule, read from the PKGBUILDs, `packages`, the .install scripts, tools/bump-wolos.sh
//! and these tests (no makepkg). Baselines are growth-only: BASELINE names today's known
//! violations; a new violation fails, and so does a baseline entry that no longer violates (delete
//! it). The doc and the tests list the same ids (rp_catalogue_matches_docs).
#![allow(dead_code)]
use std::{
    collections::{BTreeMap, BTreeSet},
    fs,
    path::{Path, PathBuf},
};

/// (rule, item) pairs that violate today. Remove an entry when fixed; never add one.
const BASELINE: &[(&str, &str)] = &[
    // RP-024: a recipe directory that is not a line of `packages` (built by hand).
    ("RP-024", "wolos-browser-config"),
    // RP-034: wolos-base reaches hyprland-wolos through wolos-system (transitive build dep).
    ("RP-034", "wolos-base: hyprland-wolos"),
    // RP-051: contract tests of other repos' files or of the image, not of a recipe directory.
    ("RP-051", "boot_setup_sealed_units.rs"),
    ("RP-051", "hypr_conf.rs"),
    ("RP-051", "image_repo_sums.rs"),
    ("RP-051", "modules.rs"),
];

fn root() -> PathBuf {
    let d = Path::new(env!("CARGO_MANIFEST_DIR"))
        .canonicalize()
        .unwrap();
    d.ancestors()
        .find(|p| p.join("LICENSE").is_file())
        .unwrap_or(&d)
        .to_path_buf()
}

fn read(p: &str) -> String {
    let path = root().join(p);
    fs::read_to_string(&path).unwrap_or_else(|e| panic!("{}: {e}", path.display()))
}

/// Package directories (those with a PKGBUILD) and their text, sorted.
fn recipes() -> Vec<(String, String)> {
    let mut v = vec![];
    for e in fs::read_dir(root()).unwrap().flatten() {
        let n = e.file_name().to_string_lossy().to_string();
        if e.path().join("PKGBUILD").is_file() {
            v.push((n.clone(), read(&format!("{n}/PKGBUILD"))));
        }
    }
    v.sort();
    v
}

struct Line {
    name: String,
    dir: String,
    deps: Vec<String>,
    paths: Vec<String>,
}

fn packages() -> Vec<Line> {
    read("packages")
        .lines()
        .filter(|l| !l.trim().is_empty() && !l.starts_with('#'))
        .map(|l| {
            let f: Vec<&str> = l.split_whitespace().collect();
            Line {
                name: f[0].into(),
                dir: f.get(1).copied().unwrap_or("").into(),
                deps: match f.get(2).copied() {
                    None | Some("-") => vec![],
                    Some(d) => d.split(',').map(String::from).collect(),
                },
                paths: f.iter().skip(3).map(|s| s.to_string()).collect(),
            }
        })
        .collect()
}

fn unquote(s: &str) -> String {
    let t = s.trim();
    for q in ['\'', '"'] {
        if t.len() >= 2 && t.starts_with(q) && t.ends_with(q) {
            return t[1..t.len() - 1].to_string();
        }
    }
    t.to_string()
}

/// `name=value` at the start of a line (first one), without quotes or a trailing comment.
fn scalar(text: &str, name: &str) -> Option<String> {
    let pre = format!("{name}=");
    text.lines().find_map(|l| {
        let v = l.strip_prefix(&pre)?;
        if v.starts_with('(') {
            return None;
        }
        let v = match v.find("  #") {
            Some(i) if !v.starts_with(['\'', '"']) => &v[..i],
            _ => v,
        };
        Some(unquote(v.split(" #").next().unwrap_or(v).trim_end()))
    })
}

/// `name=( ... )` at the start of a line: its words, quotes removed, comments skipped.
fn array(text: &str, name: &str) -> Option<Vec<String>> {
    let pre = format!("{name}=(");
    let at = text.lines().scan(0usize, |off, l| {
        let o = *off;
        *off += l.len() + 1;
        Some((o, l))
    });
    let start = at
        .filter(|(_, l)| l.starts_with(&pre))
        .map(|(o, _)| o + pre.len())
        .next()?;
    let (mut out, mut cur, mut have) = (vec![], String::new(), false);
    let (mut q, mut paren) = (None::<char>, 0);
    let mut it = text[start..].chars().peekable();
    while let Some(c) = it.next() {
        match (q, c) {
            (Some(k), c) if c == k => q = None,
            (Some(_), c) => cur.push(c),
            (None, '\'' | '"') => {
                q = Some(c);
                have = true;
            }
            (None, '#') => {
                for d in it.by_ref() {
                    if d == '\n' {
                        break;
                    }
                }
            }
            (None, '(') => {
                paren += 1;
                cur.push(c);
            }
            (None, ')') if paren > 0 => {
                paren -= 1;
                cur.push(c);
            }
            (None, ')') => break,
            (None, c) if c.is_whitespace() => {
                if have || !cur.is_empty() {
                    out.push(std::mem::take(&mut cur));
                    have = false;
                }
            }
            (None, c) => {
                cur.push(c);
                have = true;
            }
        }
    }
    if have || !cur.is_empty() {
        out.push(cur);
    }
    Some(out)
}

/// Body of `name() {` up to the first line that is exactly `}`.
fn func(text: &str, name: &str) -> Option<String> {
    let head = format!("{name}() {{");
    let mut it = text.lines();
    it.find(|l| l.starts_with(&head))?;
    Some(it.take_while(|l| *l != "}").collect::<Vec<_>>().join("\n"))
}

fn funcs(text: &str, prefix: &str) -> Vec<String> {
    text.lines()
        .filter(|l| l.starts_with(prefix) && l.contains("() {"))
        .filter_map(|l| func(text, l.split("()").next().unwrap()))
        .collect()
}

/// Every package-time function body of a recipe (package, package_*).
fn package_bodies(text: &str) -> Vec<String> {
    funcs(text, "package")
}

fn is_sha(s: &str) -> bool {
    s.len() == 40
        && s.bytes()
            .all(|b| b.is_ascii_hexdigit() && !b.is_ascii_uppercase())
}

fn baselined(rule: &str, item: &str) -> bool {
    BASELINE.iter().any(|(r, i)| *r == rule && *i == item)
}

/// Fail on a violation that is not baselined, and on a baseline entry that does not violate.
fn verdict(rule: &str, found: Vec<String>) {
    let found: BTreeSet<String> = found.into_iter().collect();
    let new: Vec<_> = found.iter().filter(|i| !baselined(rule, i)).collect();
    assert!(
        new.is_empty(),
        "{rule}: violations (docs/RULES-packaging.md): {new:#?}"
    );
    let stale: Vec<_> = BASELINE
        .iter()
        .filter(|(r, i)| *r == rule && !found.contains(*i))
        .collect();
    assert!(
        stale.is_empty(),
        "{rule}: baseline entries that no longer violate, delete them: {stale:?}"
    );
}

/// Run `f` on every recipe; it returns the violations of that recipe.
fn each(rule: &str, f: impl Fn(&str, &str) -> Vec<String>) {
    let mut found = vec![];
    for (d, t) in recipes() {
        for v in f(&d, &t) {
            found.push(if v.is_empty() {
                d.clone()
            } else {
                format!("{d}: {v}")
            });
        }
    }
    verdict(rule, found);
}

fn missing(t: &str, k: &str) -> Vec<String> {
    if scalar(t, k).is_some() {
        vec![]
    } else {
        vec![String::new()]
    }
}

// ---- the recipes' metadata ------------------------------------------------------------------
#[test]
fn rp_001_pkgdesc() {
    each("RP-001", |_, t| {
        if scalar(t, "pkgdesc").is_some_and(|v| !v.is_empty()) {
            vec![]
        } else {
            vec![String::new()]
        }
    });
}
#[test]
fn rp_002_url() {
    each("RP-002", |_, t| missing(t, "url"));
}
#[test]
fn rp_003_license_array() {
    each("RP-003", |_, t| match array(t, "license") {
        Some(a) if !a.is_empty() => vec![],
        _ => vec![String::new()],
    });
}
#[test]
fn rp_004_arch() {
    each("RP-004", |_, t| match array(t, "arch") {
        Some(a) if !a.is_empty() => vec![],
        _ => vec![String::new()],
    });
}
#[test]
fn rp_005_pkgver_and_pkgrel() {
    each("RP-005", |_, t| {
        let mut v = vec![];
        let ver = scalar(t, "pkgver");
        if ver
            .as_deref()
            .is_none_or(|s| s.is_empty() || s.contains('-'))
        {
            v.push("pkgver missing or has a hyphen".into());
        }
        if scalar(t, "pkgrel").is_none_or(|s| s.parse::<u32>().is_err()) {
            v.push("pkgrel is not an integer".into());
        }
        v
    });
}
#[test]
fn rp_006_pkgname_or_pkgbase() {
    each("RP-006", |_, t| {
        if t.lines()
            .any(|l| l.starts_with("pkgname=") || l.starts_with("pkgbase="))
        {
            vec![]
        } else {
            vec![String::new()]
        }
    });
}
#[test]
fn rp_007_spdx_header() {
    each("RP-007", |_, t| {
        if t.lines()
            .next()
            .is_some_and(|l| l.starts_with("# SPDX-License-Identifier:"))
        {
            vec![]
        } else {
            vec!["first line".into()]
        }
    });
}
#[test]
fn rp_008_sha256sums_present_with_source() {
    each("RP-008", |_, t| {
        match (array(t, "source"), array(t, "sha256sums")) {
            (Some(s), None) if !s.is_empty() => vec!["source without sha256sums".into()],
            _ => vec![],
        }
    });
}
#[test]
fn rp_009_one_checksum_per_source() {
    each("RP-009", |_, t| {
        match (array(t, "source"), array(t, "sha256sums")) {
            (Some(s), Some(c)) if s.len() != c.len() => {
                vec![format!("{} sources, {} sums", s.len(), c.len())]
            }
            _ => vec![],
        }
    });
}
#[test]
fn rp_010_skip_only_where_documented() {
    // SKIP is for a git source pinned by #commit= and for the files named here, each with its reason.
    const OK: &[(&str, &str)] = &[("wolos-base", "base.manifest")]; // written by prebuild.sh just before makepkg
    each("RP-010", |d, t| {
        let (Some(s), Some(c)) = (array(t, "source"), array(t, "sha256sums")) else {
            return vec![];
        };
        s.iter()
            .zip(&c)
            .filter(|(src, sum)| {
                *sum == "SKIP"
                    && !src.contains("#commit=")
                    && !OK.iter().any(|(dd, f)| *dd == d && src == f)
                    && !((src.ends_with(".asc") || src.ends_with(".sig"))
                        && t.contains("validpgpkeys="))
            })
            .map(|(src, _)| format!("SKIP for {src}"))
            .collect()
    });
}
#[test]
fn rp_011_sha256_values_are_64_hex_or_expansions() {
    each("RP-011", |_, t| {
        array(t, "sha256sums")
            .unwrap_or_default()
            .iter()
            .filter(|s| {
                *s != "SKIP"
                    && !s.starts_with('$')
                    && !(s.len() == 64
                        && s.bytes()
                            .all(|b| b.is_ascii_hexdigit() && !b.is_ascii_uppercase()))
            })
            .map(|s| format!("bad sum {s}"))
            .collect()
    });
}
#[test]
fn rp_012_git_sources_pin_a_commit() {
    each("RP-012", |_, t| {
        array(t, "source")
            .unwrap_or_default()
            .iter()
            .filter(|s| s.contains("git+") && !s.contains("#commit="))
            .map(|s| format!("git source without #commit=: {s}"))
            .collect()
    });
}
#[test]
fn rp_013_no_branch_or_tag_sources() {
    each("RP-013", |_, t| {
        array(t, "source")
            .unwrap_or_default()
            .iter()
            .filter(|s| s.contains("#branch=") || s.contains("#tag=") || s.contains("#revision="))
            .map(|s| s.to_string())
            .collect()
    });
}
#[test]
fn rp_014_no_or_true_in_package() {
    each("RP-014", |_, t| {
        package_bodies(t)
            .iter()
            .flat_map(|b| b.lines())
            .filter(|l| l.contains("|| true") && !l.trim_start().starts_with('#'))
            .map(|l| l.trim().to_string())
            .collect()
    });
}
#[test]
fn rp_015_install_dm_has_explicit_modes() {
    each("RP-015", |_, t| {
        package_bodies(t)
            .iter()
            .flat_map(|b| b.lines())
            .filter(|l| {
                let w: Vec<&str> = l.split_whitespace().collect();
                w.first() == Some(&"install")
                    && w.iter()
                        .skip(1)
                        .any(|a| a.starts_with("-D") || a.starts_with("-pD"))
                    && !w.iter().any(|a| {
                        a.starts_with("-Dm")
                            || a.starts_with("-m")
                            || a.starts_with("-pDm")
                            || a.contains("m6")
                            || a.contains("m7")
                    })
            })
            .map(|l| l.trim().to_string())
            .collect()
    });
}
#[test]
fn rp_016_no_world_writable_chmod() {
    each("RP-016", |_, t| {
        t.lines()
            .filter(|l| !l.trim_start().starts_with('#'))
            .filter(|l| {
                l.contains("chmod")
                    && (l.contains(" 777")
                        || l.contains(" 666")
                        || l.contains("a+w")
                        || l.contains("o+w")
                        || l.contains(" 0777")
                        || l.contains("ugo+w"))
            })
            .map(|l| l.trim().to_string())
            .collect()
    });
}
#[test]
fn rp_017_no_network_or_privilege_in_build_functions() {
    each("RP-017", |_, t| {
        let mut bodies = package_bodies(t);
        bodies.extend(funcs(t, "build"));
        bodies.extend(funcs(t, "prepare"));
        bodies
            .iter()
            .flat_map(|b| b.lines())
            .filter(|l| !l.trim_start().starts_with('#'))
            .filter(|l| {
                ["curl ", "wget ", "sudo ", "pacman -S"].iter().any(|x| {
                    l.trim_start().starts_with(x)
                        || l.contains(&format!("| {x}"))
                        || l.contains(&format!("&& {x}"))
                })
            })
            .map(|l| l.trim().to_string())
            .collect()
    });
}
#[test]
fn rp_018_package_function_exists() {
    each("RP-018", |_, t| {
        if t.lines()
            .any(|l| l.starts_with("package() {") || l.starts_with("package_"))
            || t.contains("eval \"package_")
        {
            vec![]
        } else {
            vec![String::new()]
        }
    });
}
#[test]
fn rp_019_package_writes_under_pkgdir() {
    each("RP-019", |_, t| {
        package_bodies(t)
            .iter()
            .flat_map(|b| b.lines())
            .filter(|l| !l.trim_start().starts_with('#'))
            .filter(|l| {
                let w: Vec<&str> = l.split_whitespace().collect();
                matches!(
                    w.first(),
                    Some(&"install") | Some(&"cp") | Some(&"mkdir") | Some(&"ln") | Some(&"mv")
                ) && w.last().is_some_and(|a| {
                    ["/usr/", "/etc/", "/opt/", "/var/", "/boot/"]
                        .iter()
                        .any(|p| unquote(a).starts_with(p))
                })
            })
            .map(|l| l.trim().to_string())
            .collect()
    });
}
#[test]
fn rp_020_license_ids_are_spdx_style() {
    const OLD: &[&str] = &[
        "GPL", "GPL2", "GPL3", "LGPL", "LGPL2.1", "LGPL3", "BSD", "Apache", "custom", "AGPL",
    ];
    each("RP-020", |_, t| {
        array(t, "license")
            .unwrap_or_default()
            .iter()
            .filter(|l| OLD.contains(&l.as_str()) || l.starts_with("custom:"))
            .map(|l| l.to_string())
            .collect()
    });
}
#[test]
fn rp_021_arch_is_x86_64_or_any() {
    each("RP-021", |_, t| {
        array(t, "arch")
            .unwrap_or_default()
            .iter()
            .filter(|a| *a != "x86_64" && *a != "any")
            .map(|a| a.to_string())
            .collect()
    });
}
#[test]
fn rp_022_https_only() {
    each("RP-022", |_, t| {
        let mut v: Vec<String> = array(t, "source")
            .unwrap_or_default()
            .into_iter()
            .filter(|s| s.contains("http://"))
            .collect();
        if scalar(t, "url").is_some_and(|u| u.starts_with("http://")) {
            v.push("url is http".into());
        }
        v
    });
}
#[test]
fn rp_023_backup_paths_are_relative() {
    each("RP-023", |_, t| {
        array(t, "backup")
            .unwrap_or_default()
            .into_iter()
            .filter(|b| b.starts_with('/'))
            .collect()
    });
}
#[test]
fn rp_043_pkgdesc_style() {
    each("RP-043", |_, t| {
        let d = scalar(t, "pkgdesc").unwrap_or_default();
        let mut v = vec![];
        if d.ends_with('.') {
            v.push("pkgdesc ends with a period".into());
        }
        if d.len() > 160 {
            v.push("pkgdesc longer than 160".into());
        }
        v
    });
}

// ---- the packages file ----------------------------------------------------------------------
#[test]
fn rp_024_every_recipe_dir_is_in_packages() {
    let dirs: BTreeSet<String> = packages().into_iter().map(|l| l.dir).collect();
    verdict(
        "RP-024",
        recipes()
            .into_iter()
            .map(|(d, _)| d)
            .filter(|d| !dirs.contains(d))
            .collect(),
    );
}
#[test]
fn rp_025_every_packages_dir_has_a_pkgbuild() {
    verdict(
        "RP-025",
        packages()
            .into_iter()
            .filter(|l| !root().join(&l.dir).join("PKGBUILD").is_file())
            .map(|l| format!("{}: {}", l.name, l.dir))
            .collect(),
    );
}
#[test]
fn rp_026_package_names_are_unique() {
    let mut seen = BTreeSet::new();
    verdict(
        "RP-026",
        packages()
            .into_iter()
            .filter(|l| !seen.insert(l.name.clone()))
            .map(|l| l.name)
            .collect(),
    );
}
#[test]
fn rp_027_every_line_has_four_fields() {
    verdict(
        "RP-027",
        packages()
            .into_iter()
            .filter(|l| l.dir.is_empty() || l.paths.is_empty())
            .map(|l| l.name)
            .collect(),
    );
}
#[test]
fn rp_028_deps_are_earlier_lines() {
    let mut seen = BTreeSet::new();
    let mut v = vec![];
    for l in packages() {
        for d in &l.deps {
            if !seen.contains(d) {
                v.push(format!("{}: {d}", l.name));
            }
        }
        seen.insert(l.name);
    }
    verdict("RP-028", v);
}
#[test]
fn rp_029_waves_resolve_without_a_cycle() {
    // lib/plan.py in wolos-build: waves of packages whose local deps are in earlier waves.
    let p = packages();
    let mut done: BTreeSet<String> = BTreeSet::new();
    let mut left: Vec<&Line> = p.iter().collect();
    let mut v = vec![];
    while !left.is_empty() {
        let (ready, rest): (Vec<&Line>, Vec<&Line>) = left
            .into_iter()
            .partition(|l| l.deps.iter().all(|d| done.contains(d)));
        if ready.is_empty() {
            v = rest.iter().map(|l| l.name.clone()).collect();
            break;
        }
        done.extend(ready.iter().map(|l| l.name.clone()));
        left = rest;
    }
    verdict("RP-029", v);
}
#[test]
fn rp_030_paths_start_with_the_package_dir() {
    verdict(
        "RP-030",
        packages()
            .into_iter()
            .filter(|l| {
                !l.paths
                    .iter()
                    .any(|p| p.starts_with(&format!("{}/", l.dir)))
            })
            .map(|l| l.name)
            .collect(),
    );
}
#[test]
fn rp_031_paths_exist() {
    verdict(
        "RP-031",
        packages()
            .into_iter()
            .flat_map(|l| l.paths.into_iter().map(move |p| (l.name.clone(), p)))
            .filter(|(_, p)| !root().join(p.trim_end_matches('/')).exists())
            .map(|(n, p)| format!("{n}: {p}"))
            .collect(),
    );
}
#[test]
fn rp_032_same_dir_same_deps_and_paths() {
    let mut by: BTreeMap<String, (Vec<String>, Vec<String>)> = BTreeMap::new();
    let mut v = vec![];
    for l in packages() {
        match by.get(&l.dir) {
            Some((d, p)) if *d != l.deps || *p != l.paths => v.push(l.name.clone()),
            Some(_) => {}
            None => {
                by.insert(l.dir.clone(), (l.deps.clone(), l.paths.clone()));
            }
        }
    }
    verdict("RP-032", v);
}
#[test]
fn rp_033_package_named_like_its_dir_is_declared_by_its_recipe() {
    let mut v = vec![];
    for l in packages() {
        if l.name != l.dir {
            continue;
        }
        let t = read(&format!("{}/PKGBUILD", l.dir));
        let n = scalar(&t, "pkgbase").or_else(|| scalar(&t, "pkgname"));
        let multi = array(&t, "pkgname");
        let declared = n.as_deref() == Some(&l.name)
            || multi.is_some_and(|a| {
                a.iter()
                    .any(|x| x.contains(&l.name) || x.contains("$pkgbase"))
            });
        if !declared {
            v.push(l.name);
        }
    }
    verdict("RP-033", v);
}
#[test]
fn rp_034_local_deps_appear_in_the_recipe_or_its_build_files() {
    // A [wolos] package a recipe is built against is named in its depends/makedepends, or in
    // prebuild.sh/build.conf (wolos-base reads packages through prebuild.sh).
    let mut v = vec![];
    for l in packages() {
        let mut text = read(&format!("{}/PKGBUILD", l.dir));
        for f in ["prebuild.sh", "build.conf"] {
            if let Ok(x) = fs::read_to_string(root().join(&l.dir).join(f)) {
                text.push_str(&x);
            }
        }
        for d in &l.deps {
            if !text.contains(d.as_str()) {
                v.push(format!("{}: {d}", l.name));
            }
        }
    }
    verdict("RP-034", v);
}
#[test]
fn rp_035_no_duplicate_pkgname_across_recipes() {
    let mut seen: BTreeMap<String, String> = BTreeMap::new();
    let mut v = vec![];
    for (d, t) in recipes() {
        let names = array(&t, "pkgname")
            .or_else(|| scalar(&t, "pkgname").map(|s| vec![s]))
            .unwrap_or_default();
        for n in names.into_iter().filter(|n| !n.contains('$')) {
            if let Some(o) = seen.insert(n.clone(), d.clone()) {
                v.push(format!("{n} in {o} and {d}"));
            }
        }
    }
    verdict("RP-035", v);
}
#[test]
fn rp_036_provides_that_replace_an_arch_package_also_conflict() {
    // A provides of an Arch package name (not a library or a virtual capability) needs the conflict.
    const VIRTUAL: &[&str] = &[
        "web-browser",
        "wayland-compositor",
        "VIRTUALBOX-GUEST-MODULES",
    ];
    let mut v = vec![];
    for (d, t) in recipes() {
        let conflicts: Vec<String> = array(&t, "conflicts")
            .unwrap_or_default()
            .into_iter()
            .map(|c| c.split(['=', '<', '>']).next().unwrap().to_string())
            .collect();
        for p in array(&t, "provides").unwrap_or_default() {
            let base = p.split(['=', '<', '>']).next().unwrap().to_string();
            if base.contains(".so") || VIRTUAL.contains(&base.as_str()) || conflicts.contains(&base)
            {
                continue;
            }
            v.push(format!("{d}: provides {base} without conflicts"));
        }
    }
    verdict("RP-036", v);
}
#[test]
fn rp_037_a_package_never_conflicts_with_itself() {
    let mut v = vec![];
    for (d, t) in recipes() {
        let own = scalar(&t, "pkgname")
            .or_else(|| scalar(&t, "pkgbase"))
            .unwrap_or_default();
        if array(&t, "conflicts").unwrap_or_default().contains(&own) {
            v.push(d);
        }
    }
    verdict("RP-037", v);
}
#[test]
fn rp_038_sizes_in_plan_py_name_real_packages() {
    // Only when wolos-build sits next to this repo (CI checks it out; absent: nothing to check).
    let Ok(plan) = fs::read_to_string(root().parent().unwrap().join("wolos-build/lib/plan.py"))
    else {
        return;
    };
    let names: BTreeSet<String> = packages().into_iter().map(|l| l.name).collect();
    let block: String = plan
        .lines()
        .skip_while(|l| !l.starts_with("SIZES"))
        .take_while(|l| !l.starts_with("DEFAULT_SIZE"))
        .collect::<Vec<_>>()
        .join("\n");
    let v = block
        .split('"')
        .skip(1)
        .step_by(2)
        .filter(|k| !names.contains(*k))
        .map(String::from)
        .collect();
    verdict("RP-038", v);
}

// ---- pins and tools/bump-wolos.sh -----------------------------------------------------------
const BUMPED: &[&str] = &[
    "_wolos_commit",
    "_brand_commit",
    "_shell_commit",
    "_os_release_sha256",
];

#[test]
fn rp_039_wolos_commit_is_a_full_sha_and_one_value() {
    let mut vals = BTreeSet::new();
    let mut v = vec![];
    for (d, t) in recipes() {
        if let Some(c) = scalar(&t, "_wolos_commit") {
            if !is_sha(&c) {
                v.push(format!("{d}: {c}"));
            }
            vals.insert(c);
        }
    }
    if vals.len() > 1 {
        v.push(format!("different pins: {vals:?}"));
    }
    verdict("RP-039", v);
}
#[test]
fn rp_040_brand_and_shell_commits_are_full_shas() {
    let mut v = vec![];
    for (d, t) in recipes() {
        for k in ["_brand_commit", "_shell_commit"] {
            if let Some(c) = scalar(&t, k)
                && !is_sha(&c)
            {
                v.push(format!("{d}: {k}={c}"));
            }
        }
    }
    verdict("RP-040", v);
}
#[test]
fn rp_041_every_pin_variable_is_one_bump_wolos_rewrites() {
    let bump = read("tools/bump-wolos.sh");
    let mut v = vec![];
    for (d, t) in recipes() {
        for line in t.lines() {
            let Some((k, _)) = line.split_once('=') else {
                continue;
            };
            let wolos_pin = k.starts_with('_')
                && (k.ends_with("_commit") || k.ends_with("_sha256"))
                && (t.contains("wonderlandos/wolos.git") || t.contains("/wonderlandos/wolos/"));
            if wolos_pin && !BUMPED.contains(&k) && k != "_commit" {
                v.push(format!(
                    "{d}: {k} is pinned from wolos but bump-wolos.sh does not rewrite it"
                ));
            }
            if BUMPED.contains(&k) && !bump.contains(&format!("^{k}=")) {
                v.push(format!("{d}: {k} not rewritten by bump-wolos.sh"));
            }
        }
    }
    verdict("RP-041", v);
}
#[test]
fn rp_042_bump_wolos_rewrites_only_variables_some_recipe_has() {
    let bump = read("tools/bump-wolos.sh");
    let all: String = recipes().into_iter().map(|(_, t)| t).collect();
    let v = BUMPED
        .iter()
        .filter(|k| bump.contains(&format!("^{k}=")) && !all.contains(&format!("\n{k}=")))
        .map(|k| k.to_string())
        .collect();
    verdict("RP-042", v);
}
#[test]
fn rp_044_bump_wolos_names_existing_dirs() {
    let bump = read("tools/bump-wolos.sh");
    let mut v = vec![];
    for tok in bump.split_whitespace() {
        if let Some(d) = tok.strip_suffix("/*)").or_else(|| tok.strip_suffix("/*"))
            && d.chars().all(|c| c.is_ascii_alphanumeric() || c == '-')
            && !d.is_empty()
            && d != "*"
            && !root().join(d).is_dir()
        {
            v.push(d.to_string());
        }
    }
    verdict("RP-044", v);
}
#[test]
fn rp_045_wolos_pinned_recipes_get_bumped_by_pkgrel() {
    // bump-wolos.sh edits pkgrel with sed on `^pkgrel=`: each pinned recipe keeps that exact form.
    let mut v = vec![];
    for (d, t) in recipes() {
        if scalar(&t, "_wolos_commit").is_some()
            && !t
                .lines()
                .any(|l| l.starts_with("pkgrel=") && l[7..].chars().all(|c| c.is_ascii_digit()))
        {
            v.push(d);
        }
    }
    verdict("RP-045", v);
}

// ---- .install scripts -----------------------------------------------------------------------
fn installs() -> Vec<String> {
    let mut v = vec![];
    for (d, _) in recipes() {
        for e in fs::read_dir(root().join(&d)).unwrap().flatten() {
            let n = e.file_name().to_string_lossy().to_string();
            if n.ends_with(".install") {
                v.push(format!("{d}/{n}"));
            }
        }
    }
    v.sort();
    v
}

#[test]
fn rp_046_install_scripts_hold_only_pre_post_functions() {
    const OK: &[&str] = &[
        "pre_install",
        "post_install",
        "pre_upgrade",
        "post_upgrade",
        "pre_remove",
        "post_remove",
    ];
    let mut v = vec![];
    for f in installs() {
        for (n, l) in read(&f).lines().enumerate() {
            let top = !l.starts_with([' ', '\t'])
                && !l.trim().is_empty()
                && !l.starts_with('#')
                && l != "}";
            if !top {
                continue;
            }
            match l.strip_suffix("() {") {
                Some(name) if OK.contains(&name) => {}
                _ => v.push(format!("{f}:{}: {l}", n + 1)),
            }
        }
    }
    verdict("RP-046", v);
}
#[test]
fn rp_047_every_install_file_is_named_by_its_recipe() {
    let mut v = vec![];
    for f in installs() {
        let (d, n) = f.split_once('/').unwrap();
        if scalar(&read(&format!("{d}/PKGBUILD")), "install").as_deref() != Some(n) {
            v.push(f);
        }
    }
    verdict("RP-047", v);
}
#[test]
fn rp_048_install_equals_names_an_existing_file() {
    let mut v = vec![];
    for (d, t) in recipes() {
        if let Some(i) = scalar(&t, "install")
            && !root().join(&d).join(&i).is_file()
        {
            v.push(format!("{d}: {i}"));
        }
    }
    verdict("RP-048", v);
}
#[test]
fn rp_049_install_scripts_do_not_fail_the_transaction() {
    // Hooks that run tools which may be absent end in `|| :` or `|| true`; `exit 1` aborts the install.
    let mut v = vec![];
    for f in installs() {
        for (n, l) in read(&f).lines().enumerate() {
            if l.trim_start().starts_with("exit 1") || l.contains("set -e") {
                v.push(format!("{f}:{}", n + 1));
            }
        }
    }
    verdict("RP-049", v);
}
#[test]
fn rp_050_install_scripts_end_with_a_newline() {
    verdict(
        "RP-050",
        installs()
            .into_iter()
            .filter(|f| !read(f).ends_with('\n'))
            .collect(),
    );
}

// ---- these tests ----------------------------------------------------------------------------
fn test_files() -> Vec<String> {
    let mut v: Vec<String> = fs::read_dir(root().join("checks/tests"))
        .unwrap()
        .flatten()
        .map(|e| e.file_name().to_string_lossy().to_string())
        .filter(|n| n.ends_with(".rs"))
        .collect();
    v.sort();
    v
}

/// Quoted `"<dir>/..."` path literals of a test file whose first segment looks like a directory.
fn quoted_dirs(text: &str) -> BTreeSet<String> {
    let mut out = BTreeSet::new();
    for part in text.split('"').skip(1).step_by(2) {
        if let Some((d, rest)) = part.split_once('/')
            && !d.is_empty()
            && !rest.is_empty()
            && d.chars()
                .all(|c| c.is_ascii_lowercase() || c.is_ascii_digit() || c == '-')
        {
            out.insert(d.to_string());
        }
    }
    out
}

#[test]
fn rp_051_every_checks_test_names_a_real_package_dir() {
    // Tests of this repo's files read recipes by path; a test that names none of this repo's
    // directories tests the sibling repos (listed here with the reason) or the tests themselves.
    const OTHER: &[&str] = &["siblings_copy.rs", "rules_packaging.rs"];
    let dirs: BTreeSet<String> = fs::read_dir(root())
        .unwrap()
        .flatten()
        .filter(|e| e.path().is_dir())
        .map(|e| e.file_name().to_string_lossy().to_string())
        .collect();
    let mut v = vec![];
    for f in test_files() {
        if OTHER.contains(&f.as_str()) {
            continue;
        }
        if !quoted_dirs(&read(&format!("checks/tests/{f}")))
            .iter()
            .any(|d| dirs.contains(d))
        {
            v.push(f);
        }
    }
    verdict("RP-051", v);
}
#[test]
fn rp_052_no_test_reads_a_pkgbuild_that_does_not_exist() {
    let mut v = vec![];
    for f in test_files() {
        // Fixture tests write throwaway recipes (foo/PKGBUILD) into temp dirs: not real packages.
        if f == "rules_packaging.rs" || read(&format!("checks/tests/{f}")).contains("fs::write") {
            continue;
        }
        for part in read(&format!("checks/tests/{f}"))
            .split('"')
            .skip(1)
            .step_by(2)
        {
            if let Some(d) = part.strip_suffix("/PKGBUILD")
                && !d.contains(['{', ' '])
                && !root().join(d).join("PKGBUILD").is_file()
            {
                v.push(format!("{f}: {part}"));
            }
        }
    }
    verdict("RP-052", v);
}
#[test]
fn rp_053_every_test_file_has_spdx_and_a_what_this_checks_line() {
    let mut v = vec![];
    for f in test_files() {
        let t = read(&format!("checks/tests/{f}"));
        let mut l = t.lines();
        if !l
            .next()
            .is_some_and(|x| x.starts_with("// SPDX-License-Identifier:"))
        {
            v.push(format!("{f}: no SPDX first line"));
        }
        if !t.lines().take(12).any(|x| x.starts_with("//!")) {
            v.push(format!("{f}: no //! description"));
        }
    }
    verdict("RP-053", v);
}
#[test]
fn rp_054_every_test_file_has_a_test() {
    verdict(
        "RP-054",
        test_files()
            .into_iter()
            .filter(|f| !read(&format!("checks/tests/{f}")).contains("#[test]"))
            .collect(),
    );
}

/// Rule ids of docs/RULES-packaging.md (`| RP-nnn |` rows) and of the `fn rp_nnn_` tests here.
#[test]
fn rp_catalogue_matches_docs() {
    let docs: BTreeSet<String> = read("docs/RULES-packaging.md")
        .lines()
        .filter_map(|l| l.strip_prefix("| RP-").map(|r| r[..3].to_string()))
        .collect();
    let me = read("checks/tests/rules_packaging.rs");
    let tests: BTreeSet<String> = me
        .lines()
        .filter_map(|l| l.strip_prefix("fn rp_").map(|r| r[..3].to_string()))
        .filter(|n| n.chars().all(|c| c.is_ascii_digit()))
        .collect();
    assert_eq!(
        docs, tests,
        "docs/RULES-packaging.md and rules_packaging.rs list different rules"
    );
    assert!(
        tests.len() >= 40,
        "{} rules, at least 40 wanted",
        tests.len()
    );
}

#[test]
fn rp_baseline_names_real_rules() {
    // Without whitespace, so a call rustfmt breaks over lines (verdict, newline, "RP-024") counts.
    let me: String = read("checks/tests/rules_packaging.rs")
        .chars()
        .filter(|c| !c.is_whitespace())
        .collect();
    for (r, _) in BASELINE {
        assert!(
            me.contains(&format!("verdict(\"{r}\"")) || me.contains(&format!("each(\"{r}\"")),
            "{r}: no such rule"
        );
    }
}
