// The reading view: pages typeset by the core, drawn with Core Text into bitmaps and moved by Core Animation, which runs
// in the window server: no app work per frame in "Slide", ProMotion and Low Power Mode handled by the system, nothing
// drawn while idle. "Scroll" shows the book as one column of tiles a screen high; only their positions change per frame.

import AppKit
import QuartzCore

/// What the view asks its window for.
enum Command {
    case contents, bookmarks, highlights, settings, search, help, closePanels, menu, note(String)
    /// A link's short target in the note sheet.
    case preview(String)
    /// An image (by id) or a table (by its first row) in the view (`Viewer`).
    case image(String), table(ForzacLocator)
}

/// The pill after a jump, as on Android: it goes when the reader turns this many pages (or screens) away.
private let pillPages: Int32 = 2
private let pillLabel = tr("← Back · ⌃O")

/// The bookmark ribbon (Android's): width, length below the window's top edge, the part tucked above it, the
/// swallowtail's depth; bordeaux on light and on dark themes.
private let ribbonWidth: CGFloat = 16
private let ribbonLength: CGFloat = 44
private let ribbonTuck: CGFloat = 12
private let ribbonTail: CGFloat = ribbonWidth * 0.5625

final class PageView: NSView {
    let doc: OpaquePointer
    let fonts = Fonts()
    private(set) var settings: ForzacSettings
    private(set) var systemDark: Bool
    var theme: Int { Int(withUnsafePointer(to: settings) { forzac_settings_theme($0, systemDark) }) }
    var palette: Palette { themes[theme] }
    var scrolling: Bool { settings.turning == 1 }

    var chapter: UInt32
    var pageCount: UInt32 = 0
    /// The page shown; in a spread its left (even) page.
    var page: UInt32 = 0
    /// Whether the window shows two pages side by side ("Slide" in a wide window, `forzac_settings_spread`).
    private(set) var spread = false
    /// How far a spread's pages are drawn towards the gutter (`forzac_settings_spread_inset`).
    private var inset: CGFloat = 0
    var locator: ForzacLocator
    private var toLast = false
    /// What the current layout was made for: the size, the settings that change it, the chapter.
    private var laidOutFor: [Float]?
    private var images: [UInt32: CGImage] = [:]
    private var pageAreas: [UInt32: [Area]] = [:]
    /// Decoded book images by the address of their bytes (the document keeps them).
    private var pictures: [UnsafeRawPointer: CGImage] = [:]
    private let current = CALayer()
    private let previous = CALayer()
    private let focusLayer = CALayer()
    /// The focused `Tab` stop of the page (or of the screen in "Scroll").
    private var focus: Int?
    /// Where the current page's (or spread's) ribbon hangs: the offset of the page holding the bookmark.
    private var ribbonDx: CGFloat = 0
    /// Where jumps started, the latest last (`⌃O`).
    private var back: [ForzacLocator] = []
    /// The matches of the last search gone to (start, end), and the one highlighted (`n`/`N` move it).
    private var matches: [(ForzacLocator, ForzacLocator)] = []
    private var currentMatch: Int?
    /// Horizontal trackpad scrolling not yet turned into a page turn, and whether this swipe already turned one.
    private var swipe: (CGFloat, Bool) = (0, false)
    /// Where the mouse went down: a click that moved is a drag, not a turn.
    private var mouseDownAt: CGPoint?
    /// Where the last jump arrived and the pages turned since ("Slide"), for the "← Back" pill; nil once the reader
    /// returned or read on.
    private var pill: (arrived: ForzacLocator, turns: Int32)?
    private let pillLayer = CALayer()
    private let pillText = CATextLayer()
    /// "Slide": the end page after the book's last page is shown. "Scroll": the end screen fills the window.
    private(set) var atEnd = false
    private var columnEnd = false
    private var endImage: CGImage?
    private let endLayer = CALayer()
    /// The end page (or end screen) is shown: the position is saved as finished.
    var finished: Bool { scrolling ? columnEnd : atEnd }
    /// The bookmarks' places (the document holds them) and the ribbons of the two page layers.
    private var bookmarks: [ForzacLocator] = []
    private let ribbon = CAShapeLayer()
    private let previousRibbon = CAShapeLayer()
    /// The bookmarks changed (to save them).
    var onBookmarks: (() -> Void)?

    // Selection and highlights (`quotes`).
    /// The selection (or the keyboard's cursor), where the cursor goes once the words shown allow, the highlights, the bar.
    var textSelection: TextSelection?
    var pendingMove: PendingMove?
    private(set) var highlights: [LocRange] = []
    var selectionBar: SelectionBar?
    /// The bar's items (y down) for clicks; the bar's layer.
    var barItems: [(CGRect, BarAction)] = []
    let barLayer = CALayer()
    /// A drag with the mouse (y down): where it was pressed, where it is, whether it selects yet; the edge timer.
    var drag: (press: CGPoint, at: CGPoint, started: Bool)?
    var edgeTimer: Timer?
    var edgeDir: Int32 = 0
    var dragLink: CADisplayLink?
    /// The highlights changed (to save them).
    var onHighlights: (() -> Void)?

    // "Scroll".
    private var offset: CGFloat = 0
    private var tiles: [TileKey: CALayer] = [:]
    private var tileAreas: [TileKey: [Area]] = [:]
    private var motion: (from: CGFloat, to: CGFloat, start: CFTimeInterval?)?
    private var link: CADisplayLink?
    private var marginTop: CGFloat = 0
    private(set) var lineHeight: CGFloat = 30

    /// The count of the book's pages running (`reader-menu`) and the size and settings it is for; the pause before it
    /// starts (a resize restarts it).
    private var count: OpaquePointer?
    private var countFor: [Float]?
    private var countStart: DispatchWorkItem?
    /// The book's pages were counted (the menu, the lists).
    var onPages: (() -> Void)?
    /// Asked first for every key while it returns true (the menu while it is open).
    var menuKey: ((NSEvent) -> Bool)?
    /// Whether the menu is open: a click on the page closes it.
    var menuOpen: (() -> Bool)?

    var onFirstFrame: ((PageView) -> Void)?
    /// The reading position changed.
    var onMoved: (() -> Void)?
    /// The settings changed (to save them and recolour the panels).
    var onSettings: (() -> Void)?
    var onCommand: ((Command) -> Void)?
    /// Whether a panel is over the page: a click on the page closes it instead of acting.
    var panelOpen: (() -> Bool)?

    init(doc: OpaquePointer, at: ForzacLocator, settings: ForzacSettings, systemDark: Bool) {
        self.doc = doc
        self.chapter = at.chapter
        self.locator = at
        self.settings = settings
        self.systemDark = systemDark
        super.init(frame: .zero)
        wantsLayer = true
        layerContentsRedrawPolicy = .never
        for l in [previous, current] {
            l.contentsGravity = .resize
            l.actions = ["contents": NSNull(), "position": NSNull(), "bounds": NSNull()]
            layer!.addSublayer(l)
        }
        // Each page layer has its ribbon, which slides with it.
        for (page, r) in [(current, ribbon), (previous, previousRibbon)] {
            r.actions = ["position": NSNull(), "bounds": NSNull(), "hidden": NSNull(), "fillColor": NSNull(), "path": NSNull()]
            r.isHidden = true
            r.shadowOpacity = 0.3
            r.shadowRadius = 1.5
            r.shadowOffset = CGSize(width: 0, height: -1.5)
            page.addSublayer(r)
        }
        focusLayer.actions = ["sublayers": NSNull(), "contents": NSNull(), "position": NSNull(), "bounds": NSNull()]
        layer!.addSublayer(focusLayer)
        endLayer.actions = ["contents": NSNull(), "position": NSNull(), "bounds": NSNull(), "hidden": NSNull()]
        endLayer.isHidden = true
        layer!.insertSublayer(endLayer, below: focusLayer)
        // The pill fades (its opacity's implicit 0.25 s animation, shortened to Android's 150 ms when it changes).
        pillLayer.actions = ["position": NSNull(), "bounds": NSNull(), "backgroundColor": NSNull()]
        pillLayer.opacity = 0
        pillText.actions = ["contents": NSNull(), "position": NSNull(), "bounds": NSNull(), "foregroundColor": NSNull()]
        pillText.alignmentMode = .center
        pillLayer.addSublayer(pillText)
        layer!.addSublayer(pillLayer)
        barLayer.actions = ["position": NSNull(), "bounds": NSNull(), "sublayers": NSNull(), "backgroundColor": NSNull()]
        barLayer.opacity = 0
        barLayer.cornerRadius = 14
        barLayer.shadowOpacity = 0.18
        barLayer.shadowRadius = 10
        barLayer.shadowOffset = CGSize(width: 0, height: -4)
        layer!.addSublayer(barLayer)
    }

    required init?(coder: NSCoder) { fatalError() }

    override var acceptsFirstResponder: Bool { true }

    override func layout() {
        super.layout()
        relayout()
        show(animated: 0)
    }

    /// A frame-sized view is not laid out again on its own when the window resizes it.
    override func setFrameSize(_ size: NSSize) {
        super.setFrameSize(size)
        needsLayout = true
    }

    override func viewDidChangeBackingProperties() {
        images = [:]
        clearTiles()
        show(animated: 0)
    }

    var scale: CGFloat { window?.backingScaleFactor ?? 2 }

