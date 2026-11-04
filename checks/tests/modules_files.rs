// SPDX-License-Identifier: GPL-2.0-or-later
//! What this file checks (wolos files the module packages rely on, ADR 0032): the root helper
//! `modules-install` (installer, firstboot, the ai-local GPU backend), firstboot's drain of the
//! pending file, the user steps (`ai-local`, `modules-enable-ai-claude`), Claude Code's managed
//! settings (written by the wolos-module-ai-claude package, not wolos-system), the WolOS agent skill, and the console configs under desktop/overlay (zsh, starship,
//! fish, nushell, bash, PowerShell, ghostty, zellij, mise). The scripts run in a sandbox with fake
//! pacman, arch-chroot, wol, claude and systemd-run on PATH.
#![allow(dead_code, unused_imports)]
#[path = "support/siblings.rs"]
mod siblings;

use std::{
    fs,
    os::unix::fs::PermissionsExt,
    path::{Path, PathBuf},
    process::{Command, Output},
};

fn repo(p: &str) -> PathBuf {
    siblings::root().join(p)
}

fn read(p: &str) -> String {
    fs::read_to_string(repo(p)).unwrap_or_else(|e| panic!("{p}: {e}"))
}

/// A fresh directory with `bin/` holding the given fake programs (`name`, sh body).
fn sandbox(name: &str, fakes: &[(&str, &str)]) -> PathBuf {
    let d = std::env::temp_dir().join(format!("wolos-mod-{name}-{}", std::process::id()));
    let _ = fs::remove_dir_all(&d);
    fs::create_dir_all(d.join("bin")).unwrap();
    for (n, body) in fakes {
        let p = d.join("bin").join(n);
        fs::write(&p, format!("#!/bin/sh\n{body}\n")).unwrap();
        fs::set_permissions(&p, fs::Permissions::from_mode(0o755)).unwrap();
    }
    d
}

fn run(script: &str, d: &Path, env: &[(&str, String)], args: &[&str]) -> Output {
    let mut c = Command::new("sh");
    c.arg(repo(script))
        .args(args)
        .env_clear()
        .env("PATH", format!("{}:/usr/bin:/bin", d.join("bin").display()))
        .env("HOME", d.join("home"))
        .env("LOG", d.join("log"));
    for (k, v) in env {
        c.env(k, v);
    }
    c.output().unwrap()
}

fn log(d: &Path) -> String {
    fs::read_to_string(d.join("log")).unwrap_or_default()
}

fn header_ok(p: &str) {
    let t = read(p);
    assert!(
        t.starts_with("#!/bin/sh\n# SPDX-License-Identifier: GPL-2.0-or-later\n"),
        "{p}"
    );
    assert!(t.contains("\nset -eu\n"), "{p}: set -eu");
    let o = Command::new("sh").arg("-n").arg(repo(p)).output().unwrap();
    assert!(
        o.status.success(),
        "{p}: {}",
        String::from_utf8_lossy(&o.stderr)
    );
}

/// The fake pacman's first lines: logs its arguments, a `--dbpath DIR` shown as `--dbpath DB` (DIR
/// is a fresh temp one), and records in `$LOG.db` whether DIR is the temporary copy modules-install
/// makes under $TMPDIR: `local` a link to the slot's, `sync` a real copy.
const PACMAN_DB_CHECK: &str = r#"if [ "$1" = --dbpath ]; then
  case $2 in "$TMPDIR"/wolos-modules-db.*) ;; *) echo "dbpath outside TMPDIR: $2" >> "$LOG.db" ;; esac
  if [ "$(readlink "$2/local")" = "$WOLOS_PACMAN_DB/local" ] && [ ! -L "$2/sync" ] && [ -f "$2/sync/wolos.db" ]; then
    echo copy >> "$LOG.db"
  else
    echo "not a copy: $2" >> "$LOG.db"
  fi
  shift 2; echo "pacman --dbpath DB $*" >> "$LOG"
else
  echo "pacman $*" >> "$LOG"
fi
"#;

/// What the fake pacman found in the --dbpath directories it was given, one line per call.
fn db_checks(d: &Path) -> String {
    fs::read_to_string(d.join("log.db")).unwrap_or_default()
}

/// Nothing (no temporary database) is left behind in $TMPDIR.
fn tmp_clean(d: &Path) -> bool {
    fs::read_dir(d.join("tmp")).unwrap().next().is_none()
}

/// A fake `ip` for `ip -4|-6 route show default`: a route on -4 only, or none at all.
const IP_ONLINE: &str = "case \"$*\" in *-4*) echo 'default via 10.0.2.2 dev eth0' ;; esac";
const IP_OFFLINE: &str = "exit 0";

