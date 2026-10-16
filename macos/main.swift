// Forzac for macOS: an AppKit shell over forzac-core.
//
// One window per book; books open from Finder ("Open With", double-click, the Dock icon), `⌘O` or the command line.
// Started without a book, a window lists the recent books. Positions, recent books and settings are remembered in
// ~/Library/Application Support/Forzac/state.
//
// Spike flags: --measure (time to the first presented page), --png out.png (first page), --chapter N, --page N (the
// page of the chapter --png saves), --panel settings|contents|note:ID and --theme N (with --png: the whole window with
// that panel open, in that theme), --scroll (in "Scroll"); with any of them the reading state is neither read nor written.

import AppKit

let processStart = Date()

let helpText = """
j  l  →  ↓  Space  PgDn  Return    next page (Scroll: j ↓ a line, ⌃D half a screen)
k  h  ←  ↑  Shift+Space  PgUp    previous page (Scroll: k ↑ a line, ⌃U half a screen)
m    pages or scroll
]  [    next / previous chapter
g  G  Home  End    chapter start / end
o    contents  (/ filters)
Tab  Shift+Tab    notes and links on the page, Return opens
⌃O  ⌘[    back after a jump
s  ⌘,    settings
/  ⌘F    search;  n  N  ⌘G  ⇧⌘G    next / previous match
+  −  0    text size, default size
t  T    theme
f  ⌃⌘F    full screen
?    this help
q  ⌘W    close

Forzac is free. Support it: forzac.truenrth.app
"""

/// Whether macOS shows a dark appearance.
func systemIsDark() -> Bool { NSApp.effectiveAppearance.bestMatch(from: [.aqua, .darkAqua]) == .darkAqua }

/// A book's window: the page, its panels and the help.
final class Reader: NSObject, NSWindowDelegate {
    let window: NSWindow
    let view: PageView
    let path: String
    private let container = NSView()
    private let help = NSTextField(labelWithString: helpText)
    private var settingsSheet: SettingsSheet?
    private var contents: ContentsPanel?
    private var search: SearchPanel?
    private weak var note: NoteSheet?
    private let state: OpaquePointer?
    private var saveTimer: Timer?
    var onClose: ((Reader) -> Void)?

    init?(path: String, state: OpaquePointer?, chapter: UInt32? = nil) {
        guard let doc = forzac_open(path) else { return nil }
        self.path = path
        self.state = state
        var at = ForzacLocator(chapter: chapter ?? 0, block: 0, offset: 0)
        if let state, chapter == nil { _ = forzac_state_position(state, doc, &at) }
        // Measurements use the defaults in Light.
        var settings = forzac_state_settings(state)
        if state == nil {
            let args = CommandLine.arguments
            settings.theme = args.firstIndex(of: "--theme").flatMap { Int32(args[$0 + 1]) } ?? 0
            settings.turning = args.contains("--scroll") ? 1 : 0
        }
        view = PageView(doc: doc, at: at, settings: settings, systemDark: systemIsDark())
        let title = String(cString: forzac_title(doc))
        window = NSWindow(contentRect: NSRect(x: 0, y: 0, width: 760, height: 980), styleMask: [.titled, .closable, .miniaturizable, .resizable],
                          backing: .buffered, defer: false)
        super.init()
        window.title = title.isEmpty ? "Forzac" : title
        window.representedURL = URL(fileURLWithPath: path)
        window.tabbingMode = .disallowed
        window.isReleasedWhenClosed = false
        container.wantsLayer = true
        window.contentView = container
        window.delegate = self
        view.frame = container.bounds
        view.autoresizingMask = [.width, .height]
        container.addSubview(view)
        help.font = .systemFont(ofSize: 13)
        help.isHidden = true
        help.wantsLayer = true
        help.layer?.backgroundColor = NSColor.windowBackgroundColor.withAlphaComponent(0.95).cgColor
        help.layer?.cornerRadius = 12
        container.addSubview(help)
        view.onCommand = { [weak self] in self?.run($0) }
        view.onMoved = { [weak self] in
            self?.saveTimer?.invalidate()
            self?.saveTimer = Timer.scheduledTimer(withTimeInterval: 1, repeats: false) { _ in self?.savePosition() }
        }
        view.onSettings = { [weak self] in
            guard let self else { return }
            if let state = self.state { _ = withUnsafePointer(to: self.view.settings) { forzac_state_save_settings(state, $0) } }
            self.window.appearance = NSAppearance(named: themes[self.view.theme].paper < 0x808080 ? .darkAqua : .aqua)
        }
        window.appearance = NSAppearance(named: view.palette.paper < 0x808080 ? .darkAqua : .aqua)
        savePosition() // an opened book is a recent book, even if it is closed without turning a page
        NSDocumentController.shared.noteNewRecentDocumentURL(URL(fileURLWithPath: path))
    }