    /// Typesets the chapter when the size, the settings or the chapter changed; keeps the reading position.
    func relayout() {
        let size = bounds.size
        guard size.width > 0, size.height > 0 else { return }
        let s = settings
        // The pages depend on the mode: "Scroll" is never a spread.
        let key: [Float] = [Float(size.width), Float(size.height), Float(s.size), Float(s.family), Float(s.spacing), Float(s.align), Float(s.margins), Float(s.turning), Float(chapter)]
        if laidOutFor == key { return }
        let styleChanged = laidOutFor.map { Array($0.dropLast()) != Array(key.dropLast()) } ?? true
        spread = withUnsafePointer(to: settings) { forzac_settings_spread($0, Float(size.width), Float(size.height)) }
        inset = CGFloat(withUnsafePointer(to: settings) { forzac_settings_spread_inset($0, Float(size.width), Float(size.height)) })
        pageCount = withUnsafePointer(to: settings) { forzac_paginate(doc, chapter, Float(size.width), Float(size.height), $0) }
        var top: Float = 0, line: Float = 0
        forzac_metrics(doc, &top, &line)
        (marginTop, lineHeight) = (CGFloat(top), CGFloat(line))
        if toLast {
            page = left(pageCount - 1)
            locator = forzac_page_start(doc, page) // turned back into this chapter: its last page (or spread)
        } else {
            // The reading position stays where it is: leaving a spread shows the page that holds it.
            page = left(forzac_page_of(doc, locator))
        }
        toLast = false
        laidOutFor = key
        images = [:]
        endImage = nil
        pageAreas = [:]
        focus = nil
        if styleChanged {
            scheduleCount(Array(key.dropLast()))
            pictures = [:]
            clearTiles()
            if scrolling { offset = CGFloat(forzac_flow_y(doc, locator)) - marginTop }
        }
    }

    // MARK: The book's pages

    /// Counts the book's pages for `key` (size and settings) on a background queue, 0.3 s after it last changed; a count
    /// for another key is stopped.
    private func scheduleCount(_ key: [Float]) {
        guard countFor != key else { return }
        if let c = count { forzac_count_stop(c) }
        count = nil
        countFor = key
        countStart?.cancel()
        let work = DispatchWorkItem { [weak self] in self?.startCount(key) }
        countStart = work
        DispatchQueue.main.asyncAfter(deadline: .now() + 0.3, execute: work)
    }

    private func startCount(_ key: [Float]) {
        guard countFor == key, let c = withUnsafePointer(to: settings, { forzac_count_new(doc, Float(bounds.width), Float(bounds.height), $0) }) else { return }
        count = c
        let address = UInt(bitPattern: c)
        DispatchQueue.global(qos: .utility).async {
            forzac_count_run(OpaquePointer(bitPattern: address))
            DispatchQueue.main.async { [weak self] in
                let c = OpaquePointer(bitPattern: address)!
                defer { forzac_count_free(c) }
                guard let self, self.count == c else { return }
                self.count = nil
                // The pages' feet change: drawn again (Core Animation goes on moving the layers meanwhile).
                guard forzac_count_install(self.doc, c) else { return }
                self.images = [:]
                if !self.scrolling { self.show(animated: 0) }
                self.onPages?()
            }
        }
    }

    /// The reader moved: a running count waits for a pause before its next chapter.
    private func countMoved() {
        if let c = count { forzac_count_moved(c) }
    }

    /// The book page (from 0) the reader is on, once the pages are counted.
    var bookPage: UInt32? { forzac_book_pages(doc) > 0 ? forzac_book_page_of(doc, locator) : nil }

    // MARK: Slide

    // MARK: Spreads

    /// A page's width: half the view in a spread.
    private var pageWidth: CGFloat { spread ? bounds.width / 2 : bounds.width }

    /// The page to show for `index`: in a spread, the left page of its pair (every chapter starts on a left page).
    func left(_ index: UInt32) -> UInt32 { spread ? index & ~1 : index }

    /// The pages shown with `index` first and their offsets from the view's left edge: the page, or a spread's left
    /// page and its right one when the chapter has it, both moved towards the gutter.
    private func shownPages(_ index: UInt32) -> [(UInt32, CGFloat)] {
        guard spread else { return [(index, 0)] }
        return index + 1 < pageCount ? [(index, inset), (index + 1, pageWidth - inset)] : [(index, inset)]
    }

    /// The page (or spread) as a bitmap at the backing scale; `index` is its (left) page.
    private func image(_ index: UInt32) -> CGImage? {
        if let i = images[index] { return i }
        let h = bounds.height
        let image = bitmap(bounds.size, scale: scale, background: palette.paper) { ctx in
            for (p, dx) in shownPages(index) {
                // Highlights under the text, and among the page's areas (`Tab`, clicks), where the page is drawn.
                let marked = highlights.map { ($0, pageMarks(p, $0)) }
                ctx.saveGState()
                ctx.translateBy(x: dx, y: 0)
                drawBands(marked.map(\.1), in: ctx, height: h)
                let page = forzac_page(doc, p)
                pageAreas[p] = withHighlights(areas(of: page), marked).map { $0.offsetBy(dx: dx) }
                drawPage(page, in: ctx, height: h, palette: palette, fonts: fonts, pictures: &pictures)
                ctx.restoreGState()
            }
        }
        images[index] = image
        return image
    }

    /// The end page after the book's last page, as a bitmap at the backing scale.
    private func endPage() -> CGImage? {
        if let i = endImage { return i }
        let h = bounds.height
        let centre = (bounds.width - pageWidth) / 2 // in a spread, centred on the window
        endImage = bitmap(bounds.size, scale: scale, background: palette.paper) { ctx in
            ctx.translateBy(x: centre, y: 0)
            drawPage(forzac_end_page(doc), in: ctx, height: h, palette: palette, fonts: fonts, pictures: &pictures)
        }
        return endImage
    }

    /// Shows the current page; `animated` +1 slides it in from the right, −1 from the left.
    func show(animated dir: CGFloat) {
        layer!.backgroundColor = color(palette.paper)
        defer { updatePill() }
        if scrolling {
            current.contents = nil
            previous.contents = nil
            CATransaction.begin()
            CATransaction.setDisableActions(true)
            current.frame = bounds
            previousRibbon.isHidden = true
            CATransaction.commit()
            placeTiles()
            return
        }
        endLayer.isHidden = true
        guard let img = atEnd ? endPage() : image(page) else { return }
        let frame = bounds
        CATransaction.begin()
        CATransaction.setDisableActions(true)
        if dir != 0 {
            previous.contents = current.contents
            previous.contentsScale = scale
            previous.frame = frame
            previousRibbon.isHidden = ribbon.isHidden
            placeRibbon(previousRibbon, dx: ribbonDx)
        }
        current.contents = img
        current.contentsScale = scale
        current.frame = frame
        CATransaction.commit()
        updateRibbon()
        if dir != 0 {
            countMoved()
            let timing = CAMediaTimingFunction(controlPoints: 0.215, 0.61, 0.355, 1) // ease-out cubic
            for (l, from, to) in [(current, frame.midX + dir * frame.width, frame.midX), (previous, frame.midX, frame.midX - dir * frame.width)] {
                let a = CABasicAnimation(keyPath: "position.x")
                a.fromValue = from
                a.toValue = to
                a.duration = 0.2
                a.timingFunction = timing
                l.position.x = to
                l.add(a, forKey: "turn")
            }
        }
        drawFocus()
        if let f = onFirstFrame {
            onFirstFrame = nil
            CATransaction.setCompletionBlock { f(self) }
        }
    }

    func turn(_ delta: Int32) {
        if scrolling { return scroll(by: CGFloat(delta) * (bounds.height - lineHeight)) }
        if atEnd {
            // Back from the end page to the last page of the text; forward does nothing.
            guard delta < 0 else { return }
            atEnd = false
            show(animated: -1)
            onMoved?()
            return
        }
        let target = Int64(page) + Int64(delta) * (spread ? 2 : 1)
        if pill != nil { pill!.turns += delta }
        if delta > 0 && target >= Int64(pageCount) && chapter + 1 == forzac_chapter_count(doc) {
            // Past the book's last page: the end page; the position stays at the last page.
            atEnd = true
            focus = nil
            show(animated: 1)
            onMoved?()
            return
        }
        if target >= 0 && target < Int64(pageCount) {
            page = UInt32(target)
            locator = forzac_page_start(doc, page)
        } else if delta > 0 && chapter + 1 < forzac_chapter_count(doc) {
            chapter += 1
            locator = ForzacLocator(chapter: chapter, block: 0, offset: 0)
            relayout()
        } else if delta < 0 && chapter > 0 {
            chapter -= 1
            toLast = true
            locator = ForzacLocator(chapter: chapter, block: 0, offset: 0)
            relayout()
        } else {
            return
        }
        focus = nil
        show(animated: CGFloat(delta.signum()))
        onMoved?()
    }

    func jumpChapter(_ delta: Int32) {
        let target = Int64(chapter) + Int64(delta)
        guard target >= 0 && target < Int64(forzac_chapter_count(doc)) else { return }
        go(to: ForzacLocator(chapter: UInt32(target), block: 0, offset: 0), remember: false)
    }

    func edge(end: Bool) {
        if scrolling {
            motion = nil
            let h = CGFloat(forzac_flow_height(doc, chapter))
            offset = end ? max(0, h - bounds.height) : 0
            placeTiles()
            settle()
            return
        }
        page = end ? left(pageCount - 1) : 0
        locator = forzac_page_start(doc, page)
        focus = nil
        atEnd = false
        show(animated: 0)
        onMoved?()
    }

