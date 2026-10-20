// The reading bar of a book window (`macos-reading-bar`), as in Apple Books: the page fills the window under a
// transparent title bar, and a toolbar — the window buttons and back to the library at the left, Contents, Text
// settings, Search, Bookmark at the right — fades in when the pointer moves and out 2 s after it stops, the pointer
// with it. The page's running head and ribbon stay in sight between them. A key or a click on the page hides it at
// once; the reading menu holds it while it is open.

import AppKit

private extension NSToolbarItem.Identifier {
    static let back = Self("back"), contents = Self("contents"), settings = Self("settings"), search = Self("search"),
               bookmark = Self("bookmark")
}

/// The theme's paper under the bar's two groups of buttons, fading out towards the middle over 24 points: a long running
/// head goes out under the buttons instead of running into them.
private final class Edges: NSView {
    let gradient = CAGradientLayer()
    /// How far the left and the right group reach from the window's edges.
    var reach: (left: CGFloat, right: CGFloat) = (0, 0) { didSet { needsLayout = true } }

    override init(frame: NSRect) {
        super.init(frame: frame)
        wantsLayer = true
        gradient.startPoint = CGPoint(x: 0, y: 0.5)
        gradient.endPoint = CGPoint(x: 1, y: 0.5)
        gradient.actions = ["bounds": NSNull(), "position": NSNull(), "locations": NSNull(), "colors": NSNull()]
        layer!.addSublayer(gradient)
    }

    required init?(coder: NSCoder) { fatalError() }

    func colour(_ paper: UInt32) { gradient.colors = [color(paper), color(paper), color(paper, alpha: 0), color(paper, alpha: 0), color(paper), color(paper)] }

    override func layout() {
        super.layout()
        gradient.frame = bounds
        let w = max(bounds.width, 1)
        let stops = [0, reach.left, reach.left + 24, w - reach.right - 24, w - reach.right, w].map { NSNumber(value: Double(min(max($0 / w, 0), 1))) }
        gradient.locations = stops
    }
}

final class ReadingBar: NSResponder, NSToolbarDelegate {
    private weak var window: NSWindow?
    private weak var page: PageView?
    private let edges = Edges()
    private var bookmarkItem: NSToolbarItem?
    private var timer: Timer?
    private var shown = false
    /// Held open by the reading menu.
    private var held = false
    /// Whether the bar may show now (no panel, help, note or view over the page).
    var canShow: (() -> Bool)?
    var onBack: (() -> Void)?
    var onCommand: ((Command) -> Void)?

    init(window: NSWindow, page: PageView, in container: NSView) {
        self.window = window
        self.page = page
        super.init()
        window.styleMask.insert(.fullSizeContentView)
        window.titlebarAppearsTransparent = true
        window.titleVisibility = .hidden
        window.titlebarSeparatorStyle = .none
        let toolbar = NSToolbar(identifier: "reading")
        toolbar.delegate = self
        toolbar.displayMode = .iconOnly
        toolbar.allowsUserCustomization = false
        window.toolbar = toolbar
        window.toolbarStyle = .unified
        edges.alphaValue = 0
        edges.frame = CGRect(x: 0, y: container.bounds.height - height, width: container.bounds.width, height: height)
        edges.autoresizingMask = [.width, .minYMargin]
        container.addSubview(edges, positioned: .above, relativeTo: page)
        container.addTrackingArea(NSTrackingArea(rect: .zero, options: [.mouseMoved, .activeInKeyWindow, .inVisibleRect], owner: self))
        self.container?.alphaValue = 0
        self.container?.isHidden = true
    }

    required init?(coder: NSCoder) { fatalError() }

    /// The title bar's height with the toolbar (52 points in a window).
    private var height: CGFloat {
        guard let window else { return 52 }
        let h = window.frame.height - window.contentLayoutRect.maxY
        return h > 0 ? h : 52
    }

    /// The view holding the window buttons and the toolbar. Not API: when it is not found the bar stays a standard
    /// toolbar, always shown.
    private var container: NSView? { window?.standardWindowButton(.closeButton)?.superview?.superview }

    // MARK: Showing and hiding

    override func mouseMoved(with e: NSEvent) {
        guard canShow?() ?? true else { return }
        show()
        restartTimer()
    }

    /// Whether the pointer is over the bar's strip.
    func overBar(_ point: CGPoint? = nil) -> Bool {
        guard let window else { return false }
        return (point ?? window.mouseLocationOutsideOfEventStream).y >= window.frame.height - height
    }

