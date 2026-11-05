// SPDX-License-Identifier: GPL-2.0-or-later
//! What this file checks (wolos-pkgbuilds, atlas-browser, AT-12): the recipe is a repack of our
//! signed artifact that replaces the previous browser package and keeps the paths the WolOS
//! browser theme writes; `tools/lint.sh --only atlas` rejects the unsafe defaults of L-07; and the
//! launcher `atlas.sh` does what the plan says (L-01..L-06), run in a scratch HOME with a stub
//! engine: defaults before flags files before the command line, Wayland only in a Wayland session,
//! VA-API only with a render node, the profile of the previous package moved once and whole (never
//! over an existing one, never under a running process), default-browser entries and the flags
//! file following, the keyring left alone.
//!
//! Needs sh, sed, mv, readlink and bash; skipped with a note without them.
use std::{
    fs,
    os::unix::fs::symlink,
    path::{Path, PathBuf},
    process::{Command, Stdio},
    sync::atomic::{AtomicUsize, Ordering},
};

fn repo() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .unwrap()
        .to_path_buf()
}

fn tools() -> bool {
    ["sh", "bash", "sed", "mv"]
        .iter()
        .all(|t| Command::new(t).arg("--version").output().is_ok())
}

fn scratch(tag: &str) -> PathBuf {
    static N: AtomicUsize = AtomicUsize::new(0);
    let t = std::env::temp_dir().join(format!(
        "wolos-atlas-{tag}-{}-{}",
        std::process::id(),
        N.fetch_add(1, Ordering::SeqCst)
    ));
    let _ = fs::remove_dir_all(&t);
    fs::create_dir_all(&t).unwrap();
    t
}

fn recipe() -> String {
    fs::read_to_string(repo().join("atlas-browser/PKGBUILD")).expect("atlas-browser/PKGBUILD")
}

// ---------------------------------------------------------------- the recipe

#[test]
fn recipe_replaces_the_previous_browser_and_keeps_the_theme_paths() {
    let r = recipe();
    for want in [
        "pkgbase=atlas-browser",
        "pkgname=(atlas-browser atlas-browser-locales)",
        "provides=(web-browser)",
        "replaces=(helium-browser)",
        "conflicts=(helium-browser)",
        "install -Dm755 atlas.sh \"$pkgdir/usr/bin/atlas\"",
        "ln -s /usr/share/atlas/initial_preferences \"$pkgdir/opt/atlas/initial_preferences\"",
        "install -Dm644 atlas.desktop \"$pkgdir/usr/share/applications/atlas.desktop\"",
    ]
    .iter()
    {
        assert!(r.contains(want), "PKGBUILD lacks: {want}");
    }
    // QA-A9-IMAGE-07: the artifact is the lane's, published on our own CDN, and the sha256 in the
    // recipe pins it byte for byte. Nothing in the pipeline signs an Atlas artifact, so the recipe
    // has no signature source and no validpgpkeys (it required an .asc nobody could produce).
    assert!(r.contains("url='https://github.com/wonderlandos/atlas'"));
    assert!(
        r.contains(
            "source=(\"https://cdn.wolos.org/atlas/${pkgver}/atlas-${pkgver}-x86_64_linux.tar.xz\""
        ),
        "the tarball comes from cdn.wolos.org/atlas/<pkgver>/ (the lane's upload)"
    );
    let code: Vec<&str> = r
        .lines()
        .filter(|l| !l.trim_start().starts_with('#'))
        .collect();
    for l in &code {
        assert!(
            !l.contains(".asc") && !l.contains(".sig") && !l.starts_with("validpgpkeys"),
            "a signature source or validpgpkeys with no signer: {l}"
        );
    }
    assert!(
        !code.iter().any(|l| l.trim() == "SKIP"),
        "every source is pinned by sha256, none is SKIP"
    );
    assert!(
        !repo().join("atlas-browser/keys").exists(),
        "no key of a signer that does not exist"
    );
    // The previous browser's names are gone from what ships.
    for f in ["atlas.sh", "atlas.desktop"] {
        let t = fs::read_to_string(repo().join("atlas-browser").join(f)).unwrap();
        assert!(
            !t.contains("/opt/helium") && !t.contains("/usr/bin/helium"),
            "{f}"
        );
    }
}