    /// Goes to `at` (a contents entry, a link); `remember` keeps the place left for `⌃O`.
    func go(to at: ForzacLocator, remember: Bool = true) {
        dismissSelection()
        if remember {
            back.append(locator)
            if back.count > 50 { back.removeFirst() }
            pill = (at, 0)
        }
        locator = at
        chapter = at.chapter
        focus = nil
        motion = nil
        atEnd = false
        laidOutFor = nil
        relayout()
        if scrolling { offset = CGFloat(forzac_flow_y(doc, locator)) - marginTop }
        show(animated: 0)
        onMoved?()
    }

    /// Back to where the last jump started.
    func goBack() {
        pill = nil
        guard let at = back.popLast() else { return updatePill() }
        go(to: at, remember: false)
    }

    /// The "← Back · ⌃O" pill over the foot while the last jump is fresh: the ink as its fill, the paper as its text
    /// (Android's `ReturnPill`).
    private func updatePill() {
        if let p = pill {
            let far: Bool
            if !scrolling {
                far = abs(p.turns) >= pillPages
            } else if p.arrived.chapter == locator.chapter {
                far = abs(forzac_flow_y(doc, locator) - forzac_flow_y(doc, p.arrived)) >= Float(pillPages) * Float(bounds.height)
            } else {
                far = motion == nil
            }
            if far { pill = nil }
        }
        let visible = pill != nil && !back.isEmpty && !finished
        let font = interfaceFont(14)
        CATransaction.begin()
        CATransaction.setAnimationDuration(0.15)
        if visible {
            pillText.string = NSAttributedString(string: pillLabel, attributes: [.font: font, .foregroundColor: nsColor(palette.paper)])
            pillText.contentsScale = scale
            let text = pillText.preferredFrameSize()
            let (w, h) = (text.width + 36, text.height + 20)
            CATransaction.setDisableActions(true)
            pillLayer.backgroundColor = color(palette.ink)
            pillLayer.cornerRadius = h / 2
            // Centred on the foot (half the bottom margin, 56 px), as Android's pill replaces the page number.
            pillLayer.frame = CGRect(x: (bounds.width - w) / 2, y: 28 - h / 2, width: w, height: h)
            pillText.frame = CGRect(x: 18, y: 10, width: text.width, height: text.height)
            CATransaction.setDisableActions(false)
        }
        pillLayer.opacity = visible ? 1 : 0
        CATransaction.commit()
    }

    // MARK: Settings

    func apply(_ new: ForzacSettings) {
        if new.turning != settings.turning { switchMode() }
        let before = theme
        settings = new
        if theme != before { recoloured() }
        relayout()
        show(animated: 0)
        onSettings?()
    }

    func step(_ setting: UInt32, _ delta: Int32) {
        var s = settings
        forzac_settings_step(&s, setting, delta, systemDark)
        apply(s)
    }

    func setSystemDark(_ dark: Bool) {
        let before = theme
        systemDark = dark
        if theme != before {
            recoloured()
            show(animated: 0)
            onSettings?()
        }
    }

    /// The ink is baked into the bitmaps: a new theme draws them again, without a new layout, cross-fading the window's
    /// content (page and panels) into the new colours in 200 ms.
    private func recoloured() {
        let fade = CATransition()
        fade.type = .fade
        fade.duration = 0.2
        (superview?.layer ?? layer)?.add(fade, forKey: "theme")
        images = [:]
        endImage = nil
        clearTiles()
    }

    /// "Slide" ↔ "Scroll", keeping the line at the top of the text.
    private func switchMode() {
        motion = nil
        focus = nil
        atEnd = false
        columnEnd = false
        if scrolling {
            locator = forzac_flow_locator(doc, chapter, Float(offset + marginTop))
            chapter = locator.chapter
            settings.turning = 0
            laidOutFor = nil
            clearTiles()
        } else {
            settings.turning = 1
            // The page's first line: after a re-layout the reading position lies inside the page.
            locator = forzac_page_start(doc, page)
            offset = CGFloat(forzac_flow_y(doc, locator)) - marginTop
        }
    }

    // MARK: Scroll

    struct TileKey: Hashable {
        let chapter: UInt32
        let index: Int
    }

    private var gap: CGFloat { lineHeight * 4 }

    private func clearTiles() {
        for t in tiles.values { t.removeFromSuperlayer() }
        tiles = [:]
        tileAreas = [:]
    }

    /// Moves to the chapter the window's top is in and keeps the offset inside the book.
    private func normalize() {
        let count = forzac_chapter_count(doc)
        while true {
            let length = CGFloat(forzac_flow_height(doc, chapter)) + gap
            if offset >= length && chapter + 1 < count {
                chapter += 1
                shift(-length)
            } else if offset < 0 && chapter > 0 {
                chapter -= 1
                shift(CGFloat(forzac_flow_height(doc, chapter)) + gap)
            } else {
                break
            }
        }
        let last = chapter + 1 == count
        // After the last paragraph, the end screen a window high.
        let end = last ? CGFloat(forzac_flow_height(doc, chapter)) : max(0, CGFloat(forzac_flow_height(doc, chapter)) - bounds.height)
        let clamped = last ? min(max(offset, 0), end) : max(offset, 0)
        if clamped != offset {
            offset = clamped
            if let m = motion { motion = (m.from, min(max(m.to, chapter == 0 ? 0 : -.infinity), last ? end : .infinity), m.start) }
        }
    }

    private func shift(_ by: CGFloat) {
        offset += by
        if let m = motion { motion = (m.from + by, m.to + by, m.start) }
    }

    /// Calls `body(top, key)` for the tiles reaching into `[from, to)` (view coordinates, y down), building those not
    /// yet drawn; `top` is the tile's top on screen.
    private func forTiles(from: CGFloat, to: CGFloat, _ body: (CGFloat, TileKey) -> Void) {
        let h = bounds.height
        let tile = max(1, h.rounded())
        var c = chapter
        // On a whole pixel: text moved by fractions of a pixel would be resampled in every frame of a movement.
        var top = -offset.rounded()
        while top < to && c < forzac_chapter_count(doc) {
            let length = CGFloat(forzac_flow_height(doc, c))
            let first = max(0, Int(((from - top) / tile).rounded(.down)) - 1)
            let last = Int(((to - top) / tile).rounded(.down))
            if first <= last {
                for k in first...last where CGFloat(k) * tile < length {
                    let key = TileKey(chapter: c, index: k)
                    if tiles[key] == nil { buildTile(key, tile: tile) }
                    body(top + CGFloat(k) * tile, key)
                }
            }
            top += length + gap
            c += 1
        }
    }

    private func buildTile(_ key: TileKey, tile: CGFloat) {
        let layer = CALayer()
        layer.actions = ["position": NSNull(), "bounds": NSNull(), "contents": NSNull()]
        let (image, height) = tileImage(key, tile: tile)
        layer.contents = image
        layer.contentsScale = scale
        layer.bounds = CGRect(x: 0, y: 0, width: bounds.width, height: height)
        self.layer!.insertSublayer(layer, below: focusLayer)
        tiles[key] = layer
    }

    /// Tile `key`'s bitmap (its highlights under the text) and height; its areas go to `tileAreas`.
    private func tileImage(_ key: TileKey, tile: CGFloat) -> (CGImage?, CGFloat) {
        let w = bounds.width
        let (y0, y1) = (Float(CGFloat(key.index) * tile), Float(CGFloat(key.index + 1) * tile))
        let marked = highlights.map { ($0, flowMarks(key.chapter, y0, y1, $0)) }
        let bands = marked.map(\.1)
        let content = forzac_flow(doc, key.chapter, y0, y1)
        tileAreas[key] = withHighlights(areas(of: content), marked)
        // A tile's last line (or image, or table row) may hang below it: the bitmap reaches to its bottom and is
        // transparent, the paper is the view's background.
        var bottom = tile
        for i in 0..<Int(content.run_count) where content.runs[i].count > 0 {
            let r = content.runs[i]
            bottom = max(bottom, CGFloat(r.positions[2 * Int(r.count) - 1] + r.size * 0.4))
        }
        for i in 0..<Int(content.image_count) { bottom = max(bottom, CGFloat(content.images[i].rect.y + content.images[i].rect.h)) }
        for i in 0..<Int(content.rule_count) { bottom = max(bottom, CGFloat(content.rules[i].rect.y + content.rules[i].rect.h)) }
        bottom = bottom.rounded(.up)
        let image = bitmap(CGSize(width: w, height: bottom), scale: scale, background: nil) { ctx in
            drawBands(bands, in: ctx, height: bottom)
            drawPage(content, in: ctx, height: bottom, palette: palette, fonts: fonts, pictures: &pictures)
        }
        return (image, bottom)
    }

    /// Puts the tiles on screen where the offset says; drops tiles far from it.
    private func placeTiles() {
        guard bounds.height > 0 else { return }
        countMoved()
        normalize()
        let h = bounds.height
        var shown = Set<TileKey>()
        CATransaction.begin()
        CATransaction.setDisableActions(true)
        forTiles(from: 0, to: h) { top, key in
            shown.insert(key)
            // Layers are y up: the tile's top `top` pixels below the window's top.
            tiles[key]?.frame = CGRect(x: 0, y: h - top - (tiles[key]?.bounds.height ?? 0), width: bounds.width, height: tiles[key]?.bounds.height ?? 0)
            tiles[key]?.isHidden = false
        }
        for (key, t) in tiles where !shown.contains(key) { t.isHidden = true }
        // The end screen after the last chapter's column, when it reaches into the window.
        let count = forzac_chapter_count(doc)
        var top = -offset.rounded()
        for c in chapter..<count { top += CGFloat(forzac_flow_height(doc, c)) + (c + 1 < count ? gap : 0) }
        if top < h, let img = endPage() {
            endLayer.contents = img
            endLayer.contentsScale = scale
            endLayer.frame = CGRect(x: 0, y: h - top - bounds.height, width: bounds.width, height: bounds.height)
            endLayer.isHidden = false
        } else {
            endLayer.isHidden = true
        }
        CATransaction.commit()
        if tiles.count > 16 {
            for key in tiles.keys where !shown.contains(key) && (key.chapter != chapter || abs(key.index - Int(offset / max(1, h))) > 2) {
                tiles.removeValue(forKey: key)?.removeFromSuperlayer()
                tileAreas[key] = nil
            }
        }
        drawFocus()
        updateRibbon()
        if let f = onFirstFrame {
            onFirstFrame = nil
            DispatchQueue.main.async { f(self) }
        }
    }

