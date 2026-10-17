// Forzac for macOS: an AppKit shell over forzac-core.
//
// One window per book; books open from Finder ("Open With", double-click, the Dock icon), `⌘O`, the library or the
// command line. Started without a book, the library shows (`Library.swift`). Positions, the library's books and settings
// are remembered in ~/Library/Application Support/Forzac/state (`--state PATH`: another file), cover thumbnails in
// ~/Library/Caches/Forzac/covers.
//
// Spike flags: --measure (time to the first presented page), --png out.png (first page), --chapter N, --page N (the
// page of the chapter --png saves), --panel settings|contents|bookmarks|highlights|help|note:ID|image:ID|table:CHAPTER.BLOCK and --theme N (with
// --png: the whole window with that panel or view open, in that theme; --keys "ll+\t\r" then sends those keys to it
// first, 0.3 s apart: letters, digits, + - =, \t Tab, \r Return, \e Esc, an upper-case letter with Shift; --mouse
// "down X Y; drag X Y; up X Y; click X Y N" then sends those mouse events to the page, 0.15 s apart, at points of the
// page, y down), --scroll (in "Scroll"), --menus (prints the menus' titles and quits); with any of them the reading
// state is neither read nor written. --library shows the library from the state (with --png: saves it once its covers
// have arrived, after --keys, and quits).

import AppKit

let processStart = Date()

/// `\t`, `\r` and `\e` in a `--keys` value as Tab, Return and Esc.
func unescape(_ s: String) -> String {
    s.replacingOccurrences(of: "\\t", with: "\t").replacingOccurrences(of: "\\r", with: "\r").replacingOccurrences(of: "\\e", with: "\u{1b}")
}

/// Whether macOS shows a dark appearance.
func systemIsDark() -> Bool { NSApp.effectiveAppearance.bestMatch(from: [.aqua, .darkAqua]) == .darkAqua }

/// A book's window: the page, its panels and the help.
final class Reader: NSObject, NSWindowDelegate {
    let window: NSWindow
    let view: PageView
    let path: String
    private let container = NSView()
    private let help = NSTextField(labelWithString: String(cString: forzac_help(1)))
    private var settingsSheet: SettingsSheet?
    private var contents: ContentsPanel?
    private var search: SearchPanel?
    private weak var note: NoteSheet?
    /// The image or table view over the page, while it is open.
    private weak var viewer: Viewer?
    private let state: OpaquePointer?
    private var saveTimer: Timer?
    var onClose: ((Reader) -> Void)?

    /// Why the last `init` failed: the core's reason (`forzac_open`), 0 when it did not.
    static var openError: UInt32 = 0
    /// Keeps the display awake while reading; ended 10 minutes after the last input or when the window resigns.
    private var activity: NSObjectProtocol?
    private var awakeTimer: Timer?
    private var inputMonitor: Any?

    init?(path: String, state: OpaquePointer?, chapter: UInt32? = nil) {
        Reader.openError = 0
        guard let doc = forzac_open(path, &Reader.openError) else { return nil }
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
        window.title = title.isEmpty ? tr("Forzac") : title
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
        if let state {
            forzac_state_load_bookmarks(state, doc)
            forzac_state_load_quotes(state, doc)
        }
        view.reloadBookmarks()
        view.reloadHighlights(redraw: true)
        view.onHighlights = { [weak self] in
            guard let self, let state = self.state else { return }
            if !forzac_state_save_quotes(state, self.view.doc) { fputs("cannot save the reading state\n", stderr) }
        }
        view.onBookmarks = { [weak self] in
            guard let self, let state = self.state else { return }
            if !forzac_state_save_bookmarks(state, self.view.doc) { fputs("cannot save the reading state\n", stderr) }
        }
        view.panelOpen = { [weak self] in self?.panelOpen ?? false }
        view.onMoved = { [weak self] in
            self?.saveTimer?.invalidate()
            self?.saveTimer = Timer.scheduledTimer(withTimeInterval: 1, repeats: false) { _ in self?.savePosition() }
        }
        view.onSettings = { [weak self] in
            guard let self else { return }
            if let state = self.state { _ = withUnsafePointer(to: self.view.settings) { forzac_state_save_settings(state, $0) } }
            self.window.appearance = NSAppearance(named: themes[self.view.theme].paper < 0x808080 ? .darkAqua : .aqua)
            self.viewer?.recolour()
        }
        window.appearance = NSAppearance(named: view.palette.paper < 0x808080 ? .darkAqua : .aqua)
        savePosition() // an opened book is in the library, even if it is closed without turning a page
        NSDocumentController.shared.noteNewRecentDocumentURL(URL(fileURLWithPath: path))
        if state != nil { cacheCover(doc) }
        if state != nil {
            inputMonitor = NSEvent.addLocalMonitorForEvents(matching: [.keyDown, .leftMouseDown, .scrollWheel, .swipe]) { [weak self] e in
                if let self, e.window === self.window { self.touch() }
                return e
            }
        }
    }