    func show() {
        window.center()
        window.makeKeyAndOrderFront(nil)
        window.makeFirstResponder(view)
    }

    func run(_ command: Command) {
        switch command {
        case .settings:
            if settingsSheet == nil {
                let sheet = SettingsSheet(view: view)
                sheet.pin(in: container)
                settingsSheet = sheet
            }
            closePanels()
            settingsSheet?.open()
        case .contents:
            if contents == nil {
                let panel = ContentsPanel(view: view)
                container.addSubview(panel)
                let share = panel.widthAnchor.constraint(equalTo: container.widthAnchor, multiplier: 0.45)
                share.priority = NSLayoutConstraint.Priority(490) // below the window's own size: the panel never resizes it
                NSLayoutConstraint.activate([
                    panel.topAnchor.constraint(equalTo: container.topAnchor), panel.bottomAnchor.constraint(equalTo: container.bottomAnchor),
                    panel.trailingAnchor.constraint(equalTo: container.trailingAnchor), share,
                    panel.widthAnchor.constraint(greaterThanOrEqualToConstant: 280), panel.widthAnchor.constraint(lessThanOrEqualToConstant: 460),
                ])
                contents = panel
            }
            closePanels()
            contents?.open()
        case .search:
            if search == nil {
                let panel = SearchPanel(view: view)
                container.addSubview(panel)
                let share = panel.widthAnchor.constraint(equalTo: container.widthAnchor, multiplier: 0.45)
                share.priority = NSLayoutConstraint.Priority(490) // below the window's own size: the panel never resizes it
                NSLayoutConstraint.activate([
                    panel.topAnchor.constraint(equalTo: container.topAnchor), panel.bottomAnchor.constraint(equalTo: container.bottomAnchor),
                    panel.trailingAnchor.constraint(equalTo: container.trailingAnchor), share,
                    panel.widthAnchor.constraint(greaterThanOrEqualToConstant: 280), panel.widthAnchor.constraint(lessThanOrEqualToConstant: 460),
                ])
                search = panel
            }
            closePanels()
            search?.open()
        case .note(let id):
            closePanels()
            guard let sheet = NoteSheet(view: view, id: id, windowSize: container.bounds.size) else { return }
            sheet.pin(in: container)
            note = sheet
            window.makeFirstResponder(sheet)
        case .help:
            help.sizeToFit()
            help.setFrameOrigin(NSPoint(x: (container.bounds.width - help.frame.width) / 2, y: (container.bounds.height - help.frame.height) / 2))
            help.isHidden.toggle()
        case .closePanels:
            help.isHidden = true
        }
    }

    private func closePanels() {
        help.isHidden = true
        settingsSheet?.isHidden = true
        contents?.isHidden = true
        search?.isHidden = true
        note?.removeFromSuperview()
    }

    /// The whole window as drawn, panels included (`--panel`).
    func savePNG(_ path: String) {
        guard let layer = container.layer ?? view.layer?.superlayer,
              let image = bitmap(container.bounds.size, scale: window.backingScaleFactor, background: nil, { layer.render(in: $0) }),
              let dest = CGImageDestinationCreateWithURL(URL(fileURLWithPath: path) as CFURL, "public.png" as CFString, 1, nil) else { return }
        CGImageDestinationAddImage(dest, image, nil)
        CGImageDestinationFinalize(dest)
        print("saved \(path)")
    }

    func savePosition() {
        saveTimer?.invalidate()
        saveTimer = nil
        guard let state else { return }
        if !forzac_state_save_position(state, view.doc, view.locator) { fputs("cannot save the reading state\n", stderr) }
    }

    func windowWillClose(_ notification: Notification) {
        savePosition()
        onClose?(self)
    }
}

/// The recent books, shown when Forzac starts without a book: `Return` or a double-click opens one.
final class RecentWindow: NSObject, NSTableViewDataSource, NSTableViewDelegate {
    let window: NSWindow
    private let books: [(path: String, title: String, authors: String, progress: Float)]
    private let table = NSTableView()
    private let open: (String) -> Void