/// A registry with three records and the fakes modules-install calls.
fn modules_box(name: &str, pacman_fails_for: &str) -> PathBuf {
    let d = sandbox(
        name,
        &[
            (
                "pacman",
                &format!(
                    "{PACMAN_DB_CHECK}case \" $* \" in *' {pacman_fails_for} '*) exit 1 ;; esac\nexit 0"
                ),
            ),
            (
                "arch-chroot",
                "echo \"chroot $1\" >> \"$LOG\"; shift; exec \"$@\"",
            ),
            (
                "wol",
                "echo \"wol $*\" >> \"$LOG\"; echo '{\"backend\": \"cuda\", \"vram_mb\": 8192, \"gpu\": \"x\"}'",
            ),
            ("logger", "exit 0"),
            // A default route: the machine is online unless a test says otherwise (audit I6-3).
            ("ip", IP_ONLINE),
        ],
    );
    // The slot's package database (read-only on a sealed slot): `local` and `sync`.
    fs::create_dir_all(d.join("db/local")).unwrap();
    fs::create_dir_all(d.join("db/sync")).unwrap();
    fs::write(d.join("db/sync/wolos.db"), "lists").unwrap();
    fs::create_dir_all(d.join("tmp")).unwrap();
    let reg = d.join("reg");
    fs::create_dir_all(&reg).unwrap();
    for id in ["dev", "ai-local", "creative"] {
        let rec = if id == "ai-local" {
            r#"{"backend": "ollama"}"#
        } else {
            "{}"
        };
        fs::write(reg.join(format!("{id}.json")), rec).unwrap();
    }
    d
}

fn menv(d: &Path) -> Vec<(&'static str, String)> {
    vec![
        ("WOL_MODULES_DIR", d.join("reg").display().to_string()),
        (
            "WOL_MODULES_PENDING",
            d.join("state/modules-pending").display().to_string(),
        ),
        (
            "WOL_PACKAGES_PENDING",
            d.join("state/packages-pending").display().to_string(),
        ),
        (
            "WOL_MODULES_ATTEMPTS",
            d.join("state/modules-attempts").display().to_string(),
        ),
        (
            "WOL_MODULES_GAVE_UP",
            d.join("state/modules-gave-up").display().to_string(),
        ),
        ("WOLOS_PACMAN_DB", d.join("db").display().to_string()),
        ("TMPDIR", d.join("tmp").display().to_string()),
        ("WOLOS_MODULES_TIMEOUT", "30".into()),
    ]
}

fn pending(d: &Path) -> Option<String> {
    fs::read_to_string(d.join("state/modules-pending")).ok()
}

fn pkg_pending(d: &Path) -> Option<String> {
    fs::read_to_string(d.join("state/packages-pending")).ok()
}

#[test]
fn modules_install_installs_queues_and_drains() {
    header_ok("files/usr/lib/wolos/modules-install");
    let s = "files/usr/lib/wolos/modules-install";
    // Bad and unknown ids are refused before anything runs.
    let d = modules_box("bad", "-");
    for bad in ["../x", "Dev", "gaming", "-x"] {
        let o = run(s, &d, &menv(&d), &[bad]);
        assert_eq!(
            o.status.code(),
            Some(2),
            "{bad}: {}",
            String::from_utf8_lossy(&o.stderr)
        );
    }
    assert_eq!(log(&d), "", "nothing ran");
    // Installed: download bounded, against a temporary copy of the database (the slot's is on the
    // read-only seal, audit5 C9-1), then the plain install through the wrapper; nothing queued.
    // Both refresh the repo lists (-y): a system installed offline has none, an older one names
    // files the mirror no longer has (pre-release offline-modules run).
    let o = run(s, &d, &menv(&d), &["dev"]);
    assert!(o.status.success(), "{}", String::from_utf8_lossy(&o.stderr));
    assert_eq!(
        log(&d),
        "pacman --dbpath DB -Syw --noconfirm --needed wolos-module-dev\npacman -Sy --noconfirm --needed wolos-module-dev\n"
    );
    assert_eq!(db_checks(&d), "copy\n");
    assert!(tmp_clean(&d), "the temporary database is removed");
    assert_eq!(pending(&d), None);
    // ai-local takes the backend `wol hw compute` names, in the same transaction, as root.
    let d = modules_box("local", "-");
    assert!(run(s, &d, &menv(&d), &["ai-local"]).status.success());
    assert!(
        log(&d).contains("pacman -Sy --noconfirm --needed wolos-module-ai-local ollama-cuda\n"),
        "{}",
        log(&d)
    );
    // A failing module is queued (exit 1), the others install; --root goes through arch-chroot.
    let d = modules_box("fail", "wolos-module-creative");
    let root = d.join("target");
    fs::create_dir_all(&root).unwrap();
    let o = run(
        s,
        &d,
        &menv(&d),
        &["--root", root.to_str().unwrap(), "dev", "creative"],
    );
    assert_eq!(o.status.code(), Some(1));
    assert!(
        log(&d).contains(&format!("chroot {}", root.display())),
        "{}",
        log(&d)
    );
    // Into another root the chroot's own database is writable: no temporary copy there.
    assert!(!log(&d).contains("--dbpath"), "{}", log(&d));
    // install-run refreshed that root itself when it was online: no -y there.
    assert!(log(&d).contains("pacman -Sw --noconfirm --needed wolos-module-dev\n"));
    assert_eq!(pending(&d).as_deref(), Some("creative\n"));
    // Drain: reads the pending file (bad lines dropped), installs, removes what installed.
    fs::write(
        d.join("state/modules-pending"),
        "creative\n../evil\ndev\ncreative\n",
    )
    .unwrap();
    fs::write(
        d.join("bin/pacman"),
        "#!/bin/sh\necho \"pacman $*\" >> \"$LOG\"\nexit 0\n",
    )
    .unwrap();
    let o = run(s, &d, &menv(&d), &["--drain"]);
    assert!(o.status.success(), "{}", String::from_utf8_lossy(&o.stderr));
    assert_eq!(pending(&d), None, "drained");
    assert!(!log(&d).contains("evil"));
    assert!(tmp_clean(&d));
    // A media format (--package) that does not install is queued in its own file, packages-pending;
    // the module still installs. A drain installs it and removes the file.
    let d = modules_box("pkg", "flac");
    let o = run(s, &d, &menv(&d), &["--package", "flac", "dev"]);
    assert_eq!(o.status.code(), Some(1));
    assert_eq!(pending(&d), None);
    assert_eq!(pkg_pending(&d).as_deref(), Some("flac\n"));
    assert!(log(&d).contains("pacman -Sy --noconfirm --needed wolos-module-dev\n"));
    assert!(tmp_clean(&d));
    for bad in ["-x", "a b", "../x"] {
        let o = run(s, &d, &menv(&d), &["--package", bad]);
        assert_eq!(o.status.code(), Some(2), "{bad}");
    }
    fs::write(
        d.join("state/packages-pending"),
        "flac\n--noconfirm\nflac\n",
    )
    .unwrap();
    fs::write(
        d.join("bin/pacman"),
        format!("#!/bin/sh\n{PACMAN_DB_CHECK}exit 0\n"),
    )
    .unwrap();
    fs::remove_file(d.join("log")).unwrap();
    let o = run(s, &d, &menv(&d), &["--drain"]);
    assert!(o.status.success(), "{}", String::from_utf8_lossy(&o.stderr));
    assert_eq!(pkg_pending(&d), None, "drained");
    assert_eq!(
        log(&d),
        "pacman --dbpath DB -Syw --noconfirm --needed flac\npacman -Sy --noconfirm --needed flac\n",
        "each name once, the option line dropped"
    );
    // --queue (offline installer): nothing runs, the ids and packages are queued once.
    let d = modules_box("queue", "-");
    fs::create_dir_all(d.join("state")).unwrap();
    fs::write(d.join("state/modules-pending"), "dev\n").unwrap();
    let o = run(
        s,
        &d,
        &menv(&d),
        &["--queue", "--package", "flac", "dev", "ai-local"],
    );
    assert_eq!(o.status.code(), Some(1));
    assert_eq!(log(&d), "");
    assert_eq!(pending(&d).as_deref(), Some("dev\nai-local\n"));
    assert_eq!(pkg_pending(&d).as_deref(), Some("flac\n"));
    // Queued is not failed: no try is counted.
    assert!(!d.join("state/modules-attempts").exists());
    // No ids, nothing pending: nothing to do.
    let d = modules_box("none", "-");
    assert!(run(s, &d, &menv(&d), &["--drain"]).status.success());
}

