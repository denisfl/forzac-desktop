# Publishing a release

How a version of Forzac for desktop gets from this repository to readers: a public GitHub repository with a release
per tag, the download links on forzac.truenrth.app, and the AUR. Nothing here is automatic and nothing has been
published yet; each step is run by the maintainer. Dry-run for 0.1.0 on 2026-10-17 (an M1 Mac with Docker): every
local build and checksum command in step 4, and in step 7 `updpkgsums`, `.SRCINFO` and `makepkg -si` of `forzac-bin`
(makepkg takes a source file lying next to the PKGBUILD instead of downloading it, so the URLs did not have to exist).
The GitHub release, the site and the AUR push cannot be tried without publishing; they follow the tools'
documentation.

What a release is made of. The asset names carry no version (decided 2026-10-19), so
`https://github.com/denisfl/forzac-desktop/releases/latest/download/<name>` always gives the newest one and the site's
links never change; the version stays inside (the DMG's volume, the archives' top directory, the package):

| File | For | Made by |
|---|---|---|
| `Forzac-macos-arm64.dmg` | Mac, Apple Silicon, macOS 14+ | `macos/build.sh`, `scripts/package-macos.sh` |
| `forzac-linux-x86_64.tar.gz` | AUR `forzac-bin`, any x86_64 Linux with GTK 4.14+ | `scripts/package-linux.sh` |
| `forzac-linux-aarch64.tar.gz` | AUR `forzac-bin` on ARM, any aarch64 Linux with GTK 4.14+ | `scripts/package-linux.sh` |
| `forzac-linux-arm64.deb` | Debian 13, Raspberry Pi OS 13 (arm64) | `scripts/package-deb.sh` |
| `forzac-desktop-X.Y.Z.tar.gz` | the source (MPL-2.0), the AUR `forzac` package | GitHub's tag archive (`git archive` locally) |
| `SHA256SUMS` | checking any of the above | the release workflow (`sha256sum`) |

The scripts name their files with the version (`Forzac-X.Y.Z-macos-arm64.dmg`, …); the release workflow renames them
and writes `SHA256SUMS` before it drafts the release.

Sizes for 0.1.0: DMG 9.0 MB, archives 6.3 and 6.4 MB, `.deb` 3.7 MB, source 5.0 MB.

## 1. Decided

- **The source is a public GitHub repository, `denisfl/forzac-desktop`, with a release per tag `vX.Y.Z`** (the
  maintainer, 2026-10-17). MPL-2.0 (section 3.2) asks that whoever receives the program in executable form is told how
  to get its source: the repository and the tag's source archive do that, and the binaries carry `LICENSE`.
- **The files are the GitHub release's assets.** The release workflow (`.github/workflows/release.yml`) builds them on
  the tag (free runners for a public repository, macOS included) and drafts the release; the AUR packages already
  point at GitHub (`packaging/aur/`). The site only links to them, so site deploys never touch the downloads.
  Cloudflare would allow the files as site assets too (25 MiB per file; the biggest, the DMG, is 9 MB), but a deploy
  uploads exactly what is in `site/public/` and the binaries would have to live outside the Android repository's git.
- **The download links are on the site's main page**, under "Get it on Google Play" (section 6).

Below, `$REL` is `https://github.com/denisfl/forzac-desktop/releases/download/vX.Y.Z`.

Before the repository goes public, once:

- Look through the history: it becomes public with every commit's author name and e-mail address. Books were never
  committed (`.gitignore`, `FORZAC_CORPUS`), but check for anything personal in docs and notes
  (`git log --all --stat`, `git grep -i` for names, paths, e-mail addresses).
- Create the empty repository `denisfl/forzac-desktop` on GitHub (public, no README or licence: both are here), then
  `git remote add origin git@github.com:denisfl/forzac-desktop.git && git push -u origin main`. CI runs on that push
  (`.github/workflows/ci.yml`); it has to be green before the first tag.
- On GitHub: Settings › General — Issues on, Discussions and Wiki off unless wanted. The default read-only workflow
  token is enough: the release job asks for `contents: write` itself.

## 2. Before the release

The checklist; every item is done on hardware, not in containers (`CLAUDE.md`, `docs/roadmap.md`).

- [ ] Every change planned for the release is archived (`openspec list` shows none of them active); `main` is clean
      and its CI is green: `cargo fmt --all --check`, `cargo clippy --workspace --all-targets -- -D warnings` (GTK
      dev files, e.g. in `ci/debian.Dockerfile`), `cargo test -p forzac-core`, the corpus test
      `FORZAC_CORPUS=~/Books cargo test -p forzac-core --test corpus`.
