# Spike: native desktop shells over a Rust core

October 2026. Goal: check the stack chosen in [ADR 0001](adr/0001-stack.md) — a Rust typesetting core, a GTK 4 shell
for Linux and an AppKit shell for macOS — against the priorities: speed, text and animation quality, battery life.

## What exists

- `core/` — FB2 / `.fb2.zip` parser (quick-xml, encoding from the XML declaration), typographic clean-up (no-break
  spaces after short Russian words and before em dashes), Liang hyphenation on hyph-utf8 patterns (`ru`, `en-us`),
  shaping with HarfRust over the bundled PT Serif, Knuth–Plass line breaking, pagination with the Android app's rules,
  pages as positioned glyph runs, a C ABI for Swift. 9 unit tests.
- `gtk/` — a window per book, pages as `GskTextNode`s cached per page, a 200 ms slide on the frame clock, keyboard
  shortcuts, six themes, text size presets, help on `?` with the single "support" line. Spike flags: `--measure`,
  `--png`, `--turns N`, `--chapter N`.
- `macos/` — the same reader in AppKit: Core Text draws the glyph runs into a bitmap at the backing scale, Core
  Animation slides the pages, keys by physical key code, standard menu (⌘Q, ⌘W, ⌃⌘F). Flags `--measure`, `--png`,
  `--chapter N`.
- `ci/` — a Fedora container (GTK 4.22, aarch64) that builds the GTK shell and runs it under a headless Weston.
- `scripts/measure.sh` — the same measurements on a real Linux desktop.
- `data/` — `.desktop` file, a MIME type for FB3, a sketch of the AUR `PKGBUILD`.

Not in the spike: EPUB and FB3, images, tables, formulas, footnotes and links, selection, search, contents, the
scroll mode, persistence of the reading position, glyph fallback for characters PT Serif lacks.

## Results

Book: Vonnegut, *Cat's Cradle*, `.fb2.zip`, 128 chapters (and three more books for the core numbers).

### Core (macOS, Apple Silicon, release build)

| | |
|---|---|
| Parse a book | 5–26 ms |
| Load Russian hyphenation patterns | 1–2 ms |
| Typeset a whole book (warm caches) | 118–348 ms for 173–461 pages, about 0.6 ms per page |
| Glyph runs of one page | 3–5 µs |

A chapter is typeset in a few milliseconds, so a re-layout after a change of size or window width needs no
background thread.

### macOS shell (Apple Silicon, Retina)

| | |
|---|---|
| Launch to the first presented page | ≈230 ms warm (≈430 ms first run); most of it is AppKit launching before `applicationDidFinishLaunching` |
| CPU while idle, 10 s | 0.03 s |
| Memory (RSS) | 26 MB |
| Binary | 6.2 MB with fonts and hyphenation patterns |

Page turns run in the window server (Core Animation): the app does no work per frame.

### GTK shell (container: Fedora, GTK 4.22, aarch64, headless Weston, software rendering)

| | |
|---|---|
| Start to the first presented page | 125–170 ms warm (1.1–1.8 s first run: shader and font caches) |
| CPU while idle, 10 s | 0 ticks |
| Page turns at 60 Hz (software GL) | p50 17 ms; missed frames are expected on llvmpipe and say nothing about hardware |
| Memory (RSS) | 250 MB with Mesa llvmpipe; to be measured with a real GPU |

The rendered pages (`--png`) were checked by eye at scale 1 and 2: justified lines, hyphens drawn, glyph positions
identical to the macOS shell (the core breaks lines the same way on both).

### Raspberry Pi 5 (real hardware)

Pi 5 Model B (8 GB), Raspberry Pi OS 13 (trixie), GTK 4.18.6, Mesa V3D 7.1 (OpenGL ES, Vulkan 1.3), powered from a Mac's
USB-C port (no throttling). No monitor: `scripts/pi-headless.sh` starts a second labwc with a headless output rendered
on the GPU (GLES2), set to 1920×1200 at 120 Hz; the binary was built in `ci/debian.Dockerfile` (same base).

| | `ngl` | `vulkan` | `gl` |
|---|---|---|---|
| Start to the first presented page (warm) | 139–165 ms | 152–268 ms | 138–177 ms |
| 20 animated page turns at 120 Hz | 485–493 frames, 1–4 missed | 489–498 frames, 2–4 missed | 0 missed at the 60 Hz threshold; not rerun at 120 |
| Same at 60 Hz (`ngl`) | 241 frames, 0 missed | | |

- First run ever: 2.6 s (shader and font caches); then ~140 ms.
- Idle: 0 CPU ticks in 10 s; RSS 100 MB with the V3D driver.
- Text at scales 1, 1.25, 1.6, 2 (screenshots with `grim`): crisp, no compositor scaling.
- All GSK renderers work on this Mesa; the GTK default is fine. No crash or regression like the ones reported for the
  Pi 4.
- At 120 Hz 2–4 of ~495 frames per twenty turns come late, none at 60 Hz. All of them are the second frame of a turn
  (`--turns` prints the index): the frame clock waking up from idle takes two vsyncs before the animation runs at
  full rate. Frames inside the animation never miss. So it is ~8 ms of extra input latency at the start of a turn, not
  a stutter in the motion. Building the neighbouring pages ahead of time did not change it (tried and dropped). The
  animation now starts its clock at its first frame; before, a stale frame time after a pause made the first frame
  jump ahead.
- `--turns` now takes the vsync period from the median frame interval: GTK reported 16.67 ms on the 120 Hz output.

### Linux distributions in containers