/// audit6 I6-3: offline, a failed drain is no try. A laptop installed offline and used offline
/// for five boots keeps its modules queued; the tries start when a route exists.
#[test]
fn offline_drains_are_not_tries_online_ones_are() {
    let s = "files/usr/lib/wolos/modules-install";
    let d = modules_box("offline", "wolos-module-creative");
    fs::create_dir_all(d.join("state")).unwrap();
    fs::write(d.join("bin/ip"), format!("#!/bin/sh\n{IP_OFFLINE}\n")).unwrap();
    fs::write(d.join("state/modules-pending"), "creative\n").unwrap();
    fs::write(d.join("state/packages-pending"), "flac\n").unwrap();
    fs::write(
        d.join("bin/pacman"),
        "#!/bin/sh\necho \"pacman $*\" >> \"$LOG\"\nexit 1\n",
    )
    .unwrap();
    // Six offline drains (the limit is five): every one fails, nothing is counted or given up.
    for n in 1..=6 {
        let o = run(s, &d, &menv(&d), &["--drain"]);
        assert_eq!(o.status.code(), Some(1), "drain {n}");
        assert_eq!(pending(&d).as_deref(), Some("creative\n"), "drain {n}");
        assert_eq!(pkg_pending(&d).as_deref(), Some("flac\n"), "drain {n}");
        assert!(
            !d.join("state/modules-attempts").exists(),
            "drain {n} counted a try offline"
        );
        assert!(!d.join("state/modules-gave-up").exists(), "drain {n}");
    }
    assert!(
        String::from_utf8_lossy(&run(s, &d, &menv(&d), &["--drain"]).stderr).contains("no network"),
        "says why nothing was counted"
    );
    // Tries counted before stay as they are, offline.
    fs::write(d.join("state/modules-attempts"), "m creative 4\np flac 2\n").unwrap();
    run(s, &d, &menv(&d), &["--drain"]);
    assert_eq!(
        fs::read_to_string(d.join("state/modules-attempts")).unwrap(),
        "m creative 4\np flac 2\n"
    );
    // The route comes up: the same failure now counts, and the fifth try gives up.
    fs::write(d.join("bin/ip"), format!("#!/bin/sh\n{IP_ONLINE}\n")).unwrap();
    run(s, &d, &menv(&d), &["--drain"]);
    let gave = fs::read_to_string(d.join("state/modules-gave-up")).unwrap_or_default();
    assert_eq!(gave, "m creative\n", "the fifth counted try gives up");
    assert_eq!(pending(&d), None);
    assert_eq!(pkg_pending(&d).as_deref(), Some("flac\n"));
    assert_eq!(
        fs::read_to_string(d.join("state/modules-attempts")).unwrap_or_default(),
        "p flac 3\n",
        "flac had 2 tries, now 3"
    );
    // A v6-only route counts as online too.
    let d = modules_box("v6", "wolos-module-creative");
    fs::create_dir_all(d.join("state")).unwrap();
    fs::write(
        d.join("bin/ip"),
        "#!/bin/sh\ncase \"$*\" in *-6*) echo 'default via fe80::1 dev wlan0' ;; esac\n",
    )
    .unwrap();
    fs::write(d.join("state/modules-pending"), "creative\n").unwrap();
    let o = run(s, &d, &menv(&d), &["--drain"]);
    assert_eq!(o.status.code(), Some(1));
    assert!(
        fs::read_to_string(d.join("state/modules-attempts"))
            .unwrap()
            .contains("m creative 1\n")
    );
}