#[test]
fn main_package_has_thirteen_locale_packs_and_no_chromedriver() {
    let r = recipe();
    let line = r
        .lines()
        .find(|l| l.starts_with("_main_locales=("))
        .expect("_main_locales");
    let packs: Vec<&str> = line
        .trim_start_matches("_main_locales=(")
        .trim_end_matches(')')
        .split_whitespace()
        .collect();
    assert_eq!(packs.len(), 13, "{packs:?}");
    // The 12 WolOS languages (wolos core::region::LANGUAGES) plus the two Chromium splits.
    for want in [
        "en-US", "en-GB", "pt-BR", "pt-PT", "es", "es-419", "fr", "de", "it", "nl", "pl", "ja",
        "zh-CN",
    ] {
        assert!(packs.contains(&want), "{want} missing from {packs:?}");
    }
    // chromedriver is only ever removed; the lint rule says the same.
    for l in r
        .lines()
        .filter(|l| !l.trim_start().starts_with('#') && l.contains("chromedriver"))
    {
        assert!(l.contains("rm -f"), "chromedriver outside rm: {l}");
    }
    // The locales package holds what the main one does not, and needs the main one.
    assert!(r.contains("depends=(\"atlas-browser=${pkgver}-${pkgrel}\")"));
}

#[test]
fn packages_list_builds_the_new_recipe_not_the_old() {
    let p = fs::read_to_string(repo().join("packages")).unwrap();
    let rows: Vec<&str> = p
        .lines()
        .filter(|l| !l.starts_with('#') && !l.trim().is_empty())
        .collect();
    assert!(rows.iter().any(|l| l.starts_with("atlas-browser ")));
    assert!(!rows.iter().any(|l| l.starts_with("helium-browser ")));
    let bits = fs::read_to_string(repo().join("checks/exec-bits.txt")).unwrap();
    assert!(bits.lines().any(|l| l == "atlas-browser/atlas.sh"));
}

// ---------------------------------------------------------------- the lint rule

fn lint_fixture() -> PathBuf {
    let t = scratch("lint");
    fs::create_dir_all(t.join("tools")).unwrap();
    fs::copy(repo().join("tools/lint.sh"), t.join("tools/lint.sh")).unwrap();
    fs::create_dir_all(t.join("atlas-browser")).unwrap();
    for f in ["PKGBUILD", "atlas.sh", "atlas.desktop"] {
        fs::copy(
            repo().join("atlas-browser").join(f),
            t.join("atlas-browser").join(f),
        )
        .unwrap();
    }
    t
}

fn lint_atlas(t: &Path) -> (bool, String) {
    let o = Command::new("bash")
        .args(["tools/lint.sh", "--only", "atlas"])
        .current_dir(t)
        .output()
        .unwrap();
    (
        o.status.success(),
        String::from_utf8_lossy(&o.stderr).into_owned(),
    )
}