    init(state: OpaquePointer, open: @escaping (String) -> Void) {
        var n: UInt32 = 0
        let list = forzac_state_recent(state, 20, &n)!
        books = (0..<Int(n)).map { (String(cString: list[$0].path), String(cString: list[$0].title), String(cString: list[$0].authors), list[$0].progress) }
        self.open = open
        window = NSWindow(contentRect: NSRect(x: 0, y: 0, width: 560, height: 640), styleMask: [.titled, .closable, .miniaturizable, .resizable], backing: .buffered, defer: false)
        super.init()
        window.title = "Forzac"
        window.isReleasedWhenClosed = false
        let hint = label(books.isEmpty ? "Open a book: ⌘O, or drop it on Forzac in the Dock." : "Recent books — Return opens, ⌘O opens another.", interfaceFont(13), 0x6E6E6E)
        hint.textColor = .secondaryLabelColor
        table.addTableColumn(NSTableColumn(identifier: .init("book")))
        table.headerView = nil
        table.rowHeight = 56
        table.dataSource = self
        table.delegate = self
        table.target = self
        table.doubleAction = #selector(openSelected)
        let scroll = NSScrollView()
        scroll.documentView = table
        scroll.hasVerticalScroller = true
        scroll.translatesAutoresizingMaskIntoConstraints = false
        let content = NSView()
        content.addSubview(hint)
        content.addSubview(scroll)
        NSLayoutConstraint.activate([
            hint.topAnchor.constraint(equalTo: content.topAnchor, constant: 16), hint.leadingAnchor.constraint(equalTo: content.leadingAnchor, constant: 20),
            scroll.topAnchor.constraint(equalTo: hint.bottomAnchor, constant: 12), scroll.leadingAnchor.constraint(equalTo: content.leadingAnchor),
            scroll.trailingAnchor.constraint(equalTo: content.trailingAnchor), scroll.bottomAnchor.constraint(equalTo: content.bottomAnchor),
        ])
        window.contentView = content
        if !books.isEmpty { table.selectRowIndexes([0], byExtendingSelection: false) }
        window.center()
        window.makeKeyAndOrderFront(nil)
        window.makeFirstResponder(table)
        let keys = NSEvent.addLocalMonitorForEvents(matching: .keyDown) { [weak self] e in
            guard let self, e.window === self.window, e.keyCode == 0x24 || e.keyCode == 0x4C else { return e }
            self.openSelected()
            return nil
        }
        NotificationCenter.default.addObserver(forName: NSWindow.willCloseNotification, object: window, queue: .main) { _ in
            if let keys { NSEvent.removeMonitor(keys) }
        }
    }

    @objc func openSelected() {
        guard table.selectedRow >= 0 else { return }
        open(books[table.selectedRow].path)
    }

    func numberOfRows(in tableView: NSTableView) -> Int { books.count }

    func tableView(_ tableView: NSTableView, viewFor tableColumn: NSTableColumn?, row: Int) -> NSView? {
        let b = books[row]
        let title = label(b.title.isEmpty ? URL(fileURLWithPath: b.path).lastPathComponent : b.title, bookFont("PT Serif", 15, bold: true), 0)
        title.textColor = .labelColor
        title.lineBreakMode = .byTruncatingTail
        let detail = label([b.authors, "\(Int((b.progress * 100).rounded()))% read"].filter { !$0.isEmpty }.joined(separator: " · "), interfaceFont(12), 0)
        detail.textColor = .secondaryLabelColor
        let column = NSStackView(views: [title, detail])
        column.orientation = .vertical
        column.alignment = .leading
        column.spacing = 2
        column.edgeInsets = NSEdgeInsets(top: 8, left: 16, bottom: 8, right: 16)
        return column
    }
}

final class AppDelegate: NSObject, NSApplicationDelegate, NSMenuDelegate {
    let args = CommandLine.arguments
    /// The reading state; nil for measurement runs, which neither read nor write it.
    var state: OpaquePointer?
    var readers: [Reader] = []
    var recent: RecentWindow?
    private var appearance: NSKeyValueObservation?
    private var launched = false

    func value(after flag: String) -> String? {
        guard let i = args.firstIndex(of: flag), i + 1 < args.count else { return nil }
        return args[i + 1]
    }

    var measuring: Bool { ["--measure", "--png", "--chapter", "--page", "--panel", "--scroll"].contains { args.contains($0) } }

    func applicationWillFinishLaunching(_ note: Notification) {
        if !measuring {
            let support = FileManager.default.urls(for: .applicationSupportDirectory, in: .userDomainMask)[0]
            state = forzac_state_open(support.appendingPathComponent("Forzac/state").path)
        }
        buildMenu()
    }

