// The reading stats window (`reading-stats`): a month or a year from the reading log of every device — "Month · Year",
// the period with arrows, three figures, the reading calendar, the finished books — in the reader's theme, as the
// Android app's screen; and its 1080 × 1920 picture for the share menu. The numbers, the calendar's geometry and the
// texts come from the core (`forzac_stats`); this file only draws them. Esc and ⌘W close the window.

import AppKit

/// The shades of the link colour for the levels 1–4 of a day (level 0 is the line colour).
private let levelAlpha: [CGFloat] = [0, 0.3, 0.5, 0.75, 1]
/// The content's widest column and its margin in the window.
private let column: CGFloat = 560
private let margin: CGFloat = 24

/// A period's stats as Swift values (`ForzacStats` lives only until the next call).
struct StatsModel {
    struct Cell {
        let day: Int64
        let rect: CGRect
        let level: Int
        let today, future: Bool
        let number: UInt32
        let minutes: UInt32
    }

    let year: Int32
    let month: UInt32
    let title, big, small, fileName: String
    let figures: [(String, String)]
    let canPrevious, canNext: Bool
    let height, radius: CGFloat
    let cells: [Cell]
    let labels: [(String, CGRect, Bool)]
    let books: [(String, String)]

    /// The stats of `month` of `year` (0: the year; `year` 0: the current month), the calendar `width` wide at `scale`.
    init(_ state: OpaquePointer, year: Int32, month: UInt32, width: CGFloat, scale: CGFloat, columns: UInt32) {
        let s = forzac_stats(state, year, month, Float(width), Float(scale), columns, zoneOffset())
        func str(_ p: UnsafePointer<CChar>?) -> String { p.map { String(cString: $0) } ?? "" }
        func rect(_ r: ForzacRect) -> CGRect { CGRect(x: CGFloat(r.x), y: CGFloat(r.y), width: CGFloat(r.w), height: CGFloat(r.h)) }
        self.year = s.year
        self.month = s.month
        title = str(s.title)
        big = str(s.big)
        small = str(s.small)
        fileName = str(s.file_name)
        figures = [(str(s.figures.0), str(s.captions.0)), (str(s.figures.1), str(s.captions.1)), (str(s.figures.2), str(s.captions.2))]
        canPrevious = s.can_previous
        canNext = s.can_next
        height = CGFloat(s.height)
        radius = CGFloat(s.radius)
        cells = (0..<Int(s.cell_count)).map { i in
            let c = s.cells[i]
            return Cell(day: c.day, rect: rect(c.rect), level: Int(c.level), today: c.today, future: c.future, number: c.number, minutes: c.minutes)
        }
        labels = (0..<Int(s.label_count)).map { i in (str(s.labels[i].text), rect(s.labels[i].rect), s.labels[i].centered) }
        books = (0..<Int(s.book_count)).map { i in (str(s.books[i].title), str(s.books[i].caption)) }
    }
}

/// `text` in `font` and colour, with an optional alignment for wrapped text.
private func attributed(_ text: String, _ font: NSFont, _ rgb: UInt32, alpha: CGFloat = 1, centered: Bool = false) -> NSAttributedString {
    let style = NSMutableParagraphStyle()
    style.alignment = centered ? .center : .natural
    style.lineBreakMode = .byWordWrapping
    return NSAttributedString(string: text, attributes: [.font: font, .foregroundColor: nsColor(rgb, alpha: alpha), .paragraphStyle: style])
}

/// Draws `text` with its top left at `p` (y down) wrapped to `width`; returns its height.
@discardableResult
private func drawText(_ text: NSAttributedString, at p: CGPoint, width: CGFloat = .greatestFiniteMagnitude, lines: Int = 0) -> CGFloat {
    let options: NSString.DrawingOptions = [.usesLineFragmentOrigin, .truncatesLastVisibleLine]
    var bounds = text.boundingRect(with: CGSize(width: width, height: .greatestFiniteMagnitude), options: options)
    if lines > 0, let font = text.attribute(.font, at: 0, effectiveRange: nil) as? NSFont {
        bounds.size.height = min(bounds.height, ceil(font.ascender - font.descender + font.leading) * CGFloat(lines) + 1)
    }
    text.draw(with: CGRect(x: p.x, y: p.y, width: width == .greatestFiniteMagnitude ? bounds.width + 1 : width, height: bounds.height), options: options)
    return bounds.height
}

