# Design

## Context

The help is two text blocks in `core/src/i18n.rs` (Linux and macOS, each with its Russian twin) shown as a centred
label: Adwaita's OSD style on GTK, a grey `NSTextField` on the Mac. The card replaces both. It follows the web reader's
look (`site/public/read/reader.css` in the Android repository: the theme's `--sheet`, `--line`, `--muted`, `--link`,
22 px corners, the UI font) so that the web reader can take it over one to one if it gets keys.

## Decisions

### The help as data, in the core

`i18n::help(platform) -> Help { title, sections: Vec<Section { title, rows: Vec<Row { keys, meaning }> }>, support }`,
everything translated except the keys. One table serves both platforms: each row has its Linux keys, its macOS keys
and its meaning; a row whose keys are empty on a platform is left out there (Share on Linux), and the Mouse section's
rows have no keys on either (they are kept by an explicit flag, not by empty keys). `keys` is one string, the key caps
separated by spaces ("k h ← ↑ Shift+Space PgUp"); a cap never contains a space. The Russian rows are ordinary entries
of the catalogue, keyed by the English meaning, so the completeness test covers them once the table's strings are in
its list of strings the core asks for itself.

The C ABI keeps `forzac_help(platform)` and returns the help flattened: the title on the first line, a section as a
line starting with `# `, a row as `keys\tmeaning` (keys may be empty), the support line last. The Mac parses it; GTK
uses the struct.

### Layout (both shells, in points / logical pixels)

- Dimming: black at 24% over the whole window. Card: the theme's sheet colour, 22 px corners, the shadow of the
  panels; 24 px from the window's edges at least; padding 28 px.
- Head: "Keys" 20 px semibold, a 32 px round close button (×) at the right, 16 px below it to the first section.
- Section: its title 12 px medium, upper case, 0.72 px letter spacing, muted; 6 px to the rows; 16 px between
  sections.
- Row: a key area 168 px wide where the caps flow and wrap, 12 px gap, the meaning 14 px in ink wrapping in 236 px;
  4 px between rows. A cap: 13 px medium, 1 px frame in the line colour, 6 px corners, 2 px by 7 px padding, at least
  24 px wide, 4 px between caps. A row without keys gives its whole width to the meaning.
- Columns: 416 px each, 40 px apart. Two columns (card 928 px) when the window is at least 976 px wide, else one
  (card 472 px). The split is the one between two sections that makes the taller column shortest. The columns are
  chosen when the help opens (GTK) or whenever the window changes size (macOS).
- Foot: a 1 px rule 20 px below the sections, then the support line 13 px muted, 16 px below the rule; the address in
  the link colour, a pointer cursor over it.
- The sections scroll inside the card when it would be taller than the window less 48 px; the head and foot stay.

### Behaviour

- Shown: the help is above everything in the window (panels and the menu included). Fade: opacity 0 → 1 in 200 ms
  ease-out, 1 → 0 in 150 ms ease-in, then hidden (GTK: a CSS transition and a timer as in `menu.rs`; macOS: Core
  Animation's opacity).
- Keys while open: `?` and `Esc` hide; `j` `↓` / `k` `↑` scroll 40 px, `Space` `PgDn` / `Shift+Space` `PgUp` a card's
  height less 40 px; everything else is swallowed. GTK: a capture-phase key controller on the window that acts only
  while the help is visible, ahead of the shortcut controller. macOS: the reader's key handling asks the help first.
- Clicks: the close button and a click on the dimmed area outside the card hide; a click on the address opens it
  (`gtk::UriLauncher`, `NSWorkspace`) and leaves the help open; other clicks on the card do nothing.

### Where it lives

Core: the table, its Russian, the flattening for the C ABI. GTK: `gtk/src/help.rs` (widgets: a dimming box, the card
with a `ScrolledWindow` of two column boxes, caps as labels in lines of boxes), theme colours in
`panels.rs`'s CSS. macOS: `macos/Help.swift` (a view laying out layers and text fields by hand, in an `NSScrollView`),
replacing the `NSTextField` in `main.swift`.

## Risks

- In a 760-pixel book window the help is one long column and scrolls: the keys are many. Accepted: two columns in a
  wide or full-screen window, `j`/`k` scroll it.
- A key cap's width comes from its text: GTK measures the caps once they are in the window and styled (unstyled
  labels measure narrower), then breaks them into lines.