    /// Moves the column by `dy` with a smooth movement; a movement running continues from where it is.
    func scroll(by dy: CGFloat) {
        focus = nil
        motion = (offset, (motion?.to ?? offset) + dy, nil)
        if link == nil {
            let l = displayLink(target: self, selector: #selector(frame(_:)))
            l.add(to: .main, forMode: .common)
            link = l
        }
    }

    @objc private func frame(_ l: CADisplayLink) {
        guard let m = motion else { return stopLink() }
        // The first frame already moves, as if the movement had started a refresh earlier.
        let start = m.start ?? (l.timestamp - (l.targetTimestamp - l.timestamp))
        motion = (m.from, m.to, start)
        let p = min(1, max(0, (l.targetTimestamp - start) / 0.15))
        let eased = 1 - pow(1 - p, 3)
        offset = m.from + (m.to - m.from) * eased
        if p >= 1 { motion = nil }
        placeTiles()
        if motion == nil {
            stopLink()
            settle()
        }
    }

    private func stopLink() {
        link?.invalidate()
        link = nil
    }

    /// The column stopped: the reading position is its top line; the tiles a screen around are drawn in idle time.
    private func settle() {
        locator = forzac_flow_locator(doc, chapter, Float(offset + marginTop))
        columnEnd = chapter + 1 == forzac_chapter_count(doc) && offset >= CGFloat(forzac_flow_height(doc, chapter)) - 0.5
        updatePill()
        onMoved?()
        DispatchQueue.main.async { [weak self] in
            guard let self, self.motion == nil, self.scrolling else { return }
            let h = self.bounds.height
            self.forTiles(from: -h, to: 2 * h) { _, _ in }
            self.placeTiles()
        }
    }

    /// The markers and links on screen, y down.
    private func visibleAreas() -> [Area] {
        if atEnd { return [] }
        if !scrolling { return shownPages(page).flatMap { pageAreas[$0.0] ?? [] } }
        var out: [Area] = []
        forTiles(from: 0, to: bounds.height) { top, key in
            out += (tileAreas[key] ?? []).map { $0.offsetBy(dy: top) }
        }
        return out.filter { $0.rect.maxY > 0 && $0.rect.minY < bounds.height }
    }

    // MARK: Bookmarks

    /// Takes the bookmarks from the document (after they were loaded or changed).
    func reloadBookmarks() {
        var n: UInt32 = 0
        let p = forzac_bookmark_places(doc, &n)
        bookmarks = (0..<Int(n)).map { p![$0] }
        updateRibbon()
    }

    /// The range of text shown: the page in "Slide" (none on the end page), the screen in "Scroll" — from the line at the
    /// top margin to the first line at or below the window's bottom, in a following chapter when the window reaches it.
    private func shownRange() -> (ForzacLocator, ForzacLocator)? {
        if scrolling {
            return (lineAt(Float(offset + marginTop)), lineAt(Float(offset + bounds.height)))
        }
        if atEnd { return nil }
        return (forzac_page_start(doc, page), forzac_page_end(doc, shownPages(page).last!.0))
    }

    /// The first line at or below height `y` of the current chapter's column, or of the chapters after it.
    private func lineAt(_ y: Float) -> ForzacLocator {
        var (c, y) = (chapter, y)
        while c < forzac_chapter_count(doc) {
            let at = forzac_flow_locator(doc, c, y)
            if forzac_flow_y(doc, at) >= y - 0.5 { return at }
            y = max(0, y - forzac_flow_height(doc, c) - Float(gap))
            c += 1
        }
        return ForzacLocator(chapter: c, block: 0, offset: 0)
    }

    private var shownMarked: Bool {
        guard !bookmarks.isEmpty, let (from, to) = shownRange() else { return false }
        return forzac_bookmarked(doc, from, to)
    }

    /// `b`: sets a bookmark on the page (or screen) shown, or removes its bookmarks; the ribbon drops in or goes up.
    func toggleBookmark() {
        guard let (from, to) = shownRange() else { return }
        let set = forzac_bookmark_toggle(doc, from, to)
        reloadBookmarks()
        onBookmarks?()
        let rest = ribbon.position.y
        let hidden = rest + ribbonLength // just out of the window
        if set {
            // Android's spring: damping ratio 0.5, stiffness 280.
            let drop = CASpringAnimation(keyPath: "position.y")
            drop.mass = 1
            drop.stiffness = 280
            drop.damping = 2 * 0.5 * sqrt(280)
            drop.fromValue = hidden
            drop.toValue = rest
            drop.duration = drop.settlingDuration
            ribbon.add(drop, forKey: "drop")
        } else {
            ribbon.isHidden = false
            let rise = CABasicAnimation(keyPath: "position.y")
            rise.fromValue = rest
            rise.toValue = hidden
            rise.duration = 0.2
            rise.timingFunction = CAMediaTimingFunction(name: .easeIn)
            rise.fillMode = .forwards
            rise.isRemovedOnCompletion = false
            CATransaction.begin()
            CATransaction.setCompletionBlock { [weak self] in
                self?.ribbon.removeAnimation(forKey: "rise")
                self?.updateRibbon()
            }
            ribbon.add(rise, forKey: "rise")
            CATransaction.commit()
        }
    }

    /// Removes the bookmark at `at` (from the list), without animation.
    func removeBookmark(_ at: ForzacLocator) {
        forzac_bookmark_remove(doc, at)
        reloadBookmarks()
        onBookmarks?()
    }

    /// The right edge of the ribbon of the page (or spread) shown, nil when it has no bookmark (the reading bar).
    var ribbonRight: CGFloat? { shownMarked ? ribbon.frame.maxX : nil }

    /// Shows the current page's ribbon when it is marked, in the theme's colour, beside the text.
    private func updateRibbon() {
        // On a spread over the right page, in its outer margin: never in the gutter.
        ribbonDx = shownPages(page).last!.1
        CATransaction.begin()
        CATransaction.setDisableActions(true)
        placeRibbon(ribbon, dx: ribbonDx)
        // A ribbon going up stays until it is gone; one dropping goes with its page.
        if ribbon.animation(forKey: "rise") == nil { ribbon.isHidden = !shownMarked }
        if ribbon.isHidden { ribbon.removeAnimation(forKey: "drop") }
        CATransaction.commit()
    }

    /// Puts `r` at the top of its page layer, beside the text column of the page `dx` from the left edge.
    private func placeRibbon(_ r: CAShapeLayer, dx: CGFloat) {
        var centre: Float = 0, corner: Float = 0
        forzac_ribbon(doc, &centre, &corner)
        // Layers are y up: the ribbon hangs from the top edge, its tuck above it.
        r.frame = CGRect(x: dx + CGFloat(centre) - ribbonWidth / 2, y: bounds.height - ribbonLength, width: ribbonWidth, height: ribbonLength + ribbonTuck)
        let path = CGMutablePath()
        path.move(to: CGPoint(x: 0, y: ribbonLength + ribbonTuck))
        path.addLine(to: CGPoint(x: ribbonWidth, y: ribbonLength + ribbonTuck))
        path.addLine(to: CGPoint(x: ribbonWidth, y: 0))
        path.addLine(to: CGPoint(x: ribbonWidth / 2, y: ribbonTail))
        path.addLine(to: CGPoint(x: 0, y: 0))
        path.closeSubpath()
        r.path = path
        r.shadowPath = path
        r.fillColor = color(palette.paper < 0x808080 ? 0xB5483F : 0x8E2F2A)
        r.shadowColor = color(0x000000)
    }

    // MARK: Markers and links

    func focusNext(_ delta: Int) {
        let all = stops(visibleAreas())
        guard !all.isEmpty else { return }
        focus = focus.map { (($0 + delta) % all.count + all.count) % all.count } ?? (delta > 0 ? 0 : all.count - 1)
        drawFocus()
    }

    func unfocus() {
        focus = nil
        drawFocus()
    }

    // MARK: Search

    /// Goes to match `i` of `all` (remembered for the way back) and highlights it.
    func goToMatch(_ all: [(ForzacLocator, ForzacLocator)], _ i: Int) {
        guard all.indices.contains(i) else { return }
        matches = all
        currentMatch = i
        go(to: all[i].0)
    }

    /// The next (`delta` 1) or previous (−1) match of the last search; without a highlight, from the reading position.
    func nextMatch(_ delta: Int) {
        let before = { (a: ForzacLocator, b: ForzacLocator) in (a.chapter, a.block, a.offset) < (b.chapter, b.block, b.offset) }
        let here = locator
        let i = currentMatch.map { $0 + delta } ?? (delta > 0 ? matches.firstIndex { !before($0.0, here) } : matches.lastIndex { before($0.0, here) })
        guard let i, matches.indices.contains(i) else { return }
        currentMatch = i
        go(to: matches[i].0, remember: false)
    }

    /// Removes the match highlight; false when there was none.
    @discardableResult
    func clearMatch() -> Bool {
        let had = currentMatch != nil
        currentMatch = nil
        drawFocus()
        return had
    }

    /// The highlighted match's rectangles on screen, y down: on the page, or in the column's current and next chapter.
    private func matchRects() -> [CGRect] {
        guard !atEnd, let i = currentMatch, let (from, to) = matches[safe: i] else { return [] }
        var rects = [ForzacRect](repeating: ForzacRect(x: 0, y: 0, w: 0, h: 0), count: 8)
        var out: [CGRect] = []
        func add(_ n: UInt32, dx: CGFloat = 0, dy: CGFloat) {
            out += rects[0..<Int(n)].map { CGRect(x: CGFloat($0.x) + dx, y: CGFloat($0.y) + dy, width: CGFloat($0.w), height: CGFloat($0.h)) }
        }
        if scrolling {
            var top = -offset.rounded()
            for c in [chapter, chapter + 1] where c < forzac_chapter_count(doc) {
                let height = forzac_flow_height(doc, c)
                if from.chapter == c { add(forzac_flow_marks(doc, c, 0, height, from, to, &rects, 8), dy: top) }
                top += CGFloat(height) + gap
            }
        } else if from.chapter == chapter {
            for (p, dx) in shownPages(page) { add(forzac_marks(doc, p, from, to, &rects, 8), dx: dx, dy: 0) }
        }
        return out
    }

    private func drawFocus() {
        CATransaction.begin()
        CATransaction.setDisableActions(true)
        focusLayer.sublayers = nil
        // The match: soft bars in the ink, Android's selection colour.
        for m in matchRects() {
            let r = CALayer()
            r.backgroundColor = color(palette.ink, alpha: 0.14)
            r.cornerRadius = 4
            r.frame = CGRect(x: m.minX - 2, y: bounds.height - m.maxY, width: m.width + 4, height: m.height)
            focusLayer.addSublayer(r)
        }
        let areas = visibleAreas()
        if let f = focus, let stop = stops(areas)[safe: f] {
            for l in focusLayers(areas, stop, palette: palette, height: bounds.height) { focusLayer.addSublayer(l) }
        }
        for l in selectionLayers() { focusLayer.addSublayer(l) }
        focusLayer.frame = bounds
        CATransaction.commit()
        updateBar()
    }

    /// Opens the focused marker, link, image or table; false when nothing is focused.
    func openFocused() -> Bool {
        let areas = visibleAreas()
        guard let f = focus, let stop = stops(areas)[safe: f] else { return false }
        open(areas[stop.lowerBound])
        return true
    }

    /// Opens a marker or link (a note or a short target in the sheet, a place in the book here, an address in the
    /// browser), an image or a table (in the view).
    func open(_ area: Area) {
        if area.kind == 4 { return openHighlight(area.at) }
        if area.isNote { return onCommand?(.note(area.target)) ?? () }
        if area.kind == 2 { return onCommand?(.image(area.target)) ?? () }
        if area.kind == 3 { return onCommand?(.table(area.at)) ?? () }
        var at = ForzacLocator(chapter: 0, block: 0, offset: 0)
        var text: UnsafePointer<CChar>?
        switch forzac_link(doc, area.target, &at, &text) {
        case 1: go(to: at)
        case 2: if let text { onCommand?(.note(String(cString: text))) }
        case 3: if let text, let url = URL(string: String(cString: text)) { NSWorkspace.shared.open(url) }
        case 4: onCommand?(.preview(area.target))
        default: break
        }
    }

    /// Whether opening `area` jumps to another place in the book, rather than showing something over the page.
    func jumps(_ area: Area) -> Bool {
        var at = ForzacLocator(chapter: 0, block: 0, offset: 0)
        var text: UnsafePointer<CChar>?
        return area.kind == 1 && forzac_link(doc, area.target, &at, &text) == 1
    }

    /// Image `id` decoded, from the pages' cache when a page has shown it.
    func picture(_ id: String) -> CGImage? {
        var len = 0
        guard let bytes = forzac_image(doc, id, &len) else { return nil }
        let key = UnsafeRawPointer(bytes)
        if pictures[key] == nil {
            let data = Data(bytesNoCopy: UnsafeMutableRawPointer(mutating: key), count: len, deallocator: .none)
            if let source = CGImageSourceCreateWithData(data as CFData, nil) { pictures[key] = CGImageSourceCreateImageAtIndex(source, 0, nil) }
        }
        return pictures[key]
    }

    override func mouseDown(with e: NSEvent) {
        let p = convert(e.locationInWindow, from: nil)
        mouseDownAt = p
        drag = (CGPoint(x: p.x, y: bounds.height - p.y), CGPoint(x: p.x, y: bounds.height - p.y), false)
    }

    override func mouseDragged(with e: NSEvent) {
        let p = convert(e.locationInWindow, from: nil)
        dragUpdate(CGPoint(x: p.x, y: bounds.height - p.y))
    }

    override func mouseUp(with e: NSEvent) {
        if dragEnd() { return } // a drag that selected
        // A click closes the menu; the second click of a double click that opened it still selects a word.
        if menuOpen?() == true {
            onCommand?(.closePanels)
            if e.clickCount < 2 { return }
        }
        if panelOpen?() == true { return onCommand?(.closePanels) ?? () }
        let p = convert(e.locationInWindow, from: nil)
        if let down = mouseDownAt, hypot(p.x - down.x, p.y - down.y) > 4 { return } // a drag, not a click
        if pillLayer.opacity > 0, pillLayer.frame.contains(p) { return goBack() }
        let y = bounds.height - p.y
        if selectionClick(e.clickCount, CGPoint(x: p.x, y: y), shift: e.modifierFlags.contains(.shift)) { return }
        // A marker or link in a table goes before the table.
        let hits = visibleAreas().filter { $0.rect.insetBy(dx: -4, dy: -4).contains(CGPoint(x: p.x, y: y)) }
        var centre: Float = 0, corner: Float = 0
        forzac_ribbon(doc, &centre, &corner)
        if let a = hits.first(where: { !$0.isViewable }) ?? hits.first {
            open(a)
        } else if y < marginTop && p.x >= CGFloat(corner) + (spread ? pageWidth - inset : 0) { // the right page's corner in a spread
            toggleBookmark()
        } else if p.x >= bounds.width / 3 && p.x <= bounds.width * 2 / 3 {
            if e.clickCount == 1 { onCommand?(.menu) } // the middle opens the menu (`reader-menu`), in both modes
        } else if !scrolling {
            // The side thirds turn the page, as taps do on Android.
            turn(p.x < bounds.width / 3 ? -1 : 1)
        }
    }

    override func scrollWheel(with e: NSEvent) {
        if scrolling {
            if e.hasPreciseScrollingDeltas {
                motion = nil
                offset -= e.scrollingDeltaY
                placeTiles()
                if e.phase == .ended || e.momentumPhase == .ended || (e.phase == [] && e.momentumPhase == []) { settle() }
            } else {
                scroll(by: -e.scrollingDeltaY * lineHeight)
            }
            return
        }
        if !e.hasPreciseScrollingDeltas {
            // A mouse wheel: one step, one page.
            if e.scrollingDeltaY != 0 { turn(e.scrollingDeltaY < 0 ? 1 : -1) }
            return
        }
        // A trackpad swipe: one turn per swipe, in the direction the fingers move.
        if e.phase == .began { swipe = (0, false) }
        let d = abs(e.scrollingDeltaX) > abs(e.scrollingDeltaY) ? e.scrollingDeltaX : 0
        swipe.0 += d
        if !swipe.1 && abs(swipe.0) > 60 {
            swipe.1 = true
            turn(swipe.0 < 0 ? 1 : -1)
        }
    }

    // MARK: Keys

    /// Keys by physical key code, so they work in any layout (Russian included).
    override func keyDown(with e: NSEvent) {
        if menuKey?(e) == true { return }
        if selecting && selectionKey(e) { return }
        // `?` by the character typed: its key and Shift differ between layouts, and as `/` with Shift it would search.
        if e.characters == "?" { return onCommand?(.help) ?? () }
        let shift = e.modifierFlags.contains(.shift)
        let control = e.modifierFlags.contains(.control)
        let line = lineHeight, screen = bounds.height - lineHeight
        switch e.shortcutKey {
        case 0x02 where control: scrolling ? scroll(by: bounds.height / 2) : turn(1)          // ⌃D
        case 0x20 where control: scrolling ? scroll(by: -bounds.height / 2) : turn(-1)        // ⌃U
        case 0x1F where control: goBack()                                                    // ⌃O
        case 0x31 where shift: scrolling ? scroll(by: -screen) : turn(-1)                    // Shift+Space
        case 0x30: focusNext(shift ? -1 : 1)                                                  // Tab
        case 0x24, 0x4C: if !openFocused() { onCommand?(.menu) }                               // Return
        case 0x26, 0x7D: scrolling ? scroll(by: line) : turn(1)                               // j ↓
        case 0x28, 0x7E: scrolling ? scroll(by: -line) : turn(-1)                             // k ↑
        case 0x25, 0x7C, 0x31, 0x79: turn(1)                                                  // l → Space PgDn
        case 0x04, 0x7B, 0x74, 0x33: turn(-1)                                                 // h ← PgUp Backspace
        case 0x1E: jumpChapter(1)                                                             // ]
        case 0x21: jumpChapter(-1)                                                            // [
        case 0x05 where shift, 0x77: edge(end: true)                                          // G End
        case 0x05, 0x73: edge(end: false)                                                     // g Home
        case 0x18, 0x45: step(2, 1)                                                           // = + (keypad +)
        case 0x1B, 0x4E: step(2, -1)                                                          // - (keypad -)
        case 0x1D, 0x52: var s = settings; s.size = forzac_settings_default().size; apply(s)  // 0
        case 0x11: step(0, shift ? -1 : 1)                                                     // t T
        case 0x2E: step(6, 1)                                                                 // m
        case 0x1F: onCommand?(.contents)                                                      // o
        case 0x01: onCommand?(.settings)                                                      // s
        case 0x03: window?.toggleFullScreen(nil)                                              // f
        case 0x2C where shift: onCommand?(.help)                                              // ?
        case 0x2C: onCommand?(.search)                                                        // /
        case 0x35: if focus == nil { clearMatch() } else { unfocus() }; onCommand?(.closePanels) // Esc
        case 0x2D: nextMatch(shift ? -1 : 1)                                                   // n N
        case 0x0B: if shift { onCommand?(.bookmarks) } else { toggleBookmark() }                // b B
        case 0x09: startSelection()                                                           // v
        case 0x0C: window?.performClose(nil)                                                  // q
        default: super.keyDown(with: e)
        }
    }

    func savePNG(_ path: String) {
        guard let img = image(page), let dest = CGImageDestinationCreateWithURL(URL(fileURLWithPath: path) as CFURL, "public.png" as CFString, 1, nil) else { return }
        CGImageDestinationAddImage(dest, img, nil)
        CGImageDestinationFinalize(dest)
        print("saved \(path) (\(img.width)×\(img.height))")
    }
}

extension Array {
    subscript(safe i: Int) -> Element? { indices.contains(i) ? self[i] : nil }
}

// MARK: - Selection and highlights (`quotes`)

/// A text range `[start, end)`: a word, a selection, a highlight.
struct LocRange: Equatable {
    var start, end: ForzacLocator
}

extension ForzacLocator: @retroactive Equatable, @retroactive Comparable {
    public static func == (a: ForzacLocator, b: ForzacLocator) -> Bool { (a.chapter, a.block, a.offset) == (b.chapter, b.block, b.offset) }
    public static func < (a: ForzacLocator, b: ForzacLocator) -> Bool { (a.chapter, a.block, a.offset) < (b.chapter, b.block, b.offset) }
}

/// A selection: the cursor's word and, once anchored, the anchor's word; `keyboard` when the cursor is shown.
struct TextSelection {
    var cursor: LocRange
    var anchor: LocRange?
    var keyboard: Bool
    var range: LocRange? { anchor.map { LocRange(start: min($0.start, cursor.start), end: max($0.end, cursor.end)) } }
}

enum SelectionBar: Equatable {
    case selection, highlight(LocRange)
}

enum BarAction {
    case highlight, copy, share, delete
}

/// Where the cursor goes once the words of what is shown allow it: the next (previous) word, the word on the next
/// (previous) line nearest a middle.
enum PendingMove {
    case step(Int32), line(Int32, CGFloat)
}

/// Android's marker colour: the Forzac ochre at 32 %, 30 % on dark themes.
private let markerRGB: UInt32 = 0xC49A52

extension PageView {
    var selecting: Bool { textSelection != nil || selectionBar != nil }