    func applicationDidFinishLaunching(_ note: Notification) {
        launched = true
        let png = value(after: "--png")
        if let path = args.dropFirst().first(where: { !$0.hasPrefix("--") && $0 != png && UInt32($0) == nil && !$0.hasPrefix("-NS") }) {
            guard let reader = open(path, chapter: value(after: "--chapter").flatMap { UInt32($0) }) else {
                fputs("\(path): not a readable book\n", stderr)
                exit(2)
            }
            if args.contains("--measure") || png != nil { measure(reader, png: png) }
        }
        appearance = NSApp.observe(\.effectiveAppearance) { [weak self] _, _ in
            let dark = systemIsDark()
            self?.readers.forEach { $0.view.setSystemDark(dark) }
        }
        // Books from Finder arrive through `application(_:open:)`; without any, the recent books.
        DispatchQueue.main.async { [weak self] in
            guard let self, self.readers.isEmpty, let state = self.state else { return }
            self.showRecent(state)
        }
        NSApp.activate(ignoringOtherApps: true)
    }

    func application(_ application: NSApplication, open urls: [URL]) {
        for url in urls { _ = open(url.path) }
    }

    @discardableResult
    func open(_ path: String, chapter: UInt32? = nil) -> Reader? {
        if let reader = readers.first(where: { $0.path == path }) {
            reader.window.makeKeyAndOrderFront(nil)
            return reader
        }
        guard let reader = Reader(path: path, state: state, chapter: chapter) else {
            if launched {
                let alert = NSAlert()
                alert.messageText = "Not a readable book"
                alert.informativeText = URL(fileURLWithPath: path).lastPathComponent
                alert.runModal()
            }
            return nil
        }
        reader.onClose = { [weak self] r in
            self?.readers.removeAll { $0 === r }
            if self?.readers.isEmpty == true, let state = self?.state { DispatchQueue.main.async { self?.showRecent(state) } }
        }
        readers.append(reader)
        reader.show()
        recent?.window.close()
        recent = nil
        return reader
    }

    func showRecent(_ state: OpaquePointer) {
        recent = RecentWindow(state: state) { [weak self] path in self?.open(path) }
    }

    func measure(_ reader: Reader, png: String?) {
        let parsed = Date().timeIntervalSince(processStart)
        reader.view.onFirstFrame = { v in
            if self.args.contains("--measure") {
                print(String(format: "startup: parsed %.1f ms, first page presented %.1f ms, pages in chapter %d", parsed * 1000,
                             Date().timeIntervalSince(processStart) * 1000, v.pageCount))
            }
            if let p = self.value(after: "--page").flatMap({ UInt32($0) }) { v.page = min(p, v.pageCount - 1) }
            guard let png, let panel = self.value(after: "--panel") ?? (self.args.contains("--scroll") ? "" : nil) else {
                if let png { v.savePNG(png) }
                return NSApp.terminate(nil)
            }
            v.show(animated: 0)
            if !panel.isEmpty { reader.run(panel == "settings" ? .settings : panel == "contents" ? .contents : .note(String(panel.dropFirst(5)))) }
            DispatchQueue.main.asyncAfter(deadline: .now() + 0.5) {
                reader.savePNG(png)
                NSApp.terminate(nil)
            }
        }
    }

    func applicationShouldTerminateAfterLastWindowClosed(_ app: NSApplication) -> Bool { true }

    func applicationWillTerminate(_ notification: Notification) {
        readers.forEach { $0.savePosition() }
    }

    // MARK: Menus

    private var key: Reader? { readers.first { $0.window.isKeyWindow } ?? readers.first }

    @objc func openDocument(_ sender: Any?) {
        let panel = NSOpenPanel()
        panel.allowsMultipleSelection = true
        panel.allowedContentTypes = []
        panel.allowsOtherFileTypes = true
        if panel.runModal() == .OK { panel.urls.forEach { open($0.path) } }
    }

    @objc func showSettings(_ sender: Any?) { key?.run(.settings) }
    @objc func showContents(_ sender: Any?) { key?.run(.contents) }
    @objc func find(_ sender: Any?) { key?.run(.search) }
    @objc func findNext(_ sender: Any?) { key?.view.nextMatch(1) }
    @objc func findPrevious(_ sender: Any?) { key?.view.nextMatch(-1) }
    @objc func toggleScroll(_ sender: Any?) { key?.view.step(6, 1) }
    @objc func nextTheme(_ sender: Any?) { key?.view.step(0, 1) }
    @objc func biggerText(_ sender: Any?) { key?.view.step(2, 1) }
    @objc func smallerText(_ sender: Any?) { key?.view.step(2, -1) }
    @objc func defaultText(_ sender: Any?) {
        guard let v = key?.view else { return }
        var s = v.settings
        s.size = forzac_settings_default().size
        v.apply(s)
    }
    @objc func nextChapter(_ sender: Any?) { key?.view.jumpChapter(1) }
    @objc func previousChapter(_ sender: Any?) { key?.view.jumpChapter(-1) }
    @objc func goBack(_ sender: Any?) { key?.view.goBack() }
    @objc func showHelp(_ sender: Any?) { key?.run(.help) }
    @objc func openRecent(_ sender: NSMenuItem) { if let path = sender.representedObject as? String { open(path) } }