#[test]
fn firstboot_hands_queued_modules_to_their_own_unit() {
    header_ok_firstboot();
    let d = sandbox(
        "firstboot",
        &[
            ("wol", "echo \"wol $*\" >> \"$LOG\""),
            ("systemd-run", "echo \"systemd-run $*\" >> \"$LOG\""),
            ("logger", "exit 0"),
        ],
    );
    let st = d.join("state");
    fs::create_dir_all(&st).unwrap();
    let env = [
        ("WOLOS_STATE", st.display().to_string()),
        ("WOLOS_LIB", "/usr/lib/wolos".to_string()),
        (
            "WOL_MODULES_PENDING",
            st.join("modules-pending").display().to_string(),
        ),
        (
            "WOL_PACKAGES_PENDING",
            st.join("packages-pending").display().to_string(),
        ),
        ("TMPDIR", d.display().to_string()),
    ];
    // Nothing queued: drivers done, firstboot-done written.
    assert!(
        run("files/usr/lib/wolos/firstboot", &d, &env, &[])
            .status
            .success()
    );
    assert!(st.join("firstboot-done").exists());
    assert!(!log(&d).contains("systemd-run"));
    // Queued modules: started in the background in their own unit. The drivers are in, so firstboot
    // is done: the queue is drained on every connection and gives up on its own (audit C8-7).
    fs::remove_file(st.join("firstboot-done")).unwrap();
    fs::write(st.join("modules-pending"), "creative\n").unwrap();
    assert!(
        run("files/usr/lib/wolos/firstboot", &d, &env, &[])
            .status
            .success()
    );
    let l = log(&d);
    assert!(
        l.contains("systemd-run --no-block --unit=wolos-modules-pending")
            && l.contains("/usr/lib/wolos/modules-install --drain"),
        "{l}"
    );
    // ST6-2: a failed transient unit stays loaded and its name stays taken, so the drain unit is
    // collected whatever its end, and "something stays queued" (exit 1) is not a failure.
    assert!(
        l.contains(" --collect ") && l.contains("--property=SuccessExitStatus=1"),
        "{l}"
    );
    assert!(
        st.join("firstboot-done").exists(),
        "the queue does not hold firstboot"
    );
    // Only media formats queued (packages-pending): the same unit starts.
    fs::remove_file(st.join("firstboot-done")).unwrap();
    fs::remove_file(st.join("modules-pending")).unwrap();
    fs::write(st.join("packages-pending"), "flac\n").unwrap();
    fs::remove_file(d.join("log")).unwrap();
    assert!(
        run("files/usr/lib/wolos/firstboot", &d, &env, &[])
            .status
            .success()
    );
    let l = log(&d);
    assert!(
        l.contains("systemd-run --no-block --unit=wolos-modules-pending")
            && l.contains("/usr/lib/wolos/modules-install --drain"),
        "{l}"
    );
    // ST6-2: a failed transient unit stays loaded and its name stays taken, so the drain unit is
    // collected whatever its end, and "something stays queued" (exit 1) is not a failure.
    assert!(
        l.contains(" --collect ") && l.contains("--property=SuccessExitStatus=1"),
        "{l}"
    );
    assert!(st.join("firstboot-done").exists());
    // The drivers did not install (offline): the queue still starts, firstboot runs again next boot.
    fs::remove_file(st.join("firstboot-done")).unwrap();
    fs::remove_file(d.join("log")).unwrap();
    fs::write(
        d.join("bin/wol"),
        "#!/bin/sh\necho \"wol $*\" >> \"$LOG\"\n[ \"$2\" != boot ]\n",
    )
    .unwrap();
    assert!(
        run("files/usr/lib/wolos/firstboot", &d, &env, &[])
            .status
            .success()
    );
    let l = log(&d);
    assert!(
        l.contains("wol hw boot --online") && l.contains("modules-install --drain"),
        "{l}"
    );
    assert!(
        !st.join("firstboot-done").exists(),
        "drivers not in: not done"
    );
}

