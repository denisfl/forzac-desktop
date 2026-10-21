// The settings window (`settings`): the interface language, and how syncing works with its state, the other devices
// and "Sync Now". A window of its own in the system's appearance, with AppKit's controls; Esc and ⌘W close it.

import AppKit
import SystemConfiguration

/// The computer's name as Sharing settings show it: how the other devices name this one (`sync`).
func computerName() -> String { SCDynamicStoreCopyComputerName(nil, nil) as String? ?? "" }

private let width: CGFloat = 420

/// Esc closes the window.
private final class EscWindow: NSWindow {
    override func cancelOperation(_ sender: Any?) { performClose(sender) }
}

final class SettingsWindow: NSObject, NSWindowDelegate {
    let window: NSWindow = EscWindow(contentRect: .zero, styleMask: [.titled, .closable], backing: .buffered, defer: false)
    private let state: OpaquePointer
    private let folder = NSTextField(labelWithString: "")
    private let status = NSTextField(labelWithString: "")
    private let devices = NSStackView()
    private let syncNow = NSButton(title: tr("Sync Now"), target: nil, action: nil)
    var onChangeFolder: (() -> Void)?
    var onSyncNow: (() -> Void)?
    var onClose: (() -> Void)?

    init(state: OpaquePointer) {
        self.state = state
        super.init()
        window.title = tr("Settings")
        window.isReleasedWhenClosed = false
        window.delegate = self

        let language = NSPopUpButton()
        // The languages in their own words, whatever the interface's.
        language.addItems(withTitles: [tr("As the system"), "English", "Русский"])
        language.selectItem(at: Int(forzac_language_choice(state)))
        language.target = self
        language.action = #selector(chooseLanguage(_:))

        let change = NSButton(title: tr("Change…"), target: self, action: #selector(changeFolder(_:)))
        folder.lineBreakMode = .byTruncatingMiddle
        folder.setContentCompressionResistancePriority(.defaultLow, for: .horizontal)
        let folderColumn = NSStackView(views: [caption(tr("Books folder")), folder])
        folderColumn.orientation = .vertical
        folderColumn.alignment = .leading
        folderColumn.spacing = 2
        let folderRow = NSStackView(views: [folderColumn, change])
        folderRow.alignment = .centerY
        folderRow.distribution = .fill
        folderRow.widthAnchor.constraint(equalToConstant: width).isActive = true

        devices.orientation = .vertical
        devices.alignment = .leading
        devices.spacing = 2
        syncNow.target = self
        syncNow.action = #selector(sync(_:))

        let stack = NSStackView(views: [
            heading(tr("Language")), language, note(tr("Forzac will speak the chosen language after a restart.")),
            heading(tr("Sync")), note(tr("Your place, bookmarks, highlights, shelves and book details are the same in Forzac on every computer and phone that shares the Books folder. Keep the folder in sync with Dropbox, iCloud Drive or Syncthing; on Android only Syncthing keeps a folder on the phone.")),
            folderRow, status, caption(tr("Other devices")), devices, syncNow,
        ])
        stack.orientation = .vertical
        stack.alignment = .leading
        stack.spacing = 8
        stack.edgeInsets = NSEdgeInsets(top: 20, left: 24, bottom: 24, right: 24)
        // Room between the sections and around the folder.
        stack.setCustomSpacing(24, after: stack.views[2])
        stack.setCustomSpacing(16, after: stack.views[4])
        stack.setCustomSpacing(12, after: folderRow)
        stack.setCustomSpacing(16, after: devices)
        window.contentView = stack
        window.setContentSize(stack.fittingSize)
        window.center()
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
