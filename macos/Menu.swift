// The reading menu over the page, as the web reader's (`#bar-top`, `#bar-bottom`) and Android's `ReaderMenu`: at the
// top back to the library, the section and the book, the contents; at the bottom a slider over the whole book with the
// chapters' ticks, "Page N of M", the time left in the chapter and "Aa". A click in the middle third of the page or
// Return opens it; a click on the page or Esc closes it; ←/→/h/l move the slider and Return goes there.

import AppKit
import CoreText
import QuartzCore

/// The chevron and the list of Lucide (ISC), on their 24×24 grid.
private let chevronLeft: [Shape] = [.line(15, 18, 9, 12), .line(9, 12, 15, 6)]
private let list: [Shape] = [6, 12, 18].flatMap { y -> [Shape] in [.line(3, y, 3.01, y), .line(8, y, 21, y)] }

/// A button of the bars: a child view centred, an action on click, the ink at 6 % under the pointer.
private final class BarButton: NSView {
    let child: NSView
    var action: (() -> Void)?
    private var tint: UInt32 = 0

    init(_ child: NSView, tooltip: String) {
        self.child = child
        super.init(frame: .zero)
        wantsLayer = true
        toolTip = tooltip
        addSubview(child)
    }

    required init?(coder: NSCoder) { fatalError() }

    override var isFlipped: Bool { true }

    func colour(_ p: Palette) { tint = p.ink }

    override func layout() {
        super.layout()
        let size = child.intrinsicContentSize
        child.frame = CGRect(x: (bounds.width - size.width) / 2, y: (bounds.height - size.height) / 2, width: size.width, height: size.height)
    }

    override func updateTrackingAreas() {
        super.updateTrackingAreas()
        trackingAreas.forEach(removeTrackingArea)
        addTrackingArea(NSTrackingArea(rect: bounds, options: [.mouseEnteredAndExited, .activeInKeyWindow], owner: self))
    }

    override func mouseEntered(with e: NSEvent) { layer?.backgroundColor = color(tint, alpha: 0.06) }
    override func mouseExited(with e: NSEvent) { layer?.backgroundColor = nil }
    override func mouseDown(with e: NSEvent) {}
    override func mouseUp(with e: NSEvent) {
        layer?.backgroundColor = nil
        if bounds.contains(convert(e.locationInWindow, from: nil)) { action?() }
    }
}

/// The slider over the book's pages, as Android's `ProgressSlider`: a 2 px track in the line colour, a 1 × 8 px tick at
/// each chapter's first page, the part read in the ink, a 16 px thumb. Pressing or dragging scrubs, releasing goes.
private final class Slider: NSView {
    private let track = CALayer(), ticks = CAShapeLayer(), done = CALayer(), thumb = CALayer()
    private var count: UInt32 = 0, page: UInt32 = 0, starts: [UInt32] = []
    var onScrub: ((UInt32) -> Void)?
    var onRelease: (() -> Void)?

    override init(frame: NSRect) {
        super.init(frame: frame)
        wantsLayer = true
        for l in [track, ticks, done, thumb] {
            l.actions = ["position": NSNull(), "bounds": NSNull(), "path": NSNull(), "backgroundColor": NSNull(), "strokeColor": NSNull()]
            layer!.addSublayer(l)
        }
        thumb.cornerRadius = 8
        ticks.lineWidth = 1
    }

    required init?(coder: NSCoder) { fatalError() }

    override var isFlipped: Bool { true }

    func set(count: UInt32, page: UInt32, starts: [UInt32], palette p: Palette) {
        (self.count, self.page, self.starts) = (count, page, starts)
        track.backgroundColor = color(p.line)
        ticks.strokeColor = color(p.muted, alpha: 0.6)
        done.backgroundColor = color(p.ink)
        thumb.backgroundColor = color(p.ink)
        needsLayout = true
    }