    /// Sets the highlights (from the reading state) without saving them.
    func setHighlights(_ list: [LocRange]) {
        highlights = list.sorted { ($0.start, $0.end) < ($1.start, $1.end) }
        redrawPages()
    }

    /// Pages and tiles have the highlights baked in: drawn again when they change.
    private func redrawPages() {
        images = [:]
        pageAreas = [:]
        clearTiles()
        show(animated: 0)
    }

    // Words and rectangles shown (y down).

    /// The chapters reaching into the window and where their columns start on screen ("Scroll").
    private func screenChapters() -> [(UInt32, CGFloat)] {
        var out: [(UInt32, CGFloat)] = []
        var (c, top) = (chapter, -offset.rounded())
        while top < bounds.height && c < forzac_chapter_count(doc) {
            out.append((c, top))
            top += CGFloat(forzac_flow_height(doc, c)) + gap
            c += 1
        }
        return out
    }

    /// The words shown: the page's, or in "Scroll" those on lines wholly in the window.
    func shownWords() -> [ForzacWord] {
        var n: UInt32 = 0
        if scrolling {
            var out: [ForzacWord] = []
            let h = bounds.height
            for (c, top) in screenChapters() {
                let p = forzac_flow_words(doc, c, Float(-top), Float(h - top), &n)
                out += (0..<Int(n)).map { p![$0] }.filter { CGFloat($0.rect.y) >= -0.5 && CGFloat($0.rect.y + $0.rect.h) <= h + 0.5 }
            }
            return out
        }
        if atEnd { return [] }
        // A spread's two pages: the right one's words where it is drawn.
        return shownPages(page).flatMap { index, dx in
            let p = forzac_page_words(doc, index, &n)
            return (0..<Int(n)).map { i -> ForzacWord in
                var w = p![i]
                w.rect.x += Float(dx)
                return w
            }
        }
    }

