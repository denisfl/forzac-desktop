## Context

See proposal.md for why. The tools exist on Linux: the GTK shell's `--measure` (start to the first presented frame),
`--turns N` (frame intervals inside the animations) and `scripts/measure.sh` / `scripts/pi-headless.sh` (three
openings, 20 turns, idle CPU ticks from `/proc`). The AppKit shell had only `--measure`, counted from the process
start, and no way to count frames or idle ticks; the spike (`docs/spike.md`) measured it once by hand: ≈230 ms from
launch to the first page, warm. Releases are drafted by `.github/workflows/release.yml` and checked by hand as
`docs/publishing.md` lists. The Pi 5's SD card is the maintainer's music server: Forzac runs on its USB-stick system,
and only with the maintainer's consent each time.

## Goals / Non-Goals

**Goals:** one command per machine that prints every budget of `specs/performance`; the Mac measured the same way as
the Pi; the fixes the measurements call for; the x86_64 packages checked in containers; the release notes.

**Non-Goals:** measuring x86_64 or Hyprland hardware (there is none at hand, `docs/roadmap.md`); continuous
benchmarking in CI (containers render in software and say nothing about speed); splitting a one-chapter book into
pages before its layout ends.

## Decisions

### 1. Where the clock starts

The Pi's budget counts from the process start, as `--measure` always has: it is an upper bound of opening a book and
the spike's numbers compare. On the Mac AppKit's own launch takes 65–80 ms before `applicationDidFinishLaunching`
(and dyld before that), more than half of a 150 ms budget for work Forzac does not do, while most books open in a
running app (from the library or Finder). So the Mac's `--measure` also prints "opening to the first page": from the
moment the shell asks to open the book to the first presented frame. Both numbers stay in the output; the budget
reads the second. *Alternative:* one start for both — then the Mac's budget would mostly measure AppKit.

### 2. Frames of a page turn on the Mac

A turn on the Mac is a Core Animation animation: the window server draws its frames and the app does no work per frame.
`--turns N` turns N pages 0.4 s apart and, from the moment each turn is committed to the end of its 0.2 s animation,
records the timestamps of a display link on the main thread; an interval over 1.5 refresh periods is a missed frame,
and the line printed is the GTK shell's. A late callback means the main thread was busy, which can only overcount:
the animation itself would go on. *Alternative:* Instruments' Animation Hitches template (`xctrace`) — exact, but a
trace to read by hand and not a line in a script; kept for a second look when the count is not 0.

### 3. Idle ticks on the Mac

`ps -o time=` gives a process's CPU time in hundredths of a second, the same unit as Linux's `/proc/PID/stat` ticks,
so the script prints the same line on both. *Alternative:* `proc_pid_rusage` gives nanoseconds; finer than the
budget asks and a compiled helper for the script.

### 4. Valid conditions are the script's job

A display link stops while the display sleeps, and a locked Mac gets no display cycles for a while, so the first
page waits for them: both gave plausible but wrong numbers while measuring (the first page at 600 ms instead of
150). On the Mac `scripts/measure.sh` keeps the display awake for its run (`caffeinate -d -u -w`) and warns when the
screen is locked; it prints the macOS version, the chip and the power source with the results. The Pi runs headless
through `scripts/pi-headless.sh`, which already sets the output's mode.

### 5. Fixes found by measuring

- The book window was laid out twice on opening: its chapter typeset at the default frame inside `Reader.init`, when
  the reading bar made the content full-size, then again when ordering the window front fitted it to the screen (or
  a remembered frame replaced it). The window is now created full-size and placed — remembered or default,
  constrained to the screen, centred or cascaded — before the page is added, so the chapter is typeset once at its
  final size. For a one-chapter 316-page EPUB this took the first page from 1.1 s to 0.4 s (two layouts of 195 ms
  each became one). For the test books the runs went from ≈650 to ≈220 ms from launch, but on a locked screen,
  where the old second layout also waited for a display cycle: the unlocked numbers are 3.1's.
- The keys card built its dozens of labels in every book window; it builds them the first time it shows (−20 to
  −45 ms per window).

### 6. x86_64 in containers

`ci/arch-check.sh` in the emulated x86_64 Arch image builds and installs both AUR packages with `makepkg -si`
(`forzac` from the working tree as the tag's archive would hold it, `forzac-bin` from an archive laid out by
`scripts/package-linux.sh`), checks the desktop file and MIME defaults, and runs the app under a headless sway at
four scales. The PKGBUILDs carry the published release's checksums since 0.18.0, so the check sets them to `SKIP`
for its local archives. The Debian amd64 archive build of `docs/publishing.md` step 4 stays the fallback when the
release workflow cannot run.

## Risks / Trade-offs

- [The Mac's frame count watches the main thread, not the window server] → it can only report more misses than there
  are; a non-zero count is checked with Instruments' Animation Hitches before it is called a regression.
- [Measured on a locked screen by a session running alone] → the script warns; the 1.0.0 numbers are repeated by the
  maintainer with the screen unlocked before the release is published.
- [The Pi's 120 Hz budget: the spike saw 1–4 late second frames per 20 turns, the frame clock waking from idle] → if
  it shows again, the release notes call it out (≈8 ms of extra latency at the start of a turn, no stutter inside it)
  or a fix lands first; the maintainer decides.
- [The Pi's chapter budget: on the M1 a chapter takes ≈0.6 ms a page — 31 ms for a 48-page EPUB chapter — and the
  Pi 5 is slower] → measured in 4.2; if ordinary chapters miss 50 ms, the maintainer chooses between a budget per
  page and laying out the pages around the reading position first.
- [One-chapter books] → their first page waits for the whole chapter (0.4 s on the M1, more on the Pi); listed as a
  known issue, not hidden by choosing other test books.

## Migration Plan

Nothing to migrate: no state or setting changes. The window frame logic keeps the saved frame's name, so book windows
open where they were left.
