## 1. Core

- [x] 1.1 `update` module: `RELEASES`, `PAGE`, `VERSION`, `due`, `record`, `notice`, `dismiss`; tests for `version`
      (`v0.9.1`, `0.9`, `v1.0.0-rc1`, garbage), `newer`, `due_at` (never, 3 h, 25 h, future), `record` with GitHub's
      answer and with an error body, `notice` after `dismiss` and after a newer release — `cargo test -p forzac-core`
- [x] 1.2 The two strings in the catalogue (English and Russian) — the catalogue test passes
- [x] 1.3 C ABI (`forzac_update_*`) and `macos/forzac.h` — an ABI test in `ffi` passes

## 2. GTK shell

- [x] 2.1 The check when the library is built: due, not measuring, no `packaged` marker, one at a time; `curl` through
      `gio::Subprocess`; `record` and the line updated when the answer comes — `cargo clippy -p forzac-gtk` clean
- [x] 2.2 The line in the head: text, link to the page, "×" dismisses, hidden while searching, elided when narrow

## 3. macOS shell

- [x] 3.1 The check with `URLSession` (ephemeral, `User-Agent: Forzac`, 10 s) when the library window opens; the line
      in the head, the link, "×", hidden while searching or too narrow — `macos/build.sh` builds

## 4. Packaging and docs

- [x] 4.1 `share/forzac/packaged` in both PKGBUILDs and `scripts/package-deb.sh`; README: one sentence for packagers
      about the check and the marker

## 5. Verification

- [x] 5.1 `cargo fmt --all --check`, clippy (`-p forzac-core` on macOS; the workspace in the Linux container), tests;
      both shells build
- [x] 5.2 Mac by hand: a state with an older `update-latest`-faking version shows the line; "download" opens the page;
      "×" hides it and a restart keeps it hidden; a real request against GitHub with the version lowered for the test;
      offline shows nothing; screenshot
- [x] 5.3 Linux (Pi 5 or the container): the archive layout checks, the layout with `packaged` does not; the library's
      `--measure` first frame as before

Checked on 2026-10-19. Mac (M1, macOS 14.5), with a state file of its own (`--state`): a remembered 0.9.1 shows the line
in the empty library and next to the buttons (`--library --png`, English and Russian); a build with the version lowered
to 0.8.9 for the test asked GitHub, recorded 0.9.0 and showed the line while the library was open; "×" hid it and a
restart kept it hidden; "download" handed the focus to the default browser (the page itself was not seen: the browser is
outside what the session may capture). Linux in the Debian 13 arm64 container instead of the Pi 5 (booted as another
system): clippy and the release build pass; the archive layout asked GitHub and recorded 0.9.0, the layout with
`packaged` made no request and wrote no state; the line rendered in the head (`--png`). Not driven on Linux: clicks on
"download" and "×" (no pointer under headless Weston); dismissing is covered by the core's tests. The library's first
frame is unchanged by construction: measurement runs do not check.
