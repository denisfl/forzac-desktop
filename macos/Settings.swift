// The settings window (`settings`): two panes in a toolbar as Mac settings have them — "General" with the theme and the
// interface language, "Books" with the Books folder and how syncing works, its state, the other devices and "Sync Now".
// A window of its own in the system's appearance, with AppKit's controls; it reopens on the last pane, titled by it;
// Esc and ⌘W close it.

import AppKit
import SystemConfiguration

/// The computer's name as Sharing settings show it: how the other devices name this one (`sync`).
func computerName() -> String { SCDynamicStoreCopyComputerName(nil, nil) as String? ?? "" }

private let width: CGFloat = 420

private extension NSToolbarItem.Identifier {
    static let general = Self("general")
    static let books = Self("books")
}

final class SettingsWindow: NSObject, NSWindowDelegate, NSToolbarDelegate {
    let window: NSWindow = EscWindow(contentRect: .zero, styleMask: [.titled, .closable], backing: .buffered, defer: false)
    private let state: OpaquePointer
    private let folder = NSTextField(labelWithString: "")
    private let status = NSTextField(labelWithString: "")
    private let devices = NSStackView()
    private let syncNow = NSButton(title: tr("Sync Now"), target: nil, action: nil)
    var onChangeFolder: (() -> Void)?
    var onShowFolder: (() -> Void)?
    var onSyncNow: (() -> Void)?
    /// A theme chosen (`settings`, "Theme"): the reading theme's index.
    var onTheme: ((Int32) -> Void)?
    var onClose: (() -> Void)?
    private var panes: [NSToolbarItem.Identifier: NSView] = [:]
    private var swatches: [ThemeSwatch] = []
    private static let paneKey = "SettingsPane"