    private func x(_ p: UInt32) -> CGFloat { 8 + (count <= 1 ? 0 : (bounds.width - 16) * CGFloat(p) / CGFloat(count - 1)) }

    override func layout() {
        super.layout()
        let y = bounds.height / 2
        track.frame = CGRect(x: 8, y: y - 1, width: bounds.width - 16, height: 2)
        let path = CGMutablePath()
        for s in starts where s > 0 {
            let tx = x(s).rounded() + 0.5
            path.move(to: CGPoint(x: tx, y: y - 4))
            path.addLine(to: CGPoint(x: tx, y: y + 4))
        }
        ticks.path = path
        done.frame = CGRect(x: 8, y: y - 1, width: x(page) - 8, height: 2)
        thumb.frame = CGRect(x: x(page) - 8, y: y - 8, width: 16, height: 16)
    }

    private func page(at e: NSEvent) -> UInt32 {
        guard count > 1 else { return 0 }
        let fraction = min(1, max(0, (convert(e.locationInWindow, from: nil).x - 8) / (bounds.width - 16)))
        return UInt32((fraction * CGFloat(count - 1)).rounded())
    }

    override func mouseDown(with e: NSEvent) { onScrub?(page(at: e)) }
    override func mouseDragged(with e: NSEvent) { onScrub?(page(at: e)) }
    override func mouseUp(with e: NSEvent) { onRelease?() }
}

/// A bar: the paper with a 1 px line on its edge towards the page; its children laid out by `place`.
private final class Bar: NSView {
    let rule = CALayer()
    let lineAtTop: Bool
    var place: ((Bar) -> Void)?

    init(lineAtTop: Bool) {
        self.lineAtTop = lineAtTop
        super.init(frame: .zero)
        wantsLayer = true
        rule.actions = ["position": NSNull(), "bounds": NSNull(), "backgroundColor": NSNull()]
        layer!.addSublayer(rule)
    }

    required init?(coder: NSCoder) { fatalError() }

    override var isFlipped: Bool { true }
    // Clicks on the bars stay there: the page under them does not get them.
    override func mouseDown(with e: NSEvent) {}
    override func mouseUp(with e: NSEvent) {}

    override func layout() {
        super.layout()
        rule.frame = CGRect(x: 0, y: lineAtTop ? 0 : bounds.height - 1, width: bounds.width, height: 1)
        place?(self)
    }
}

final class ReaderMenu {
    private weak var view: PageView?
    private let top = Bar(lineAtTop: false)
    private let bottom = Bar(lineAtTop: true)
    private let back = BarButton(IconView(chevronLeft, size: 24), tooltip: tr("Back to library"))
    private let contents = BarButton(IconView(list, size: 24), tooltip: tr("Contents"))
    private let aa = BarButton(label("Aa", bookFont("PT Serif", 18), 0), tooltip: tr("Text settings"))
    private let section = label("", interfaceFont(15, .medium), 0)
    private let book = label("", interfaceFont(12), 0)
    private let slider = Slider()
    private let pageLine = label("", interfaceFont(14, .medium), 0)
    private let hint = label("", interfaceFont(12), 0)
    /// The book page the slider shows while the reader moves it; nil: the reading position's.
    private var scrub: UInt32?
    private(set) var isOpen = false
    var onBack: (() -> Void)?
    var onContents: (() -> Void)?
    var onSettings: (() -> Void)?

    private static let topHeight: CGFloat = 56
    /// 16 above the slider (28), 12, the row (44), 18 below: the web reader's `.bar-bottom`.
    private static let bottomHeight: CGFloat = 118