/// The calendar of `m` with its top left at `origin` (y down), in the current graphics context.
func drawCalendar(_ m: StatsModel, at origin: CGPoint, palette p: Palette, numbers: NSFont, labels: NSFont, stroke: CGFloat) {
    guard let ctx = NSGraphicsContext.current?.cgContext else { return }
    for (text, r, centered) in m.labels {
        let t = attributed(text, labels, p.muted)
        let x = centered ? r.midX - t.size().width / 2 : r.minX
        drawText(t, at: CGPoint(x: origin.x + x, y: origin.y + r.minY))
    }
    for c in m.cells {
        let r = c.rect.offsetBy(dx: origin.x, dy: origin.y)
        let path = CGPath(roundedRect: r, cornerWidth: m.radius, cornerHeight: m.radius, transform: nil)
        if !c.future {
            ctx.setFillColor(c.level == 0 ? color(p.line) : color(p.link, alpha: levelAlpha[c.level]))
            ctx.addPath(path)
            ctx.fillPath()
        }
        if c.today {
            ctx.setStrokeColor(color(p.ink))
            ctx.setLineWidth(stroke)
            ctx.addPath(CGPath(roundedRect: r.insetBy(dx: stroke / 2, dy: stroke / 2), cornerWidth: m.radius, cornerHeight: m.radius, transform: nil))
            ctx.strokePath()
        }
        if c.number > 0 {
            let ink = c.future ? p.muted : c.level >= 3 ? p.paper : p.ink
            let t = attributed(String(c.number), numbers, ink, alpha: c.future ? 0.6 : 1)
            let size = t.size()
            drawText(t, at: CGPoint(x: r.midX - size.width / 2, y: r.midY - size.height / 2))
        }
    }
}

/// The period's picture for sharing, 1080 × 1920 PNG, as the Android app's `StatsImage`.
func statsPicture(_ state: OpaquePointer, year: Int32, month: UInt32, palette p: Palette) -> Data? {
    let (w, h, edge): (CGFloat, CGFloat, CGFloat) = (1080, 1920, 88)
    let width = w - 2 * edge
    let m = StatsModel(state, year: year, month: month, width: width, scale: 2.5, columns: 4)
    guard let ctx = CGContext(data: nil, width: Int(w), height: Int(h), bitsPerComponent: 8, bytesPerRow: 0, space: CGColorSpace(name: CGColorSpace.sRGB)!,
                              bitmapInfo: CGImageAlphaInfo.premultipliedFirst.rawValue) else { return nil }
    ctx.setFillColor(color(p.paper))
    ctx.fill(CGRect(x: 0, y: 0, width: w, height: h))
    ctx.translateBy(x: 0, y: h)
    ctx.scaleBy(x: 1, y: -1)
    NSGraphicsContext.saveGraphicsState()
    NSGraphicsContext.current = NSGraphicsContext(cgContext: ctx, flipped: true)
    defer { NSGraphicsContext.restoreGraphicsState() }
    let serif = { (size: CGFloat) in bookFont("Literata", size) }

    // The period large: the month and its year under it, or the year.
    var top = edge
    if m.month > 0 {
        top += drawText(attributed(m.big, serif(150), p.ink), at: CGPoint(x: edge, y: top))
        top += 4 + drawText(attributed(m.small, serif(56), p.muted), at: CGPoint(x: edge, y: top + 4))
    } else {
        top += drawText(attributed(m.big, serif(220), p.ink), at: CGPoint(x: edge, y: top))
    }
    // "Forzac" small at the bottom.
    let brand = attributed(tr("Forzac"), serif(44), p.muted)
    let brandSize = brand.size()
    let bottom = h - edge - brandSize.height
    drawText(brand, at: CGPoint(x: (w - brandSize.width) / 2, y: bottom))

    // The figures: one size for the three, the largest that fits.
    let weights: [CGFloat] = [1.4, 1, 1]
    let widths = weights.map { width * $0 / weights.reduce(0, +) }
    var size: CGFloat = 96
    while size > 40 && zip(m.figures, widths).contains(where: { attributed($0.0, serif(size), p.ink).size().width > $1 - 40 }) { size -= 4 }
    let valueHeight = ceil(serif(size).ascender - serif(size).descender)
    let captions = zip(m.figures, widths).map { attributed($0.1, interfaceFont(38), p.muted).boundingRect(with: CGSize(width: $1 - 40, height: 1000), options: .usesLineFragmentOrigin).height }
    let figuresHeight = valueHeight + 8 + (captions.max() ?? 0)
    let gap: CGFloat = 96
    var y = top + ((bottom - top) - (m.height + gap + figuresHeight)) / 2
    drawCalendar(m, at: CGPoint(x: edge, y: y), palette: p, numbers: interfaceFont(32), labels: interfaceFont(28), stroke: 4)
    y += m.height + gap
    var x = edge
    for ((value, caption), cw) in zip(m.figures, widths) {
        drawText(attributed(value, serif(size), p.ink), at: CGPoint(x: x, y: y))
        drawText(attributed(caption, interfaceFont(38), p.muted), at: CGPoint(x: x, y: y + valueHeight + 8), width: cw - 40)
        x += cw
    }
    guard let image = ctx.makeImage() else { return nil }
    return NSBitmapImageRep(cgImage: image).representation(using: .png, properties: [:])
}

