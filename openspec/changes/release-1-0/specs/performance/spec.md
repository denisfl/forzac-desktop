## Purpose

The speed a reader can count on: how soon a book shows its first page, how smoothly pages turn and how quiet Forzac
is while nothing happens, as budgets checked on the reference hardware before a release.

## ADDED Requirements

### Requirement: The test hardware and books
The budgets SHALL be checked on the reference hardware of `docs/roadmap.md`: the Raspberry Pi 5 (Raspberry Pi OS 13,
its GPU driving a headless 1920×1200 output at 120 Hz) and the M1 MacBook Pro (macOS 14, its built-in 60 Hz display,
awake and unlocked). They hold for books whose chapters are chapters: an FB2 novel of 300–400 pages and an EPUB with a
table of contents. A book whose whole text is one chapter is bounded by laying out that chapter instead. x86_64 Linux
and Hyprland are not measured on hardware; the release notes SHALL say so and ask Omarchy users for reports.

#### Scenario: A book that is one long chapter
- **WHEN** a 316-page EPUB with its whole text in one file is opened on the Mac
- **THEN** it is not held to the first-page budget; its first page comes after the chapter is laid out, and the
  release notes list it as a known issue

#### Scenario: Omarchy
- **WHEN** the release notes of 1.0.0 are read
- **THEN** they say that x86_64 and Hyprland were built and installed in containers only, and ask Omarchy users to
  report how Forzac runs there

### Requirement: The first page
Opening a test book, warm (a second opening of the same book, caches filled), SHALL show its first page within 200 ms
on the Pi 5, counted from the start of the process, and within 150 ms on the Mac, counted from the request to open
the book (a double-click in Finder or the library while Forzac runs; the process launch, AppKit's own, is reported
beside it). The time ends when the first frame with the page is presented. The book's window SHALL be laid out once
on opening: its frame, remembered or default and fitted to the screen, is final before the page is typeset.

#### Scenario: Warm opening on the Pi 5
- **WHEN** `forzac book.fb2.zip --measure` runs three times on the Pi 5's headless output
- **THEN** the second and third runs print a first presented page within 200 ms of the start

#### Scenario: Opening on the Mac
- **WHEN** `scripts/measure.sh book.fb2.zip` runs on the M1 with the screen unlocked
- **THEN** its second and third openings print "opening to the first page" within 150 ms

#### Scenario: A window taller than the screen
- **WHEN** a book opens on a Mac whose screen is lower than the window's default height
- **THEN** its chapter is typeset once, at the window's final size

### Requirement: Page turns
An animated page turn SHALL present every frame on time at the display's refresh rate: in 20 turns in "Slide", no
frame inside a turn's animation may come later than 1.5 refresh intervals after the one before it — at 120 Hz on the
Pi 5's headless output and at 60 Hz on the Mac's display.

#### Scenario: Twenty turns on the Mac
- **WHEN** `forzac book --chapter 5 --turns 20` runs on the M1
- **THEN** it prints "refresh 16.67 ms (60 Hz)" and "missed 0"

#### Scenario: Twenty turns on the Pi 5
- **WHEN** `scripts/pi-headless.sh` runs the same 20 turns at 120 Hz with the default GTK renderer
- **THEN** it prints "missed 0"

### Requirement: Quiet while idle
A book window that nobody touches SHALL use no CPU: 0 ticks (hundredths of a second of CPU) over 10 seconds, starting
3 seconds after it opened, on both the Pi 5 and the Mac.

#### Scenario: Idle book
- **WHEN** a book is open on a page and left alone, and `scripts/measure.sh` counts CPU ticks for 10 seconds
- **THEN** it prints "idle: 0 CPU ticks in 10 s"

### Requirement: Laying out a chapter
Typesetting any chapter of a test book SHALL take under 50 ms on the Pi 5, so that a change of font, size or window
size re-typesets without a visible wait.

#### Scenario: The slowest chapter
- **WHEN** `cargo run --release -p forzac-core --example bench -- book.fb2.zip` runs on the Pi 5
- **THEN** its "slowest chapter" is under 50 ms

### Requirement: The evidence
Each release SHALL come with the measurements it was checked against: the output of `scripts/measure.sh` on the Mac
and of `scripts/pi-headless.sh` on the Pi 5, summarised in the release notes with the date, the hardware and the book.
A budget not met SHALL be named in the release notes as a known issue, or the release waits for its fix.

#### Scenario: Release notes of 1.0.0
- **WHEN** `docs/releases/1.0.0.md` is read
- **THEN** it gives the first page, the missed frames and the idle ticks measured on each machine, and what was not
  measured
