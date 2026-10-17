## Context

Both shells hold their strings as literals: the GTK shell in Rust (`gtk/src/*.rs`, the help in `main.rs`), the Mac
shell in Swift (`macos/*.swift`, menus in `main.swift`); the core draws "Finished" on the end page and makes "Chapter N"
for untitled chapters. The Android app has `values/strings.xml` and `values-ru/strings.xml` with plurals; its
`.desktop`-like names are already in `data/app.truenrth.forzac.desktop` (`Name[ru]=Форзац`). About 120 strings.

## Goals / Non-Goals

**Goals:** both shells in Russian from one catalogue, the Android app's words, plurals, a test that keeps the
catalogue complete.

**Non-Goals:** other languages, an in-app language setting, gettext tooling.

## Decisions

**One catalogue in the core (`core/src/i18n.rs`), keyed by the English text.** The proposal named gettext (Linux) and
`.strings` (macOS); with two shells and one maintainer that is two catalogues to keep in step, and gettext adds build
tooling to the `.deb` and the AUR packages. A Rust table `(English, Russian)` read into a map once serves both, and the
English text as the key keeps call sites readable (`tr("Contents")`), as gettext does. Placeholders are named
(`{name}`, `{n}`) and filled by `fill`; counts go through `tr_count(n, one, other)` (English one/other; Russian one, few,
many by the CLDR rule for `ru`). The two help texts move into the core (`i18n::help(Platform)`), so their English is
written once.

**The language is the core's, set once by each shell at start.** `Language::from_locales(list)` takes the first of the
preferred languages that is `en` or `ru` (by its primary subtag), else English; GTK passes `glib::language_names()`,
macOS `Locale.preferredLanguages`. The core uses it for "Chapter N" and "Finished". A global (an atomic) is enough: the
language does not change while Forzac runs.

**C ABI.** `forzac_set_language(locales)` (comma separated), `forzac_tr(english)` (the Russian, or null: the caller
keeps its English), `forzac_tr_count(n, one, other)` (valid until the next call), `forzac_help(platform)`. Swift wraps
them in `tr`, `trCount`, `fill`.

**The completeness test** reads the shells' sources from the core's test (`CARGO_MANIFEST_DIR/../gtk/src`,
`../macos`): every `tr("…")`, `tr_count(…, "…", "…")` and `trCount(…, "…", "…")` literal must have a Russian entry, and
every entry must be asked for by a shell or the core. Literals are unescaped (`\n`, `\"`, `\\`).

**Words.** Where Android has the string, its words; desktop-only strings in the same voice. "Highlight" is "Цитата"
(Android), the page modes "Сдвиг" and "Лента" under "Листание", the pill "← Вернуться · Ctrl+O". The app's name in
Russian is "Форзац", as the `.desktop` file and Android have it. Help keeps key names (`j`, `Ctrl+O`).

**macOS system panels.** `CFBundleLocalizations` (`en`, `ru`) in `Info.plist`, so `NSOpenPanel`, the share menu and
alerts' buttons follow the system language.

## Risks / Trade-offs

- [Longer Russian labels: the tabs, the bar] → checked on both machines; the contents panel already widens to fit its
  header (at most 460 px).
- [A string added without `tr`] → not caught by the test; the by-hand pass covers every screen in Russian.