    /// Reading: the display stays awake for 10 minutes after this, while the window is in front.
    func touch() {
        guard window.isKeyWindow else { return }
        if activity == nil {
            activity = ProcessInfo.processInfo.beginActivity(options: [.idleDisplaySleepDisabled], reason: tr("Reading a book"))
        }
        awakeTimer?.invalidate()
        awakeTimer = Timer.scheduledTimer(withTimeInterval: 10 * 60, repeats: false) { [weak self] _ in self?.sleepAllowed() }
    }

    private func sleepAllowed() {
        awakeTimer?.invalidate()
        awakeTimer = nil
        if let activity { ProcessInfo.processInfo.endActivity(activity) }
        activity = nil
    }

    func windowDidBecomeKey(_ notification: Notification) { if state != nil { touch() } }
    func windowDidResignKey(_ notification: Notification) { sleepAllowed() }

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
                sheet.isHidden = true
                settingsSheet = sheet
            }
            if let sheet = settingsSheet { toggle(sheet) { sheet.open() } }
        case .contents, .bookmarks, .highlights:
            let tab: ContentsTab = {
                switch command {
                case .bookmarks: return .bookmarks
                case .highlights: return .highlights
                default: return .contents
                }
            }()
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
                panel.isHidden = true
                contents = panel
            }
            guard let panel = contents else { return }
            // Open on another tab: that tab; else open or close, as `o` and `B` do.
            if !panel.isHidden && panel.tab != tab { panel.show(tab) } else { toggle(panel) { panel.open(tab) } }
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
                panel.isHidden = true
                search = panel
            }
            if let panel = search { toggle(panel) { panel.open() } }
        case .note(let id), .preview(let id):
            closePanels()
            var preview = false
            if case .preview = command { preview = true }
            guard let sheet = NoteSheet(view: view, id: id, preview: preview, windowSize: container.bounds.size) else { return }
            sheet.back = viewer
            sheet.pin(in: container)
            note = sheet
            window.makeFirstResponder(sheet)
        case .image(let id):
            openViewer(.image(id))
        case .table(let at):
            openViewer(.table(at))
        case .help:
            help.sizeToFit()
            help.setFrameOrigin(NSPoint(x: (container.bounds.width - help.frame.width) / 2, y: (container.bounds.height - help.frame.height) / 2))
            help.isHidden.toggle()
        case .closePanels:
            closePanels()
            window.makeFirstResponder(view)
        }
    }

    /// Shows an image or a table over the page, filling the window.
    private func openViewer(_ subject: Viewer.Subject) {
        closePanels()
        guard viewer == nil, let v = Viewer(page: view, subject: subject) else { return }
        v.frame = container.bounds
        v.autoresizingMask = [.width, .height]
        container.addSubview(v, positioned: .below, relativeTo: help)
        viewer = v
        window.makeFirstResponder(v)
        v.show()
    }

    /// Shows `panel` (closing the others), or closes it when it is open: `s`, `o` and `/` open and close their panels.
    private func toggle(_ panel: NSView, open: () -> Void) {
        let wasOpen = !panel.isHidden
        closePanels()
        if wasOpen { window.makeFirstResponder(view) } else { open() }
    }

    /// Whether a panel or the help is over the page: a click on the page then closes it.
    var panelOpen: Bool {
        [settingsSheet, contents, search].contains { $0?.isHidden == false } || note?.superview != nil || !help.isHidden
    }

    private func closePanels() {
        help.isHidden = true
        settingsSheet?.isHidden = true
        contents?.isHidden = true
        search?.isHidden = true
        note?.removeFromSuperview()
    }

    /// A key press as the keyboard would send it to what has the keys (`--keys`).
    func send(_ key: Character) {
        let codes: [Character: UInt16] = ["\t": 0x30, "\r": 0x24, "\u{1b}": 0x35]
        let text = String(key).lowercased()
        let shift = key.isUppercase
        guard let e = NSEvent.keyEvent(with: .keyDown, location: .zero, modifierFlags: shift ? .shift : [], timestamp: 0, windowNumber: window.windowNumber,
                                       context: nil, characters: text, charactersIgnoringModifiers: text, isARepeat: false, keyCode: codes[key] ?? 0xFFFF)
        else { return }
        window.sendEvent(e)
    }

    /// A mouse event at `(x, y)` of the page (y down) as the mouse would send it (`--mouse`): `down`, `drag`, `up`, or
    /// `click` (a press and a release, the `n`th click in a row).
    func mouse(_ kind: String, _ x: CGFloat, _ y: CGFloat, _ n: Int = 1) {
        let at = view.convert(CGPoint(x: x, y: view.bounds.height - y), to: nil)
        let types: [NSEvent.EventType] = kind == "down" ? [.leftMouseDown] : kind == "drag" ? [.leftMouseDragged] : kind == "up" ? [.leftMouseUp] : [.leftMouseDown, .leftMouseUp]
        for type in types {
            guard let e = NSEvent.mouseEvent(with: type, location: at, modifierFlags: [], timestamp: ProcessInfo.processInfo.systemUptime, windowNumber: window.windowNumber,
                                             context: nil, eventNumber: 0, clickCount: n, pressure: type == .leftMouseUp ? 0 : 1) else { continue }
            window.sendEvent(e)
        }
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
        if !forzac_state_save_position(state, view.doc, view.locator, view.finished) { fputs("cannot save the reading state\n", stderr) }
    }

    func windowWillClose(_ notification: Notification) {
        savePosition()
        sleepAllowed()
        if let inputMonitor { NSEvent.removeMonitor(inputMonitor) }
        onClose?(self)
    }
}