- [ ] Measurements on the Pi 5 against the budgets (`release-1-0`): build with `PI=raspberry scripts/pi-dev.sh
      book.fb2.zip`, then on the Pi `scripts/pi-headless.sh ~/forzac-dev/bin/forzac book.fb2.zip`; first page,
      missed frames in page turns, idle CPU ticks. On the Mac: start-up and idle as in `macos-app`. Results go into the
      release notes.
- [ ] By hand on the Pi 5 (over `scripts/pi-vnc.sh` or Raspberry Pi Connect) and on the Mac: open an FB2, an
      `.fb2.zip`, an EPUB and an FB3; both page modes; contents, notes, search, settings, the image and table view,
      reopening at the same line; `?` shows the help with the support line.
- [ ] The packages from step 4 installed as a reader would: the `.deb` on the Pi 5 (`sudo apt install
      ./forzac-linux-arm64.deb`, then `xdg-open book.fb2` from the desktop session); both AUR packages in the Arch
      container (`ci/arch-check.sh`, ~10 min under emulation); the DMG on the Mac.
- [ ] Mac, before the first public DMG (moved here from `macos-release`): the DMG downloaded from the draft release on a
      **fresh macOS user account**, opened by following the line under the site's macOS link (the Gatekeeper step), on
      macOS 14 and on a macOS 15 Mac.