#[test]
fn lint_rejects_unsafe_defaults_and_a_signer_that_does_not_exist() {
    if !tools() {
        eprintln!("bash missing: lint rule not run");
        return;
    }
    let t = lint_fixture();
    let (ok, err) = lint_atlas(&t);
    assert!(ok, "{err}");

    let sh = fs::read_to_string(t.join("atlas-browser/atlas.sh")).unwrap();
    for flag in [
        "--no-sandbox",
        "--disable-site-isolation-trials",
        "--disable-web-security",
        "--remote-debugging-port=9222",
    ] {
        fs::write(
            t.join("atlas-browser/atlas.sh"),
            sh.replace(
                "defaults=\"--password-store=gnome-libsecret\"",
                &format!("defaults=\"--password-store=gnome-libsecret {flag}\""),
            ),
        )
        .unwrap();
        let (ok, err) = lint_atlas(&t);
        assert!(
            !ok && err.contains("L-07") && err.contains(flag),
            "{flag}: {err}"
        );
    }
    fs::write(t.join("atlas-browser/atlas.sh"), &sh).unwrap();

    // A flags file anywhere in the repo is a shipped default too.
    fs::create_dir_all(t.join("wolos-system")).unwrap();
    fs::write(t.join("wolos-system/atlas-flags.conf"), "--no-sandbox\n").unwrap();
    let (ok, err) = lint_atlas(&t);
    assert!(
        !ok && err.contains("wolos-system/atlas-flags.conf"),
        "{err}"
    );
    fs::remove_dir_all(t.join("wolos-system")).unwrap();

    // chromedriver may be removed, never installed.
    let r = recipe();
    fs::write(
        t.join("atlas-browser/PKGBUILD"),
        r.replace(
            "  rm -f \"$pkgdir/opt/atlas/chromedriver\"\n",
            "  install -Dm755 \"$_dir/chromedriver\" \"$pkgdir/opt/atlas/chromedriver\"\n",
        ),
    )
    .unwrap();
    let (ok, err) = lint_atlas(&t);
    assert!(!ok && err.contains("chromedriver"), "{err}");
    fs::write(t.join("atlas-browser/PKGBUILD"), &r).unwrap();

    // A signature source or a pinned signing key with no signer behind it is refused (the lane
    // signs nothing: QA-A9-IMAGE-07); a SKIP checksum next to it is how it used to slip through.
    fs::write(
        t.join("atlas-browser/PKGBUILD"),
        r.replace(
            "x86_64_linux.tar.xz\"\n",
            "x86_64_linux.tar.xz\"\n  \"https://cdn.wolos.org/atlas/${pkgver}/atlas-${pkgver}-x86_64_linux.tar.xz.asc\"\n",
        ),
    )
    .unwrap();
    let (ok, err) = lint_atlas(&t);
    assert!(!ok && err.contains("signature source"), "{err}");
    fs::write(
        t.join("atlas-browser/PKGBUILD"),
        format!("{r}validpgpkeys=('C3B6062A2BA6EEABC1DA0F5766C5F2B181374033')\n"),
    )
    .unwrap();
    let (ok, err) = lint_atlas(&t);
    assert!(!ok && err.contains("validpgpkeys"), "{err}");
    fs::write(t.join("atlas-browser/PKGBUILD"), &r).unwrap();
    // A source that is not on the CDN path of the lane is refused too.
    fs::write(
        t.join("atlas-browser/PKGBUILD"),
        r.replace(
            "https://cdn.wolos.org/atlas/",
            "https://github.com/wonderlandos/atlas/releases/download/",
        ),
    )
    .unwrap();
    let (ok, err) = lint_atlas(&t);
    assert!(!ok && err.contains("cdn.wolos.org/atlas"), "{err}");
    fs::write(t.join("atlas-browser/PKGBUILD"), &r).unwrap();

    // The old paths do not satisfy the rule.
    fs::write(
        t.join("atlas-browser/PKGBUILD"),
        r.replace("/usr/bin/atlas", "/usr/bin/helium"),
    )
    .unwrap();
    let (ok, err) = lint_atlas(&t);
    assert!(!ok && err.contains("lacks /usr/bin/atlas"), "{err}");
    let _ = fs::remove_dir_all(&t);
}

// ---------------------------------------------------------------- the launcher

const OLD_ID: &str = "net.imput.helium";

struct Box {
    t: PathBuf,
    script: PathBuf,
}

impl Box {
    /// A scratch root: home/, etc/, dri/, opt/atlas/atlas-wrapper (prints its arguments), and a
    /// copy of atlas.sh with the absolute system paths moved under the scratch root.
    fn new() -> Box {
        let t = scratch("launch");
        for d in ["home", "etc", "dri", "opt/atlas"] {
            fs::create_dir_all(t.join(d)).unwrap();
        }
        let wrapper = t.join("opt/atlas/atlas-wrapper");
        fs::write(&wrapper, "#!/bin/sh\nprintf '%s\\n' \"$@\"\n").unwrap();
        Command::new("chmod")
            .arg("755")
            .arg(&wrapper)
            .status()
            .unwrap();
        let text = fs::read_to_string(repo().join("atlas-browser/atlas.sh")).unwrap();
        for lit in [
            "/opt/atlas/atlas-wrapper",
            "/etc/atlas-flags.conf",
            "/etc/$old_flags",
            "/dev/dri/renderD*",
        ] {
            assert!(
                text.contains(lit),
                "atlas.sh no longer has {lit}: the test must follow"
            );
        }
        let patched = text
            .replace("/opt/atlas/", &format!("{}/opt/atlas/", t.display()))
            .replace("/etc/", &format!("{}/etc/", t.display()))
            .replace("/dev/dri/", &format!("{}/dri/", t.display()));
        let script = t.join("atlas.sh");
        fs::write(&script, patched).unwrap();
        Box { t, script }
    }