fn header_ok_firstboot() {
    let t = read("files/usr/lib/wolos/firstboot");
    // firstboot is `set -u` by design: every step may fail and is retried (its header says so).
    assert!(t.starts_with("#!/bin/sh\n# SPDX-License-Identifier: GPL-2.0-or-later\n"));
    assert!(t.contains("modules-pending") && t.contains("modules-install"));
}

#[test]
fn local_ai_user_step_never_installs() {
    header_ok("files/usr/lib/wolos/ai-local");
    let d = sandbox(
        "ailocal",
        &[
            (
                "wol",
                "echo \"wol $*\" >> \"$LOG\"; echo '{\"backend\": \"rocm\", \"vram_mb\": 1, \"gpu\": \"x\"}'",
            ),
            ("pacman", "echo \"pacman $*\" >> \"$LOG\"; exit 1"),
        ],
    );
    let o = run("files/usr/lib/wolos/ai-local", &d, &[], &[]);
    assert!(o.status.success(), "a missing backend is a note");
    assert!(String::from_utf8_lossy(&o.stderr).contains("ollama-rocm"));
    assert_eq!(log(&d), "wol hw compute --json\npacman -Qq ollama-rocm\n");
}

#[test]
fn claude_step_registers_wol_mcp_and_links_the_skill_once() {
    header_ok("files/usr/lib/wolos/modules-enable-ai-claude");
    let d = sandbox(
        "claude",
        &[(
            "claude",
            "echo \"claude $*\" >> \"$LOG\"; [ \"$2\" != get ] || [ -f \"$HOME/.mcp\" ] || exit 1; [ \"$2\" != add ] || touch \"$HOME/.mcp\"",
        )],
    );
    let home = d.join("home");
    fs::create_dir_all(home.join(".agents/skills/wolos")).unwrap(); // the person's own: kept
    let env = [(
        "WOLOS_AGENT_SKILL",
        "/usr/share/wolos/agents/skills/wolos".to_string(),
    )];
    for _ in 0..2 {
        let o = run(
            "files/usr/lib/wolos/modules-enable-ai-claude",
            &d,
            &env,
            &[],
        );
        assert!(o.status.success(), "{}", String::from_utf8_lossy(&o.stderr));
    }
    assert_eq!(
        log(&d),
        "claude mcp get wolos\nclaude mcp add --scope user wolos -- wol mcp\nclaude mcp get wolos\n"
    );
    assert_eq!(
        fs::read_link(home.join(".claude/skills/wolos")).unwrap(),
        Path::new("/usr/share/wolos/agents/skills/wolos")
    );
    assert!(home.join(".agents/skills/wolos").is_dir());
    assert!(fs::read_link(home.join(".agents/skills/wolos")).is_err());
}

/// Runs a PKGBUILD's package() into a fresh pkgdir (as makepkg does, no root) and returns the pkgdir.
fn packaged(dir: &str, tag: &str) -> PathBuf {
    let pkgdir = std::env::temp_dir().join(format!("wolos-pkg-{dir}-{tag}-{}", std::process::id()));
    let _ = fs::remove_dir_all(&pkgdir);
    fs::create_dir_all(&pkgdir).unwrap();
    let out = Command::new("bash")
        .current_dir(repo(dir))
        .arg("-c")
        .arg("set -eu; source ./PKGBUILD; package")
        .env("pkgdir", &pkgdir)
        .env("srcdir", pkgdir.join("src-unused"))
        .output()
        .unwrap();
    assert!(
        out.status.success(),
        "{dir} package(): {}",
        String::from_utf8_lossy(&out.stderr)
    );
    pkgdir
}

/// ADR 0032: the module ships Claude Code's system-wide policy, wolos-system does not (a machine
/// without the module has no Claude Code managed policy at all).
#[test]
fn claude_managed_settings_ship_with_the_module_only() {
    let pkgdir = packaged("wolos-module-ai-claude", "only");
    let f = pkgdir.join("etc/claude-code/managed-settings.json");
    assert!(f.is_file(), "the module installs {}", f.display());
    assert_eq!(
        fs::metadata(&f).unwrap().permissions().mode() & 0o777,
        0o644
    );
    let files: Vec<PathBuf> = {
        let mut v = Vec::new();
        let mut stack = vec![pkgdir.clone()];
        while let Some(d) = stack.pop() {
            for e in fs::read_dir(&d).unwrap().flatten() {
                if e.path().is_dir() {
                    stack.push(e.path());
                } else {
                    v.push(e.path());
                }
            }
        }
        v
    };
    assert_eq!(files, std::slice::from_ref(&f), "the module's only file");
    let pkg = "wolos-module-ai-claude";
    let backup = Command::new("bash")
        .current_dir(repo(pkg))
        .arg("-c")
        .arg("source ./PKGBUILD; printf '%s\\n' \"${backup[@]}\"")
        .output()
        .unwrap();
    assert_eq!(
        String::from_utf8_lossy(&backup.stdout).trim(),
        "etc/claude-code/managed-settings.json",
        "a local edit survives an upgrade"
    );
    // wolos-system copies the wolos files/ tree and desktop/overlay whole: neither holds it, and
    // its backup= no longer names it.
    for tree in ["files", "desktop/overlay"] {
        assert!(
            !repo(&format!("{tree}/etc/claude-code")).exists(),
            "{tree}/etc/claude-code would ship with wolos-system on every machine"
        );
    }
    assert!(!read("wolos-system/PKGBUILD").contains("claude-code"));
    let _ = fs::remove_dir_all(&pkgdir);
}