- [ ] Release notes (`docs/releases/X.Y.Z.md`, used as the GitHub release's text): what is new, what is not checked
      (Hyprland and x86_64 hardware: built and installed in a container only), known issues.

## 3. Version and tag

The version lives in three places:

```bash
# Cargo.toml: [workspace.package] version = "X.Y.Z"; then the two crates' versions in Cargo.lock (builds use --locked):
cargo update --workspace
# packaging/aur/forzac/PKGBUILD and packaging/aur/forzac-bin/PKGBUILD: pkgver=X.Y.Z, pkgrel=1
# macos/Info.plist may stay as it is: scripts/package-macos.sh writes the version into the built app.
git commit -am "Release X.Y.Z"
git tag -a vX.Y.Z -m "Forzac X.Y.Z"
```

Push the tag: the release workflow builds the two Linux archives (on Ubuntu 24.04, so they run on glibc 2.39 and
newer), the `.deb` (in Debian 13) and the DMG, and drafts a release with them; GitHub adds the tag's source archive
itself.

```bash
git push origin main vX.Y.Z
gh run watch                               # or the Actions tab
```

Then fetch the draft's files for checking (step 4 builds them locally instead when CI is not available). Build and
check from a clean checkout of the tag, not from the working directory: other sessions may have uncommitted work in it.

```bash
git worktree add --detach ../forzac-release vX.Y.Z
cd ../forzac-release
mkdir -p dist
gh release download vX.Y.Z --dir dist     # the draft's assets
```

## 4. Build the files locally (without CI) and the checksums

Only when the workflow cannot run; otherwise skip to the checksums at the end of this step. All commands run in
`../forzac-release`. Docker builds the Linux files in Debian 13 (the base of Raspberry Pi OS 13, so the `.deb` links
its GTK and glibc; the archives then need glibc 2.41: Debian 13, Ubuntu 25.04, Fedora 42, Arch and newer, not Ubuntu
24.04 as the workflow's archives do). Use the `stable` Rust toolchain for macOS (`rust-toolchain.toml`); if Homebrew's
older `cargo` comes first in `PATH`, put `~/.rustup/toolchains/stable-aarch64-apple-darwin/bin` before it.

```bash
V=X.Y.Z
# Source archive (prefix forzac-desktop-X.Y.Z/, as the AUR forzac package expects)
git archive --format=tar.gz --prefix=forzac-desktop-$V/ -o dist/forzac-desktop-$V.tar.gz v$V

# Linux aarch64 archive and the Raspberry Pi OS package (native on an Apple Silicon Mac, ~3 min)
docker build -q -t forzac-debian -f ci/debian.Dockerfile ci
docker run --rm -v "$PWD":/src -v forzac-pi-target:/target -w /src/dist forzac-debian bash -c \
  "cargo build --release --locked -q -p forzac-gtk --manifest-path /src/Cargo.toml \
   && /src/scripts/package-linux.sh forzac-$V-linux-aarch64 && /src/scripts/package-deb.sh $V"

# Linux x86_64 archive (emulated on an Apple Silicon Mac: about 6 minutes on the M1, the image included)
docker build -q --platform linux/amd64 -t forzac-debian-amd64 -f ci/debian.Dockerfile ci
docker run --rm --platform linux/amd64 -v "$PWD":/src -v forzac-amd64-target:/target -w /src/dist forzac-debian-amd64 \
  bash -c "cargo build --release --locked -q -p forzac-gtk --manifest-path /src/Cargo.toml \
   && /src/scripts/package-linux.sh forzac-$V-linux-x86_64"

# Mac: the app signed ad hoc, the DMG and its own .sha256
macos/build.sh
(cd dist && ../scripts/package-macos.sh $V)
```

The release names and the checksums, as the workflow makes them, checked right away:

```bash
cd dist
mv Forzac-$V-macos-arm64.dmg Forzac-macos-arm64.dmg && rm -f Forzac-$V-macos-arm64.dmg.sha256
for a in x86_64 aarch64; do mv forzac-$V-linux-$a.tar.gz forzac-linux-$a.tar.gz; done
mv forzac_${V}_arm64.deb forzac-linux-arm64.deb
shasum -a 256 Forzac-macos-arm64.dmg forzac-linux-*.tar.gz forzac-linux-arm64.deb > SHA256SUMS
shasum -a 256 -c SHA256SUMS
```

`dist/` now holds the release files and `SHA256SUMS` (and, after a local build, `forzac-desktop-$V.tar.gz`). Install
them as in the checklist (step 2) before publishing.

## 5. Publish the release

```bash
# after a local build (step 4) only; the workflow's draft has the files already:
# gh release upload vX.Y.Z dist/*.dmg dist/forzac-linux-*.tar.gz dist/*.deb dist/SHA256SUMS
gh release edit vX.Y.Z --notes-file docs/releases/X.Y.Z.md --draft=false --latest
```

Then check every URL and checksum from outside:

```bash
mkdir -p /tmp/forzac-check && cd /tmp/forzac-check
curl -fsSLO "$REL/SHA256SUMS"
for f in $(awk '{print $2}' SHA256SUMS); do curl -fsSLO "$REL/$f"; done
shasum -a 256 -c SHA256SUMS
```

## 6. The download links on the site

The site (`~/dev/forzac/site`, in the Android repository) is one page, `public/index.html`, in English and Russian (a
`lang` switch, every text in a `data-lang="en"` and a `data-lang="ru"` span). No separate download page (decided
2026-10-19): Google Play, then the desktop links, then the web reader.

**Main page, under "Get it on Google Play".** The hero's `<p class="cta ui">` holds the Google Play pill (`.play`) and
the quiet "or read in your browser" link (`.read`). Below it, a row of two small links with an icon each, introduced by
"Also for computers:" / "Также для компьютеров:": outline pills in `var(--line)` with `var(--muted)` text and the icon
in `currentColor`, the same `Golos Text` 15 px as `.read`, wrapping under each other on a phone. Quieter than the
Google Play pill, which stays the main action for phones.

- **macOS** downloads the disk image straight away:
  `https://github.com/denisfl/forzac-desktop/releases/latest/download/Forzac-macos-arm64.dmg`. Under the row, one line
  in the muted 13 px text: "Apple Silicon, macOS 14+. First open: System Settings → Privacy & Security → Open Anyway" /
  "Apple Silicon, macOS 14+. При первом запуске: Системные настройки → Конфиденциальность и безопасность → Все равно
  открыть" (the app is not notarised; [docs/macos-first-open.md](macos-first-open.md) has the full steps for the
  release notes).
- **Linux** opens the latest release: `https://github.com/denisfl/forzac-desktop/releases/latest`. There is no single
  file for every Linux; the release notes say which to take: the archives (`tar xzf forzac-linux-x86_64.tar.gz`, then
  `forzac-X.Y.Z-linux-x86_64/bin/forzac book.fb2`), the `.deb` for Debian 13 and Raspberry Pi OS
  (`sudo apt install ./forzac-linux-arm64.deb`), and for Arch and Omarchy the AUR (section 7) or, until the AUR
  account exists, the PKGBUILD from this repository:

  ```bash
  git clone --depth 1 https://github.com/denisfl/forzac-desktop && cd forzac-desktop/packaging/aur/forzac-bin && makepkg -si
  ```

  then the default reader command (the packages print it):

  ```bash
  xdg-mime default app.truenrth.forzac.desktop application/x-fictionbook+xml application/x-zip-compressed-fb2 application/x-fb3 application/epub+zip
  ```

  Icons, inline SVG in `currentColor` from Lucide (ISC): `laptop` for macOS (Apple does not allow its logo on
  third-party downloads) and `square-terminal` for Linux, one stroke style.

The links never change between releases; nothing on the site is touched when a version comes out.

Deploy as the site's `wrangler.jsonc` says (the reader's WebAssembly first if it changed, then `npm run deploy` in
`site/`), and open `/` in both languages on a phone and a desktop browser; both download links work.