    fn home(&self) -> PathBuf {
        self.t.join("home")
    }

    fn write(&self, rel: &str, text: &str) {
        let p = self.t.join(rel);
        fs::create_dir_all(p.parent().unwrap()).unwrap();
        fs::write(p, text).unwrap();
    }

    fn read(&self, rel: &str) -> String {
        fs::read_to_string(self.t.join(rel)).unwrap_or_default()
    }

    /// The arguments the engine got, and the launcher's stderr.
    fn run(&self, wayland: bool, args: &[&str]) -> (Vec<String>, String) {
        let mut c = Command::new("sh");
        c.arg(&self.script).args(args);
        c.env_clear()
            .env("PATH", std::env::var("PATH").unwrap())
            .env("HOME", self.home());
        if wayland {
            c.env("WAYLAND_DISPLAY", "wayland-1");
        }
        let o = c.output().unwrap();
        assert!(o.status.success(), "{}", String::from_utf8_lossy(&o.stderr));
        (
            String::from_utf8_lossy(&o.stdout)
                .lines()
                .map(String::from)
                .collect(),
            String::from_utf8_lossy(&o.stderr).into_owned(),
        )
    }
}

impl Drop for Box {
    fn drop(&mut self) {
        let _ = fs::remove_dir_all(&self.t);
    }
}

#[test]
fn launcher_defaults_then_flags_files_then_the_command_line() {
    if !tools() {
        return;
    }
    let b = Box::new();
    b.write(
        "etc/atlas-flags.conf",
        "# system\n--system-one\n\n--system-two --system-three\n",
    );
    b.write("home/.config/atlas-flags.conf", "--user-one\n");
    let (a, err) = b.run(true, &["--cmd-one", "https://example.org/a b"]);
    assert_eq!(err, "");
    assert_eq!(
        a,
        [
            "--password-store=gnome-libsecret",
            "--ozone-platform=wayland",
            "--system-one",
            "--system-two",
            "--system-three",
            "--user-one",
            "--cmd-one",
            "https://example.org/a b",
        ]
    );
}

#[test]
fn launcher_wayland_hint_and_render_node() {
    if !tools() {
        return;
    }
    let b = Box::new();
    // No Wayland session, no render node: the engine picks, software video.
    let (a, _) = b.run(false, &[]);
    assert_eq!(
        a,
        [
            "--password-store=gnome-libsecret",
            "--ozone-platform-hint=auto"
        ]
    );
    assert!(!a.iter().any(|x| x.contains("--ozone-platform=wayland")));
    // A render node adds VA-API, once.
    b.write("dri/renderD128", "");
    let (a, _) = b.run(true, &[]);
    assert_eq!(
        a,
        [
            "--password-store=gnome-libsecret",
            "--ozone-platform=wayland",
            "--enable-features=AcceleratedVideoDecodeLinuxGL,AcceleratedVideoEncoder",
        ]
    );
    // L-07 on the real output: nothing unsafe in any of the defaults.
    for x in &a {
        for bad in [
            "--no-sandbox",
            "--disable-site-isolation-trials",
            "--disable-web-security",
            "--remote-debugging",
        ] {
            assert!(!x.contains(bad), "{x}");
        }
    }
}