#[test]
fn claude_managed_settings_add_wolos_without_locking_other_servers() {
    let pkgdir = packaged("wolos-module-ai-claude", "policy");
    let path = pkgdir.join("etc/claude-code/managed-settings.json");
    let t = fs::read_to_string(&path).unwrap();
    // WolOS's own gate asks for every wol action, so Claude does not ask twice.
    assert!(t.contains("\"mcp__wolos\""));
    // Root goes through wol-admind only: the agent never runs the escalation tools itself.
    for d in ["sudo", "su", "pkexec", "run0", "doas"] {
        assert!(t.contains(&format!("\"Bash({d}:*)\"")), "deny {d}");
    }
    // Not exclusive (ADR 0032): no allow-list of servers, no managed server set.
    for no in ["allowedMcpServers", "mcpServers", "deniedMcpServers"] {
        assert!(!t.contains(no), "{no}");
    }
    assert!(t.contains("\"autoUpdatesChannel\": \"stable\""));
    // Well-formed JSON object (std-only: balanced, quotes even).
    assert_eq!(t.matches('{').count(), t.matches('}').count());
    assert_eq!(t.matches('[').count(), t.matches(']').count());
    assert_eq!(t.matches('"').count() % 2, 0);
    let o = Command::new("python3")
        .args(["-c", "import json,sys; json.load(open(sys.argv[1]))"])
        .arg(&path)
        .output();
    if let Ok(o) = o {
        assert!(o.status.success(), "{}", String::from_utf8_lossy(&o.stderr));
    }
    let _ = fs::remove_dir_all(&pkgdir);
}

#[test]
fn the_skill_names_only_verbs_that_exist() {
    let s = read("files/usr/share/wolos/agents/skills/wolos/SKILL.md");
    assert!(s.starts_with("---\nname: wolos\ndescription: "));
    let fm_end = s[4..].find("\n---\n").expect("front matter closes");
    assert!(fm_end < 1100, "short front matter");
    for want in [
        "wol mcp",
        "wol api --json",
        "wol plugin check",
        "wol plugin new",
        "plugin.toml",
    ] {
        assert!(s.contains(want), "{want}");
    }
    let cli = read("crates/controller/src/cli.rs");
    // Every `wol <a> <b>` and every table cell verb is a command row in the CLI.
    let mut checked = 0;
    for cell in s.split('`').skip(1).step_by(2) {
        let words: Vec<&str> = cell
            .trim_start_matches("wol ")
            .split_whitespace()
            .take_while(|w| {
                w.starts_with(|c: char| c.is_ascii_lowercase())
                    && w.chars().all(|c| c.is_ascii_lowercase() || c == '-')
            })
            .collect();
        let first = cell.split_whitespace().next().unwrap_or("");
        let known = [
            "about",
            "sys",
            "update",
            "theme",
            "appearance",
            "accent",
            "wallpaper",
            "audio",
            "display",
            "battery",
            "network",
            "wifi",
            "bluetooth",
            "hw",
            "focus",
            "notify",
            "remind",
            "windows",
            "webapp",
            "open",
            "files",
            "history",
            "undo",
            "restore",
            "modules",
            "plugin",
            "sessions",
            "api",
            "mcp",
        ];
        if !(cell.starts_with("wol ") || known.contains(&first)) || words.is_empty() {
            continue;
        }
        let path = words
            .iter()
            .map(|w| format!("\"{w}\""))
            .collect::<Vec<_>>()
            .join(", ");
        // `wifi on|off|join` stops at the `|`: the group's own row (`&["wifi"]`) is checked.
        if words[0] == "sessions" {
            continue; // the verbs land with the CLI wiring (crates/controller/src/sessions.rs)
        }
        assert!(
            cli.contains(&format!("&[{path}]")),
            "SKILL.md names `{cell}`, cli.rs has no &[{path}]"
        );
        checked += 1;
    }
    assert!(checked > 20, "{checked} verbs checked");
}