    /// Rectangles over `r` shown (y down).
    func shownRects(_ r: LocRange) -> [CGRect] {
        var buf = [ForzacRect](repeating: ForzacRect(x: 0, y: 0, w: 0, h: 0), count: 256)
        var out: [CGRect] = []
        func add(_ n: UInt32, _ dy: CGFloat, dx: CGFloat = 0) { out += buf[0..<Int(n)].map { CGRect(x: CGFloat($0.x) + dx, y: CGFloat($0.y) + dy, width: CGFloat($0.w), height: CGFloat($0.h)) } }
        if scrolling {
            let h = bounds.height
            for (c, top) in screenChapters() { add(forzac_flow_marks(doc, c, Float(-top), Float(h - top), r.start, r.end, &buf, 256), 0) }
        } else if !atEnd {
            for (p, dx) in shownPages(page) { add(forzac_marks(doc, p, r.start, r.end, &buf, 256), 0, dx: dx) }
        }
        return out
    }

    fileprivate func pageMarks(_ index: UInt32, _ r: LocRange) -> [CGRect] {
        var buf = [ForzacRect](repeating: ForzacRect(x: 0, y: 0, w: 0, h: 0), count: 256)
        let n = forzac_marks(doc, index, r.start, r.end, &buf, 256)
        return buf[0..<Int(n)].map { CGRect(x: CGFloat($0.x), y: CGFloat($0.y), width: CGFloat($0.w), height: CGFloat($0.h)) }
    }

    fileprivate func flowMarks(_ chapter: UInt32, _ y0: Float, _ y1: Float, _ r: LocRange) -> [CGRect] {
        guard r.start.chapter <= chapter && r.end.chapter >= chapter else { return [] }
        var buf = [ForzacRect](repeating: ForzacRect(x: 0, y: 0, w: 0, h: 0), count: 256)
        let n = forzac_flow_marks(doc, chapter, y0, y1, r.start, r.end, &buf, 256)
        return buf[0..<Int(n)].map { CGRect(x: CGFloat($0.x), y: CGFloat($0.y), width: CGFloat($0.w), height: CGFloat($0.h)) }
    }

    /// Android's bands under the text: rounded, 2 px wider on each side, in the marker colour (`ctx` y up, `h` high).
    fileprivate func drawBands(_ bands: [[CGRect]], in ctx: CGContext, height h: CGFloat) {
        ctx.setFillColor(markerColor)
        for r in bands.joined() {
            ctx.addPath(CGPath(roundedRect: CGRect(x: r.minX - 2, y: h - r.maxY, width: r.width + 4, height: r.height), cornerWidth: 3, cornerHeight: 3, transform: nil))
            ctx.fillPath()
        }
    }

    var markerColor: CGColor { color(markerRGB, alpha: palette.paper < 0x808080 ? 0.30 : 0.32) }

    /// `areas` with the highlights' line rectangles added in reading order: each before the first area after its first
    /// line (kind 4, `at` its start).
    fileprivate func withHighlights(_ areas: [Area], _ marked: [(LocRange, [CGRect])]) -> [Area] {
        var areas = areas
        for (q, rects) in marked {
            guard let first = rects.first else { continue }
            let at = areas.firstIndex { $0.rect.minY > first.midY || ($0.rect.maxY > first.minY && $0.rect.minY < first.maxY && $0.rect.minX > first.minX) } ?? areas.count
            areas.insert(contentsOf: rects.map { Area(rect: $0, kind: 4, target: "", at: q.start) }, at: at)
        }
        return areas
    }

    private func word(_ w: ForzacWord) -> LocRange { LocRange(start: w.start, end: w.end) }

    private func wordAt(_ words: [ForzacWord], _ p: CGPoint) -> Int? {
        let i = words.withUnsafeBufferPointer { forzac_word_at($0.baseAddress, UInt32($0.count), Float(p.x), Float(p.y)) }
        return i < 0 ? nil : Int(i)
    }

    // Mouse.