## 7. AUR

**On 2026-10-19 AUR account registration was closed** ("temporarily", because of automated sign-ups; no manual queue).
Watch aur-general and the Arch news for it to reopen; until then the site points Arch users at `makepkg -si` in
`packaging/aur/forzac-bin` of this repository (section 6), and the steps below wait.

Once: an account on <https://aur.archlinux.org> (register, confirm the e-mail), an SSH key for it:

```bash
ssh-keygen -t ed25519 -f ~/.ssh/aur -C aur
# paste ~/.ssh/aur.pub into My Account › SSH Public Key on aur.archlinux.org
cat >> ~/.ssh/config <<'EOF'
Host aur.archlinux.org
  IdentityFile ~/.ssh/aur
  User aur
EOF
```

The PKGBUILDs already point at GitHub: `forzac` takes the tag's source archive
(`https://github.com/denisfl/forzac-desktop/archive/v$pkgver.tar.gz`, which unpacks to `forzac-desktop-$pkgver/`), and
`forzac-bin` the release's two Linux archives. What they still need are the checksums and `.SRCINFO`.

`.SRCINFO` and checksums need Arch tools; the Arch container has them (`ci/arch.Dockerfile`; `updpkgsums` is in
`pacman-contrib`):

```bash
docker run --rm -it --platform linux/amd64 -v "$PWD":/src forzac-arch bash
sudo pacman -S --noconfirm pacman-contrib
for p in forzac forzac-bin; do
  cp -r /src/packaging/aur/$p ~/$p && cd ~/$p
  updpkgsums                               # downloads the published files, writes sha256sums
  makepkg --printsrcinfo > .SRCINFO
  cp PKGBUILD .SRCINFO /src/packaging/aur/$p/ && cd ~
done
```

`forzac-bin` lists a source per architecture; `updpkgsums` fills both. Commit the PKGBUILDs with their checksums and
`.SRCINFO` here, then push each to its AUR repository (an empty clone creates the package on the first push; the AUR
branch is `master`):

```bash
for p in forzac forzac-bin; do
  git clone ssh://aur@aur.archlinux.org/$p.git /tmp/aur-$p
  cp packaging/aur/$p/{PKGBUILD,.SRCINFO,forzac.install} /tmp/aur-$p/
  git -C /tmp/aur-$p add PKGBUILD .SRCINFO forzac.install
  git -C /tmp/aur-$p commit -m "Forzac $V"
  git -C /tmp/aur-$p push origin master
done
```

Check from a clean Arch (the container has no `yay`): `git clone https://aur.archlinux.org/forzac-bin.git`, then
`cd forzac-bin && makepkg -si`, and `xdg-mime query default application/x-fictionbook+xml`.

**A new version:** steps 3–5, then in each PKGBUILD `pkgver=X.Y.Z` and `pkgrel=1`, `updpkgsums`,
`makepkg --printsrcinfo > .SRCINFO`, commit, push. A fix to the packaging alone (no new release files) raises
`pkgrel` instead. The AUR shows the package as out of date when a user flags it; the e-mail comes to the account.

## 8. Order of steps

1. The repository on GitHub (section 1) — once.
2. Checklist (section 2) on the release candidate built from `main`.
3. Version, commit, tag, push (section 3); the workflow drafts the release.
4. Download the draft's files (or build them locally), checksums (section 4); install them as a reader would.
5. Publish the release (section 5) and check the URLs and checksums from outside.
6. Download links on the main page and the desktop page (section 6), deploy, check both languages.
7. AUR (section 7), install from the AUR in the container.
8. Mark the release done: the release notes linked from the page, `docs/roadmap.md` updated, the worktree removed
   (`git worktree remove ../forzac-release`).

Withdrawing a broken release: point the page's links back to the previous version (deploy), then mark the GitHub
release as a draft again or delete it; on the AUR, bump `pkgrel` with the previous version's files rather than
deleting the package.