final class AppDelegate: NSObject, NSApplicationDelegate, NSMenuDelegate {
    let args = CommandLine.arguments
    /// The reading state; nil for measurement runs, which neither read nor write it.
    var state: OpaquePointer?
    var readers: [Reader] = []
    var library: LibraryWindow?
    private var appearance: NSKeyValueObservation?

    func value(after flag: String) -> String? {
        guard let i = args.firstIndex(of: flag), i + 1 < args.count else { return nil }
        return args[i + 1]
    }

    var measuring: Bool { ["--measure", "--png", "--chapter", "--page", "--panel", "--scroll", "--menus"].contains { args.contains($0) } }

    func applicationWillFinishLaunching(_ note: Notification) {
        if !measuring || args.contains("--library") {
            let support = FileManager.default.urls(for: .applicationSupportDirectory, in: .userDomainMask)[0]
            state = forzac_state_open(value(after: "--state") ?? support.appendingPathComponent("Forzac/state").path)
        }
        buildMenu()
    }

    func applicationDidFinishLaunching(_ note: Notification) {
        // `--menus`: the menus' titles, to check the interface language, then quit.
        if args.contains("--menus") {
            for top in NSApp.mainMenu?.items ?? [] {
                print("\(top.submenu?.title ?? ""): " + (top.submenu?.items.map { $0.isSeparatorItem ? "|" : $0.title } ?? []).joined(separator: ", "))
            }
            exit(0)
        }
        let png = value(after: "--png")
        // Values of the flags that take one are not books.
        let values = Set(["--png", "--chapter", "--page", "--panel", "--theme", "--keys", "--mouse", "--state", "-AppleLanguages"].compactMap { value(after: $0) })
        if let path = args.dropFirst().first(where: { !$0.hasPrefix("-") && !values.contains($0) && UInt32($0) == nil }) {
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
        if args.contains("--library"), let state {
            showLibrary(state)
            if let png, let library { measureLibrary(library, png: png) }
        }
        // Books from Finder arrive through `application(_:open:)`; without any, the library.
        DispatchQueue.main.async { [weak self] in
            guard let self, self.readers.isEmpty, self.library == nil, let state = self.state else { return }
            self.showLibrary(state)
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
            // In the Android app's words, without PDF.
            let reason = [1: tr("unsupported format — FB2, FB3 or EPUB needed"), 2: tr("the book has no text"),
                          3: tr("the book is DRM-protected and can't be opened"), 4: tr("the file is damaged"), 5: tr("the file is unavailable")][Reader.openError] ?? tr("the file is damaged")
            let message = fill(tr("Couldn't open “{name}”: {reason}"), ["name": URL(fileURLWithPath: path).lastPathComponent, "reason": reason])
            fputs(message + "\n", stderr)
            // Also for a book that arrives with the launch (Finder, the Dock): after it, so the alert has an app to show in.
            if !measuring {
                DispatchQueue.main.async { [weak self] in
                    let alert = NSAlert()
                    alert.messageText = message
                    alert.runModal()
                    if let self, self.readers.isEmpty, let state = self.state { self.showLibrary(state) }
                }
            }
            return nil
        }
        reader.onClose = { [weak self] r in
            self?.readers.removeAll { $0 === r }
            if self?.readers.isEmpty == true, let state = self?.state { DispatchQueue.main.async { self?.showLibrary(state) } }
        }
        readers.append(reader)
        reader.show()
        library?.window.close()
        return reader
    }

    /// The library, or the open one brought forward.
    func showLibrary(_ state: OpaquePointer) {
        if let library {
            library.window.makeKeyAndOrderFront(nil)
            return
        }
        let shown = LibraryWindow(state: state) { [weak self] path in self?.open(path) }
        library = shown
        // Closing the library with no book open quits (the app stays after its last book window: the library follows).
        NotificationCenter.default.addObserver(forName: NSWindow.willCloseNotification, object: shown.window, queue: .main) { [weak self] _ in
            if self?.library === shown { self?.library = nil }
            if self?.readers.isEmpty == true { NSApp.terminate(nil) }
        }
    }

    /// `--library --png`: once the covers have arrived, the `--keys` 0.3 s apart, then the window saved, then quit.
    func measureLibrary(_ library: LibraryWindow, png: String) {
        library.onSettled = { [weak self] in
            print(String(format: "library: %d books, covers loaded %.1f ms", library.bookCount, Date().timeIntervalSince(processStart) * 1000))
            let keys = Array(self?.value(after: "--keys").map(unescape) ?? "")
            for (i, key) in keys.enumerated() {
                DispatchQueue.main.asyncAfter(deadline: .now() + 0.5 + 0.3 * Double(i)) { library.send(key) }
            }
            DispatchQueue.main.asyncAfter(deadline: .now() + 0.8 + 0.3 * Double(keys.count)) {
                (self?.library ?? library).savePNG(png)
                NSApp.terminate(nil)
            }
        }
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
            if !panel.isEmpty { reader.run(self.command(panel)) }
            let keys = Array(self.value(after: "--keys").map(unescape) ?? "")
            for (i, key) in keys.enumerated() {
                DispatchQueue.main.asyncAfter(deadline: .now() + 0.5 + 0.3 * Double(i)) { reader.send(key) }
            }
            let events = (self.value(after: "--mouse") ?? "").split(separator: ";").map { $0.split(separator: " ").map(String.init) }.filter { $0.count >= 3 }
            let start = 0.5 + 0.3 * Double(keys.count)
            for (i, e) in events.enumerated() {
                DispatchQueue.main.asyncAfter(deadline: .now() + start + 0.15 * Double(i)) {
                    reader.mouse(e[0], CGFloat(Double(e[1]) ?? 0), CGFloat(Double(e[2]) ?? 0), e.count > 3 ? Int(e[3]) ?? 1 : 1)
                }
            }
            DispatchQueue.main.asyncAfter(deadline: .now() + 0.8 + 0.3 * Double(keys.count) + 0.15 * Double(events.count)) {
                reader.savePNG(png)
                NSApp.terminate(nil)
            }
        }
    }