    init(view: PageView, in container: NSView, below sibling: NSView) {
        self.view = view
        for l in [section, book, pageLine, hint] {
            l.lineBreakMode = .byTruncatingTail
            l.maximumNumberOfLines = 1
        }
        section.alignment = .center
        book.alignment = .center
        // Tabular figures, so the line does not jitter while the slider moves.
        pageLine.font = NSFont(descriptor: interfaceFont(14, .medium).fontDescriptor.addingAttributes(
            [.featureSettings: [[NSFontDescriptor.FeatureKey.typeIdentifier: kNumberSpacingType, NSFontDescriptor.FeatureKey.selectorIdentifier: kMonospacedNumbersSelector]]]), size: 14)
        aa.layer!.cornerRadius = 22
        aa.layer!.borderWidth = 1
        back.layer!.cornerRadius = 24
        contents.layer!.cornerRadius = 24
        [back, section, book, contents].forEach(top.addSubview)
        [slider, pageLine, hint, aa].forEach(bottom.addSubview)
        top.place = { [unowned self] bar in
            let w = bar.bounds.width
            back.frame = CGRect(x: 4, y: 4, width: 48, height: 48)
            contents.frame = CGRect(x: w - 52, y: 4, width: 48, height: 48)
            section.frame = CGRect(x: 60, y: 9, width: w - 120, height: 20)
            book.frame = CGRect(x: 60, y: 30, width: w - 120, height: 16)
        }
        bottom.place = { [unowned self] bar in
            let x = max(20, (bar.bounds.width - 640) / 2), width = bar.bounds.width - 2 * x
            slider.frame = CGRect(x: x, y: 16, width: width, height: 28)
            aa.frame = CGRect(x: x + width - 52, y: 56, width: 52, height: 44)
            let text = width - 64
            if pageLine.isHidden {
                hint.frame = CGRect(x: x, y: 56 + 14, width: text, height: 16)
            } else {
                pageLine.frame = CGRect(x: x, y: 56 + 3, width: text, height: 19)
                hint.frame = CGRect(x: x, y: 56 + 25, width: text, height: 16)
            }
        }
        top.autoresizingMask = [.width, .minYMargin]
        bottom.autoresizingMask = [.width, .maxYMargin]
        for bar in [top, bottom] {
            bar.isHidden = true
            container.addSubview(bar, positioned: .below, relativeTo: sibling)
        }
        back.action = { [weak self] in self?.onBack?() }
        contents.action = { [weak self] in self?.onContents?() }
        aa.action = { [weak self] in self?.onSettings?() }
        slider.onScrub = { [weak self] in self?.scrubTo($0) }
        slider.onRelease = { [weak self] in self?.go() }
        view.onPages = { [weak self] in if self?.isOpen == true { self?.showPlace() } }
    }

    private func frames(open: Bool) -> (CGRect, CGRect) {
        guard let c = top.superview?.bounds else { return (.zero, .zero) }
        let (th, bh) = (ReaderMenu.topHeight, ReaderMenu.bottomHeight)
        // Closed: half their height towards the window's edge (AppKit's y up).
        let shift: (CGFloat, CGFloat) = open ? (0, 0) : (th / 2, -bh / 2)
        return (CGRect(x: 0, y: c.height - th + shift.0, width: c.width, height: th), CGRect(x: 0, y: shift.1, width: c.width, height: bh))
    }

    func open() {
        guard !isOpen, let view else { return }
        isOpen = true
        view.dismissSelection()
        scrub = nil
        let p = view.palette
        for bar in [top, bottom] {
            bar.layer!.backgroundColor = color(p.paper)
            bar.rule.backgroundColor = color(p.line)
        }
        for b in [back, contents, aa] { b.colour(p) }
        for icon in [back.child, contents.child] { (icon as? IconView)?.tint = nsColor(p.ink) }
        (aa.child as? NSTextField)?.textColor = nsColor(p.ink)
        aa.layer!.borderColor = color(p.line)
        section.textColor = nsColor(p.ink)
        pageLine.textColor = nsColor(p.ink)
        book.textColor = nsColor(p.muted)
        hint.textColor = nsColor(p.muted)
        showPlace()
        let (closedTop, closedBottom) = frames(open: false)
        top.frame = closedTop
        bottom.frame = closedBottom
        for bar in [top, bottom] {
            bar.alphaValue = 0
            bar.isHidden = false
        }
        let (openTop, openBottom) = frames(open: true)
        NSAnimationContext.runAnimationGroup { ctx in
            ctx.duration = 0.2
            ctx.timingFunction = CAMediaTimingFunction(name: .easeOut)
            top.animator().alphaValue = 1
            bottom.animator().alphaValue = 1
            top.animator().frame = openTop
            bottom.animator().frame = openBottom
        }
    }

