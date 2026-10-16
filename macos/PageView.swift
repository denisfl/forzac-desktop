// The reading view: pages typeset by the core, drawn with Core Text into bitmaps and moved by Core Animation, which runs
// in the window server: no app work per frame in "Slide", ProMotion and Low Power Mode handled by the system, nothing
// drawn while idle. "Scroll" shows the book as one column of tiles a screen high; only their positions change per frame.

import AppKit
import QuartzCore

/// What the view asks its window for.
enum Command {
    case contents, settings, search, help, closePanels, note(String)
}

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
    var page: UInt32 = 0
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
    /// Where jumps started, the latest last (`⌃O`).
    private var back: [ForzacLocator] = []
    /// The matches of the last search gone to (start, end), and the one highlighted (`n`/`N` move it).
    private var matches: [(ForzacLocator, ForzacLocator)] = []
    private var currentMatch: Int?
    /// Horizontal trackpad scrolling not yet turned into a page turn, and whether this swipe already turned one.
    private var swipe: (CGFloat, Bool) = (0, false)

    // "Scroll".
    private var offset: CGFloat = 0
    private var tiles: [TileKey: CALayer] = [:]
    private var tileAreas: [TileKey: [Area]] = [:]
    private var motion: (from: CGFloat, to: CGFloat, start: CFTimeInterval?)?
    private var link: CADisplayLink?
    private var marginTop: CGFloat = 0
    private var lineHeight: CGFloat = 30

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
        focusLayer.actions = ["sublayers": NSNull(), "contents": NSNull(), "position": NSNull(), "bounds": NSNull()]
        layer!.addSublayer(focusLayer)
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
        let key: [Float] = [Float(size.width), Float(size.height), Float(s.size), Float(s.family), Float(s.spacing), Float(s.align), Float(s.margins), Float(chapter)]
        if laidOutFor == key { return }
        let styleChanged = laidOutFor.map { Array($0.dropLast()) != Array(key.dropLast()) } ?? true
        pageCount = withUnsafePointer(to: settings) { forzac_paginate(doc, chapter, Float(size.width), Float(size.height), $0) }
        var top: Float = 0, line: Float = 0
        forzac_metrics(doc, &top, &line)
        (marginTop, lineHeight) = (CGFloat(top), CGFloat(line))
        if toLast {
            page = pageCount - 1
            locator = forzac_page_start(doc, page) // turned back into this chapter: its last page
        } else {
            page = forzac_page_of(doc, locator)
        }
        toLast = false
        laidOutFor = key
        images = [:]
        pageAreas = [:]
        focus = nil
        if styleChanged {
            pictures = [:]
            clearTiles()
            if scrolling { offset = CGFloat(forzac_flow_y(doc, locator)) - marginTop }
        }
    }

    // MARK: Slide

    /// The page as a bitmap at the backing scale.
    private func image(_ index: UInt32) -> CGImage? {
        if let i = images[index] { return i }
        let h = bounds.height
        let image = bitmap(bounds.size, scale: scale, background: palette.paper) { ctx in
            let page = forzac_page(doc, index)
            pageAreas[index] = areas(of: page)
            drawPage(page, in: ctx, height: h, palette: palette, fonts: fonts, pictures: &pictures)
        }
        images[index] = image
        return image
    }

    /// Shows the current page; `animated` +1 slides it in from the right, −1 from the left.
    func show(animated dir: CGFloat) {
        layer!.backgroundColor = color(palette.paper)
        if scrolling {
            current.contents = nil
            previous.contents = nil
            placeTiles()
            return
        }
        guard let img = image(page) else { return }
        let frame = bounds
        CATransaction.begin()
        CATransaction.setDisableActions(true)
        if dir != 0 {
            previous.contents = current.contents
            previous.contentsScale = scale
            previous.frame = frame
        }
        current.contents = img
        current.contentsScale = scale
        current.frame = frame
        CATransaction.commit()
        if dir != 0 {
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
        let target = Int64(page) + Int64(delta)
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
        page = end ? pageCount - 1 : 0
        locator = forzac_page_start(doc, page)
        focus = nil
        show(animated: 0)
        onMoved?()
    }

    /// Goes to `at` (a contents entry, a link); `remember` keeps the place left for `⌃O`.
    func go(to at: ForzacLocator, remember: Bool = true) {
        if remember {
            back.append(locator)
            if back.count > 50 { back.removeFirst() }
        }
        locator = at
        chapter = at.chapter
        focus = nil
        motion = nil
        laidOutFor = nil
        relayout()
        if scrolling { offset = CGFloat(forzac_flow_y(doc, locator)) - marginTop }
        show(animated: 0)
        onMoved?()
    }

    /// Back to where the last jump started.
    func goBack() {
        guard let at = back.popLast() else { return }
        go(to: at, remember: false)
    }

    // MARK: Settings

    func apply(_ new: ForzacSettings) {
        if new.turning != settings.turning { switchMode() }
        let recolour = new.theme != settings.theme
        settings = new
        if recolour { recoloured() }
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

    /// The ink is baked into the bitmaps: a new theme draws them again, without a new layout.
    private func recoloured() {
        images = [:]
        clearTiles()
    }

    /// "Slide" ↔ "Scroll", keeping the line at the top of the text.
    private func switchMode() {
        motion = nil
        focus = nil
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
        let end = max(0, CGFloat(forzac_flow_height(doc, chapter)) - bounds.height)
        let last = chapter + 1 == count
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
        let w = bounds.width
        let layer = CALayer()
        layer.actions = ["position": NSNull(), "bounds": NSNull(), "contents": NSNull()]
        let content = forzac_flow(doc, key.chapter, Float(CGFloat(key.index) * tile), Float(CGFloat(key.index + 1) * tile))
        tileAreas[key] = areas(of: content)
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
        layer.contents = bitmap(CGSize(width: w, height: bottom), scale: scale, background: nil) { ctx in
            drawPage(content, in: ctx, height: bottom, palette: palette, fonts: fonts, pictures: &pictures)
        }
        layer.contentsScale = scale
        layer.bounds = CGRect(x: 0, y: 0, width: w, height: bottom)
        self.layer!.insertSublayer(layer, below: focusLayer)
        tiles[key] = layer
    }

    /// Puts the tiles on screen where the offset says; drops tiles far from it.
    private func placeTiles() {
        guard bounds.height > 0 else { return }
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
        CATransaction.commit()
        if tiles.count > 16 {
            for key in tiles.keys where !shown.contains(key) && (key.chapter != chapter || abs(key.index - Int(offset / max(1, h))) > 2) {
                tiles.removeValue(forKey: key)?.removeFromSuperlayer()
                tileAreas[key] = nil
            }
        }
        drawFocus()
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
        if !scrolling { return pageAreas[page] ?? [] }
        var out: [Area] = []
        forTiles(from: 0, to: bounds.height) { top, key in
            out += (tileAreas[key] ?? []).map { Area(rect: $0.rect.offsetBy(dx: 0, dy: top), isNote: $0.isNote, target: $0.target) }
        }
        return out.filter { $0.rect.maxY > 0 && $0.rect.minY < bounds.height }
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
        guard let i = currentMatch, let (from, to) = matches[safe: i] else { return [] }
        var rects = [ForzacRect](repeating: ForzacRect(x: 0, y: 0, w: 0, h: 0), count: 8)
        var out: [CGRect] = []
        func add(_ n: UInt32, dy: CGFloat) {
            out += rects[0..<Int(n)].map { CGRect(x: CGFloat($0.x), y: CGFloat($0.y) + dy, width: CGFloat($0.w), height: CGFloat($0.h)) }
        }
        if scrolling {
            var top = -offset.rounded()
            for c in [chapter, chapter + 1] where c < forzac_chapter_count(doc) {
                let height = forzac_flow_height(doc, c)
                if from.chapter == c { add(forzac_flow_marks(doc, c, 0, height, from, to, &rects, 8), dy: top) }
                top += CGFloat(height) + gap
            }
        } else if from.chapter == chapter {
            add(forzac_marks(doc, page, from, to, &rects, 8), dy: 0)
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
            let h = bounds.height
            for a in areas[stop] {
                let r = CALayer()
                r.backgroundColor = color(palette.link, alpha: 0.22)
                r.cornerRadius = 4
                r.frame = CGRect(x: a.rect.minX - 3, y: h - a.rect.maxY - 2, width: a.rect.width + 6, height: a.rect.height + 4)
                focusLayer.addSublayer(r)
            }
        }
        focusLayer.frame = bounds
        CATransaction.commit()
    }

    /// Opens the focused marker or link; false when nothing is focused.
    func openFocused() -> Bool {
        let areas = visibleAreas()
        guard let f = focus, let stop = stops(areas)[safe: f] else { return false }
        open(areas[stop.lowerBound])
        return true
    }

    private func open(_ area: Area) {
        if area.isNote { return onCommand?(.note(area.target)) ?? () }
        var at = ForzacLocator(chapter: 0, block: 0, offset: 0)
        var text: UnsafePointer<CChar>?
        switch forzac_link(doc, area.target, &at, &text) {
        case 1: go(to: at)
        case 2: if let text { onCommand?(.note(String(cString: text))) }
        case 3: if let text, let url = URL(string: String(cString: text)) { NSWorkspace.shared.open(url) }
        default: break
        }
    }

    override func mouseUp(with e: NSEvent) {
        if panelOpen?() == true { return onCommand?(.closePanels) ?? () }
        let p = convert(e.locationInWindow, from: nil)
        let y = bounds.height - p.y
        if let a = visibleAreas().first(where: { $0.rect.insetBy(dx: -4, dy: -4).contains(CGPoint(x: p.x, y: y)) }) {
            open(a)
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
        let shift = e.modifierFlags.contains(.shift)
        let control = e.modifierFlags.contains(.control)
        let line = lineHeight, screen = bounds.height - lineHeight
        switch e.shortcutKey {
        case 0x02 where control: scrolling ? scroll(by: bounds.height / 2) : turn(1)          // ⌃D
        case 0x20 where control: scrolling ? scroll(by: -bounds.height / 2) : turn(-1)        // ⌃U
        case 0x1F where control: goBack()                                                    // ⌃O
        case 0x31 where shift: scrolling ? scroll(by: -screen) : turn(-1)                    // Shift+Space
        case 0x30: focusNext(shift ? -1 : 1)                                                  // Tab
        case 0x24, 0x4C: if !openFocused() { turn(1) }                                         // Return
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