    private func restartTimer() {
        timer?.invalidate()
        timer = nil
        guard !held else { return }
        timer = Timer.scheduledTimer(withTimeInterval: 2, repeats: false) { [weak self] _ in
            guard let self else { return }
            self.timer = nil
            if self.overBar() { return self.restartTimer() }
            self.hide()
            // The pointer resting on the page goes with the bar.
            if let window = self.window, window.isKeyWindow,
               window.contentView?.bounds.contains(window.mouseLocationOutsideOfEventStream) == true {
                NSCursor.setHiddenUntilMouseMoves(true)
            }
        }
    }

    func show() {
        guard !shown else { return }
        shown = true
        if let page { edges.colour(page.palette.paper) }
        edges.frame = CGRect(x: 0, y: edges.superview!.bounds.height - height, width: edges.superview!.bounds.width, height: height)
        // The left group: the window buttons (none in full screen) and back; the right group: four buttons.
        let zoom = window?.standardWindowButton(.zoomButton)
        let buttons = zoom.flatMap { $0.window === window && !$0.isHidden ? $0.convert($0.bounds, to: nil).maxX : nil } ?? 0
        edges.reach = (buttons + 56, 4 * 32 + 44)
        updateBookmark()
        container?.isHidden = false
        NSAnimationContext.runAnimationGroup { ctx in
            ctx.duration = 0.15
            ctx.timingFunction = CAMediaTimingFunction(name: .easeOut)
            container?.animator().alphaValue = 1
            edges.animator().alphaValue = 1
        }
    }

    /// Fades the bar out (a key, a click on the page, a panel opening, the timer).
    func hide() {
        timer?.invalidate()
        timer = nil
        guard shown, !held else { return }
        shown = false
        NSAnimationContext.runAnimationGroup({ ctx in
            ctx.duration = 0.3
            ctx.timingFunction = CAMediaTimingFunction(name: .easeIn)
            container?.animator().alphaValue = 0
            edges.animator().alphaValue = 0
        }, completionHandler: { [weak self] in
            // Hidden, the title bar gives the clicks in its strip back to the page.
            guard let self, !self.shown else { return }
            self.container?.isHidden = true
        })
    }

    /// The reading menu opened: the bar shows and stays.
    func hold() {
        show()
        held = true
        timer?.invalidate()
        timer = nil
    }

    /// The reading menu closed (by a key or a click): the bar fades.
    func release() {
        held = false
        hide()
    }

    // MARK: Toolbar

    /// The bookmark button is filled only on a marked page whose ribbon is under the right group of buttons (a narrow
    /// window): elsewhere the ribbon itself shows the bookmark.
    func updateBookmark() {
        let covered = page?.ribbonRight.map { $0 > edges.bounds.width - edges.reach.right - 24 } ?? false
        bookmarkItem?.image = NSImage(systemSymbolName: covered ? "bookmark.fill" : "bookmark", accessibilityDescription: tr("Bookmark"))
    }

    func toolbarDefaultItemIdentifiers(_ toolbar: NSToolbar) -> [NSToolbarItem.Identifier] {
        [.back, .flexibleSpace, .contents, .settings, .search, .bookmark]
    }

    func toolbarAllowedItemIdentifiers(_ toolbar: NSToolbar) -> [NSToolbarItem.Identifier] { toolbarDefaultItemIdentifiers(toolbar) }

    func toolbar(_ toolbar: NSToolbar, itemForItemIdentifier id: NSToolbarItem.Identifier, willBeInsertedIntoToolbar flag: Bool) -> NSToolbarItem? {
        let item = NSToolbarItem(itemIdentifier: id)
        let buttons: [NSToolbarItem.Identifier: (String, String, String)] = [
            .back: ("chevron.left", tr("Back to library"), tr("Back to library")),
            .contents: ("list.bullet", tr("Contents"), "\(tr("Contents")) (o)"),
            .settings: ("textformat.size", tr("Text settings"), "\(tr("Text settings")) (s)"),
            .search: ("magnifyingglass", tr("Search"), "\(tr("Search")) (/)"),
            .bookmark: ("bookmark", tr("Bookmark"), "\(tr("Bookmark")) (b)"),
        ]
        guard let (name, label, tip) = buttons[id] else { return nil }
        item.image = NSImage(systemSymbolName: name, accessibilityDescription: label)
        item.label = label
        item.toolTip = tip
        item.isBordered = true
        item.target = self
        item.action = #selector(act(_:))
        if id == .bookmark { bookmarkItem = item }
        return item
    }

    @objc private func act(_ sender: NSToolbarItem) {
        // With the system's keyboard navigation on, a clicked toolbar button keeps the keys: they belong to the page
        // (a panel the action opens takes them from there).
        if let page { window?.makeFirstResponder(page) }
        switch sender.itemIdentifier {
        case .back: onBack?()
        case .contents: onCommand?(.contents)
        case .settings: onCommand?(.settings)
        case .search: onCommand?(.search)
        case .bookmark: page?.toggleBookmark()
        default: break
        }
    }
}
