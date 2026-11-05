#!/bin/bash
# SPDX-License-Identifier: GPL-2.0-or-later
# userland-matrix.sh: the gate for swapping a GNU tool for a uutils one (README "Userland and GNU").
# Runs every form of find/xargs, diff/cmp, ps/pgrep/pkill/free/uptime and the coreutils that WolOS
# scripts, the updater (crates/core/src/update.rs, jobs.rs, controller login.rs) and the Arch tools
# every system runs (makepkg, mkinitcpio, dkms, update-ca-trust, devtools) call, with the tools found
# in $PATH, and compares each case's stdout (and exit code) with the one GNU printed (golden/).
#
#   userland-matrix.sh check [SECTION...]    compare with golden/; exit 1 if an ours-/third- case differs
#   userland-matrix.sh write DIR [SECTION..] write DIR/SECTION.out (run it on GNU tools: the golden files)
#
# Sections: coreutils findutils diffutils procps (default: all). Case names say who needs them:
# ours-* WolOS code and scripts, third-* Arch packages on the image, extra-* wider compatibility (a
# difference there is a warning). A tool that fails one ours-/third- case is not switched.
# Stdout only: error texts differ between implementations and nothing parses them.
set -u
here=$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)
mode=${1:-check}; shift || :
out=
if [ "$mode" = write ]; then out=${1:?write DIR}; shift; mkdir -p "$out"; fi
sections=("$@"); [ ${#sections[@]} -gt 0 ] || sections=(coreutils findutils diffutils procps)

srt() { LC_ALL=C sort "$@"; }
nn() { sed 's/[0-9][0-9]*/N/g'; }

# case NAME 'CMD': runs CMD (bash) in the section's fixture dir; records stdout.
results=
case_() {
  local n=$1 o
  o=$(cd "$W" && eval "$2" 2>/dev/null </dev/null)
  printf '== %s\n%s\n' "$n" "$o" >> "$results"
}

fixture_common() {
  umask 022; W=$(mktemp -d); results=$W/.results; : > "$results"; me=$(id -u)
}

sec_findutils() {
  fixture_common; cd "$W" || return
  mkdir -p state build/sub esp/loader/entries esp/EFI/wolos top mods/6.1-wolos mods/6.1-arch work/etc/kernel src assets proj/.git proj/checks proj/a/src proj/b tp/d/e tp/empty tp/lnk
  touch state/pacdb-old state/pacdb-key state/other build/a.o build/sub/b.o build/sub/c.txt top/@wolos-a top/@scratch
  ln -s /nonexistent build/broken; ln -s a.o build/good
  touch esp/loader/entries/wolos-b.conf 'esp/loader/entries/wolos-b+3.conf' 'esp/loader/entries/wolos-b+2-1.conf' esp/loader/entries/wolos-b-linux.conf esp/loader/entries/wolos-a.conf
  touch esp/EFI/wolos/wolos-b.efi esp/EFI/wolos/wolos-b-linux.efi esp/EFI/wolos/wolos-a.efi mods/6.1-wolos/vmlinuz mods/6.1-arch/vmlinuz
  printf 'root=x rootflags=subvol=@wolos-a,compress=zstd\n' > work/etc/kernel/cmdline
  echo x > ref; sleep 1; touch newer.pkg.tar.zst build/new.pkg.tar.zst; echo hi > src/main.cr; echo hi > assets/a.png
  touch proj/pacman.conf proj/mirrorlist proj/PKGBUILD proj/b/mirrorlist-x proj/a/src/mirrorlist proj/checks/pacman.conf proj/.git/pacman.conf
  printf '#!/bin/bash\necho x\n' > script.sh
  echo x > tp/f; echo y > tp/d/e/g; ln tp/f tp/hard; chmod 744 tp/f; chmod 600 tp/d/e/g; ln -s f tp/lnk/ok; ln -s gone tp/lnk/dead; ln -s f tp/sl; : > tp/a.ko.zst
  # shellcheck disable=SC2034 # S, N, me: read by the eval'd case commands
  S=srt
  # WolOS: wolos-base/{prebuild,base-manifest}.sh, tools/lint.sh, update.rs, wol-zoom, builder scripts
  case_ ours-mindepth-name-not-exec  'find state -mindepth 1 -maxdepth 1 -name "pacdb-*" ! -name pacdb-key -exec echo {} + ; echo rc=$?'
  case_ ours-printf-P-nl             'find build \( -type f -o -type l \) -printf "%P\n" | $S'
  case_ ours-printf-P-nul            'find build \( -type f -o -type l \) -printf "%P\0" | sort -z | tr "\0" "|"'
  case_ ours-printf-P-l-nul          'find build -type l -printf "%P\0%l\0" | sort -z | tr "\0" "|"'
  case_ ours-L-type-l-printf-delete  'cp -a build b2; find -L b2 -type l -printf "Removing %P\n" -delete; find b2 | $S'
  case_ ours-name-o-delete           'cp -a build b3; find b3 -type f -name "*.o" -delete; find b3 | $S'
  case_ ours-newer                   'find build -name "*.pkg.tar.zst" -newer ref'
  case_ ours-printf-f                'find build -maxdepth 1 -name "*.pkg.tar.zst" -printf "%f\n"'
  case_ ours-printf-f-space          'find build -maxdepth 1 -name "*.o" -printf "%f "'
  case_ ours-printf-s-f              'find build -maxdepth 1 -type f -printf "%s %f\n" | $S'
  case_ ours-prune-user              'find state -prune -user "$(id -u)"; find state -prune -user nosuchuser9; echo rc=$?'
  case_ ours-update-cmdline-sed      'find work/etc -maxdepth 2 -path work/etc/kernel/cmdline -exec sed -i -E "s#subvol=/?@wolos-[ab]([ ,]|\$)#subvol=@wolos-b\1#" {} + ; cat work/etc/kernel/cmdline'
  case_ ours-update-L-cp-t           'mkdir -p stage; find -L build -maxdepth 1 -type f -name "*.pkg.tar.zst" -exec cp --reflink=auto -t stage -- {} + ; find stage | $S'
  case_ ours-update-exec-semicolon   'find top -maxdepth 1 -name "@wolos-a" -exec false {} \; ; echo rc=$?'
  case_ ours-update-exec-plus-fails  'find top -maxdepth 1 -name "@wolos-a" -exec false {} + ; echo rc=$?'
  case_ ours-update-exec-nomatch     'find top -maxdepth 1 -name "@nope" -exec false {} + ; echo rc=$?'
  case_ ours-update-entries-delete   'cp -a esp e2; find e2/loader/entries -maxdepth 1 \( -name "wolos-b.conf" -o -name "wolos-b+*.conf" -o -name "wolos-b-*.conf" \) -delete; find e2/loader/entries | $S'
  case_ ours-update-efi-delete       'cp -a esp e3; find e3/EFI/wolos -maxdepth 1 \( -name "wolos-b.efi" -o -name "wolos-b-*.efi" \) -delete; find e3/EFI/wolos | $S'
  case_ ours-scx-vmlinuz             'find mods -mindepth 2 -maxdepth 2 -name vmlinuz -path "*wolos*"'
  case_ ours-lint-prune-print0       'find proj \( -path proj/.git -o -path proj/checks -o -path "proj/*/src" -o -path "proj/*/pkg" \) -prune -o -type f \( -name pacman.conf -o -name "*mirrorlist*" \) ! -name PKGBUILD -print0 | sort -z | tr "\0" "|"'
  case_ ours-exec-grep-lZ-xargs      'find . -maxdepth 1 -type f -exec grep -lZE "^#!/(usr/)?bin/(env )?(sh|bash)" {} + | xargs -0 -r echo'
  case_ ours-exec-sha256-plus        'find build -type f -exec sha256sum {} + | sort -k2'
  case_ ours-newer-two-dirs          'find src assets -type f -newer ref'
  case_ ours-not-path                'find . -name "*.cr" -not -path "*/target/*" -not -path "*/lib/*"'
  case_ ours-delete-not-name         'cp -a build b4; find b4 -maxdepth 1 ! -name "*.rcgu.o" -type f -delete; find b4 | $S'
  case_ ours-type-f-wc               'find build -type f | wc -l'
  case_ ours-mindepth-printf-f       'find build -mindepth 1 -maxdepth 1 -printf "%f\n" | $S'
  case_ ours-not-path-git            'find proj -type f -not -path "*/.git/*" | $S'
  case_ ours-xargs-0-r-empty         ': | xargs -0 -r echo ran; echo rc=$?'
  case_ ours-xargs-0-r-args          'printf "a\0b\0" | xargs -0 -r echo'
  case_ ours-xargs-0-sha256-dd       'printf "ref\0newer.pkg.tar.zst\0" | xargs -0 sha256sum -- | sort -k2'
  case_ ours-xargs-basename          'echo /a/b/cc | xargs basename'
  case_ ours-xargs-0-r-grep          'printf "src/main.cr\0" | xargs -0 -r grep -lI -m1 -E "^hi" --'
  # Arch packages on every system: makepkg tidy/*, mkinitcpio, dkms, update-ca-trust, devtools
  case_ third-execdir-touch-plus     'cp -a tp t2; find t2 -mindepth 1 -execdir touch -hcd @0 {} + ; echo rc=$?; stat -c "%n %Y" t2/f t2/d | $S'
  case_ third-depth-empty-or-xtype   'cp -a tp t3; find t3 -depth \( -type d -empty -delete \) -or \( -xtype l -delete \); find t3 | $S'
  case_ third-xtype-f                'find tp -xtype f | $S'
  case_ third-xtype-l                'find tp -xtype l | $S'
  case_ third-perm-u-plus-x          'find tp -type f -perm -u+x'
  case_ third-perm-links-plus1       'find tp -type f -perm -u+w -links +1 -print0 | sort -z | tr "\0" "|"'
  case_ third-links-1                'find tp -type f -perm -u+w -links 1 -print0 | sort -z | tr "\0" "|"'
  case_ third-links-exec-stat        'find tp -type f -links +1 -exec stat -c "%i %n" {} + | cut -d" " -f2 | $S'
  case_ third-follow                 'find tp/lnk -maxdepth 1 -type f -follow'
  case_ third-L-type-l-delete        'cp -a tp t4; find -L t4/lnk -maxdepth 1 -type l -delete; find t4/lnk | $S'
  case_ third-maxdepth0-empty-exec   'find tp/empty -maxdepth 0 -empty -exec echo empty \;'
  case_ third-prune-printf-l         'find tp/sl -prune -printf "%l"'
  case_ third-not-type-d-name        'find tp ! -type d -name "*.ko*"'
  case_ third-type-d-empty-delete    'cp -a tp t5; find t5 -type d -empty -delete; find t5 | $S'
  case_ third-exec-plus-zstd-style   'find tp -type f -name "*.ko.zst" -exec echo {} +'
  case_ third-regex                  'find tp -regex ".*\.ko\.zst"'
  case_ third-mtime                  'find tp -type f -mtime -1 | wc -l'
  case_ third-newermt                'find tp -type f -newermt 2000-01-01 | wc -l'
  case_ third-samefile               'find tp -samefile tp/f | $S'
  case_ third-quit                   'find tp -name f -quit; echo rc=$?'
  case_ third-print0-sort-z          'find tp -name "*.ko*" -print0 | sort -z | tr "\0" "|"'
  case_ third-xargs-a-file           'printf "a\nb\n" > af; xargs -a af echo'
  case_ third-xargs-r-long           'printf "" | xargs --no-run-if-empty echo ran; echo rc=$?'
  # wider compatibility: a warning, not a block
  case_ extra-ignore-readdir-race    'find tp -noleaf -ignore_readdir_race -maxdepth 0'
  case_ extra-xargs-t-trace          'printf "a\n" | xargs -t echo 2>&1'
  case_ extra-printf-formats         'find tp -maxdepth 1 -name f -printf "%p|%h|%f|%d|%y|%m|%s|%n|%l|%P|%H\n"'
  case_ extra-regextype              'find tp -regextype posix-extended -regex ".*/(f|g)" | $S'
  case_ extra-files0-from            'printf "tp/f\0tp/d\0" > list0; find -files0-from list0 -maxdepth 0'
  case_ extra-xargs-P                'printf "a\nb\n" | xargs -P2 -n1 echo | $S'
  case_ extra-xargs-I                'printf "a\nb\n" | xargs -I{} echo x{}'
  case_ extra-xargs-d                'printf "a,b,c" | xargs -d, echo'
  case_ extra-xargs-E                'printf "a\nSTOP\nb\n" | xargs -E STOP echo'
  case_ extra-xargs-exit-255         'printf "a\n" | xargs sh -c "exit 255"; echo rc=$?'
  case_ extra-xargs-exit-fail        'printf "a\n" | xargs false; echo rc=$?'
}

sec_diffutils() {
  fixture_common; cd "$W" || return
  printf 'a\nb\nc\nd\ne\n' > a; printf 'a\nB\nc\nd\nf\ng\n' > b; cp a a2; printf 'a\nb\nc\nd\ne' > nonl
  mkdir -p da db; echo 1 > da/x; echo 2 > db/x; echo only > da/only; echo 3 > db/new; printf ' a  b\n' > w1; printf 'a b\n' > w2
  # WolOS: wolos-build tests (diff -u); Arch packages: devtools checkpkg/diffpkg/pkgctl, dkms, libtool, autoconf
  case_ ours-diff-u-differs          'diff -u a b | tail -n +3; echo rc=${PIPESTATUS[0]}'
  case_ ours-diff-u-same             'diff -u a a2; echo rc=$?'
  case_ ours-cmp-s-same              'cmp -s a a2; echo rc=$?'
  case_ ours-cmp-s-differ            'cmp -s a b; echo rc=$?'
  case_ ours-cmp-s-missing           'cmp -s a nosuch; echo rc=$?'
  case_ third-diff-u-label           'diff -u --label one --label two a b; echo rc=$?'
  case_ third-diff-up                'diff -up a b | tail -n +3; echo rc=${PIPESTATUS[0]}'
  case_ third-diff-c                 'diff -c a b | tail -n +3; echo rc=${PIPESTATUS[0]}'
  case_ third-diff-Nupr              'diff -Nupr da db | grep -v "^[-+][-+][-+]"; echo rc=${PIPESTATUS[0]}'
  case_ third-diff-rq                'diff -rq da db; echo rc=$?'
  case_ third-diff-w                 'diff -w w1 w2; echo rc=$?'
  case_ third-diff-suppress-sbs      'diff --side-by-side --suppress-common-lines --width=40 a b; echo rc=$?'
  case_ third-diff-color             'diff --color=always -u a b | tail -n +3 | cat -v; echo rc=${PIPESTATUS[0]}'
  case_ third-diff-stdin             'cat a | diff -u - b | tail -n +3; echo rc=${PIPESTATUS[1]}'
  case_ third-diff-process-subst     'diff -u <(echo a) <(echo b) | tail -n +3; echo rc=${PIPESTATUS[0]}'
  case_ third-diff-no-newline        'diff a nonl; echo rc=$?'
  case_ third-diff-version           'diff --version | head -1 | cut -c1-4'
  case_ extra-diff-normal            'diff a b; echo rc=$?'
  case_ extra-diff-ed                'diff -e a b | tr "\n" "|"; echo rc=${PIPESTATUS[0]}'
  case_ extra-diff-i                 'printf "A\n" > ca; printf "a\n" > cb; diff -i ca cb; echo rc=$?'
  case_ extra-diff-B                 'printf "a\n\nb\n" > bl1; printf "a\nb\n" > bl2; diff -B bl1 bl2; echo rc=$?'
  case_ extra-diff-binary            'printf "x\0y" > bin1; printf "x\0z" > bin2; diff bin1 bin2; echo rc=$?'
  case_ extra-cmp-plain              'cmp a b; echo rc=$?'
  case_ extra-cmp-l                  'cmp -l a b; echo rc=$?'
  case_ extra-cmp-n                  'cmp -n 3 a b; echo rc=$?'
  case_ extra-cmp-stdin              'cat a | cmp -s - a2; echo rc=${PIPESTATUS[1]}'
}

sec_procps() {
  fixture_common; cd "$W" || return
  sleep 300 </dev/null >/dev/null 2>&1 & S1=$!
  (exec -a wolos-probe sleep 301) </dev/null >/dev/null 2>&1 & S2=$!
  sleep 0.5
  N=nn
  # WolOS: crates/core jobs.rs PS_ARGS, controller login.rs (pkill -U/-f), hw.rs (pgrep -x), theme/apps.rs
  # (pkill --signal --uid --exact), journeys (pgrep -u -x -f -c -o -n -a, ps -o ... -C), iso tests (free -m)
  case_ ours-ps-ax-pid-ppid          "ps -ax -o pid= -o ppid= | awk -v p=$S1 -v me=$$ '\$1==p{print \"found\", (\$2==me)}'"
  case_ ours-ps-o-C-header           'ps -o pid,ppid,etimes,args -C sleep | head -1 | tr -s " " | sed "s/ *\$//"'
  case_ ours-ps-comm                 "ps -o comm= -p $S1 | tr -d ' '"
  case_ ours-pgrep-x                 'pgrep -x sleep | grep -c . | $N; pgrep -x nosuchproc9; echo rc=$?'
  case_ ours-pgrep-x-u               'pgrep -u $me -x sleep | head -1 | $N'
  case_ ours-pgrep-f                 'pgrep -f "sleep 300" | grep -c . | $N'
  case_ ours-pgrep-c                 'pgrep -c -x sleep | $N'
  case_ ours-pgrep-o-n               'test "$(pgrep -ox sleep)" -le "$(pgrep -nx sleep)" && echo oldest-le-newest'
  case_ ours-pgrep-af                'pgrep -af "sleep 300" | head -1 | sed "s/^[0-9]*/N/"'
  case_ ours-pgrep-u-name            'pgrep -u $(id -nu) -x sleep | head -1 | $N'
  case_ ours-pgrep-u-only            'pgrep -u $me | grep -c . | $N'
  case_ ours-pgrep-bracket-trick     'pgrep -f "[s]leep 30" | head -1 | $N'
  case_ ours-pkill-x-nomatch         'pkill -x nosuchproc9; echo rc=$?'
  case_ ours-pkill-signal-uid-exact  'sleep 400 </dev/null >/dev/null 2>&1 & P=$!; pkill --signal TERM --uid $me --exact "sleep|footclient"; echo rc=$?; wait $P; echo done'
  case_ ours-pkill-U-f               'sleep 401 </dev/null >/dev/null 2>&1 & P=$!; sleep 0.3; pkill -U $me -f "sleep 401"; echo rc=$?; wait $P; echo killed'
  case_ ours-pkill-f                 'sleep 403 </dev/null >/dev/null 2>&1 & P=$!; sleep 0.3; pkill -f "sleep 403"; echo rc=$?; wait $P; echo killed'
  case_ ours-pkill-KILL-u-f          'sleep 404 </dev/null >/dev/null 2>&1 & P=$!; sleep 0.3; pkill -KILL -u $me -f "sleep 404"; echo rc=$?; wait $P; echo killed'
  case_ ours-free-m                  'free -m | awk "NR==2{print (\$2>0)}"'
  case_ extra-uptime-p               'uptime -p | cut -c1-2'
  case_ extra-top-b                  'top -b -n1 | head -1 | cut -c1-6'
  case_ extra-pgrep-l                'pgrep -l -x sleep | head -1 | sed "s/^[0-9]*/N/"'
  case_ extra-pidof                  'pidof sleep | wc -w | $N'
  case_ extra-sysctl-n               'sysctl -n kernel.ostype'
  case_ extra-watch-version          'watch --version | head -1 | cut -c1-5'
  kill $S1 $S2 2>/dev/null
}

sec_coreutils() {
  fixture_common; cd "$W" || return
  mkdir -p d/sub; printf 'one\ntwo\nthree\n' > d/f; printf 'b\na\nc\n' > d/s; ln -s f d/l; printf '{"version":"1.2"}\n' > j
  # WolOS: update.rs (cp --reflink=auto -t, mv -T, rm -rf, mkdir -p), wolos-base PKGBUILD (install -Dm644 /dev/stdin,
  # ln -s, sed|install), base-manifest.sh (sha256sum, readlink, sort -z), scripts under files/usr/lib/wolos
  case_ ours-cp-reflink-t            'mkdir -p dst; cp --reflink=auto -t dst -- d/f; cat dst/f | wc -l'
  case_ ours-cp-a-reflink            'cp -a --reflink=auto d d2; ls -lA d2 | tail -n +2 | awk "{print \$1, \$9, \$10, \$11}"'
  case_ ours-cp-L                    'cp -L d/l lcopy; cat lcopy | wc -l'
  case_ ours-install-Dm644-stdin     'echo hi | install -Dm644 /dev/stdin out/a/b/c; stat -c "%a" out/a/b/c; cat out/a/b/c'
  case_ ours-install-d               'install -d -m755 idir/x/y; stat -c "%a" idir/x/y'
  case_ ours-install-Dm755-file      'install -Dm755 d/f out2/bin/f; stat -c "%a" out2/bin/f'
  case_ ours-ln-s                    'ln -s uu-ls lsl; readlink lsl'
  case_ ours-ln-sfn                  'ln -s a lk; ln -sfn b lk; readlink lk'
  case_ ours-mv-T                    'mkdir m1 m2; touch m1/x; mv -T m1 m3; ls m3; touch mvf; mv -f mvf mvg; ls mv* | tr "\n" " "'
  case_ ours-rm-rf-missing           'rm -rf nosuch/a/b; echo rc=$?; rm -f nosuch; echo rc=$?'
  case_ ours-mkdir-p-mode            'mkdir -p -m 700 mp/a/b; stat -c "%a" mp/a/b'
  case_ ours-mktemp-d                't=$(mktemp -d); test -d $t && echo dir; rmdir $t; t=$(mktemp "${TMPDIR:-/tmp}/wolos.XXXXXX"); test -f $t && echo file; rm $t'
  case_ ours-readlink-f-m            'readlink -f d/l | sed "s|^$PWD/||"; readlink -m d/nosuch/x | sed "s|^$PWD/||"; readlink d/l'
  case_ ours-realpath                'realpath d/l | sed "s|^$PWD/||"; realpath -m d/x/../y | sed "s|^$PWD/||"; realpath --relative-to=d d/sub'
  case_ ours-sha256sum               'sha256sum d/s; sha256sum < d/s; sha256sum -- d/s; sha256sum d/s | sha256sum -c -'
  case_ ours-sha512-md5-b2           'md5sum d/s; sha512sum d/s | cut -c1-16; b2sum d/s | cut -c1-16'
  case_ ours-sort-z                  'printf "b\0a\0c\0" | sort -z | tr "\0" "|"'
  case_ ours-sort-k-V-u              'printf "10 b\n9 a\n9 a\n" | sort -k1,1 -V -u; printf "10\n9\n" | sort -n; printf "b\na\n" | LC_ALL=C sort -r'
  case_ ours-stat-c                  'stat -c "%a %F %s %n" d/f; stat -c %Y ref 2>/dev/null; stat -L -c %F d/l; stat -c "%u:%g" d/f | sed "s/[0-9]*/N/g"'
  case_ ours-touch-d-h               'touch -hcd @0 d/l; stat -c %Y d/f | sed "s/[0-9]*/N/"; touch -d @0 d/f; stat -c %Y d/f; touch -c nosuchf; echo rc=$?'
  case_ ours-chmod                   'chmod 440 d/f; stat -c %a d/f; chmod u+x,g-r d/f; stat -c %a d/f; chmod -R go-rwx d; stat -c %a d/sub'
  case_ ours-chown-self              'chown $(id -u):$(id -g) d/s; echo rc=$?; chown -h --reference=d/s d/l; echo rc=$?'
  case_ ours-truncate                'truncate -s 4096 tr1; stat -c %s tr1; truncate -s 0 tr1; stat -c %s tr1'
  case_ ours-dd                      'dd if=/dev/zero of=dd1 bs=1k count=4 status=none; stat -c %s dd1; dd if=d/s bs=1 count=3 skip=1 status=none | od -An -c | tr -s " "; dd if=d/s of=dd2 conv=notrunc,fsync status=none; echo rc=$?'
  case_ ours-du                      'du -sb d/s | cut -f1; du -s --apparent-size d/s | cut -f1; du -h d/s | cut -f1 | tr -d " ."; du -sk d | cut -f1 | sed "s/[0-9]*/N/"'
  case_ ours-df                      'df --output=avail -B1 . | tail -1 | sed "s/[0-9]*/N/"; df -P . | head -1 | cut -c1-10; df --output=source,fstype . | head -1 | tr -s " "'
  case_ ours-date                    'date -u -d @0 +%Y-%m-%dT%H:%M:%SZ; date +%s | sed "s/[0-9]*/N/"; date -u -d "2026-11-04 12:00:00" +%s; date -u --iso-8601=seconds -d @86400'
  case_ ours-head-tail               'head -c 5 d/s | od -An -c | tr -s " "; head -n -1 d/s | tr "\n" ,; tail -n +2 d/s | tr "\n" ,; tail -c 2 d/s | od -An -c | tr -s " "; head -n1 d/s'
  case_ ours-cut-tr-wc               'echo a:b:c | cut -d: -f2,3; echo a:b | tr -d :; echo "A b" | tr "[:upper:]" "[:lower:]"; wc -l < d/s | tr -d " "; wc -c d/s | cut -d" " -f1'
  case_ ours-tee-a                   'echo 1 | tee tee1 >/dev/null; echo 2 | tee -a tee1 >/dev/null; wc -l < tee1 | tr -d " "'
  case_ ours-timeout                 'timeout 1 true; echo rc=$?; timeout 0.2 sleep 5; echo rc=$?; timeout -k 0.1 0.2 sleep 5; echo rc=$?; timeout -s INT 0.2 sleep 5; echo rc=$?'
  case_ extra-timeout-kill-status    'timeout -s KILL 0.2 sleep 5; echo rc=$?; timeout -s INT -k 1 0.2 sleep 5; echo rc=$?'
  case_ ours-env                     'env -i A=1 B=2 sh -c "echo \$A\$B"; env -u HOME sh -c "echo \${HOME:-unset}"; FOO=x env | grep -c "^FOO="'
  case_ ours-id-whoami               'id -u | sed "s/[0-9]*/N/"; id -nu | sed "s/./x/g" | head -c1; id -G | wc -w | sed "s/[0-9]*/N/"; test "$(id -un)" = "$(whoami)" && echo same'
  case_ ours-basename-dirname        'basename /a/b/c.txt .txt; basename -s .txt /a/b/c.txt; dirname /a/b/c; basename -a /x/y /z/w | tr "\n" " "'
  case_ ours-seq-yes-printf          'seq 3 | tr "\n" ,; seq 2 2 8 | tr "\n" ,; seq -w 08 10 | tr "\n" ,; yes ab | head -2 | tr "\n" ,; printf "%s-%05d-%x\n" a 42 255'
  case_ ours-echo-n-e                'echo -n a; echo -e "b\tc"; echo -- x'
  case_ ours-test-exprs              'test -e d/f && echo e; test -L d/l && echo L; test d/f -nt d/s || echo notnewer; test -s d/f && echo s; [ -n "" ] || echo empty; [ 3 -gt 2 ] && echo gt'
  case_ ours-expr                    'expr 2 + 3; expr abc : "a\(.*\)"; expr length hello; expr 5 "<" 3; echo rc=$?'
  case_ ours-nproc-numfmt            'nproc | sed "s/[0-9]*/N/"; numfmt --to=iec 2048; numfmt --from=iec 1K'
  case_ ours-od-base64               'od -An -tx1 -N4 d/s | tr -s " "; echo hello | base64; echo aGVsbG8K | base64 -d; echo hi | base32; echo hi | basenc --base64url'
  case_ ours-uniq-comm-join-paste    'printf "a\na\nb\n" | uniq -c | tr -s " "; comm -12 d/s d/s | wc -l | tr -d " "; printf "a 1\n" > j1; printf "a 2\n" > j2; join j1 j2; paste -d, j1 j2'
  case_ ours-ls                      'ls -1A d | tr "\n" ,; ls -ld d | cut -c1-10; ls -l --time-style=+%s d/f | awk "{print \$6>0}"; ls -1 --color=never d | head -1; ls -lh d/f | awk "{print \$5}"; ls -d d/*/ ; ls -F d | tr "\n" ,'
  case_ ours-sleep-sync-nice         'sleep 0.1; echo rc=$?; sync; echo rc=$?; nice -n 5 sh -c "nice" | tr -d "\n"; echo; nohup true >/dev/null 2>&1; echo rc=$?'
  case_ ours-stdbuf-split-shuf       'echo x | stdbuf -oL cat; printf "1\n2\n3\n4\n" | split -l2 - sp_; ls sp_* | tr "\n" ,; seq 3 | shuf | sort | tr "\n" ,; seq 3 | tac | tr "\n" ,'
  case_ ours-uname-hostid-tty        'uname -s; uname -m | sed "s/./x/g"; uname -r | cut -c1-1 | sed "s/[0-9]/N/"; tty >/dev/null 2>&1; echo rc=$?'
  case_ ours-cat-fmt-fold-nl         'printf "a\nb\n" | cat -n | tr -s " \t" " "; printf "a b c d e f g\n" | fold -w 6 -s | tr "\n" ,; printf "x\ny\n" | nl -ba | tr -s " \t" " "; printf "aaa bbb ccc\n" | fmt -w 8 | tr "\n" ,'
  case_ ours-shred-unlink-link       'echo x > sh1; shred -u sh1; test -e sh1 || echo gone; echo x > ul; unlink ul; test -e ul || echo gone2; echo y > lk1; link lk1 lk2; stat -c %h lk2'
  case_ ours-mkfifo-mknod            'mkfifo ff; test -p ff && echo fifo; rm ff; mknod nn p; test -p nn && echo fifo2'
  case_ ours-who-users-groups        'who >/dev/null; echo rc=$?; users >/dev/null; echo rc=$?; groups | wc -w | sed "s/[0-9]*/N/"'
  case_ ours-factor-pr-ptx-tsort     'factor 12; printf "a b\nb c\n" | tsort | tr "\n" ,; printf "x\n" | expand -t4 | od -An -c | tr -s " "; printf "a\tb\n" | unexpand -a | od -An -c | tr -s " "'
  case_ ours-printenv-pwd-logname    'FOO=bar printenv FOO; pwd -P | sed "s|.*/||" | sed "s/./x/g" | head -c1; echo; hostid >/dev/null; echo rc=$?'
  case_ ours-version-help            'cat --version | head -1 | cut -c1-3; ls --help | grep -c -e --all | nn'
}

run_section() {
  local s=$1
  "sec_$s"
  local res=$results dir=$W
  cp "$res" "$tmp/$s.out"
  rm -rf "$dir"
}

tmp=$(mktemp -d); trap 'rm -rf "$tmp"' EXIT
for s in "${sections[@]}"; do
  declare -F "sec_$s" >/dev/null || { echo "userland-matrix: no section $s" >&2; exit 2; }
  ( run_section "$s" ) ; cd "$here" || exit 1
  if [ "$mode" = write ]; then cp "$tmp/$s.out" "$out/$s.out"; echo "wrote $out/$s.out ($(grep -c '^== ' "$tmp/$s.out") cases)"; continue; fi
  gold=$here/userland-matrix/$s.out
  [ -f "$gold" ] || { echo "userland-matrix: $gold missing" >&2; exit 2; }
  # one case = "== name" then its lines up to the next "== "
  declare -A G=() W2=()
  parse() { # parse FILE ARRAY
    local f=$1 n= line; local -n nameref=$2
    while IFS= read -r line || [ -n "$line" ]; do
      if [[ $line == '== '* ]]; then n=${line#== }; nameref[$n]=; else nameref[$n]+="$line"$'\n'; fi
    done < "$f"
  }
  parse "$gold" G; parse "$tmp/$s.out" W2
  for n in "${!G[@]}"; do
    if [ "${G[$n]}" = "${W2[$n]-MISSING}" ]; then echo "ok   $s/$n"
    else
      case $n in extra-*) echo "warn $s/$n" ;; *) echo "FAIL $s/$n" ;; esac
    fi
  done | sort > "$tmp/$s.res"
  cat "$tmp/$s.res"
  unset G W2
done
[ "$mode" = write ] && exit 0
n_fail=$(cat "$tmp"/*.res 2>/dev/null | grep -c '^FAIL ') ; n_warn=$(cat "$tmp"/*.res | grep -c '^warn ') ; n_ok=$(cat "$tmp"/*.res | grep -c '^ok ')
echo "userland-matrix: $n_ok ok, $n_warn warn (extra-*), $n_fail FAIL (ours-/third-)"
[ "$n_fail" -eq 0 ]