/// Esc closes the window.
final class EscWindow: NSWindow {
    override func cancelOperation(_ sender: Any?) { performClose(sender) }
}

/// The stats window: one at a time, brought forward on the current month when opened again.
final class StatsWindow: NSObject, NSWindowDelegate {
    let window: NSWindow = EscWindow(contentRect: NSRect(x: 0, y: 0, width: 520, height: 760), styleMask: [.titled, .closable, .miniaturizable, .resizable],
                                     backing: .buffered, defer: false)
    let page: StatsView
    private let scroll = NSScrollView()
    var onClose: (() -> Void)?

    init(state: OpaquePointer) {
        var settings = forzac_state_settings(state)
        let palette = themes[Int(forzac_settings_theme(&settings, systemIsDark()))]
        page = StatsView(state: state, palette: palette)
        super.init()
        window.title = tr("Reading stats")
        window.isReleasedWhenClosed = false
        window.tabbingMode = .disallowed
        window.minSize = NSSize(width: 420, height: 560)
        window.backgroundColor = nsColor(palette.paper)
        window.appearance = NSAppearance(named: palette.paper < 0x808080 ? .darkAqua : .aqua)
        window.delegate = self
        scroll.documentView = page
        scroll.drawsBackground = false
        scroll.hasVerticalScroller = true
        scroll.autohidesScrollers = true
        window.contentView = scroll
        page.onLayout = { [weak self] in self?.fit() }
        if let screen = NSScreen.main?.visibleFrame { window.setContentSize(CGSize(width: 520, height: min(860, screen.height * 0.9))) }
        window.center()
        fit()
    }

    /// The page as wide as the window and as tall as its content.
    private func fit() {
        let width = scroll.contentSize.width
        page.frame = CGRect(x: 0, y: 0, width: width, height: max(page.contentHeight(width), scroll.contentSize.height))
    }

    func windowDidResize(_ notification: Notification) { fit() }

    func show() {
        page.reset()
        fit()
        window.makeKeyAndOrderFront(nil)
        window.makeFirstResponder(page)
    }

    /// A key press as the keyboard would send it (`--keys`).
    func send(_ key: Character) {
        let codes: [Character: UInt16] = ["h": 0x04, "l": 0x25, "j": 0x26, "k": 0x28, "m": 0x2E, "y": 0x10, "s": 0x01, "\u{1b}": 0x35]
        let text = String(key)
        guard let e = NSEvent.keyEvent(with: .keyDown, location: .zero, modifierFlags: [], timestamp: 0, windowNumber: window.windowNumber, context: nil,
                                       characters: text, charactersIgnoringModifiers: text, isARepeat: false, keyCode: codes[key] ?? 0xFFFF) else { return }
        window.sendEvent(e)
    }

    /// The window as drawn (`--png`).
    func savePNG(_ path: String) {
        guard let view = window.contentView, let rep = view.bitmapImageRepForCachingDisplay(in: view.bounds) else { return }
        view.cacheDisplay(in: view.bounds, to: rep)
        try? rep.representation(using: .png, properties: [:])?.write(to: URL(fileURLWithPath: path))
        print("saved \(path)")
    }

    func windowWillClose(_ notification: Notification) { onClose?() }
}

