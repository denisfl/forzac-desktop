# Installing Forzac on a Mac

The text for the download page of forzac.truenrth.app. Checked on macOS 14.5; the macOS 15 steps follow Apple's
[guide](https://support.apple.com/guide/mac-help/open-a-mac-app-from-an-unknown-developer-mh40616/mac) and still need
a check on a macOS 15 Mac before publishing.

---

**Forzac for Mac** — version X.Y.Z, for Apple Silicon (M1 and later), macOS 14 or newer.

[Download Forzac-X.Y.Z-macos-arm64.dmg](…) · [SHA-256](…)

1. Open the downloaded `.dmg` and drag **Forzac** onto **Applications**.
2. Open Forzac from Applications. The first time, macOS stops it: Forzac is free and is not registered with Apple
   (that costs a yearly fee), so macOS cannot check it. You allow it once; after that Forzac opens like any app.

**macOS 14 Sonoma.** In Applications, Control-click (or right-click) Forzac and choose **Open**, then **Open** again
in the dialog.

**macOS 15 Sequoia and later.**

1. Open Forzac. macOS says that Forzac was not opened; click **Done**.
2. Open **System Settings › Privacy & Security** and scroll down to Security: next to "Forzac was blocked to protect
   your Mac" click **Open Anyway**.
3. Click **Open Anyway** in the dialog and enter your login password.

**Or in Terminal**, on any version:

```bash
xattr -dr com.apple.quarantine /Applications/Forzac.app
```

**Checking the download.** In Terminal, in the folder with the `.dmg` and the `.sha256` file:

```bash
shasum -a 256 -c Forzac-X.Y.Z-macos-arm64.dmg.sha256
```

It prints `OK` when the file is the one published here.

Forzac opens FB2 (`.fb2`, `.fb2.zip`), FB3 and EPUB books: double-click a book after choosing Forzac in Finder's
**Get Info › Open with › Change All…**, drop books on the Dock icon, or press `⌘O`.