    /// The command a `--panel` value names.
    private func command(_ panel: String) -> Command {
        let arg = String(panel.drop { $0 != ":" }.dropFirst())
        switch panel {
        case "settings": return .settings
        case "help": return .help
        case "contents": return .contents
        case "bookmarks": return .bookmarks
        case "highlights": return .highlights
        case _ where panel.hasPrefix("image:"): return .image(arg)
        case _ where panel.hasPrefix("table:"):
            let n = arg.split(separator: ".").compactMap { UInt32($0) }
            return .table(ForzacLocator(chapter: n.first ?? 0, block: n.last ?? 0, offset: 0))
        default: return .note(arg)
        }
    }

    /// Not on its own: after the last book the library shows, and closing it quits (`showLibrary`).
    func applicationShouldTerminateAfterLastWindowClosed(_ app: NSApplication) -> Bool { false }

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
    @objc func showBookmarks(_ sender: Any?) { key?.run(.bookmarks) }
    @objc func showHighlights(_ sender: Any?) { key?.run(.highlights) }
    @objc func toggleBookmark(_ sender: Any?) { key?.view.toggleBookmark() }
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
        submenu(tr("Forzac"), [
            item(tr("About Forzac"), #selector(NSApplication.orderFrontStandardAboutPanel(_:)), target: NSApp),
            .separator(),
            item(tr("Settings…"), #selector(showSettings(_:)), ","),
            .separator(),
            item(tr("Hide Forzac"), #selector(NSApplication.hide(_:)), "h", target: NSApp),
            item(tr("Quit Forzac"), #selector(NSApplication.terminate(_:)), "q", target: NSApp),
        ])
        let recentItem = NSMenuItem(title: tr("Open Recent"), action: nil, keyEquivalent: "")
        let recentMenu = NSMenu(title: tr("Open Recent"))
        recentMenu.delegate = self
        recentItem.submenu = recentMenu
        let close = NSMenuItem(title: tr("Close"), action: #selector(NSWindow.performClose(_:)), keyEquivalent: "w")
        submenu(tr("File"), [item(tr("Open…"), #selector(openDocument(_:)), "o"), recentItem, .separator(), close])
        let copy = NSMenuItem(title: tr("Copy"), action: #selector(NSText.copy(_:)), keyEquivalent: "c")
        submenu(tr("Edit"), [
            copy, .separator(),
            item(tr("Find…"), #selector(find(_:)), "f"), item(tr("Find Next"), #selector(findNext(_:)), "g"),
            item(tr("Find Previous"), #selector(findPrevious(_:)), "g", [.command, .shift]),
        ])
        let fullScreen = NSMenuItem(title: tr("Enter Full Screen"), action: #selector(NSWindow.toggleFullScreen(_:)), keyEquivalent: "f")
        fullScreen.keyEquivalentModifierMask = [.control, .command]
        submenu(tr("View"), [
            item(tr("Contents"), #selector(showContents(_:))), item(tr("Bookmarks"), #selector(showBookmarks(_:))), item(tr("Highlights"), #selector(showHighlights(_:))),
            item(tr("Pages or Scroll"), #selector(toggleScroll(_:))), item(tr("Next Theme"), #selector(nextTheme(_:))),
            .separator(),
            item(tr("Bigger Text"), #selector(biggerText(_:)), "+"), item(tr("Smaller Text"), #selector(smallerText(_:)), "-"), item(tr("Default Size"), #selector(defaultText(_:)), "0"),
            .separator(), fullScreen,
        ])
        submenu(tr("Go"), [
            item(tr("Next Chapter"), #selector(nextChapter(_:))), item(tr("Previous Chapter"), #selector(previousChapter(_:))),
            item(tr("Back"), #selector(goBack(_:)), "["),
            .separator(),
            item(tr("Bookmark"), #selector(toggleBookmark(_:)), "d"),
        ])
        let window = NSMenuItem()
        let windowMenu = NSMenu(title: tr("Window"))
        windowMenu.addItem(withTitle: tr("Minimize"), action: #selector(NSWindow.performMiniaturize(_:)), keyEquivalent: "m")
        window.submenu = windowMenu
        main.addItem(window)
        NSApp.windowsMenu = windowMenu
        submenu(tr("Help"), [item(tr("Keys"), #selector(showHelp(_:)), "?")])
        NSApp.mainMenu = main
    }
}

// The interface language: the user's preferred languages, first that Forzac has.
forzac_set_language(Locale.preferredLanguages.joined(separator: ","))
forzac_set_fallback(fallback)
forzac_set_assets(assetsDir)
let app = NSApplication.shared
app.setActivationPolicy(.regular)
let delegate = AppDelegate()
app.delegate = delegate
app.run()