#[test]
fn console_configs_are_wired_and_themed_from_the_toolkits() {
    let tk = "wolos/toolkits/";
    let z = read("desktop/overlay/usr/share/wolos/zsh/zshrc");
    for want in [
        "(( $+commands[starship] ))",
        "starship/palette.toml",
        "/usr/share/wolos/starship/starship.toml",
        "WOLOS_PROMPT:-starship} != pure",
        "/usr/share/fzf/key-bindings.zsh",
        "zoxide init zsh",
    ] {
        assert!(z.contains(want), "zshrc lacks {want}");
    }
    let st = read("desktop/overlay/usr/share/wolos/starship/starship.toml");
    assert!(st.contains("palette = \"wolos\"") && st.contains("[palettes.wolos]"));
    for k in ["accent", "muted", "ok", "error", "warn"] {
        assert!(st.contains(&format!("\n{k} = ")), "palette key {k}");
    }
    assert!(
        st.contains("ssh_only = true"),
        "user@host only over SSH, like the zsh prompt"
    );
    let fish = read("desktop/overlay/usr/share/wolos/fish/config.fish");
    assert!(fish.contains(&format!("{tk}fish/colors.fish")) && fish.contains("starship init fish"));
    assert!(
        read("desktop/overlay/etc/fish/conf.d/wolos.fish")
            .contains("/usr/share/wolos/fish/config.fish")
    );
    let nu = read("desktop/overlay/usr/share/wolos/nu/autoload/wolos.nu");
    assert!(nu.contains(&format!("{tk}nu/colors.nu")) && nu.contains("max_size = 50000"));
    let prof = read("desktop/overlay/etc/profile.d/wolos-console.sh");
    assert!(prof.contains("NU_VENDOR_AUTOLOAD_DIR:-/usr/share/wolos/nu/autoload"));
    let bash = read("desktop/overlay/usr/share/wolos/bash/bashrc");
    assert!(bash.contains(&format!("{tk}bash/colors.sh")) && bash.contains("HISTSIZE=50000"));
    let ps = read("desktop/overlay/usr/share/wolos/pwsh/profile.ps1");
    assert!(
        ps.contains("wolos/toolkits/pwsh/colors.ps1") && ps.contains("-MaximumHistoryCount 50000")
    );
    let gh = read("desktop/overlay/usr/share/wolos/ghostty/config");
    assert!(gh.contains("config-file = ?~/.config/wolos/toolkits/ghostty/theme"));
    let zj = read("desktop/overlay/etc/zellij/config.kdl");
    assert!(
        zj.contains("session_serialization true"),
        "named sessions survive a reboot"
    );
    let mise = read("desktop/overlay/etc/mise/config.toml");
    for want in [
        "lockfile = true",
        "registry_floating = false",
        "disable_backends = [\"asdf\"]",
        "paranoid = [\"plugins\", \"provenance\"]",
        "trusted_config_paths = []",
    ] {
        assert!(mise.contains(want), "mise config lacks {want}");
    }
    for (sh, f) in [
        ("bash", "desktop/overlay/usr/share/wolos/bash/bashrc"),
        ("sh", "desktop/overlay/etc/profile.d/wolos-console.sh"),
    ] {
        let o = Command::new(sh).arg("-n").arg(repo(f)).output().unwrap();
        assert!(
            o.status.success(),
            "{f}: {}",
            String::from_utf8_lossy(&o.stderr)
        );
    }
}

#[test]
fn bash_setup_runs_with_the_theme_colours_and_no_starship() {
    let d = sandbox("bashrc", &[]);
    let home = d.join("home");
    let colors = home.join(".config/wolos/toolkits/bash/colors.sh");
    fs::create_dir_all(colors.parent().unwrap()).unwrap();
    // The shipped file is the generator's render of the default theme (core theme/console.rs
    // `system_copy_is_the_default_render`), so its names are the ones `wol theme` writes; only the
    // accent changes, to tell the user file from the shipped one.
    let shipped = read("desktop/overlay/usr/share/wolos/toolkits/bash/colors.sh");
    assert!(
        shipped.contains("WOLOS_ANSI_ACCENT='38;2;110;78;233'"),
        "{shipped}"
    );
    fs::write(
        &colors,
        shipped.replace("38;2;110;78;233", "38;2;124;92;255"),
    )
    .unwrap();
    let probe = format!(
        ". {}; _wolos_ps1; printf '%s\\n' \"H=$HISTSIZE\" \"P=$PS1\"",
        repo("desktop/overlay/usr/share/wolos/bash/bashrc").display()
    );
    let o = Command::new("bash")
        .args(["--norc", "--noprofile", "-i", "-c", &probe])
        .env_clear()
        .env("HOME", &home)
        .env("PATH", "/usr/bin:/bin")
        .env("WOLOS_PROMPT", "pure")
        .output()
        .unwrap();
    let out = String::from_utf8_lossy(&o.stdout);
    assert!(o.status.success(), "{}", String::from_utf8_lossy(&o.stderr));
    assert!(out.contains("H=50000"), "{out}");
    assert!(
        out.contains("\\e[38;2;124;92;255m"),
        "theme accent in PS1: {out}"
    );
}

fn zsh_bin() -> Option<PathBuf> {
    std::env::var_os("WOLOS_TEST_ZSH")
        .map(PathBuf::from)
        .or_else(|| {
            ["/usr/bin/zsh", "/bin/zsh"]
                .iter()
                .map(PathBuf::from)
                .find(|p| p.exists())
        })
}