#[test]
fn launcher_moves_the_previous_profile_once_whole() {
    if !tools() {
        return;
    }
    let b = Box::new();
    b.write(
        &format!("home/.config/{OLD_ID}/Default/Preferences"),
        "{\"prefs\":1}",
    );
    b.write(&format!("home/.config/{OLD_ID}/Local State"), "state");
    b.write(
        &format!("home/.cache/{OLD_ID}/Default/Cache/data_0"),
        "cache",
    );
    b.write("home/.local/share/keyrings/login.keyring", "secret bytes");
    b.write(
        "home/.config/mimeapps.list",
        "[Default Applications]\nx-scheme-handler/http=helium.desktop;\ntext/html=helium.desktop;\napplication/pdf=org.gnome.Papers.desktop;\n",
    );
    b.write("home/.config/helium-flags.conf", "--user-old\n");
    let (a, err) = b.run(true, &[]);
    assert_eq!(err, "");
    // The profile and the cache moved whole; nothing is left under the old names.
    assert_eq!(
        b.read("home/.config/atlas/Default/Preferences"),
        "{\"prefs\":1}"
    );
    assert_eq!(b.read("home/.config/atlas/Local State"), "state");
    assert_eq!(b.read("home/.cache/atlas/Default/Cache/data_0"), "cache");
    assert!(!b.t.join(format!("home/.config/{OLD_ID}")).exists());
    assert!(!b.t.join(format!("home/.cache/{OLD_ID}")).exists());
    // Default-browser entries follow, other entries stay.
    assert_eq!(
        b.read("home/.config/mimeapps.list"),
        "[Default Applications]\nx-scheme-handler/http=atlas.desktop;\ntext/html=atlas.desktop;\napplication/pdf=org.gnome.Papers.desktop;\n"
    );
    // The flags file keeps its content under the new name and is read in the same run.
    assert_eq!(b.read("home/.config/atlas-flags.conf"), "--user-old\n");
    assert!(a.contains(&"--user-old".to_string()), "{a:?}");
    // The keyring is never touched.
    assert_eq!(
        b.read("home/.local/share/keyrings/login.keyring"),
        "secret bytes"
    );

    // Second run: nothing to do, nothing changes.
    b.write("home/.config/atlas/Default/Preferences", "{\"prefs\":2}");
    let (_, err) = b.run(true, &[]);
    assert_eq!(err, "");
    assert_eq!(
        b.read("home/.config/atlas/Default/Preferences"),
        "{\"prefs\":2}"
    );
}

#[test]
fn launcher_never_replaces_an_existing_profile() {
    if !tools() {
        return;
    }
    let b = Box::new();
    b.write(&format!("home/.config/{OLD_ID}/Local State"), "old");
    b.write("home/.config/atlas/Local State", "new");
    b.run(true, &[]);
    assert_eq!(b.read("home/.config/atlas/Local State"), "new");
    assert_eq!(b.read(&format!("home/.config/{OLD_ID}/Local State")), "old");
    // A user who already has the new flags file keeps it; the old one stays where it is.
    b.write("home/.config/helium-flags.conf", "--old\n");
    b.write("home/.config/atlas-flags.conf", "--new\n");
    let (a, _) = b.run(true, &[]);
    assert!(
        a.contains(&"--new".to_string()) && !a.contains(&"--old".to_string()),
        "{a:?}"
    );
    assert_eq!(b.read("home/.config/helium-flags.conf"), "--old\n");
}

#[test]
fn launcher_leaves_a_profile_that_a_running_process_holds() {
    if !tools() {
        return;
    }
    let b = Box::new();
    b.write(&format!("home/.config/{OLD_ID}/Local State"), "old");
    let mut live = Command::new("sleep")
        .arg("30")
        .stdout(Stdio::null())
        .spawn()
        .unwrap();
    let lock = b.t.join(format!("home/.config/{OLD_ID}/SingletonLock"));
    symlink(format!("somehost-{}", live.id()), &lock).unwrap();
    b.run(true, &[]);
    assert!(
        b.t.join(format!("home/.config/{OLD_ID}/Local State"))
            .exists()
    );
    assert!(!b.t.join("home/.config/atlas").exists());
    // The process ends: its stale lock no longer holds the profile.
    live.kill().unwrap();
    live.wait().unwrap();
    b.run(true, &[]);
    assert!(b.t.join("home/.config/atlas/Local State").exists());
    assert!(!b.t.join(format!("home/.config/{OLD_ID}")).exists());
}

#[test]
fn launcher_reads_a_system_flags_file_written_for_the_previous_package() {
    if !tools() {
        return;
    }
    let b = Box::new();
    b.write("etc/helium-flags.conf", "--legacy-system\n");
    let (a, err) = b.run(true, &[]);
    assert!(a.contains(&"--legacy-system".to_string()), "{a:?}");
    assert!(err.contains("/etc/atlas-flags.conf"), "{err}");
    // The new file wins as soon as it exists: the old one is ignored.
    b.write("etc/atlas-flags.conf", "--system-new\n");
    let (a, err) = b.run(true, &[]);
    assert!(
        a.contains(&"--system-new".to_string()) && !a.contains(&"--legacy-system".to_string()),
        "{a:?}"
    );
    assert_eq!(err, "");
}
