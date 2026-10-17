# Publishing a release

How a version of Forzac for desktop gets from this repository to readers: the files on forzac.truenrth.app, the
download page, and the AUR. Nothing here is automatic and nothing has been published yet; each step is run by the
maintainer. Dry-run for 0.1.0 on 2026-10-17 (an M1 Mac with Docker): every build and checksum command in steps 3–4,
and in step 7 `updpkgsums`, `.SRCINFO` and `makepkg -si` of `forzac-bin` with the PKGBUILDs pointed at
`download.forzac.truenrth.app` (makepkg takes a source file lying next to the PKGBUILD instead of downloading it). The
upload, the site and the AUR push cannot be tried without publishing; they follow the tools' documentation.

What a release is made of, for version `X.Y.Z`:

| File | For | Made by |
|---|---|---|
| `Forzac-X.Y.Z-macos-arm64.dmg` | Mac, Apple Silicon, macOS 14+ | `macos/build.sh`, `scripts/package-macos.sh` |
| `forzac-X.Y.Z-linux-x86_64.tar.gz` | AUR `forzac-bin`, any x86_64 Linux with GTK 4.14+ | `scripts/package-linux.sh` |
| `forzac-X.Y.Z-linux-aarch64.tar.gz` | AUR `forzac-bin` on ARM, any aarch64 Linux with GTK 4.14+ | `scripts/package-linux.sh` |
| `forzac_X.Y.Z_arm64.deb` | Raspberry Pi OS 13 (arm64) | `scripts/package-deb.sh` |
| `forzac-desktop-X.Y.Z.tar.gz` | the source (MPL-2.0), the AUR `forzac` package | `git archive` |
| `SHA256SUMS` | checking any of the above | `shasum -a 256` |

Sizes for 0.1.0: DMG 9.0 MB, archives 6.3 and 6.4 MB, `.deb` 3.7 MB, source 5.0 MB.

## 1. Decisions before the first release

Two decisions belong to the maintainer; the rest of this document works with either answer.

### Where the source lives

MPL-2.0 (section 3.2) asks that whoever receives the program in executable form is told how to get its source. A
source archive next to every release on the site satisfies this on its own; the binaries carry `LICENSE`, and the
download page links the archive.

**Source A. A public git repository (GitHub, `denisfl/forzac-desktop`).**
- For: history, issues and pull requests; the release workflow (`.github/workflows/release.yml`) builds every
  artefact on a tag for free (public repositories get free macOS and Linux runners), so step 4 becomes "download the
  draft release"; the AUR `forzac` package can take the tag's archive from GitHub, as AUR users expect; readers can
  see what they run.