    init(state: OpaquePointer) {
        self.state = state
        super.init()
        window.isReleasedWhenClosed = false
        window.delegate = self
        let toolbar = NSToolbar(identifier: "settings")
        toolbar.delegate = self
        toolbar.allowsUserCustomization = false
        toolbar.displayMode = .iconAndLabel
        window.toolbar = toolbar
        window.toolbarStyle = .preference

        // The six themes as "Aa" shows them; the current one marked.
        var settings = forzac_state_settings(state)
        let current = Int(forzac_settings_theme(&settings, systemIsDark()))
        for (i, palette) in themes.enumerated() {
            let swatch = ThemeSwatch(palette: palette, name: themeNames[i])
            swatch.isOn = i == current
            swatch.onChoose = { [weak self] in self?.choose(theme: i) }
            swatches.append(swatch)
        }
        let themeRow = NSStackView(views: swatches)
        themeRow.spacing = 8

        let language = NSPopUpButton()
        // The languages in their own words, whatever the interface's.
        language.addItems(withTitles: [tr("As the system"), "English", "Русский"])
        language.selectItem(at: Int(forzac_language_choice(state)))
        language.target = self
        language.action = #selector(chooseLanguage(_:))

        let change = NSButton(title: tr("Change…"), target: self, action: #selector(changeFolder(_:)))
        let show = NSButton(title: tr("Show"), target: self, action: #selector(showFolder(_:)))
        folder.lineBreakMode = .byTruncatingMiddle
        folder.setContentCompressionResistancePriority(.defaultLow, for: .horizontal)
        let folderColumn = NSStackView(views: [caption(tr("Books folder")), folder])
        folderColumn.orientation = .vertical
        folderColumn.alignment = .leading
        folderColumn.spacing = 2
        let folderRow = NSStackView(views: [folderColumn, show, change])
        folderRow.alignment = .centerY
        folderRow.distribution = .fill
        folderRow.widthAnchor.constraint(equalToConstant: width).isActive = true

        devices.orientation = .vertical
        devices.alignment = .leading
        devices.spacing = 2
        syncNow.target = self
        syncNow.action = #selector(sync(_:))

        let general = pane([heading(tr("Theme")), themeRow, heading(tr("Language")), language, note(tr("Forzac will speak the chosen language after a restart."))])
        general.setCustomSpacing(24, after: themeRow)
        let books = pane([
            heading(tr("Sync")), note(tr("Your place, bookmarks, highlights, shelves, book details and reading stats are the same in Forzac on every computer and phone that shares the Books folder. Keep the folder in sync with Dropbox, iCloud Drive or Syncthing; on Android only Syncthing keeps a folder on the phone.")),
            folderRow, status, caption(tr("Other devices")), devices, syncNow,
        ])
        // Room around the folder and before the button.
        books.setCustomSpacing(16, after: books.views[1])
        books.setCustomSpacing(12, after: folderRow)
        books.setCustomSpacing(16, after: devices)
        panes = [.general: general, .books: books]
        let last = UserDefaults.standard.string(forKey: Self.paneKey).map { NSToolbarItem.Identifier($0) } ?? .general
        select(panes[last] == nil ? .general : last)
        window.center()
    }

    private func pane(_ views: [NSView]) -> NSStackView {
        let stack = NSStackView(views: views)
        stack.orientation = .vertical
        stack.alignment = .leading
        stack.spacing = 8
        stack.edgeInsets = NSEdgeInsets(top: 20, left: 24, bottom: 24, right: 24)
        return stack
    }

    /// Shows a pane, titles the window by it and remembers it for the next opening.
    private func select(_ id: NSToolbarItem.Identifier) {
        guard let pane = panes[id] else { return }
        window.toolbar?.selectedItemIdentifier = id
        window.title = id == .general ? tr("General") : tr("Books")
        window.contentView = pane
        window.setContentSize(pane.fittingSize)
        UserDefaults.standard.set(id.rawValue, forKey: Self.paneKey)
    }

    func toolbarDefaultItemIdentifiers(_ toolbar: NSToolbar) -> [NSToolbarItem.Identifier] { [.general, .books] }
    func toolbarAllowedItemIdentifiers(_ toolbar: NSToolbar) -> [NSToolbarItem.Identifier] { [.general, .books] }
    func toolbarSelectableItemIdentifiers(_ toolbar: NSToolbar) -> [NSToolbarItem.Identifier] { [.general, .books] }

    func toolbar(_ toolbar: NSToolbar, itemForItemIdentifier id: NSToolbarItem.Identifier, willBeInsertedIntoToolbar flag: Bool) -> NSToolbarItem? {
        let item = NSToolbarItem(itemIdentifier: id)
        let (symbol, title) = id == .general ? ("gearshape", tr("General")) : ("books.vertical", tr("Books"))
        item.image = NSImage(systemSymbolName: symbol, accessibilityDescription: title)
        item.label = title
        item.target = self
        item.action = #selector(pick(_:))
        return item
    }

    @objc private func pick(_ sender: NSToolbarItem) { select(sender.itemIdentifier) }

    private func choose(theme: Int) {
        for (i, s) in swatches.enumerated() { s.isOn = i == theme }
        onTheme?(Int32(theme))
    }

    private func heading(_ text: String) -> NSTextField {
        let label = NSTextField(labelWithString: text)
        label.font = .boldSystemFont(ofSize: NSFont.systemFontSize)
        return label
    }

    private func caption(_ text: String) -> NSTextField {
        let label = NSTextField(labelWithString: text)
        label.font = .systemFont(ofSize: NSFont.smallSystemFontSize)
        label.textColor = .secondaryLabelColor
        return label
    }

    private func note(_ text: String) -> NSTextField {
        let label = NSTextField(wrappingLabelWithString: text)
        label.font = .systemFont(ofSize: NSFont.smallSystemFontSize)
        label.textColor = .secondaryLabelColor
        label.preferredMaxLayoutWidth = width
        return label
    }

    func show() {
        window.makeKeyAndOrderFront(nil)
    }

    /// Shows the sync status from `forzac_sync_status`: the Books folder, the state line, then `name\twhen` per device.
    func update(_ text: String) {
        var lines = text.components(separatedBy: "\n")
        let path = lines.removeFirst()
        folder.stringValue = (path as NSString).abbreviatingWithTildeInPath
        folder.toolTip = path
        status.stringValue = lines.isEmpty ? "" : lines.removeFirst()
        syncNow.isEnabled = true
        devices.arrangedSubviews.forEach { $0.removeFromSuperview() }
        for line in lines {
            let parts = line.components(separatedBy: "\t")
            let row = NSMutableAttributedString(string: parts[0], attributes: [.font: NSFont.systemFont(ofSize: NSFont.systemFontSize), .foregroundColor: NSColor.labelColor])
            if parts.count > 1 {
                row.append(NSAttributedString(string: " · " + parts[1], attributes: [.font: NSFont.systemFont(ofSize: NSFont.systemFontSize), .foregroundColor: NSColor.secondaryLabelColor]))
            }
            devices.addArrangedSubview(NSTextField(labelWithAttributedString: row))
        }
        if lines.isEmpty { devices.addArrangedSubview(NSTextField(labelWithString: tr("None yet"))) }
        window.setContentSize(window.contentView!.fittingSize)
    }

    @objc private func showFolder(_ sender: Any?) { onShowFolder?() }

    @objc private func chooseLanguage(_ sender: NSPopUpButton) {
        if !forzac_set_language_choice(state, UInt32(sender.indexOfSelectedItem)) { fputs("cannot save the reading state\n", stderr) }
    }

    @objc private func changeFolder(_ sender: Any?) { onChangeFolder?() }

    @objc private func sync(_ sender: Any?) {
        status.stringValue = tr("Syncing…")
        syncNow.isEnabled = false
        onSyncNow?()
    }

    func windowWillClose(_ notification: Notification) { onClose?() }
}

/// A theme as the "Aa" panel shows it: its paper with "Aa" in its ink, the name under it; a ring when chosen. A button,
/// so full keyboard access reaches it.
final class ThemeSwatch: NSButton {
    private let palette: Palette
    var onChoose: (() -> Void)?
    var isOn = false { didSet { image = render() } }

    init(palette: Palette, name: String) {
        self.palette = palette
        super.init(frame: .zero)
        title = name
        font = .systemFont(ofSize: NSFont.smallSystemFontSize)
        imagePosition = .imageAbove
        isBordered = false
        setButtonType(.momentaryChange)
        target = self
        action = #selector(chosen(_:))
        setAccessibilityLabel(name)
        image = render()
    }

    required init?(coder: NSCoder) { fatalError() }

    @objc private func chosen(_ sender: Any?) { onChoose?() }

    private func render() -> NSImage {
        let size = CGSize(width: 56, height: 44)
        return NSImage(size: size, flipped: false) { [palette, isOn] r in
            let card = NSBezierPath(roundedRect: r.insetBy(dx: 3, dy: 3), xRadius: 10, yRadius: 10)
            nsColor(palette.paper).setFill()
            card.fill()
            (isOn ? NSColor.controlAccentColor : nsColor(palette.line)).setStroke()
            card.lineWidth = isOn ? 2.5 : 1
            card.stroke()
            let text = NSAttributedString(string: "Aa", attributes: [.font: bookFont("Literata", 17), .foregroundColor: nsColor(palette.ink)])
            let t = text.size()
            text.draw(at: CGPoint(x: (r.width - t.width) / 2, y: (r.height - t.height) / 2))
            return true
        }
    }
}