    /// Open Recent: the reading state's recent books.
    func menuNeedsUpdate(_ menu: NSMenu) {
        menu.removeAllItems()
        guard let state else { return }
        var n: UInt32 = 0
        let list = forzac_state_recent(state, 15, &n)!
        for i in 0..<Int(n) {
            let title = String(cString: list[i].title)
            let path = String(cString: list[i].path)
            let item = NSMenuItem(title: title.isEmpty ? URL(fileURLWithPath: path).lastPathComponent : title, action: #selector(openRecent(_:)), keyEquivalent: "")
            item.representedObject = path
            item.target = self
            menu.addItem(item)
        }
    }

    private func buildMenu() {
        let main = NSMenu()
        func submenu(_ title: String, _ items: [NSMenuItem]) {
            let item = NSMenuItem()
            let menu = NSMenu(title: title)
            items.forEach { menu.addItem($0) }
            item.submenu = menu
            main.addItem(item)
        }
        func item(_ title: String, _ action: Selector, _ key: String = "", _ modifiers: NSEvent.ModifierFlags = .command, target: AnyObject? = nil) -> NSMenuItem {
            let i = NSMenuItem(title: title, action: action, keyEquivalent: key)
            i.keyEquivalentModifierMask = modifiers
            i.target = target ?? self
            return i
        }
        submenu("Forzac", [
            item("About Forzac", #selector(NSApplication.orderFrontStandardAboutPanel(_:)), target: NSApp),
            .separator(),
            item("Settings…", #selector(showSettings(_:)), ","),
            .separator(),
            item("Hide Forzac", #selector(NSApplication.hide(_:)), "h", target: NSApp),
            item("Quit Forzac", #selector(NSApplication.terminate(_:)), "q", target: NSApp),
        ])
        let recentItem = NSMenuItem(title: "Open Recent", action: nil, keyEquivalent: "")
        let recentMenu = NSMenu(title: "Open Recent")
        recentMenu.delegate = self
        recentItem.submenu = recentMenu
        let close = NSMenuItem(title: "Close", action: #selector(NSWindow.performClose(_:)), keyEquivalent: "w")
        submenu("File", [item("Open…", #selector(openDocument(_:)), "o"), recentItem, .separator(), close])
        submenu("Edit", [
            item("Find…", #selector(find(_:)), "f"), item("Find Next", #selector(findNext(_:)), "g"),
            item("Find Previous", #selector(findPrevious(_:)), "g", [.command, .shift]),
        ])
        let fullScreen = NSMenuItem(title: "Enter Full Screen", action: #selector(NSWindow.toggleFullScreen(_:)), keyEquivalent: "f")
        fullScreen.keyEquivalentModifierMask = [.control, .command]
        submenu("View", [
            item("Contents", #selector(showContents(_:))), item("Pages or Scroll", #selector(toggleScroll(_:))), item("Next Theme", #selector(nextTheme(_:))),
            .separator(),
            item("Bigger Text", #selector(biggerText(_:)), "+"), item("Smaller Text", #selector(smallerText(_:)), "-"), item("Default Size", #selector(defaultText(_:)), "0"),
            .separator(), fullScreen,
        ])
        submenu("Go", [
            item("Next Chapter", #selector(nextChapter(_:))), item("Previous Chapter", #selector(previousChapter(_:))),
            item("Back", #selector(goBack(_:)), "["),
        ])
        let window = NSMenuItem()
        let windowMenu = NSMenu(title: "Window")
        windowMenu.addItem(withTitle: "Minimize", action: #selector(NSWindow.performMiniaturize(_:)), keyEquivalent: "m")
        window.submenu = windowMenu
        main.addItem(window)
        NSApp.windowsMenu = windowMenu
        submenu("Help", [item("Keys", #selector(showHelp(_:)), "?")])
        NSApp.mainMenu = main
    }
}

forzac_set_fallback(fallback)
forzac_set_assets(assetsDir)
let app = NSApplication.shared
app.setActivationPolicy(.regular)
let delegate = AppDelegate()
app.delegate = delegate
app.run()