- Against: issues and pull requests have to be answered or closed; the whole history becomes public (check it first:
  the commits carry the author's e-mail address; books were never committed); an Omarchy user will expect a quick
  fix on hardware we do not have.

**Source B. Only source archives on the site.**
- For: no community upkeep, nothing public but the releases; the licence is met by `forzac-desktop-X.Y.Z.tar.gz`.
- Against: no history for readers or packagers; contributions only by e-mail; CI does not run unless the code is on
  GitHub anyway (a private repository works, but its free minutes are limited and macOS minutes count ten times), so
  releases are built locally as in step 3; some AUR users frown on sources that are not in a public repository.

A middle way: a private GitHub repository for CI, archives on the site, and the decision to open it later.

### Where the files live

The site is a Cloudflare Worker that serves only static assets: `~/dev/forzac/site/public` is uploaded by
`npm run deploy` (`wrangler.jsonc`, no Worker code). Cloudflare allows 25 MiB per asset file (20,000 files on the free
plan), so every release file fits. But a deploy uploads exactly what is in `public/` at that moment, and the site is
deployed often from the Android repository.

**Files A. R2 bucket on a subdomain (recommended).** A bucket `forzac-downloads` with the custom domain
`download.forzac.truenrth.app`; files at `https://download.forzac.truenrth.app/desktop/X.Y.Z/<file>`. Site deploys
never touch them, old versions stay, no file-size worry (wrangler uploads up to 315 MB per object), egress is free, the
free tier holds 10 GB. The download page on the site links there.

**Files B. Site assets.** Files in `site/public/download/desktop/X.Y.Z/`, URLs
`https://forzac.truenrth.app/download/desktop/X.Y.Z/<file>`. No new service, but binaries must not go into the Android
repository's git, so the folder is ignored and lives only on the machine that deploys: a deploy from another checkout
deletes the downloads, and every release adds ~30 MB to each deploy.

Below, `$BASE` is `https://download.forzac.truenrth.app/desktop` (Files A) or
`https://forzac.truenrth.app/download/desktop` (Files B).

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
- [ ] The packages from step 3 installed as a reader would: the `.deb` on the Pi 5 (`sudo apt install
      ./forzac_X.Y.Z_arm64.deb`, then `xdg-open book.fb2` from the desktop session); both AUR packages in the Arch
      container (`ci/arch-check.sh`, ~10 min under emulation); the DMG on the Mac.
- [ ] Mac, before the first public DMG (moved here from `macos-release`): the DMG downloaded from its real URL on a
      **fresh macOS user account**, opened by following the download page (the Gatekeeper step), on macOS 14 and on a
      macOS 15 Mac; screenshots of both for the page.
- [ ] Release notes (`docs/releases/X.Y.Z.md` or the page itself): what is new, what is not checked (Hyprland and
      x86_64 hardware: built and installed in a container only), known issues.

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

Build from a clean checkout of the tag, not from the working directory: other sessions may have uncommitted work in
it.

```bash
git worktree add --detach ../forzac-release vX.Y.Z
cd ../forzac-release
mkdir -p dist
```

With the code on GitHub (Source A, or a private repository) pushing the tag runs the release workflow, which drafts a
release with the archives, the `.deb` and the DMG; download them into `dist/` and go to step 5. Otherwise build
locally (step 4).

## 4. Build the files

All commands run in `../forzac-release`. Docker builds the Linux files in Debian 13 (the base of Raspberry Pi OS 13,
so the `.deb` links its GTK and glibc; the archives then need glibc 2.41: Debian 13, Ubuntu 25.04, Fedora 42, Arch and
newer, not Ubuntu 24.04 — the GitHub workflow builds them on Ubuntu 24.04 instead, which reaches further back). Use
the `stable` Rust toolchain for macOS (`rust-toolchain.toml`); if Homebrew's older `cargo` comes first in `PATH`, put
`~/.rustup/toolchains/stable-aarch64-apple-darwin/bin` before it.

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

Checksums of everything, one file the page links, checked right away:

```bash
cd dist
shasum -a 256 forzac-desktop-$V.tar.gz forzac-$V-linux-*.tar.gz forzac_${V}_arm64.deb Forzac-$V-macos-arm64.dmg \
  > SHA256SUMS
shasum -a 256 -c SHA256SUMS
```

`dist/` now holds the six release files, the DMG's own `.sha256` (the Mac page links it, `docs/macos-first-open.md`)
and `SHA256SUMS`. Install them as in the checklist (step 2) before uploading.

## 5. Upload the files

**R2 (Files A)**, once:

```bash
cd ~/dev/forzac/site                       # wrangler is installed there and logged in to the Cloudflare account
npx wrangler r2 bucket create forzac-downloads
npx wrangler r2 bucket domain add forzac-downloads --domain download.forzac.truenrth.app \
  --zone-id <the truenrth.app zone id from the dashboard> --min-tls 1.2
```

Each release (`--remote`: without it wrangler writes to a local simulation):

```bash
for f in ~/dev/forzac-release/dist/*; do
  npx wrangler r2 object put "forzac-downloads/desktop/$V/$(basename "$f")" --file "$f" --remote
done
```

Content types are guessed from the extensions; `.sha256` and `SHA256SUMS` are served as `application/octet-stream`
(a browser downloads them instead of showing them), which does not matter for `shasum -c`; add
`--content-type text/plain` to those two uploads to make them readable in the browser.

**Site assets (Files B):** copy `dist/*` to `~/dev/forzac/site/public/download/desktop/$V/`, add `public/download/`
to `site/.gitignore`, and deploy with the page (step 6).

Then check every URL and checksum from outside:

```bash
mkdir -p /tmp/forzac-check && cd /tmp/forzac-check
for f in $(awk '{print $2}' ~/dev/forzac-release/dist/SHA256SUMS) SHA256SUMS; do curl -fsSLO "$BASE/$V/$f"; done
shasum -a 256 -c SHA256SUMS
```

## 6. The download page

The site (`~/dev/forzac/site`, in the Android repository) has one page, `public/index.html`, in English and Russian
(a `lang` switch). Changes for the desktop release, to be made in that repository:

- A page `public/desktop/index.html` (served at `/desktop/`) in the same style and with the same language switch, or
  a section in `index.html` if it stays short; a "Desktop: Linux and Mac" link next to the Google Play link.
- **Mac:** the download button (the DMG), its SHA-256 link, and the first-open text from
  [docs/macos-first-open.md](macos-first-open.md) (translated for the Russian half), with the screenshots from the
  checklist. Requirements: Apple Silicon, macOS 14 or newer.
- **Arch, Omarchy:** `yay -S forzac-bin` (or `paru -S forzac-bin`; `forzac` builds from source), then the default
  reader command:

  ```bash
  xdg-mime default app.truenrth.forzac.desktop application/x-fictionbook+xml application/x-zip-compressed-fb2 application/x-fb3 application/epub+zip
  ```

  and the note that Omarchy is built and installed in CI only, so reports are welcome.
- **Raspberry Pi OS (Pi 4/5, 64-bit):**

  ```bash
  wget $BASE/X.Y.Z/forzac_X.Y.Z_arm64.deb
  sudo apt install ./forzac_X.Y.Z_arm64.deb
  ```

  then the same `xdg-mime default` command (the package prints it).
- **Other Linux (x86_64, aarch64, GTK 4.14+):** the archive runs where it is unpacked:
  `tar xzf forzac-X.Y.Z-linux-x86_64.tar.gz && forzac-X.Y.Z-linux-x86_64/bin/forzac book.fb2`.
- **Source and licence:** the source archive, "MPL-2.0", the `SHA256SUMS` link with
  `shasum -a 256 -c SHA256SUMS`, and the repository link with Source A.
- The support line stays the one the site already has; the release notes on the page or linked from it.

Deploy as the site's `wrangler.jsonc` says (the reader's WebAssembly first if it changed, then `npm run deploy` in
`site/`), and open `/desktop/` in both languages on a phone and a desktop browser.