/// The window's content, drawn in one view: the title and share button, "Month · Year", the period with its arrows,
/// the figures, the calendar, the clicked day's line and the finished books.
final class StatsView: NSView {
    private let state: OpaquePointer
    let palette: Palette
    private var year: Int32 = 0
    private var month: UInt32 = 0
    private(set) var model: StatsModel?
    /// The clicked day and its minutes.
    private var day: (Int64, UInt32)?
    /// What a click does where, and where the calendar is; laid out in `draw`.
    private var targets: [(CGRect, () -> Void)] = []
    private var calendarOrigin: CGPoint = .zero
    private(set) var shareRect: CGRect = .zero
    var onLayout: (() -> Void)?

    override var isFlipped: Bool { true }
    override var acceptsFirstResponder: Bool { true }

    init(state: OpaquePointer, palette: Palette) {
        self.state = state
        self.palette = palette
        super.init(frame: .zero)
    }

    required init?(coder: NSCoder) { fatalError() }

    /// Back to the current month.
    func reset() {
        (year, month, day) = (0, 0, nil)
        reload()
    }

    private var width: CGFloat { min(column, bounds.width - 2 * margin) }

    private func reload() {
        let m = StatsModel(state, year: year, month: month, width: max(width, 200), scale: 1, columns: 3)
        (year, month) = (m.year, m.month)
        model = m
        onLayout?()
        needsDisplay = true
    }

    override func setFrameSize(_ newSize: NSSize) {
        let changed = newSize.width != frame.width
        super.setFrameSize(newSize)
        if changed { reload() }
    }

    // MARK: Fonts and the layout's parts

    private let titleFont = bookFont("Literata", 26, bold: true)
    private let periodFont = bookFont("Literata", 21)
    private let figureFont = bookFont("Literata", 22)
    private let headingFont = bookFont("Literata", 19, bold: true)
    private let bookTitleFont = bookFont("Literata", 17)

    /// The height the content needs at `width`.
    func contentHeight(_ width: CGFloat) -> CGFloat {
        guard let m = model else { return 0 }
        let w = min(column, width - 2 * margin)
        var y: CGFloat = 20 + 36 + 16 + 44 + 12 + 44 + 20 + figuresHeight(m, w) + 24 + m.height + 40 + 8
        y += attributed(tr("Finished"), headingFont, 0).size().height + 6
        if m.books.isEmpty {
            y += 16 + attributed(tr("What you finish will appear here"), interfaceFont(15), 0).size().height
        } else {
            for (title, caption) in m.books { y += bookHeight(title, caption, w) }
        }
        return y + 24
    }

    private func figuresHeight(_ m: StatsModel, _ w: CGFloat) -> CGFloat {
        let widths = [1.5, 1, 1].map { w * $0 / 3.5 }
        let captions = zip(m.figures, widths).map {
            attributed($0.1, interfaceFont(13), 0).boundingRect(with: CGSize(width: $1 - 8, height: 1000), options: .usesLineFragmentOrigin).height
        }
        return ceil(figureFont.ascender - figureFont.descender) + 2 + (captions.max() ?? 0)
    }

    private func bookHeight(_ title: String, _ caption: String, _ w: CGFloat) -> CGFloat {
        let t = attributed(title, bookTitleFont, 0).boundingRect(with: CGSize(width: w, height: 1000), options: .usesLineFragmentOrigin).height
        let lineHeight = ceil(bookTitleFont.ascender - bookTitleFont.descender + bookTitleFont.leading)
        return 8 + min(t, 2 * lineHeight + 1) + attributed(caption, interfaceFont(13), 0).size().height + 8
    }

    // MARK: Drawing