    /// Closes the menu; false when it was not open.
    @discardableResult
    func close() -> Bool {
        guard isOpen else { return false }
        isOpen = false
        scrub = nil
        let (closedTop, closedBottom) = frames(open: false)
        NSAnimationContext.runAnimationGroup({ ctx in
            ctx.duration = 0.15
            ctx.timingFunction = CAMediaTimingFunction(name: .easeIn)
            top.animator().alphaValue = 0
            bottom.animator().alphaValue = 0
            top.animator().frame = closedTop
            bottom.animator().frame = closedBottom
        }, completionHandler: { [weak self] in
            guard let self, !self.isOpen else { return }
            self.top.isHidden = true
            self.bottom.isHidden = true
        })
        return true
    }

    /// The keys while the menu is open: ←/h and →/l move the slider a page, Return goes to the page shown (or closes
    /// the menu when it is the reader's), Esc closes. True when the key was the menu's; any other key closes the menu
    /// and does what it does.
    func key(_ e: NSEvent) -> Bool {
        guard isOpen else { return false }
        switch e.shortcutKey {
        case 0x7B, 0x04: step(-1)            // ← h
        case 0x7C, 0x25: step(1)             // → l
        case 0x24, 0x4C: go()                // Return
        case 0x35: close()                   // Esc
        default:
            close()
            return false
        }
        return true
    }

    private func step(_ delta: Int64) {
        guard let view, let current = scrub ?? view.bookPage else { return }
        let total = Int64(forzac_book_pages(view.doc))
        scrubTo(UInt32(min(max(Int64(current) + delta, 0), total - 1)))
    }

    private func scrubTo(_ page: UInt32) {
        guard let view, forzac_book_pages(view.doc) > 0 else { return }
        scrub = page
        showPlace()
    }

    /// Goes to the page the slider shows (remembered for ⌃O) and closes the menu.
    private func go() {
        let target = scrub.flatMap { $0 == view?.bookPage ? nil : $0 }
        close()
        if let view, let page = target { view.go(to: forzac_book_page_start(view.doc, page)) }
    }

    /// The labels and the slider for the reading position, or for the page the slider is moved to.
    private func showPlace() {
        guard let view else { return }
        let doc = view.doc
        section.stringValue = String(cString: forzac_section(doc, view.locator))
        book.stringValue = String(cString: forzac_title(doc))
        let total = forzac_book_pages(doc)
        slider.isHidden = total == 0
        pageLine.isHidden = total == 0
        if total > 0 {
            let shown = scrub ?? forzac_book_page_of(doc, view.locator)
            pageLine.stringValue = fill(tr("Page {n} of {m}"), ["n": "\(shown + 1)", "m": "\(total)"])
            var n: UInt32 = 0
            let starts = forzac_chapter_starts(doc, &n).map { Array(UnsafeBufferPointer(start: $0, count: Int(n))) } ?? []
            slider.set(count: total, page: shown, starts: starts, palette: view.palette)
        }
        if let page = scrub, total > 0 {
            hint.stringValue = String(cString: forzac_section(doc, forzac_book_page_start(doc, page)))
        } else {
            let minutes = forzac_minutes_left(doc, view.locator)
            hint.stringValue = minutes == 0 ? tr("Less than a minute left in chapter") : fill(tr("{n} min left in chapter"), ["n": "\(minutes)"])
        }
        bottom.needsLayout = true
    }
}