    /// The pointer moved to `p` while pressed: past a few points the drag selects from the pressed word to this one.
    func dragUpdate(_ p: CGPoint) {
        guard var d = drag else { return }
        d.at = p
        let words = shownWords()
        if !d.started {
            guard hypot(p.x - d.press.x, p.y - d.press.y) >= 4 else { drag = d; return }
            d.started = true
            if !grabHandle(d.press) {
                guard let i = wordAt(words, d.press) else { drag = d; return }
                textSelection = TextSelection(cursor: word(words[i]), anchor: word(words[i]), keyboard: false)
            }
            selectionBar = nil
        }
        drag = d
        if let i = wordAt(words, p) { textSelection?.cursor = word(words[i]) }
        // Held at a side, above or below the text: the page turns ("Slide"); out of the window: the column moves.
        if scrolling {
            if p.y < 0 || p.y > bounds.height { startGlide() }
        } else {
            let dir: Int32 = p.x > bounds.width - 24 || p.y > bounds.height - marginTop ? 1 : p.x < 24 || p.y < marginTop ? -1 : 0
            if dir != edgeDir {
                edgeTimer?.invalidate()
                edgeTimer = nil
                edgeDir = dir
                if dir != 0 {
                    edgeTimer = Timer.scheduledTimer(withTimeInterval: 0.5, repeats: true) { [weak self] _ in
                        guard let self, let at = self.drag?.at else { return }
                        self.turn(self.edgeDir)
                        self.dragUpdate(at)
                    }
                }
            }
        }
        drawFocus()
    }

    /// The press at `point` is on a handle: the selection's other end becomes the anchor, so the drag moves this one.
    private func grabHandle(_ point: CGPoint) -> Bool {
        guard let sel = textSelection, let anchor = sel.anchor, let range = sel.range else { return false }
        let rects = shownRects(range)
        guard let first = rects.first, let last = rects.last else { return false }
        let near = { (x: CGFloat, y: CGFloat) in hypot(point.x - x, point.y - y) < 14 }
        let (early, late) = anchor.start <= sel.cursor.start ? (anchor, sel.cursor) : (sel.cursor, anchor)
        if near(first.minX, first.midY) {
            textSelection = TextSelection(cursor: early, anchor: late, keyboard: sel.keyboard)
        } else if near(last.maxX, last.midY) {
            textSelection = TextSelection(cursor: late, anchor: early, keyboard: sel.keyboard)
        } else {
            return false
        }
        return true
    }