    override func draw(_ dirtyRect: NSRect) {
        guard let m = model, let ctx = NSGraphicsContext.current?.cgContext else { return }
        let p = palette
        ctx.setFillColor(color(p.paper))
        ctx.fill(bounds)
        targets = []
        let w = width
        let x0 = (bounds.width - w) / 2

        // The title and the share button.
        var y: CGFloat = 20
        drawText(attributed(tr("Reading stats"), titleFont, p.ink), at: CGPoint(x: x0, y: y), width: w - 48, lines: 1)
        shareRect = CGRect(x: x0 + w - 36, y: y, width: 36, height: 36)
        drawIcon(Icons.share, in: shareRect.insetBy(dx: 6, dy: 6), ink: p.ink, ctx: ctx)
        targets.append((shareRect, { [weak self] in self?.share() }))
        y += 36 + 16

        // "Month · Year".
        let segment = CGRect(x: x0, y: y, width: w, height: 44)
        let halves = [CGRect(x: x0, y: y, width: w / 2, height: 44), CGRect(x: x0 + w / 2, y: y, width: w / 2, height: 44)]
        ctx.saveGState()
        ctx.addPath(CGPath(roundedRect: segment, cornerWidth: 12, cornerHeight: 12, transform: nil))
        ctx.clip()
        let selected = halves[m.month > 0 ? 0 : 1]
        ctx.setFillColor(color(p.ink))
        ctx.fill(selected)
        ctx.restoreGState()
        ctx.setStrokeColor(color(p.line))
        ctx.setLineWidth(1)
        ctx.addPath(CGPath(roundedRect: segment.insetBy(dx: 0.5, dy: 0.5), cornerWidth: 12, cornerHeight: 12, transform: nil))
        ctx.strokePath()
        for (i, (name, r)) in zip([tr("Month"), tr("Year")], halves).enumerated() {
            let on = (i == 0) == (m.month > 0)
            let t = attributed(name, interfaceFont(15), on ? p.sheet : p.ink)
            let s = t.size()
            drawText(t, at: CGPoint(x: r.midX - s.width / 2, y: r.midY - s.height / 2))
            targets.append((r, { [weak self] in i == 0 ? self?.showMonth() : self?.showYear() }))
        }
        y += 44 + 12

        // The period between its arrows.
        let back = CGRect(x: x0, y: y, width: 44, height: 44), forward = CGRect(x: x0 + w - 44, y: y, width: 44, height: 44)
        drawIcon([.line(15, 18, 9, 12), .line(9, 12, 15, 6)], in: back.insetBy(dx: 10, dy: 10), ink: m.canPrevious ? p.ink : p.line, ctx: ctx)
        drawIcon([.line(9, 18, 15, 12), .line(15, 12, 9, 6)], in: forward.insetBy(dx: 10, dy: 10), ink: m.canNext ? p.ink : p.line, ctx: ctx)
        targets.append((back, { [weak self] in self?.step(-1) }))
        targets.append((forward, { [weak self] in self?.step(1) }))
        let title = attributed(m.title, periodFont, p.ink)
        let ts = title.size()
        drawText(title, at: CGPoint(x: x0 + (w - ts.width) / 2, y: y + (44 - ts.height) / 2))
        y += 44 + 20

        // The three figures.
        var x = x0
        for ((value, caption), weight) in zip(m.figures, [1.5, 1, 1] as [CGFloat]) {
            let cw = w * weight / 3.5
            drawText(attributed(value, figureFont, p.ink), at: CGPoint(x: x, y: y))
            drawText(attributed(caption, interfaceFont(13), p.muted), at: CGPoint(x: x, y: y + ceil(figureFont.ascender - figureFont.descender) + 2), width: cw - 8)
            x += cw
        }
        y += figuresHeight(m, w) + 24

        // The calendar and the clicked day.
        calendarOrigin = CGPoint(x: x0, y: y)
        drawCalendar(m, at: calendarOrigin, palette: p, numbers: interfaceFont(13), labels: interfaceFont(11), stroke: 1.5)
        y += m.height
        if let (d, minutes) = day {
            let line = attributed(String(cString: forzac_stats_day(state, d, minutes)), interfaceFont(14), p.muted)
            drawText(line, at: CGPoint(x: x0, y: y + (40 - line.size().height) / 2))
        }
        y += 40 + 8

        // The finished books, or what will be there.
        y += drawText(attributed(tr("Finished"), headingFont, p.ink), at: CGPoint(x: x0, y: y)) + 6
        if m.books.isEmpty {
            drawText(attributed(tr("What you finish will appear here"), interfaceFont(15), p.muted), at: CGPoint(x: x0, y: y + 8), width: w)
        }
        for (title, caption) in m.books {
            let h = bookHeight(title, caption, w)
            let th = drawText(attributed(title, bookTitleFont, p.ink), at: CGPoint(x: x0, y: y + 8), width: w, lines: 2)
            drawText(attributed(caption, interfaceFont(13), p.muted), at: CGPoint(x: x0, y: y + 8 + th), width: w, lines: 1)
            y += h
        }
    }

