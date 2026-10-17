# help Specification

## Purpose
What `?` shows in a book window on Linux and macOS: every key grouped by what it does, laid out as a card in the
reader's theme, and the one place where the support line appears.

## Requirements

### Requirement: The help card
`?` in a book window SHALL show the help over the page and `?` again SHALL hide it. The help SHALL be a card in the
middle of the window over the page dimmed by a translucent black layer, in the reader's theme: the theme's sheet colour,
22 px corners, its line colour for rules and key caps, its muted colour for section titles. The card SHALL have the
title "Keys" and a close button, then the keys in sections in this order: Reading, Moving, Book, Selection, View, Mouse,
Picture or table. Each row SHALL show its keys as key caps (each key or key combination in its own rounded frame) and
its meaning beside them; the Mouse section's rows have no keys. The card SHALL list every key the shell has, with the
shell's own keys (Linux: `Enter`, `Ctrl+…`, `F11`; macOS: `Return`, `⌘…`, `⌃…`, `s` share in Selection).

#### Scenario: Asking for help
- **WHEN** the reader presses `?` in a book window
- **THEN** the page dims, the card shows the "Keys" title and the seven sections with key caps, and `?` hides it again

#### Scenario: A theme
- **WHEN** the help is open with the Wave theme
- **THEN** the card is Wave's sheet colour with Wave's ink, line and muted colours

### Requirement: The help's layout
The sections SHALL fill two columns when the two-column card (928 px wide) fits the window with a 24 px margin on
each side, split between two sections in their order where the taller column is shortest, and one column (472 px)
otherwise; the choice SHALL be made at least each time the help opens.
The card SHALL be no taller than the window less 24 px above and below; when its content is taller, the sections SHALL
scroll inside it, and `j`/`k`, `↓`/`↑`, `Space`, `PgDn`/`PgUp` SHALL scroll them while the help is open.

#### Scenario: A wide window
- **WHEN** the help opens in a window 1440 wide and 960 high inside
- **THEN** the sections stand in two columns and the whole card is visible without scrolling

#### Scenario: A narrow window
- **WHEN** the help opens in a 760 × 980 book window
- **THEN** the sections stand in one column, `j` scrolls them inside the card and the page does not turn

### Requirement: Closing the help
`?`, `Esc`, the close button and a click outside the card SHALL hide the help. The card and the dimming SHALL fade in
over 200 ms and out over 150 ms; nothing SHALL be drawn while the help stands still. While the help is open, other
reader keys SHALL do nothing (on macOS the menus' `⌘` shortcuts still work).

#### Scenario: Esc
- **WHEN** the help is open and the reader presses `Esc`
- **THEN** the help fades out and the page shows as it was, with no panel opened or closed

#### Scenario: A click beside the card
- **WHEN** the help is open and the reader clicks the dimmed page outside the card
- **THEN** the help hides and the page does not turn

### Requirement: The support line
The help SHALL end, below a rule, with the single line "Forzac is free. Support it: forzac.truenrth.app", the address
in the theme's link colour; a click on the address SHALL open https://forzac.truenrth.app in the default browser. The
support link SHALL appear nowhere else in the book window and never on its own.

#### Scenario: Supporting
- **WHEN** the reader clicks "forzac.truenrth.app" in the help
- **THEN** the site opens in the browser and the help stays open

### Requirement: The help's language
The title, section titles, meanings and the support line SHALL be in the interface language; the keys SHALL stay as
they are on the keyboard ("Space", "Ctrl+O", "⌘D").

#### Scenario: Russian
- **WHEN** the interface is Russian and the reader presses `?`
- **THEN** the card is titled "Клавиши", its first section is "Чтение", and its first row's keys read `j` `l` `→` `↓`
  `Space` `PgDn`