## 7. AUR

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

Point both PKGBUILDs at the published files (a commit in this repository):

- `packaging/aur/forzac/PKGBUILD`: `source=("forzac-desktop-$pkgver.tar.gz::$BASE/$pkgver/forzac-desktop-$pkgver.tar.gz")`
  (or the GitHub tag archive with Source A: `https://github.com/denisfl/forzac-desktop/archive/v$pkgver.tar.gz`
  unpacks to the same `forzac-desktop-$pkgver/`).
- `packaging/aur/forzac-bin/PKGBUILD`: `_release=$BASE/$pkgver`.

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

1. Decisions (section 1) — once.
2. Checklist (section 2) on the release candidate built from `main`.
3. Version, commit, tag (section 3); push the tag if the code is on GitHub.
4. Build or download the files, checksums (section 4); install them as a reader would.
5. Upload (section 5) and check the URLs and checksums from outside.
6. Download page (section 6), deploy, check both languages.
7. AUR (section 7), install from the AUR in the container.
8. Mark the release done: the release notes linked from the page, `docs/roadmap.md` updated, the worktree removed
   (`git worktree remove ../forzac-release`).

Withdrawing a broken release: remove the page's links first (deploy), then the files; on the AUR, bump `pkgrel` with
the previous version's files rather than deleting the package.