    private func drawIcon(_ shapes: [Shape], in r: CGRect, ink: UInt32, ctx: CGContext) {
        let k = r.width / 24
        ctx.saveGState()
        ctx.setStrokeColor(color(ink))
        ctx.setLineWidth(1.75 * k)
        ctx.setLineCap(.round)
        ctx.setLineJoin(.round)
        for s in shapes {
            switch s {
            case let .line(ax, ay, bx, by):
                ctx.move(to: CGPoint(x: r.minX + ax * k, y: r.minY + ay * k))
                ctx.addLine(to: CGPoint(x: r.minX + bx * k, y: r.minY + by * k))
            case let .circle(cx, cy, rad):
                ctx.addEllipse(in: CGRect(x: r.minX + (cx - rad) * k, y: r.minY + (cy - rad) * k, width: 2 * rad * k, height: 2 * rad * k))
            case let .rect(rx, ry, rw, rh):
                ctx.addRect(CGRect(x: r.minX + rx * k, y: r.minY + ry * k, width: rw * k, height: rh * k))
            }
        }
        ctx.strokePath()
        ctx.restoreGState()
    }

    // MARK: Actions

    func showMonth() {
        guard month == 0, let today = StatsModel(state, year: 0, month: 0, width: 200, scale: 1, columns: 3) as StatsModel? else { return }
        month = year == today.year ? today.month : 12
        day = nil
        reload()
    }

    func showYear() {
        guard month > 0 else { return }
        month = 0
        day = nil
        reload()
    }

    func step(_ delta: Int32) {
        guard let m = model, delta < 0 ? m.canPrevious : m.canNext else { return }
        if month == 0 {
            year += delta
        } else {
            let n = Int32(year) * 12 + Int32(month) - 1 + delta
            (year, month) = (n / 12, UInt32(n % 12) + 1)
        }
        day = nil
        reload()
    }

    /// The period's picture, shared from the share button.
    func share() {
        guard let data = statsPicture(state, year: year, month: month, palette: palette), let m = model else { return }
        let dir = FileManager.default.temporaryDirectory.appendingPathComponent("Forzac", isDirectory: true)
        try? FileManager.default.createDirectory(at: dir, withIntermediateDirectories: true)
        let url = dir.appendingPathComponent(m.fileName)
        guard (try? data.write(to: url)) != nil else { return }
        NSSharingServicePicker(items: [url]).show(relativeTo: shareRect, of: self, preferredEdge: .minY)
    }

    override func mouseUp(with e: NSEvent) {
        let p = convert(e.locationInWindow, from: nil)
        if let (_, act) = targets.first(where: { $0.0.contains(p) }) { return act() }
        guard let m = model else { return }
        let inCalendar = CGPoint(x: p.x - calendarOrigin.x, y: p.y - calendarOrigin.y)
        if let c = m.cells.first(where: { $0.rect.contains(inCalendar) && !$0.future }) {
            day = (c.day, c.minutes)
            needsDisplay = true
        }
    }

    override func keyDown(with e: NSEvent) {
        guard e.modifierFlags.intersection([.command, .control, .option]).isEmpty else { return super.keyDown(with: e) }
        let scroll = enclosingScrollView
        func scrollBy(_ dy: CGFloat) {
            guard let clip = scroll?.contentView else { return }
            let y = min(max(0, clip.bounds.minY + dy), max(0, bounds.height - clip.bounds.height))
            clip.scroll(to: CGPoint(x: 0, y: y))
            scroll?.reflectScrolledClipView(clip)
        }
        let page = (scroll?.contentView.bounds.height ?? 400) - 40
        switch e.keyCode {
        case 0x7B: step(-1)
        case 0x7C: step(1)
        case 0x7D: scrollBy(40)
        case 0x7E: scrollBy(-40)
        case 0x31: scrollBy(e.modifierFlags.contains(.shift) ? -page : page)
        case 0x79: scrollBy(page)
        case 0x74: scrollBy(-page)
        case 0x73: scrollBy(-bounds.height)
        case 0x77: scrollBy(bounds.height)
        case 0x35: window?.performClose(nil)
        default:
            switch e.shortcutKey {
            case 0x04: step(-1)
            case 0x25: step(1)
            case 0x26: scrollBy(40)
            case 0x28: scrollBy(-40)
            case 0x2E: showMonth()
            case 0x10: showYear()
            case 0x01: share()
            default: super.keyDown(with: e)
            }
        }
    }
}