/// zsh/fzf.zsh: the pickers exist only with fzf, and what fzf returns lands in the line quoted.
#[test]
fn zsh_fzf_pickers_bind_only_with_fzf_and_quote_what_they_insert() {
    let rc = repo("desktop/overlay/usr/share/wolos/zsh/zshrc");
    let fz = repo("desktop/overlay/usr/share/wolos/zsh/fzf.zsh");
    assert!(read("desktop/overlay/usr/share/wolos/zsh/zshrc").contains("fzf.zsh"));
    let Some(zsh) = zsh_bin() else {
        eprintln!("zsh not on this host: fzf picker run skipped");
        return;
    };
    for f in [&rc, &fz] {
        let o = Command::new(&zsh).arg("-n").arg(f).output().unwrap();
        assert!(o.status.success(), "zsh -n {}", f.display());
    }
    let probe = |with_fzf: bool| {
        let d = sandbox(
            if with_fzf { "zfzf" } else { "znofzf" },
            if with_fzf {
                &[("fzf", "cat >/dev/null; printf '%s\n' 'a b.txt' \"it's\"")][..]
            } else {
                &[][..]
            },
        );
        fs::create_dir_all(d.join("home")).unwrap();
        let script = format!(
            "{}source {}; bindkey '^[^F'; LBUFFER='vim '; _wolos_fzf_files 2>/dev/null; print -r -- \"B=$LBUFFER\"",
            std::env::var("WOLOS_TEST_ZSH_MODULES")
                .map(|m| format!("module_path=({m}); "))
                .unwrap_or_default(),
            rc.display()
        );
        Command::new(&zsh)
            .args(["-f", "-i", "-c", &script])
            .env_clear()
            .env("HOME", d.join("home"))
            .env("TERM", "foot")
            .env("WOLOS_PROMPT", "pure")
            .env("PATH", format!("{}:/usr/bin:/bin", d.join("bin").display()))
            .current_dir(&d)
            .output()
            .unwrap()
    };
    let o = probe(true);
    let out = String::from_utf8_lossy(&o.stdout);
    assert!(out.contains("\"^[^F\" _wolos_fzf_files"), "{out}");
    assert!(out.contains(r"B=vim a\ b.txt it\'s "), "{out}");
    let o = probe(false);
    let out = String::from_utf8_lossy(&o.stdout);
    assert!(
        out.contains("\"^[^F\" undefined-key"),
        "no fzf, no picker: {out}"
    );
}

/// SHELLS-check F2/F8: a command a shipped script calls without a guard comes with the image: the
/// package that holds it is in wolos-system's `depends` or the ISO package lists. `arch-audit` was
/// called by update-check and installed by nothing (the CVE list was always empty).
#[test]
fn commands_the_shipped_scripts_call_are_installed_by_the_image() {
    let system = read("wolos-system/PKGBUILD");
    let iso: String = ["packages.x86_64", "packages.desktop"]
        .iter()
        .map(|f| read(&format!("profile/{f}")))
        .collect();
    let word = |hay: &str, w: &str| {
        hay.lines()
            .filter(|l| !l.trim_start().starts_with('#'))
            .any(|l| {
                l.split(|c: char| c.is_whitespace() || c == '(' || c == ')')
                    .any(|t| t == w)
            })
    };
    for (script, cmd, pkg) in [
        (
            "files/usr/lib/wolos/update-check",
            "arch-audit",
            "arch-audit",
        ),
        (
            "files/usr/lib/wolos/update-check",
            "checkupdates",
            "pacman-contrib",
        ),
        ("files/usr/lib/wolos/verify-gpu.sh", "lspci", "pciutils"),
    ] {
        assert!(read(script).contains(cmd), "{script} no longer calls {cmd}");
        assert!(
            word(&system, pkg) || word(&iso, pkg),
            "{script} calls {cmd}: {pkg} is in neither wolos-system depends nor the ISO lists"
        );
    }
}

/// SHELLS-check F3: ghostty reads no system-wide config, so the WolOS defaults (and through them the
/// generated theme) reach it only if wol-session links them as the user's own, never over a file
/// the user has. The header must not promise a `--config-file` launcher nobody ships.
#[test]
fn wol_session_links_the_ghostty_defaults_without_overwriting() {
    let s = read("desktop/overlay/usr/bin/wol-session");
    let list = s
        .lines()
        .skip_while(|l| !l.starts_with("for f in gtk-4.0/gtk.css"))
        .take(2)
        .collect::<String>();
    assert!(list.contains("ghostty/config"), "{list}");
    assert!(list.contains("kitty/kitty.conf"), "{list}");
    assert!(s.contains("if [ ! -e \"$cfg/$f\" ] && [ ! -L \"$cfg/$f\" ]; then"));
    for f in list
        .trim_start_matches("for f in")
        .split(|c: char| c.is_whitespace() || c == ';' || c == '\\')
        .filter(|w| w.contains('/') && !w.starts_with("do"))
    {
        let p = format!("desktop/overlay/usr/share/wolos/{f}");
        assert!(
            repo(&p).is_file(),
            "wol-session links {f}: {p} is not shipped"
        );
    }
    let gh = read("desktop/overlay/usr/share/wolos/ghostty/config");
    assert!(gh.contains("wol-session"), "the header says who links it");
    assert!(!gh.contains("WolOS starts it"), "no launcher is shipped");
    let ky = read("desktop/overlay/usr/share/wolos/kitty/kitty.conf");
    assert!(ky.contains("wol-session"), "the header says who links it");
    assert!(ky.contains("include /usr/share/wolos/toolkits/kitty/theme.conf"));
    assert!(ky.contains("include ~/.config/wolos/toolkits/kitty/theme.conf"));
    assert!(repo("desktop/overlay/usr/share/wolos/toolkits/kitty/theme.conf").is_file());
}