    private func startGlide() {
        guard dragLink == nil else { return }
        let l = displayLink(target: self, selector: #selector(glide(_:)))
        l.add(to: .main, forMode: .common)
        dragLink = l
    }

    @objc private func glide(_ l: CADisplayLink) {
        guard let at = drag?.at, at.y < 0 || at.y > bounds.height else {
            dragLink?.invalidate()
            dragLink = nil
            settle()
            return
        }
        let out = at.y < 0 ? at.y : at.y - bounds.height
        offset += out * 8 * CGFloat(l.targetTimestamp - l.timestamp)
        placeTiles()
        dragUpdate(CGPoint(x: at.x, y: min(max(at.y, 0), bounds.height - 1)))
        drag?.at = at
    }

    /// The press ended; true when it was a drag that selected (the bar shows).
    func dragEnd() -> Bool {
        edgeTimer?.invalidate()
        edgeTimer = nil
        edgeDir = 0
        dragLink?.invalidate()
        dragLink = nil
        guard let d = drag else { return false }
        drag = nil
        guard d.started else { return false }
        if textSelection?.range != nil { selectionBar = .selection }
        drawFocus()
        return true
    }

    /// A click (`n` in a row) at `p` (y down); true when the selection used it.
    func selectionClick(_ n: Int, _ p: CGPoint, shift: Bool) -> Bool {
        if let item = barItems.first(where: { $0.0.contains(p) }) {
            act(item.1, at: item.0)
            return true
        }
        let words = shownWords()
        let hit = wordAt(words, p).map { word(words[$0]) }
        let middle = scrolling || (p.x >= bounds.width / 3 && p.x <= bounds.width * 2 / 3)
        switch (n, hit) {
        case (2, let w?) where middle:
            textSelection = TextSelection(cursor: w, anchor: w, keyboard: false)
        case (3, let w?) where middle:
            let block = forzac_block_range(doc, w.start)
            textSelection = TextSelection(cursor: LocRange(start: block.end, end: block.end), anchor: LocRange(start: block.start, end: block.start), keyboard: false)
        case (_, let w?) where shift && textSelection != nil:
            if var sel = textSelection {
                if sel.anchor == nil { sel.anchor = sel.cursor }
                sel.cursor = w
                textSelection = sel
            }
        default:
            guard selecting else { return false }
            dismissSelection() // a click away from the selection clears it, and does nothing else
            return true
        }
        selectionBar = .selection
        drawFocus()
        return true
    }

    /// Shows the bar of the highlight that starts at `start` (a click, or `Enter` on its `Tab` stop).
    func openHighlight(_ start: ForzacLocator) {
        guard let q = highlights.first(where: { $0.start == start }) else { return }
        textSelection = nil
        selectionBar = .highlight(q)
        drawFocus()
    }

    // Keyboard.

    /// `v`: the selection mode, a word cursor on the first word shown.
    func startSelection() {
        guard let w = shownWords().first else { return }
        textSelection = TextSelection(cursor: word(w), anchor: nil, keyboard: true)
        selectionBar = nil
        focus = nil
        drawFocus()
    }

    /// A key while selecting; true when it was used. The mode uses every key; a highlight's bar takes `d`, `c`, `s`, `Esc`.
    func selectionKey(_ e: NSEvent) -> Bool {
        let command = e.modifierFlags.contains(.command) || e.modifierFlags.contains(.control)
        if case .highlight = selectionBar {
            switch e.shortcutKey {
            case 0x02, 0x33, 0x75: act(.delete)          // d Delete
            case 0x08 where !command: act(.copy)          // c
            case 0x01: act(.share)                        // s
            case 0x35: dismissSelection()                 // Esc
            default:
                dismissSelection()
                return false
            }
            return true
        }
        guard textSelection != nil else { return false }
        if command { return false } // ⌘C goes to Edit → Copy
        switch e.shortcutKey {
        case 0x04, 0x7B: moveCursor(.step(-1))            // h ←
        case 0x25, 0x7C: moveCursor(.step(1))             // l →
        case 0x26, 0x7D: moveCursor(.line(1, .nan))       // j ↓
        case 0x28, 0x7E: moveCursor(.line(-1, .nan))      // k ↑
        case 0x09:                                        // v
            if var sel = textSelection {
                sel.keyboard = true
                sel.anchor = sel.anchor == nil ? sel.cursor : nil
                selectionBar = sel.anchor == nil ? nil : .selection
                textSelection = sel
            }
        case 0x1F:                                        // o
            if let a = textSelection?.anchor, let c = textSelection?.cursor {
                textSelection?.anchor = c
                textSelection?.cursor = a
                textSelection?.keyboard = true
            }
        case 0x24, 0x4C: act(.highlight)                  // Return
        case 0x08: act(.copy)                             // c
        case 0x01: act(.share)                            // s
        case 0x35:                                        // Esc
            if textSelection?.anchor != nil && textSelection?.keyboard == true {
                textSelection?.anchor = nil
                selectionBar = nil
            } else {
                dismissSelection()
            }
        default: break
        }
        drawFocus()
        return true
    }

    /// Moves the cursor; past the words shown it turns the page ("Slide") or moves the column a line ("Scroll").
    private func moveCursor(_ how: PendingMove) {
        var how = how
        textSelection?.keyboard = true
        if case .line(let d, let m) = how, m.isNaN, let sel = textSelection {
            let words = shownWords()
            let middle = words.first { $0.start == sel.cursor.start }.map { CGFloat($0.rect.x + $0.rect.w / 2) } ?? 0
            how = .line(d, middle)
        }
        pendingMove = how
        resolvePending()
        guard let pending = pendingMove else { return }
        let dir: Int32 = { switch pending { case .step(let d), .line(let d, _): return d } }()
        if scrolling {
            motion = nil
            offset += CGFloat(dir) * lineHeight
            placeTiles()
            settle()
        } else {
            turn(dir)
            if atEnd { pendingMove = nil; turn(-dir) }
        }
        resolvePending()
    }

    /// Moves the cursor as `pendingMove` says when the words shown allow it.
    private func resolvePending() {
        guard let pending = pendingMove, let sel = textSelection else { return }
        let words = shownWords()
        guard !words.isEmpty else { return }
        let here = words.firstIndex { $0.start == sel.cursor.start }
        var next: Int?
        words.withUnsafeBufferPointer { b in
            let (p, n) = (b.baseAddress, UInt32(b.count))
            switch (pending, here) {
            case (.step(let d), let i?): next = Int(forzac_word_step(p, n, UInt32(i), d))
            case (.line(let d, _), let i?): next = Int(forzac_word_line(p, n, UInt32(i), d))
            case (.step(let d), nil): next = d > 0 ? 0 : words.count - 1
            case (.line(let d, let middle), nil):
                let line = d > 0 ? words[0].rect.y : words[words.count - 1].rect.y
                next = words.indices.filter { abs(words[$0].rect.y - line) < 0.5 }.min { abs(CGFloat(words[$0].rect.x + words[$0].rect.w / 2) - middle) < abs(CGFloat(words[$1].rect.x + words[$1].rect.w / 2) - middle) }
            }
        }
        guard let i = next, i >= 0 else { return }
        // A hyphenated word reached from the right: its first piece.
        let first = words.firstIndex { $0.start == words[i].start } ?? i
        textSelection?.cursor = word(words[first])
        pendingMove = nil
    }

    // Acting.

    /// Copies, highlights, shares or deletes what the bar is for; `at` is the item's rectangle (y down) for the share menu.
    func act(_ action: BarAction, at item: CGRect? = nil) {
        let range: LocRange?
        if case .highlight(let q) = selectionBar { range = q } else { range = textSelection?.range }
        guard let range else { return }
        switch action {
        case .copy:
            NSPasteboard.general.clearContents()
            NSPasteboard.general.setString(String(cString: forzac_text(doc, range.start, range.end)), forType: .string)
        case .share:
            let text = String(cString: forzac_share_text(doc, range.start, range.end))
            let at = item ?? barItems.first { $0.1 == .share }?.0 ?? CGRect(x: bounds.midX, y: bounds.midY, width: 1, height: 1)
            NSSharingServicePicker(items: [text]).show(relativeTo: CGRect(x: at.minX, y: bounds.height - at.maxY, width: at.width, height: at.height), of: self, preferredEdge: .minY)
        case .highlight:
            forzac_quote_add(doc, range.start, range.end)
            reloadHighlights()
            onHighlights?()
            dismissSelection()
            reveal(range)
            return
        case .delete:
            deleteHighlight(range)
        }
        dismissSelection()
    }

    /// Edit → Copy (`⌘C`): the selection, or the highlight whose bar is open.
    @objc func copy(_ sender: Any?) {
        guard textSelection?.range != nil || selectionBar != nil else { return }
        act(.copy)
    }

    /// Ends the selection and closes the bar.
    func dismissSelection() {
        guard textSelection != nil || selectionBar != nil || pendingMove != nil else { return }
        textSelection = nil
        selectionBar = nil
        pendingMove = nil
        drawFocus()
    }

    /// Takes the highlights from the document (after they were loaded or changed); pages and tiles are drawn again.
    func reloadHighlights(redraw: Bool = false) {
        var n: UInt32 = 0
        let p = forzac_quote_places(doc, &n)
        highlights = (0..<Int(n)).map { LocRange(start: p![$0].start, end: p![$0].end) }
        if redraw { redrawPages() }
    }

    /// Deletes the highlight `q` (from the list or its bar), without animation.
    func deleteHighlight(_ q: LocRange) {
        forzac_quote_remove(doc, q.start, q.end)
        reloadHighlights(redraw: true)
        if selectionBar == .highlight(q) { selectionBar = nil }
        focus = nil
        onHighlights?()
        drawFocus()
    }

    /// A new highlight draws in along its lines (90 ms a line, 180–600 ms): the pages drawn with it lie over the old ones,
    /// masked by bands that grow one after another in the window server.
    private func reveal(_ q: LocRange) {
        let rects = shownRects(q)
        let total = rects.reduce(0) { $0 + $1.width + 4 }
        let duration = min(0.6, max(0.18, 0.09 * Double(rects.count)))
        // Old pages and tiles stay on screen under the new ones while the stroke runs.
        images = [:]
        pageAreas = [:]
        let now = CACurrentMediaTime()
        func mask(_ frame: CGRect, top: CGFloat) -> CALayer {
            let m = CALayer()
            m.frame = CGRect(origin: .zero, size: frame.size)
            var before: CGFloat = 0
            for r in rects {
                let band = CALayer()
                band.backgroundColor = color(0x000000)
                band.anchorPoint = CGPoint(x: 0, y: 0.5)
                let w = r.width + 4
                band.frame = CGRect(x: r.minX - 2, y: frame.height - (r.maxY - top), width: w, height: r.height)
                let grow = CABasicAnimation(keyPath: "bounds.size.width")
                grow.fromValue = 0
                grow.toValue = w
                grow.beginTime = now + duration * Double(before / max(total, 1))
                grow.duration = duration * Double(w / max(total, 1))
                grow.fillMode = .backwards
                band.add(grow, forKey: "stroke")
                m.addSublayer(band)
                before += w
            }
            return m
        }
        var overlays: [(CALayer, () -> Void)] = []
        if scrolling {
            let tile = max(1, bounds.height.rounded())
            for (key, t) in tiles where !t.isHidden && key.chapter >= q.start.chapter && key.chapter <= q.end.chapter {
                let (image, _) = tileImage(key, tile: tile)
                let over = CALayer()
                over.contents = image
                over.contentsScale = scale
                over.frame = t.frame
                over.mask = mask(t.frame, top: bounds.height - t.frame.maxY)
                layer!.insertSublayer(over, above: t)
                overlays.append((over, { t.contents = image }))
            }
            for (key, t) in tiles where t.isHidden && key.chapter >= q.start.chapter && key.chapter <= q.end.chapter {
                t.removeFromSuperlayer()
                tiles[key] = nil
                tileAreas[key] = nil
            }
        } else if let image = image(page) {
            let over = CALayer()
            over.contents = image
            over.contentsScale = scale
            over.frame = current.frame
            over.mask = mask(current.frame, top: 0)
            layer!.insertSublayer(over, above: current)
            overlays.append((over, { [weak self] in self?.current.contents = self?.images[self?.page ?? 0] ?? image }))
        }
        DispatchQueue.main.asyncAfter(deadline: .now() + duration + 0.05) {
            CATransaction.begin()
            CATransaction.setDisableActions(true)
            for (over, done) in overlays {
                done()
                over.removeFromSuperlayer()
            }
            CATransaction.commit()
        }
        drawFocus()
    }

    // Drawing.

    /// The selection over the page (bands in the ink at 14 %, the handles) and the keyboard's cursor.
    fileprivate func selectionLayers() -> [CALayer] {
        resolvePending()
        guard let sel = textSelection else { return [] }
        let h = bounds.height
        var out: [CALayer] = []
        if let range = sel.range {
            let rects = shownRects(range)
            for m in rects {
                let r = CALayer()
                r.backgroundColor = color(palette.ink, alpha: 0.14)
                r.cornerRadius = 4
                r.frame = CGRect(x: m.minX - 2, y: h - m.maxY, width: m.width + 4, height: m.height)
                out.append(r)
            }
            // Handles at the ends shown: a bar the line's height and a knob above the start, below the end.
            let words = shownWords()
            if let r = rects.first, words.contains(where: { $0.start <= range.start && range.start < $0.end }) { out += handle(r.minX, r.minY, r.height, start: true) }
            if let r = rects.last, words.contains(where: { $0.start < range.end && range.end <= $0.end }) { out += handle(r.maxX, r.minY, r.height, start: false) }
        }
        if sel.keyboard {
            for m in shownRects(sel.cursor) {
                let r = CALayer()
                r.backgroundColor = color(palette.link, alpha: 0.22)
                r.cornerRadius = 4
                r.frame = CGRect(x: m.minX - 3, y: h - m.maxY + 2, width: m.width + 6, height: m.height - 4)
                out.append(r)
            }
        }
        return out
    }

    private func handle(_ x: CGFloat, _ y: CGFloat, _ lineHeight: CGFloat, start: Bool) -> [CALayer] {
        let h = bounds.height
        let knob: CGFloat = 5.5
        let bar = CALayer()
        bar.backgroundColor = color(palette.link)
        bar.frame = CGRect(x: x - 1, y: h - y - lineHeight, width: 2, height: lineHeight)
        let dot = CALayer()
        dot.backgroundColor = color(palette.link)
        dot.cornerRadius = knob
        let cy = start ? y - knob * 0.6 : y + lineHeight + knob * 0.6
        dot.frame = CGRect(x: x - knob, y: h - cy - knob, width: 2 * knob, height: 2 * knob)
        return [bar, dot]
    }

    /// The bar over the selection or a highlight: the ink as its fill, the paper as its labels (Android's `SelectionBar`).
    fileprivate func updateBar() {
        barItems = []
        let range: LocRange?
        switch selectionBar {
        case .selection?: range = textSelection?.range
        case .highlight(let q)?: range = q
        case nil: range = nil
        }
        let rects = range.map(shownRects) ?? []
        guard let bar = selectionBar, drag?.started != true, let first = rects.first else {
            barLayer.opacity = 0
            return
        }
        let bounds = rects.reduce(first) { $0.union($1) }
        let items: [(String, BarAction)] = bar == .selection
            ? [(tr("Highlight"), .highlight), (tr("Copy"), .copy), (tr("Share"), .share)]
            : [(tr("Delete"), .delete), (tr("Copy"), .copy), (tr("Share"), .share)]
        let font = interfaceFont(15, .medium)
        CATransaction.begin()
        CATransaction.setDisableActions(true)
        barLayer.sublayers = nil
        barLayer.backgroundColor = color(palette.ink)
        var x: CGFloat = 0
        var widths: [CGFloat] = []
        let height = ceil(font.ascender - font.descender) + 24
        for (i, (label, _)) in items.enumerated() {
            if i > 0 {
                let rule = CALayer()
                rule.backgroundColor = color(palette.paper, alpha: 0.2)
                rule.frame = CGRect(x: x, y: (height - 20) / 2, width: 1, height: 20)
                barLayer.addSublayer(rule)
                x += 1
            }
            let text = CATextLayer()
            text.string = NSAttributedString(string: label, attributes: [.font: font, .foregroundColor: nsColor(palette.paper)])
            text.contentsScale = scale
            let size = text.preferredFrameSize()
            text.frame = CGRect(x: x + 16, y: (height - size.height) / 2, width: size.width, height: size.height)
            barLayer.addSublayer(text)
            widths.append(size.width + 32)
            x += size.width + 32
        }
        let h = self.bounds.height
        let above = bounds.minY - 14 - height
        let top = above > 8 ? above : bounds.maxY + 14 + 8
        let left = min(max(8, bounds.midX - x / 2), max(8, self.bounds.width - x - 8))
        barLayer.frame = CGRect(x: left, y: h - top - height, width: x, height: height)
        CATransaction.commit()
        var at = left
        for ((_, action), w) in zip(items, widths) {
            barItems.append((CGRect(x: at, y: top, width: w, height: height), action))
            at += w + 1
        }
        if barLayer.opacity == 0 {
            CATransaction.begin()
            CATransaction.setAnimationDuration(0.12)
            barLayer.opacity = 1
            CATransaction.commit()
        }
    }
}