No Omarchy machine or Raspberry Pi was at hand, so the Linux side was checked in containers on an Apple Silicon Mac
(OrbStack). Rendering is software (Mesa llvmpipe / pixman), so frame timings and memory say nothing about hardware;
behaviour, packaging and the look of text at fractional scales do carry over.

| Check | Result |
|---|---|
| Arch Linux x86_64 (as Omarchy; GTK 4.24.1, `ci/arch.Dockerfile`): `makepkg` from `data/PKGBUILD`, install | builds and installs; 4.5 MiB; depends on `gtk4`, `fontconfig`, `perl-file-mimeinfo` |
| `desktop-file-validate` | valid |
| Hyprland 0.56 headless | **does not start without a DRM render node** (`CBackend::create() failed`); the container kernel has no `/dev/dri` and no `vgem`. Needs a VM with virtio-gpu or real hardware |
| sway 1.12 headless (wlroots, pixman), scales 1, 1.25, 1.6, 2 | runs; the window gets the right logical size (1600×1000 → 1280×800, 1000×625, 800×500); screenshots (`grim`) show crisp text at 1.25 and 1.6: GTK renders at the fractional scale through `wp_fractional_scale_v1`, nothing is stretched by the compositor. Hyprland speaks the same protocol, but its own behaviour is unverified |
| Wayland `app_id` | was `forzac` (the binary name); now `app.truenrth.forzac`, matching the `.desktop` file (`glib::set_prgname`) |
| Raspberry Pi OS userland (Debian trixie arm64, GTK 4.18.6, `ci/debian.Dockerfile`) | builds and runs; Debian's rustc 1.85 is too old for gtk-rs 0.22 (needs 1.92), so builds use rustup — the binary itself only needs the system GTK |

### A Hyprland VM on this Mac: not stable

To get a DRM device for Hyprland, a Fedora aarch64 VM was run on Apple's Virtualization framework (virtio-gpu,
NAT, cloud-init; a 150-line Swift runner) on an M1 with macOS 14.5. It did not get as far as Hyprland:

- The guest kernel oopsed within minutes on Fedora 44 (6.19) and Fedora 43 (6.17): a jump to address 0 from JIT-ed
  seccomp BPF, undefined-instruction panics, then slab corruption and garbage SELinux classes during package
  downloads. `arm64.nopauth arm64.nobti` and dropping the vsock device did not help.
- This matches reports of random guest crashes on Apple Virtualization ([UTM #4840](https://github.com/utmapp/UTM/issues/4840),
  [Apple forums](https://developer.apple.com/forums/thread/708063)); a newer macOS or UTM's QEMU backend may behave
  differently. Not pursued further: real hardware is the better test.
- Separately, the host could not reach the VM's NAT address at all (several VPN tunnels on the machine); keep that in
  mind for SSH to a Raspberry Pi over USB or Wi-Fi from this Mac.

### `xdg-open` and the default reader

`xdg-mime default app.truenrth.forzac.desktop …` writes the defaults correctly, and GIO (Nautilus, and any GTK file
chooser) types books by name: `.fb2` → `application/x-fictionbook+xml`, `.fb2.zip` → `application/x-zip-compressed-fb2`,
`.fb3` → `application/x-fb3` (our MIME file). So a double click in the file manager opens Forzac.

`xdg-open` on Hyprland is different: outside GNOME and KDE, xdg-utils types files with `mimetype` (from
`perl-file-mimeinfo`) if present, else with `file`, which looks at content: `.fb2` becomes `text/xml` — **Omarchy opens
that in nvim** — and `.fb2.zip` becomes `application/zip`. With `perl-file-mimeinfo` installed and
`/usr/bin/vendor_perl` in `PATH` (Arch adds it in login shells), all three resolve to Forzac. The package now depends on
`perl-file-mimeinfo`; whether a Hyprland session on Omarchy has `vendor_perl` in `PATH` is to be checked.

### Found along the way

- `PangoGlyphString::set_size` leaves new entries uninitialised; a stray `is_color` bit made GSK draw some glyphs as
  colour glyphs, i.e. invisible. The shell now sets the visual attributes explicitly (`gtk/src/page_view.rs`).
- GTK and the core must use the very same font files. The shell gives Pango a fontconfig configuration with only the
  bundled faces; a system PT Serif of another version would have other glyph ids.
- Ragged-right text (verse, headings) uses Knuth's ragged-right glue, so it gets optimal breaks too rather than the
  first-fit fallback.

## Not verified yet — needs hardware

Run `scripts/measure.sh book.fb2.zip` and send the output:

1. **Omarchy laptop** at its native refresh rate (ideally 120 Hz) and fractional scale (1.25 / 1.6): startup, turn
   frames (`missed` should be 0), idle CPU, RSS; the look of text at that scale.
2. **Raspberry Pi 5 with a monitor** (done headless above): the look of text on a real display and Hyprland itself
   (not packaged for Raspberry Pi OS).
3. **Russian keyboard layout** on Hyprland: `j`/`k` should still turn pages (GTK matches shortcuts in the first
   layout group too). Containers cannot test this: injected keys (`wtype`) come with their own keymap.
4. **Hyprland itself**: decorations off, fractional scale, `xdg-open book.fb2` (see above).
5. **macOS**: page turn smoothness on a ProMotion display; keys in a Russian layout (they use key codes).

## Next steps

1. Measurements above; pick the GTK renderer for the Pi if the default misbehaves.
2. EPUB and FB3 in the core (port from the Android app), images, footnotes, glyph fallback.
3. Reading position and recent books (SQLite in the core), contents (`o`), search (`/`), the scroll mode.
4. Packaging: AUR, a notarised macOS build, a release CI for x86_64 and aarch64.
5. Licence choice before the first public commit.
