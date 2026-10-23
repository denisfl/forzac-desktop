## 1. Measuring the Mac

- [x] 1.1 (2026-10-23: 20 turns of an FB2 and an EPUB: "refresh 16.67 ms (60 Hz), 239–240 frames, missed 0") The AppKit shell's `--turns N`: N turns 0.4 s apart, frame intervals inside each turn's animation from a display link, printed as the GTK shell prints them — check: `Forzac book --chapter 5 --turns 20` prints the refresh rate and "missed"
- [x] 1.2 (2026-10-23) `--measure` on the Mac also prints "opening to the first page", from the request to open the book — check: three runs print it
- [x] 1.3 (2026-10-23: run on the M1 for four books; the lock warning checked on a locked screen) `scripts/measure.sh` on macOS: builds with `macos/build.sh`, keeps the display awake, warns on a locked screen, prints the system, three openings, 20 turns and idle ticks (`ps` CPU time in hundredths of a second) — check: the script's output on the M1
- [x] 1.4 (2026-10-23) `docs/publishing.md` step 2 and `CLAUDE.md` name `scripts/measure.sh` for the Mac — check: the commands there run as written

## 2. Fixes found on the Mac

- [x] 2.1 (2026-10-23: one typesetting per opening, by a temporary print in `relayout`; 1.1 → 0.4 s to the first page for a one-chapter EPUB; ≈650 → ≈220 ms from launch for the test books, measured on a locked screen) The book window created full-size and placed (remembered or default frame, constrained to the screen, centred or cascaded) before its page is added — check: `--measure` before and after; a remembered frame still restores (by hand, the maintainer)
- [x] 2.2 (2026-10-23: opening to the first page 145–160 → 122–146 ms; `--panel help` and the library's `?` screenshots unchanged) The keys card makes its rows when it first shows — check: `--measure`, and the card in a book and in the library

## 3. The Mac's budgets

- [ ] 3.1 (2026-10-23, on a locked screen, display kept awake, on power: opening to the first page 128–144 ms warm, from launch 190–205 ms; 20 turns at 60 Hz, 0 of 239 missed; idle 0 ticks in 10 s; a one-chapter EPUB 345 ms to its first page; slowest chapters 3, 18 and 31 ms on the M1) `scripts/measure.sh` on the M1 with the screen unlocked, an FB2 novel and an EPUB with chapters, on battery and on power; results into `docs/releases/1.0.0.md` — check: every line within its budget or named as a known issue

## 4. The Pi 5's budgets (with the maintainer's consent, on the USB-stick system)

- [ ] 4.1 (2026-10-23: waived for now by the maintainer, no Pi tests until further notice) `PI=… scripts/pi-dev.sh` builds; `scripts/pi-headless.sh forzac book.fb2.zip` at 120 Hz: first page ≤ 200 ms warm, 20 turns "missed 0", idle 0 ticks — check: the script's output, saved with the release notes
- [ ] 4.2 (2026-10-23: waived for now by the maintainer, no Pi tests until further notice) The core's `bench` example on the Pi 5 for the test books: the slowest chapter under 50 ms — check: its output
- [ ] 4.3 (2026-10-23: waived for now by the maintainer, no Pi tests until further notice) By hand over VNC: open, turn, resize across a spread; no stutter seen — check: the maintainer's word

## 5. x86_64 in containers

- [x] 5.1 (2026-10-23: 6 min emulated on the M1; `forzac` and `forzac-bin` 0.18.0-1 installed, the desktop file valid, Forzac the default for FB2, `.fb2.zip`, FB3 and EPUB; a page at scales 1, 1.25, 1.6 and 2 under sway, pixman) `ci/arch-check.sh` in the x86_64 Arch image: both AUR packages build and install with `makepkg -si`, the desktop file validates, `xdg-mime` makes Forzac the default, the app runs under a headless sway at four scales — check: the script exits 0 and the screenshots show the page
- [x] 5.2 (2026-10-23: the check failed with "One or more files did not pass the validity check" after 0.18.0's sums went in) `ci/arch-check.sh` sets the PKGBUILDs' checksums to `SKIP` for the archives it builds from the working tree — check: 5.1 passes

## 6. Release notes and publishing

- [ ] 6.1 `docs/releases/1.0.0.md`: what 1.0 is, what changed since 0.18.0, the measurements of 3 and 4 with dates and hardware, what was not checked (x86_64 and Hyprland on hardware; a call to Omarchy users), known issues — check: read against `specs/performance` "The evidence"
- [ ] 6.2 Before the DMG is published (from `macos-release`): the maintainer opens the downloaded DMG on a fresh Mac user account following `docs/macos-first-open.md`, the Gatekeeper step on a quarantined copy, on macOS 14 and a macOS 15 Mac; screenshots of both into the download page — check: the maintainer's word and the screenshots
- [ ] 6.3 Version 1.0.0 (`Cargo.toml`, `cargo update -w --offline`, both PKGBUILDs), commit "Release 1.0.0", tag `v1.0.0` and push — only with the maintainer's yes — check: the workflow's draft has the seven files and `SHA256SUMS`
- [ ] 6.4 The draft's files installed as in `docs/publishing.md` step 2, then `gh release edit v1.0.0 --notes-file docs/releases/1.0.0.md --title "Forzac 1.0.0" --draft=false --latest` — check: the URLs and checksums from outside (step 5)
- [ ] 6.5 AUR: `updpkgsums` and `.SRCINFO` from the tag's archives in the `forzac-arch` container (`packaging/aur/`), pushed when AUR registration allows — check: `makepkg -si` of the published PKGBUILDs
