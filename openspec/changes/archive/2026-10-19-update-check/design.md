## Context

See proposal.md. The core has no network or platform code (CLAUDE.md); the reading state is one text file whose
`setting` lines hold arbitrary keys (`state::Store::setting`, `set_settings`). The GTK shell finds its data directory
with `fonts::assets()` (`FORZAC_ASSETS`, then `../share/forzac` next to the binary, then the source tree). Both
libraries have a head row: the brand on the left, the search and "Open a book" buttons on the right, the search field
in the brand's place while searching. `forzac-bin` installs the very binary of the release archive, so a build-time
flag cannot tell them apart.

## Goals / Non-Goals

**Goals:** the check and the line of the spec with the least code in each shell and no new dependency.

**Non-Goals:** a shared HTTP client in the core; retry timers; checking while the library stays open for days (the
next opening checks).

## Decisions

**The core decides, the shells fetch.** `core/src/update.rs`: `RELEASES` and `PAGE` (the URLs), `VERSION`
(`CARGO_PKG_VERSION`, the workspace version), `due(&Store) -> bool`, `record(&mut Store, answer: &str) -> bool`
(finds `"tag_name"` in the JSON by hand — one string field does not need a JSON parser —, reads the version, saves
`update-checked` and `update-latest`; false when the answer has no usable tag), `notice(&Store) -> Option<String>`
(the newer, undismissed version) and `dismiss(&mut Store)`. Pure helpers `version(tag)`, `newer(a, b)` and
`due_at(checked, now)` carry the tests. The shells only run the request and draw. Alternative: an HTTP client in the
core (`ureq` + `rustls`): one implementation, but TLS and its crypto in a typesetting core and in the macOS static
library, which already has `URLSession` — rejected.

**The trigger is the library.** The line lives only in the library, so the check starts when the library window is
built and the check is due, not at process start: a book opened from the file manager does not touch the network,
and no plumbing from the application to a library that may not exist yet. One check at a time: a thread-local flag in
GTK, a static in Swift. When the answer arrives the shell calls `record` and, if the library window is still open,
updates its line.

**GTK: `curl` through `gio::Subprocess`.** `curl -fsSL --max-time 10 -H 'Accept: application/vnd.github+json'
<RELEASES>` with stdout piped, `communicate_utf8_future` on the main context: no thread, no blocking, the process is
reaped when it exits. `curl` is in Arch's base and in Raspberry Pi OS; a missing `curl` fails the spawn, which is
ignored like any other failure. Alternatives: libsoup (a new system dependency and crate), raw TLS over
`gio::SocketClient` (an HTTP client by hand, and glib-networking is optional) — rejected.

**macOS: `URLSession` with an ephemeral configuration** (no cookies, no cache), `User-Agent: Forzac`, a 10 s timeout;
the completion hops to the main queue. C ABI: `forzac_update_due(state)`, `forzac_update_record(state, answer)`,
`forzac_update_notice(state)` (the line's version, valid until the next call on `state`, null when none),
`forzac_update_dismiss(state)`, `forzac_update_releases()` and `forzac_update_page()` (static strings).

**Packaged builds: a marker file.** `share/forzac/packaged` (empty) is installed by both PKGBUILDs and
`scripts/package-deb.sh`; the GTK shell checks `fonts::assets().join("packaged")`. Alternative: the binary's path
(`/usr/bin`) — no packaging edits, but an archive copied under `/usr` would fall silent and a packager installing
elsewhere would nag; the marker says what it means.

**The line.** In the head row, after the brand, before the buttons: muted 13 px text "Version {version} is out — " and
the link-coloured "download", then a 20 px "×" in the muted colour. GTK: a `gtk::Label` with markup (`<a href>`, the
link colour from the library's CSS) that ellipsizes at its end, opened with `gtk::UriLauncher` as the help's support
link, and a flat button; hidden while searching. macOS: a link field (the help's `Link` pattern, pointing hand,
opens with `NSWorkspace`) and a `FlatButton`, placed in `relayout()` between the brand and the leftmost button, hidden
while searching or when it does not fit. Strings: "Version {version} is out", "download" (Russian "Вышла версия
{version}", "скачать"), filled with the core's `{name}` convention.

## Risks / Trade-offs

- [GitHub's unauthenticated limit, 60 requests an hour per address] → one request a day per installation; a refused
  request is a failure and is retried at the next opening.
- [A reader who never opens the library is never told] → accepted: such a reader opens books from the file manager and
  the line would have nowhere quiet to go.
- [The release archive unpacked and also a package installed] → each copy decides from its own `share/forzac`.

## Migration Plan

Nothing to migrate: the three keys appear in the state file on the first check. Rollback removes the code; old keys
are ignored by older builds (unknown settings are kept and unused).
