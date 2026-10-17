## 1. Icon

- [x] 1.1 `scripts/icons.py`: the Android vector drawables to SVG, the Linux rounded square (hicolor SVG and PNGs
      16–512) and `macos/Forzac.icns`; outputs committed (`.gitignore` exception for `data/icons/`)
- [x] 1.2 `gtk::Window::set_default_icon_name` in `gtk/src/main.rs`; default window sizes clamped to 85% of the monitor

## 2. Packages

- [x] 2.1 `scripts/package-linux.sh`: the icons in `share/icons/hicolor`
- [x] 2.2 `packaging/aur/forzac/` (PKGBUILD from source, `forzac.install` with the hint), replacing `data/PKGBUILD`;
      licence `MPL-2.0`, icons, all font licences
- [x] 2.3 `packaging/aur/forzac-bin/` from the release archives (x86_64, aarch64), `provides`/`conflicts` `forzac`
- [x] 2.4 `scripts/package-deb.sh`: the staged tree under `/usr`, `control`, `copyright`, `postinst` with the hint;
      `dpkg-deb --root-owner-group`
- [x] 2.5 Release workflow: the `.deb` job (arm64, Debian 13 container) next to the archives

## 3. Checks

- [x] 3.1 `ci/arch-check.sh`: `makepkg -si` for `forzac`, `desktop-file-validate`, `xdg-mime default` and `query
      default`, types of `.fb2`, `.fb2.zip`, `.fb3`, `.epub`; then `forzac-bin` from a locally made archive; run it in
      the Arch x86_64 container
- [x] 3.2 Pi 5: build the `.deb` in the Debian container, `apt install` it, run the hint's command, `xdg-open
      book.fb2` in the labwc session; screenshots of the taskbar and the menu with the icon; one process for two books
- [x] 3.3 `cargo fmt --all --check`, clippy, `cargo test -p forzac-core`, the GTK shell builds

## 4. Docs

- [x] 4.1 README: install (AUR, `.deb`), the default reader, the tree; CLAUDE.md commands
