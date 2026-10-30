// A PDF's window (`pdf-reading`): pages as on paper, drawn by PDFKit off the main thread, one at a time with the
// neighbours ahead; the same keys, clicks and "Slide" as a book's. Its menu and panels use the Light theme, whatever the
// reader's theme is. The core only recognises the type and keeps the place: `Locator(0, page, 0)`.

import AppKit
import PDFKit
import QuartzCore

private let light = themes[0]
/// The height of the foot under the page: "N / M".
private let footHeight: CGFloat = 26
private let maxZoom: CGFloat = 4

/// A page's box as it shows: its rotation applied.
private func shownSize(_ page: PDFPage) -> CGSize {
    let box = page.bounds(for: .cropBox).size
    return page.rotation % 180 == 0 ? box : CGSize(width: box.height, height: box.width)
}

/// The pages `first` and, in a spread, the next one, fitted to `area` at one scale: their rectangles, y up.
private func placed(_ doc: PDFDocument, first: Int, spread: Bool, area: CGSize) -> [(index: Int, rect: CGRect, scale: CGFloat)] {
    let indices = (spread && first + 1 < doc.pageCount) ? [first, first + 1] : [first]
    let sizes = indices.compactMap { doc.page(at: $0).map(shownSize) }
    guard sizes.count == indices.count, area.width > 0, area.height > 0 else { return [] }
    let (width, height) = (sizes.map(\.width).reduce(0, +), sizes.map(\.height).max() ?? 1)
    let scale = min(area.width / max(width, 1), area.height / max(height, 1))
    var x = (area.width - width * scale) / 2
    return zip(indices, sizes).map { i, s in
        defer { x += s.width * scale }
        return (i, CGRect(x: x, y: (area.height - s.height * scale) / 2, width: s.width * scale, height: s.height * scale), scale)
    }
}

/// The part `visible` of the pages as they sit in `area`, on white, at `pixels` device pixels a point. Called on one
/// queue per document.
private func renderPages(_ doc: PDFDocument, first: Int, spread: Bool, area: CGSize, visible: CGRect, pixels: CGFloat) -> CGImage? {
    let (w, h) = (Int((visible.width * pixels).rounded()), Int((visible.height * pixels).rounded()))
    guard w > 0, h > 0, let ctx = CGContext(data: nil, width: w, height: h, bitsPerComponent: 8, bytesPerRow: 0, space: CGColorSpace(name: CGColorSpace.sRGB)!,
                                            bitmapInfo: CGImageAlphaInfo.noneSkipLast.rawValue) else { return nil }
    ctx.setFillColor(CGColor(gray: 1, alpha: 1))
    ctx.fill(CGRect(x: 0, y: 0, width: w, height: h))
    ctx.scaleBy(x: pixels, y: pixels)
    ctx.translateBy(x: -visible.minX, y: -visible.minY)
    for (i, rect, scale) in placed(doc, first: first, spread: spread, area: area) {
        guard let page = doc.page(at: i), rect.intersects(visible) else { continue }
        ctx.saveGState()
        ctx.translateBy(x: rect.minX, y: rect.minY)
        ctx.scaleBy(x: scale, y: scale)
        page.draw(with: .cropBox, to: ctx)
        ctx.restoreGState()
    }
    return ctx.makeImage()
}

/// A PDF's first page as image data and its title, for the library; nil when it cannot be read or needs a password.
func pdfCover(_ path: String) -> (data: Data, title: String)? {
    guard let doc = PDFDocument(url: URL(fileURLWithPath: path)), !doc.isLocked, doc.pageCount > 0, let page = doc.page(at: 0) else { return nil }
    let size = shownSize(page), height: CGFloat = 800
    let area = CGSize(width: height * size.width / max(size.height, 1), height: height)
    guard let image = renderPages(doc, first: 0, spread: false, area: area, visible: CGRect(origin: .zero, size: area), pixels: 1) else { return nil }
    let data = NSMutableData()
    guard let dest = CGImageDestinationCreateWithData(data, "public.png" as CFString, 1, nil) else { return nil }
    CGImageDestinationAddImage(dest, image, nil)
    guard CGImageDestinationFinalize(dest) else { return nil }
    return (data as Data, doc.documentAttributes?[PDFDocumentAttribute.titleAttribute] as? String ?? "")
}

/// The reason a PDF cannot be opened, as `forzac_open` numbers them (6: a password); 0 when it can.
func pdfProblem(_ path: String) -> UInt32 {
    guard let doc = PDFDocument(url: URL(fileURLWithPath: path)), doc.pageCount > 0 || doc.isLocked else { return 4 }
    return doc.isLocked ? 6 : 0
}

/// The pages: a white area over the foot, a slide between two pages, zoom and panning; a press that is not a drag is a
/// link, the side thirds turn and the middle opens the menu.
final class PdfView: NSView {
    private let pdf: PDFDocument
    private let queue = DispatchQueue(label: "forzac.pdf", qos: .userInitiated)
    private let renderDoc: PDFDocument
    private let settings: ForzacSettings
    private let clip = CALayer(), holder = CALayer(), base = CALayer(), sharp = CALayer(), previous = CALayer(), foot = CATextLayer()
    private let pill = CATextLayer()
    /// Rendered pages by first page, at the window's size; thrown away when it changes.
    private var images: [Int: CGImage] = [:]
    private var wanted: Set<Int> = []
    private var cachedFor = CGSize.zero
    private(set) var page: Int
    private(set) var zoom: CGFloat = 1
    private var pan = CGPoint.zero
    private var sharpTimer: Timer?
    private var sharpTicket = 0
    private var back: [Int] = []
    private var pillTurns = 0
    private var dragFrom: CGPoint?
    private var dragged = false
    private var swipe: (CGFloat, Bool) = (0, false)
    private(set) var jumped = false
    var onMoved: (() -> Void)?
    var onCommand: ((Command) -> Void)?
    var onExternal: ((URL) -> Void)?
    var menuKey: ((NSEvent) -> Bool)?
    var panelOpen: (() -> Bool)?
    var onFirstFrame: (() -> Void)?
    /// `]` and `[`: the outline's pages, in order.
    var sections: [Int] = []

    var pageCount: Int { pdf.pageCount }
    var spread: Bool {
        var s = settings
        s.turning = 0 // a PDF slides whichever mode books are read in
        return forzac_settings_spread(&s, Float(bounds.width), Float(bounds.height))
    }
    private var step: Int { spread ? 2 : 1 }
    private var scale: CGFloat { window?.backingScaleFactor ?? 2 }
    private var area: CGSize { CGSize(width: bounds.width, height: max(bounds.height - footHeight, 1)) }
    var finished: Bool { page + step >= pageCount }

    init(pdf: PDFDocument, renderDoc: PDFDocument, page: Int, settings: ForzacSettings) {
        (self.pdf, self.renderDoc, self.settings) = (pdf, renderDoc, settings)
        self.page = min(max(page, 0), max(pdf.pageCount - 1, 0))
        super.init(frame: .zero)
        wantsLayer = true
        layer!.backgroundColor = CGColor(gray: 1, alpha: 1)
        clip.masksToBounds = true
        holder.anchorPoint = .zero
        layer!.addSublayer(previous)
        layer!.addSublayer(clip)
        clip.addSublayer(holder)
        holder.addSublayer(base)
        holder.addSublayer(sharp)
        base.backgroundColor = CGColor(gray: 1, alpha: 1)
        foot.alignmentMode = .center
        foot.font = interfaceFont(12)
        foot.fontSize = 12
        foot.foregroundColor = color(light.muted)
        layer!.addSublayer(foot)
        pill.alignmentMode = .center
        pill.font = interfaceFont(14)
        pill.fontSize = 14
        pill.foregroundColor = color(light.paper)
        pill.backgroundColor = color(light.ink)
        pill.cornerRadius = 15
        pill.isHidden = true
        pill.string = tr("← Back · ⌃O")
        layer!.addSublayer(pill)
        for l in [clip, holder, base, sharp, previous, foot, pill] {
            l.actions = ["position": NSNull(), "bounds": NSNull(), "contents": NSNull(), "transform": NSNull(), "hidden": NSNull(), "frame": NSNull()]
        }
    }

    required init?(coder: NSCoder) { fatalError() }

    override var acceptsFirstResponder: Bool { true }

    override func layout() {
        super.layout()
        let size = area
        if size != cachedFor {
            cachedFor = size
            images = [:]
            wanted = []
            zoom = 1
            pan = .zero
            show(animated: 0)
        }
    }

    override func viewDidChangeBackingProperties() {
        super.viewDidChangeBackingProperties()
        images = [:]
        wanted = []
        if !bounds.isEmpty { show(animated: 0) }
    }

    // MARK: Showing

    /// Shows the current page; `animated` +1 slides it in from the right, −1 from the left. A page not rendered yet
    /// slides in white and is filled when it comes.
    private func show(animated dir: CGFloat) {
        let size = area, frame = CGRect(x: 0, y: footHeight, width: size.width, height: size.height)
        CATransaction.begin()
        CATransaction.setDisableActions(true)
        contentsScaleAll()
        if dir != 0 {
            previous.contents = base.contents
            previous.frame = frame
        }
        clip.frame = frame
        base.frame = CGRect(origin: .zero, size: size)
        base.contents = images[page]
        sharp.contents = nil
        placeHolder()
        foot.string = "\(page + 1) / \(pageCount)"
        foot.frame = CGRect(x: 0, y: 5, width: size.width, height: 16)
        foot.isHidden = pillShown
        layoutPill()
        CATransaction.commit()
        if dir != 0 {
            let timing = CAMediaTimingFunction(controlPoints: 0.215, 0.61, 0.355, 1) // ease-out cubic
            for (l, from, to) in [(clip, frame.midX + dir * frame.width, frame.midX), (previous, frame.midX, frame.midX - dir * frame.width)] {
                let a = CABasicAnimation(keyPath: "position.x")
                a.fromValue = from
                a.toValue = to
                a.duration = 0.2
                a.timingFunction = timing
                l.add(a, forKey: "turn")
            }
        }
        prepare()
    }

    private func contentsScaleAll() {
        for l in [base, sharp, previous, foot, pill] { l.contentsScale = scale }
    }

    private func placeHolder() {
        holder.bounds = CGRect(origin: .zero, size: area)
        holder.position = pan
        holder.setAffineTransform(CGAffineTransform(scaleX: zoom, y: zoom))
    }

    /// The page and its neighbours rendered on the queue, the page first; those no longer near are dropped.
    private func prepare() {
        let near = [page, page + step, page - step].filter { (0..<pageCount).contains($0) }
        images = images.filter { near.contains($0.key) }
        wanted = wanted.intersection(near)
        let (size, pixels, spread, doc) = (area, scale, spread, renderDoc)
        for p in near where images[p] == nil && !wanted.contains(p) {
            wanted.insert(p)
            queue.async { [weak self] in
                // Still wanted? A reader flipping through leaves pages behind.
                guard let self, DispatchQueue.main.sync(execute: { self.wanted.contains(p) && self.cachedFor == size }) else { return }
                let image = renderPages(doc, first: p, spread: spread, area: size, visible: CGRect(origin: .zero, size: size), pixels: pixels)
                DispatchQueue.main.async { self.rendered(p, image, for: size) }
            }
        }
        if images[page] != nil { firstFrame() }
    }

    private func rendered(_ p: Int, _ image: CGImage?, for size: CGSize) {
        guard cachedFor == size, wanted.remove(p) != nil, let image else { return }
        images[p] = image
        if p == page {
            CATransaction.begin()
            CATransaction.setDisableActions(true)
            base.contents = image
            CATransaction.commit()
            firstFrame()
        }
    }

    private func firstFrame() {
        guard let f = onFirstFrame else { return }
        onFirstFrame = nil
        f()
    }

    func savePNG(_ path: String) {
        guard let img = images[page], let dest = CGImageDestinationCreateWithURL(URL(fileURLWithPath: path) as CFURL, "public.png" as CFString, 1, nil) else { return }
        CGImageDestinationAddImage(dest, img, nil)
        CGImageDestinationFinalize(dest)
        print("saved \(path) (\(img.width)×\(img.height))")
    }

    // MARK: Moving

    func turn(_ delta: Int) {
        let target = page + delta * step
        guard (0..<pageCount).contains(target) || (delta > 0 && page + 1 < pageCount && step == 2 && page + 1 == pageCount - 1) else { return }
        let to = min(target, pageCount - 1)
        pillTurns += delta
        if abs(pillTurns) >= 2 { back = [] }
        move(to, animated: delta > 0 ? 1 : -1, jump: false)
    }

    /// Goes to `target` as a jump: the pill brings the reader back.
    func go(to target: Int) {
        guard (0..<pageCount).contains(target), target != page else { return }
        back.append(page)
        pillTurns = 0
        move(target, animated: 0, jump: true)
    }

    func goBack() {
        guard let to = back.popLast() else { return }
        pillTurns = 0
        move(to, animated: 0, jump: true)
    }

    private func move(_ to: Int, animated dir: CGFloat, jump: Bool) {
        resetZoom()
        page = to
        jumped = jump
        show(animated: dir)
        onMoved?()
    }

    private var pillShown: Bool { !back.isEmpty }

    private func layoutPill() {
        pill.isHidden = !pillShown
        guard pillShown else { return }
        let w = (pill.preferredFrameSize().width + 36).rounded()
        pill.frame = CGRect(x: (bounds.width - w) / 2, y: 2, width: w, height: 24)
        pill.cornerRadius = 12
    }

    private func edgeOrSection(_ delta: Int) {
        let target = delta > 0 ? sections.first { $0 > page } : sections.last { $0 < page }
        if let target { go(to: target) }
    }

    // MARK: Zoom

    private func resetZoom() {
        guard zoom != 1 else { return }
        zoom = 1
        pan = .zero
        sharpTimer?.invalidate()
        sharpTicket += 1
    }

    /// Zooms by `factor` around `point` (in the page area), at once; sharp when the gesture ends (`scheduleSharp`).
    private func zoomBy(_ factor: CGFloat, around point: CGPoint) {
        let next = min(max(zoom * factor, 1), maxZoom)
        guard next != zoom else { return }
        pan = CGPoint(x: point.x - (point.x - pan.x) * next / zoom, y: point.y - (point.y - pan.y) * next / zoom)
        zoom = next
        clampPan()
        CATransaction.begin()
        CATransaction.setDisableActions(true)
        sharp.contents = nil
        placeHolder()
        CATransaction.commit()
        scheduleSharp()
    }

    private func panBy(_ dx: CGFloat, _ dy: CGFloat) {
        pan.x += dx
        pan.y += dy
        clampPan()
        CATransaction.begin()
        CATransaction.setDisableActions(true)
        placeHolder()
        CATransaction.commit()
        scheduleSharp()
    }

    private func clampPan() {
        let size = area
        pan.x = min(0, max(pan.x, size.width * (1 - zoom)))
        pan.y = min(0, max(pan.y, size.height * (1 - zoom)))
    }

    /// When the gesture has been still a moment, the part in view drawn again at its zoom, over the scaled page.
    private func scheduleSharp() {
        sharpTimer?.invalidate()
        sharpTicket += 1
        guard zoom > 1 else { return }
        let ticket = sharpTicket
        sharpTimer = Timer.scheduledTimer(withTimeInterval: 0.12, repeats: false) { [weak self] _ in
            guard let self else { return }
            let size = self.area, z = self.zoom
            let visible = CGRect(x: -self.pan.x / z, y: -self.pan.y / z, width: size.width / z, height: size.height / z)
            let (first, spread, doc, pixels) = (self.page, self.spread, self.renderDoc, self.scale * z)
            self.queue.async { [weak self] in
                let image = renderPages(doc, first: first, spread: spread, area: size, visible: visible, pixels: pixels)
                DispatchQueue.main.async {
                    guard let self, ticket == self.sharpTicket else { return }
                    CATransaction.begin()
                    CATransaction.setDisableActions(true)
                    self.sharp.frame = visible
                    self.sharp.contents = image
                    CATransaction.commit()
                }
            }
        }
    }

    override func magnify(with e: NSEvent) {
        zoomBy(1 + e.magnification, around: areaPoint(e))
    }

    private func areaPoint(_ e: NSEvent) -> CGPoint {
        let p = convert(e.locationInWindow, from: nil)
        return CGPoint(x: p.x, y: p.y - footHeight)
    }

    override func scrollWheel(with e: NSEvent) {
        if e.modifierFlags.contains(.control) {
            let d = e.hasPreciseScrollingDeltas ? e.scrollingDeltaY / 100 : e.scrollingDeltaY / 10
            return zoomBy(1 + d, around: areaPoint(e))
        }
        if zoom > 1 {
            let k: CGFloat = e.hasPreciseScrollingDeltas ? 1 : 16
            return panBy(e.scrollingDeltaX * k, -e.scrollingDeltaY * k)
        }
        if !e.hasPreciseScrollingDeltas {
            if e.scrollingDeltaY != 0 { turn(e.scrollingDeltaY < 0 ? 1 : -1) }
            return
        }
        // A trackpad swipe: one turn per swipe, in the direction the fingers move.
        if e.phase == .began { swipe = (0, false) }
        swipe.0 += abs(e.scrollingDeltaX) > abs(e.scrollingDeltaY) ? e.scrollingDeltaX : 0
        if !swipe.1 && abs(swipe.0) > 60 {
            swipe.1 = true
            turn(swipe.0 < 0 ? 1 : -1)
        }
    }

    // MARK: Clicks and keys

    override func mouseDown(with e: NSEvent) {
        dragFrom = convert(e.locationInWindow, from: nil)
        dragged = false
    }

    override func mouseDragged(with e: NSEvent) {
        guard let from = dragFrom else { return }
        let p = convert(e.locationInWindow, from: nil)
        if hypot(p.x - from.x, p.y - from.y) > 4 { dragged = true }
        if dragged && zoom > 1 {
            panBy(p.x - from.x, p.y - from.y)
            dragFrom = p
        }
    }

    override func mouseUp(with e: NSEvent) {
        if dragged { return }
        if panelOpen?() == true { return onCommand?(.closePanels) ?? () }
        let p = convert(e.locationInWindow, from: nil)
        if !pill.isHidden, pill.frame.contains(p) { return goBack() }
        if let link = link(at: p) { return follow(link) }
        if p.x >= bounds.width / 3 && p.x <= bounds.width * 2 / 3 {
            onCommand?(.menu)
        } else if zoom == 1 {
            turn(p.x < bounds.width / 3 ? -1 : 1)
        }
    }

    /// The link annotation under a point of the view.
    private func link(at p: CGPoint) -> PDFAnnotation? {
        let a = CGPoint(x: (p.x - pan.x) / zoom, y: (p.y - footHeight - pan.y) / zoom)
        for (i, rect, scale) in placed(pdf, first: page, spread: spread, area: area) where rect.contains(a) {
            guard let pg = pdf.page(at: i) else { continue }
            let box = CGPoint(x: (a.x - rect.minX) / scale, y: (a.y - rect.minY) / scale)
            let point = box.applying(pg.transform(for: .cropBox).inverted())
            if let hit = pg.annotation(at: point), hit.type == "Link" { return hit }
        }
        return nil
    }

    private func follow(_ link: PDFAnnotation) {
        if let url = (link.action as? PDFActionURL)?.url ?? link.url { return onExternal?(url) ?? () }
        let destination = (link.action as? PDFActionGoTo)?.destination ?? link.destination
        if let target = destination?.page.map(pdf.index(for:)) { go(to: target) }
    }

    /// The keys of a book that mean something on a PDF; `+ - 0` zoom, and arrows pan while zoomed.
    override func keyDown(with e: NSEvent) {
        if menuKey?(e) == true { return }
        if e.characters == "?" { return onCommand?(.help) ?? () }
        let (shift, control) = (e.modifierFlags.contains(.shift), e.modifierFlags.contains(.control))
        let middle = CGPoint(x: area.width / 2, y: area.height / 2), by: CGFloat = 80
        switch e.shortcutKey {
        case 0x02 where control: turn(1)                                                       // ⌃D
        case 0x20 where control: turn(-1)                                                      // ⌃U
        case 0x1F where control: goBack()                                                      // ⌃O
        case 0x31 where shift: turn(-1)                                                        // Shift+Space
        case 0x24, 0x4C: onCommand?(.menu)                                                     // Return
        case 0x26, 0x7D: if zoom > 1 { panBy(0, by) } else { turn(1) }                         // j ↓
        case 0x28, 0x7E: if zoom > 1 { panBy(0, -by) } else { turn(-1) }                       // k ↑
        case 0x25, 0x7C: if zoom > 1 { panBy(-by, 0) } else { turn(1) }                        // l →
        case 0x04, 0x7B: if zoom > 1 { panBy(by, 0) } else { turn(-1) }                        // h ←
        case 0x31, 0x79: turn(1)                                                               // Space PgDn
        case 0x74, 0x33: turn(-1)                                                              // PgUp Backspace
        case 0x1E: edgeOrSection(1)                                                            // ]
        case 0x21: edgeOrSection(-1)                                                           // [
        case 0x05 where shift, 0x77: go(to: max(pageCount - step, 0))                          // G End
        case 0x05, 0x73: go(to: 0)                                                             // g Home
        case 0x18, 0x45: zoomBy(1.25, around: middle)                                          // = + (keypad +)
        case 0x1B, 0x4E: zoomBy(0.8, around: middle)                                           // - (keypad -)
        case 0x1D, 0x52:                                                                       // 0
            resetZoom()
            show(animated: 0)
        case 0x1F: onCommand?(.contents)                                                       // o
        case 0x03: window?.toggleFullScreen(nil)                                               // f
        case 0x2C: onCommand?(.search)                                                         // /
        case 0x35: onCommand?(.closePanels)                                                    // Esc
        case 0x0C: window?.performClose(nil)                                                   // q
        default: super.keyDown(with: e)
        }
    }
}

/// The list over a PDF's page, the contents or the search: rows with the page on the right; in the search, a field.
private final class PdfPanel: LayerView, NSTableViewDataSource, NSTableViewDelegate, NSTextFieldDelegate {
    struct Row {
        let text: NSAttributedString
        let page: Int
    }

    private let scroll = NSScrollView()
    private let table = PdfTable()
    private let title = label("", interfaceFont(17, .medium), light.ink)
    private let message = label("", interfaceFont(14), light.muted)
    private let field = NSTextField()
    private(set) var rows: [Row] = []
    var onPick: ((Int) -> Void)?
    var onClose: (() -> Void)?
    var onQuery: ((String) -> Void)?

    init(titled text: String, search: Bool) {
        super.init()
        layer!.backgroundColor = color(light.sheet)
        layer!.shadowOpacity = 0.16
        layer!.shadowRadius = 16
        title.stringValue = text
        table.addTableColumn(NSTableColumn(identifier: .init("row")))
        table.headerView = nil
        table.backgroundColor = .clear
        table.dataSource = self
        table.delegate = self
        table.panel = self
        table.rowSizeStyle = .custom
        table.intercellSpacing = .zero
        table.usesAutomaticRowHeights = true
        table.action = #selector(clicked)
        table.target = self
        scroll.documentView = table
        scroll.drawsBackground = false
        scroll.hasVerticalScroller = true
        message.isHidden = true
        message.maximumNumberOfLines = 0
        message.lineBreakMode = .byWordWrapping
        message.preferredMaxLayoutWidth = 320
        message.setContentCompressionResistancePriority(.defaultLow, for: .horizontal)
        field.isHidden = !search
        field.placeholderString = tr("Search in book")
        field.delegate = self
        field.font = interfaceFont(15)
        [title, field, scroll, message].forEach {
            $0.translatesAutoresizingMaskIntoConstraints = false
            addSubview($0)
        }
        NSLayoutConstraint.activate([
            title.topAnchor.constraint(equalTo: topAnchor, constant: 20),
            title.leadingAnchor.constraint(equalTo: leadingAnchor, constant: 20),
            field.topAnchor.constraint(equalTo: title.bottomAnchor, constant: 10),
            field.leadingAnchor.constraint(equalTo: leadingAnchor, constant: 20),
            field.trailingAnchor.constraint(equalTo: trailingAnchor, constant: -20),
            scroll.topAnchor.constraint(equalTo: search ? field.bottomAnchor : title.bottomAnchor, constant: 12),
            scroll.leadingAnchor.constraint(equalTo: leadingAnchor),
            scroll.trailingAnchor.constraint(equalTo: trailingAnchor),
            scroll.bottomAnchor.constraint(equalTo: bottomAnchor),
            message.topAnchor.constraint(equalTo: scroll.topAnchor, constant: 8),
            message.leadingAnchor.constraint(equalTo: leadingAnchor, constant: 20),
            message.trailingAnchor.constraint(equalTo: trailingAnchor, constant: -20),
        ])
    }

    required init?(coder: NSCoder) { fatalError() }

    func set(_ rows: [Row], message text: String? = nil, select: Int? = nil) {
        self.rows = rows
        table.reloadData()
        message.stringValue = text ?? ""
        message.isHidden = text == nil
        if let select, rows.indices.contains(select) {
            table.selectRowIndexes([select], byExtendingSelection: false)
            table.scrollRowToVisible(select)
        }
    }

    func focus() { window?.makeFirstResponder(field.isHidden ? table : field) }

    func numberOfRows(in tableView: NSTableView) -> Int { rows.count }

    func tableView(_ tableView: NSTableView, rowViewForRow row: Int) -> NSTableRowView? { ContentsRow() }

    func tableView(_ tableView: NSTableView, viewFor tableColumn: NSTableColumn?, row: Int) -> NSView? {
        let cell = NSTableCellView()
        let text = NSTextField(labelWithAttributedString: rows[row].text)
        text.lineBreakMode = .byTruncatingTail
        text.maximumNumberOfLines = 3
        text.setContentCompressionResistancePriority(.defaultLow, for: .horizontal)
        let number = label("\(rows[row].page + 1)", interfaceFont(12), light.muted)
        [text, number].forEach {
            $0.translatesAutoresizingMaskIntoConstraints = false
            cell.addSubview($0)
        }
        NSLayoutConstraint.activate([
            text.topAnchor.constraint(equalTo: cell.topAnchor, constant: 6),
            text.bottomAnchor.constraint(equalTo: cell.bottomAnchor, constant: -6),
            text.leadingAnchor.constraint(equalTo: cell.leadingAnchor, constant: 14),
            number.centerYAnchor.constraint(equalTo: text.centerYAnchor),
            number.trailingAnchor.constraint(equalTo: cell.trailingAnchor, constant: -14),
            number.leadingAnchor.constraint(greaterThanOrEqualTo: text.trailingAnchor, constant: 8),
        ])
        return cell
    }

    @objc private func clicked() { pick() }

    func pick() {
        if rows.indices.contains(table.selectedRow) { onPick?(rows[table.selectedRow].page) }
    }

    func controlTextDidChange(_ obj: Notification) { onQuery?(field.stringValue) }

    func control(_ control: NSControl, textView: NSTextView, doCommandBy selector: Selector) -> Bool {
        switch selector {
        case #selector(NSResponder.cancelOperation(_:)): onClose?()
        case #selector(NSResponder.insertNewline(_:)):
            if !rows.isEmpty {
                table.selectRowIndexes([max(table.selectedRow, 0)], byExtendingSelection: false)
                pick()
            }
        case #selector(NSResponder.moveDown(_:)):
            if !rows.isEmpty {
                window?.makeFirstResponder(table)
                table.selectRowIndexes([max(table.selectedRow, 0)], byExtendingSelection: false)
            }
        default: return false
        }
        return true
    }
}

private final class PdfTable: NSTableView {
    weak var panel: PdfPanel?

    override func keyDown(with e: NSEvent) {
        let step = { [self] (d: Int) in
            let to = min(max(selectedRow + d, 0), numberOfRows - 1)
            guard to >= 0 else { return }
            selectRowIndexes([to], byExtendingSelection: false)
            scrollRowToVisible(to)
        }
        switch e.shortcutKey {
        case 0x26: step(1)                          // j
        case 0x28: step(-1)                         // k
        case 0x24, 0x4C: panel?.pick()              // Return
        case 0x35, 0x1F, 0x0C: panel?.onClose?()    // Esc o q
        default: super.keyDown(with: e)
        }
    }
}

/// A PDF's reading menu: as a book's, with the outline's section for the time left, and no "Aa".
private final class PdfMenu {
    private let top = Bar(lineAtTop: false), bottom = Bar(lineAtTop: true)
    private let backButton = BarButton(IconView([.line(15, 18, 9, 12), .line(9, 12, 15, 6)], size: 24), tooltip: tr("Back to library"))
    private let contentsButton = BarButton(IconView([.line(8, 6, 21, 6), .line(8, 12, 21, 12), .line(8, 18, 21, 18), .line(3, 6, 3.01, 6), .line(3, 12, 3.01, 12), .line(3, 18, 3.01, 18)], size: 24), tooltip: tr("Contents"))
    private let bookLine = label("", interfaceFont(14, .medium), light.ink)
    private let slider = Slider()
    private let pageLine = label("", interfaceFont(14, .medium), light.ink)
    private unowned let view: PdfView
    private var scrub: Int?
    private(set) var isOpen = false
    var onBack: (() -> Void)?
    var onContents: (() -> Void)?

    init(view: PdfView, title: String, in container: NSView) {
        self.view = view
        bookLine.stringValue = title
        bookLine.lineBreakMode = .byTruncatingTail
        [backButton, bookLine, contentsButton].forEach(top.addSubview)
        [slider, pageLine].forEach(bottom.addSubview)
        top.place = { [unowned self] bar in
            backButton.frame = CGRect(x: 4, y: 4, width: 48, height: 48)
            contentsButton.frame = CGRect(x: bar.bounds.width - 52, y: 4, width: 48, height: 48)
            bookLine.frame = CGRect(x: 60, y: 19, width: bar.bounds.width - 120, height: 19)
        }
        bottom.place = { [unowned self] bar in
            let x = max(20, (bar.bounds.width - 640) / 2), width = bar.bounds.width - 2 * x
            slider.frame = CGRect(x: x, y: 12, width: width, height: 28)
            pageLine.frame = CGRect(x: x, y: 44, width: width, height: 19)
        }
        let placed: [(Bar, NSView.AutoresizingMask)] = [(top, [.width, .minYMargin]), (bottom, [.width, .maxYMargin])]
        for (bar, mask) in placed {
            bar.autoresizingMask = mask
            bar.isHidden = true
            bar.layer!.backgroundColor = color(light.paper)
            bar.rule.backgroundColor = color(light.line)
            container.addSubview(bar)
        }
        backButton.colour(light)
        contentsButton.colour(light)
        backButton.action = { [weak self] in self?.onBack?() }
        contentsButton.action = { [weak self] in
            self?.close()
            self?.onContents?()
        }
        slider.onScrub = { [weak self] in self?.scrubTo(Int($0)) }
        slider.onRelease = { [weak self] in self?.go() }
    }

    func open() {
        guard !isOpen, let c = top.superview?.bounds else { return }
        isOpen = true
        scrub = nil
        top.frame = CGRect(x: 0, y: c.height - 56, width: c.width, height: 56)
        bottom.frame = CGRect(x: 0, y: 0, width: c.width, height: 76)
        showPlace()
        for bar in [top, bottom] {
            bar.alphaValue = 0
            bar.isHidden = false
            bar.animator().alphaValue = 1
        }
    }

    @discardableResult
    func close() -> Bool {
        guard isOpen else { return false }
        isOpen = false
        scrub = nil
        for bar in [top, bottom] { bar.isHidden = true }
        return true
    }

    func key(_ e: NSEvent) -> Bool {
        guard isOpen else { return false }
        switch e.shortcutKey {
        case 0x7B, 0x04: scrubTo((scrub ?? view.page) - 1)  // ← h
        case 0x7C, 0x25: scrubTo((scrub ?? view.page) + 1)  // → l
        case 0x24, 0x4C: go()                               // Return
        case 0x35: close()                                  // Esc
        default:
            close()
            return false
        }
        return true
    }

    private func scrubTo(_ page: Int) {
        scrub = min(max(page, 0), view.pageCount - 1)
        showPlace()
    }

    private func go() {
        let target = scrub.flatMap { $0 == view.page ? nil : $0 }
        close()
        if let target { view.go(to: target) }
    }

    private func showPlace() {
        let shown = scrub ?? view.page
        pageLine.stringValue = fill(tr("Page {n} of {m}"), ["n": "\(shown + 1)", "m": "\(view.pageCount)"])
        slider.set(count: UInt32(view.pageCount), page: UInt32(shown), starts: [], palette: light)
    }
}

/// A PDF's window and what it keeps: the place, the reading log, the panels.
final class PdfReader: NSObject, NSWindowDelegate {
    let path: String
    let window: NSWindow
    let view: PdfView
    private let doc: OpaquePointer
    private let state: OpaquePointer?
    private let pdf: PDFDocument
    private let searchDoc: PDFDocument
    private let container = NSView()
    private let help = HelpView()
    private var menu: PdfMenu!
    private var contentsPanel: PdfPanel?
    private var searchPanel: PdfPanel?
    private var outline: [(title: String, level: Int, page: Int)] = []
    private var saveTimer: Timer?
    private var searchTicket = 0
    private var hits: [Int] = []
    private var inputMonitor: Any?
    var onClose: ((PdfReader) -> Void)?
    var onLibrary: (() -> Void)?

    /// Why the last `init` failed, as `forzac_open` numbers reasons (with 6: a password); 0 when it did not.
    static var openError: UInt32 = 0

    init?(path: String, state: OpaquePointer?, page: Int? = nil) {
        PdfReader.openError = 0
        let url = URL(fileURLWithPath: path)
        PdfReader.openError = pdfProblem(path)
        guard PdfReader.openError == 0, let doc = forzac_open(path, &PdfReader.openError), let pdf = PDFDocument(url: url),
              let renderDoc = PDFDocument(url: url), let searchDoc = PDFDocument(url: url) else { return nil }
        (self.path, self.doc, self.state, self.pdf, self.searchDoc) = (path, doc, state, pdf, searchDoc)
        let title = pdf.documentAttributes?[PDFDocumentAttribute.titleAttribute] as? String ?? ""
        forzac_pdf_set(doc, UInt32(pdf.pageCount), title)
        var at = ForzacLocator(chapter: 0, block: UInt32(page ?? 0), offset: 0)
        if let state, page == nil { _ = forzac_state_position(state, doc, &at) }
        var settings = forzac_state_settings(state)
        if state == nil { settings = forzac_settings_default() }
        view = PdfView(pdf: pdf, renderDoc: renderDoc, page: Int(at.block), settings: settings)
        let size = CommandLine.arguments.firstIndex(of: "--size").flatMap { i in
            CommandLine.arguments.dropFirst(i + 1).first?.split(separator: "x").compactMap { Double($0) }
        }.flatMap { $0.count == 2 && state == nil ? NSSize(width: $0[0], height: $0[1]) : nil } ?? NSSize(width: 760, height: 980)
        window = NSWindow(contentRect: NSRect(origin: .zero, size: size), styleMask: [.titled, .closable, .miniaturizable, .resizable], backing: .buffered, defer: false)
        super.init()
        let shownTitle = String(cString: forzac_title(doc))
        window.title = shownTitle.isEmpty ? tr("Forzac") : shownTitle
        window.representedURL = url
        window.tabbingMode = .disallowed
        window.isReleasedWhenClosed = false
        window.appearance = NSAppearance(named: .aqua)
        window.backgroundColor = .white
        if !(state != nil && window.setFrameUsingName("Book")) { window.center() }
        container.wantsLayer = true
        window.contentView = container
        window.delegate = self
        view.frame = container.bounds
        view.autoresizingMask = [.width, .height]
        container.addSubview(view)
        help.frame = container.bounds
        container.addSubview(help)
        outline = PdfReader.flatten(pdf)
        view.sections = outline.map(\.page)
        menu = PdfMenu(view: view, title: shownTitle, in: container)
        menu.onBack = { [weak self] in
            guard let self else { return }
            let library = self.onLibrary
            self.window.close()
            library?()
        }
        menu.onContents = { [weak self] in self?.showContents() }
        view.menuKey = { [weak self] e in self?.menu.key(e) ?? false }
        view.panelOpen = { [weak self] in self?.panelOpen ?? false }
        view.onExternal = { NSWorkspace.shared.open($0) }
        view.onCommand = { [weak self] in self?.run($0) }
        view.onMoved = { [weak self] in
            guard let self else { return }
            self.logEvent(self.view.jumped ? 2 : 1)
            self.saveTimer?.invalidate()
            self.saveTimer = Timer.scheduledTimer(withTimeInterval: 1, repeats: false) { [weak self] _ in self?.savePosition() }
        }
        if let state {
            _ = forzac_state_log(state, doc, false, true)
            savePosition()
            NSDocumentController.shared.noteNewRecentDocumentURL(url)
            let id = String(cString: forzac_book_id(doc))
            if !FileManager.default.fileExists(atPath: thumbnailURL(id).path) {
                DispatchQueue.global(qos: .utility).async { writeThumbnail(id: id, data: pdfCover(path)?.data) }
            }
            inputMonitor = NSEvent.addLocalMonitorForEvents(matching: [.keyDown, .leftMouseDown, .scrollWheel, .swipe]) { [weak self] e in
                if let self, e.window === self.window { self.logEvent(0) }
                return e
            }
        }
    }

    deinit { forzac_close(doc) }

    private static func flatten(_ pdf: PDFDocument) -> [(title: String, level: Int, page: Int)] {
        var out: [(title: String, level: Int, page: Int)] = []
        func walk(_ node: PDFOutline, _ level: Int) {
            for i in 0..<node.numberOfChildren {
                guard let child = node.child(at: i) else { continue }
                if let title = child.label, let page = child.destination?.page.map(pdf.index(for:)) ?? (child.action as? PDFActionGoTo)?.destination.page.map(pdf.index(for:)) {
                    out.append((title, level, page))
                }
                walk(child, level + 1)
            }
        }
        if let root = pdf.outlineRoot { walk(root, 0) }
        return out
    }

    func show() {
        window.makeKeyAndOrderFront(nil)
        window.makeFirstResponder(view)
    }

    private var panelOpen: Bool { contentsPanel?.isHidden == false || searchPanel?.isHidden == false || help.isOpen }

    func run(_ command: Command) {
        switch command {
        case .menu: if menu.isOpen { menu.close() } else if !panelOpen { menu.open() }
        case .contents: showContents()
        case .search: showSearch()
        case .help: if help.isOpen { help.hide() } else { help.show(in: container, palette: light) }
        case .closePanels: closePanels()
        default: break
        }
    }

    @discardableResult
    private func closePanels() -> Bool {
        let open = panelOpen || menu.isOpen
        contentsPanel?.isHidden = true
        searchPanel?.isHidden = true
        searchTicket += 1
        help.hide()
        menu.close()
        window.makeFirstResponder(view)
        return open
    }

    private func place(_ panel: PdfPanel) {
        panel.translatesAutoresizingMaskIntoConstraints = true
        let width = min(360, container.bounds.width)
        panel.frame = CGRect(x: container.bounds.width - width, y: 0, width: width, height: container.bounds.height)
        panel.autoresizingMask = [.minXMargin, .height]
        if panel.superview == nil { container.addSubview(panel) }
        panel.isHidden = false
    }

    private func makePanel(_ title: String, search: Bool) -> PdfPanel {
        let panel = PdfPanel(titled: title, search: search)
        panel.onClose = { [weak self] in self?.closePanels() }
        panel.onPick = { [weak self] page in
            self?.closePanels()
            self?.view.go(to: page)
        }
        return panel
    }

    private func showContents() {
        if contentsPanel?.isHidden == false { closePanels(); return }
        closePanels()
        let panel = contentsPanel ?? makePanel(tr("Contents"), search: false)
        contentsPanel = panel
        place(panel)
        let rows = outline.map { entry in
            let style = NSMutableParagraphStyle()
            style.firstLineHeadIndent = CGFloat(entry.level) * 14
            style.headIndent = style.firstLineHeadIndent
            return PdfPanel.Row(text: NSAttributedString(string: entry.title, attributes: [.font: interfaceFont(15), .foregroundColor: nsColor(light.ink), .paragraphStyle: style]), page: entry.page)
        }
        let here = outline.lastIndex { $0.page <= view.page }
        panel.set(rows, message: rows.isEmpty ? tr("This PDF has no contents") : nil, select: here)
        panel.focus()
    }

    private func showSearch() {
        if searchPanel?.isHidden == false { closePanels(); return }
        closePanels()
        let panel = searchPanel ?? makePanel(tr("Search"), search: true)
        searchPanel = panel
        panel.onQuery = { [weak self] in self?.search($0) }
        place(panel)
        panel.focus()
    }

    /// Reads the pages' text in the background, results arriving as they are found. A newer query stops the old one.
    private func search(_ query: String) {
        searchTicket += 1
        let ticket = searchTicket
        searchPanel?.set([])
        guard query.trimmingCharacters(in: .whitespaces).count >= 2 else { return }
        let doc = searchDoc
        DispatchQueue.global(qos: .utility).async { [weak self] in
            var hadText = false, found: [PdfPanel.Row] = [], sent = 0
            for i in 0..<doc.pageCount {
                guard DispatchQueue.main.sync(execute: { self?.searchTicket == ticket }) else { return }
                guard let text = doc.page(at: i)?.string, !text.trimmingCharacters(in: .whitespacesAndNewlines).isEmpty else { continue }
                hadText = true
                var n: UInt32 = 0
                let hits = forzac_pdf_search(text, query, &n)
                for k in 0..<n {
                    let hit = forzac_pdf_hit(hits, k)
                    let snippet = String(cString: hit.snippet!)
                    let bytes = Array(snippet.utf8)
                    let line = NSMutableAttributedString(string: snippet, attributes: [.font: interfaceFont(14), .foregroundColor: nsColor(light.ink)])
                    if let a = String(bytes: bytes[..<Int(hit.start)], encoding: .utf8), let m = String(bytes: bytes[Int(hit.start)..<Int(hit.end)], encoding: .utf8) {
                        line.addAttribute(.font, value: interfaceFont(14, .semibold), range: NSRange(location: (a as NSString).length, length: (m as NSString).length))
                    }
                    found.append(PdfPanel.Row(text: line, page: i))
                }
                forzac_pdf_hits_free(hits)
                if found.count != sent && (i % 8 == 0 || found.count - sent > 20) {
                    sent = found.count
                    let rows = found
                    DispatchQueue.main.async { if self?.searchTicket == ticket { self?.searchPanel?.set(rows) } }
                }
            }
            let (rows, scanned) = (found, !hadText)
            DispatchQueue.main.async {
                guard let self, self.searchTicket == ticket else { return }
                let none = rows.isEmpty ? (scanned ? tr("The pages of this PDF are scanned: there is no text to search") : tr("No matches")) : nil
                self.searchPanel?.set(rows, message: none)
            }
        }
    }

    // MARK: Place and log

    private func logEvent(_ kind: UInt32) {
        if state != nil { forzac_reading_event(doc, kind, ForzacLocator(chapter: 0, block: UInt32(view.page), offset: 0), zoneOffset()) }
    }

    func savePendingPosition() {
        if saveTimer != nil { savePosition() }
    }

    func savePosition() {
        saveTimer?.invalidate()
        saveTimer = nil
        guard let state else { return }
        if !forzac_state_save_position(state, doc, ForzacLocator(chapter: 0, block: UInt32(view.page), offset: 0), view.finished) {
            fputs("cannot save the reading state\n", stderr)
        }
        _ = forzac_state_log(state, doc, false, false)
    }

    func leave() {
        guard let state else { return }
        logEvent(3)
        if !forzac_state_log(state, doc, true, false) { fputs("cannot write the reading log\n", stderr) }
    }

    func windowDidResignKey(_ notification: Notification) { leave() }
    func windowDidBecomeKey(_ notification: Notification) { logEvent(0) }

    func windowWillClose(_ notification: Notification) {
        savePosition()
        leave()
        if state != nil { window.saveFrame(usingName: "Book") }
        if let inputMonitor { NSEvent.removeMonitor(inputMonitor) }
        inputMonitor = nil
        onClose?(self)
    }

    // MARK: Measurements

    /// `--png`, `--keys` and `--turns` on a PDF, as a book's: once its first page shows.
    func measure(png: String?, keys: [Character], turns: Int, opening: Date) {
        view.onFirstFrame = { [weak self] in
            guard let self else { return }
            print("first page: \(Int(-opening.timeIntervalSinceNow * 1000)) ms, \(self.view.pageCount) pages")
            var delay = 0.6
            let args = CommandLine.arguments
            let flag = { (name: String) in args.firstIndex(of: name).flatMap { args.dropFirst($0 + 1).first } }
            if let query = flag("--query") {
                self.showSearch()
                self.search(query)
                delay += 1
            }
            if let at = flag("--click")?.split(separator: ",").compactMap({ Double($0) }), at.count == 2 {
                DispatchQueue.main.asyncAfter(deadline: .now() + delay) { [weak self] in
                    guard let self else { return }
                    let point = CGPoint(x: at[0], y: self.view.bounds.height - at[1])
                    for (type, send) in [(NSEvent.EventType.leftMouseDown, self.view.mouseDown(with:)), (.leftMouseUp, self.view.mouseUp(with:))] {
                        if let e = NSEvent.mouseEvent(with: type, location: point, modifierFlags: [], timestamp: 0, windowNumber: self.window.windowNumber, context: nil, eventNumber: 0, clickCount: 1, pressure: 1) { send(e) }
                    }
                }
                delay += 0.4
            }
            for k in keys {
                let (key, window) = (k, self.window)
                DispatchQueue.main.asyncAfter(deadline: .now() + delay) { [weak self] in
                    guard let e = NSEvent.keyEvent(with: .keyDown, location: .zero, modifierFlags: [], timestamp: 0, windowNumber: window.windowNumber, context: nil,
                                                   characters: String(key), charactersIgnoringModifiers: String(key), isARepeat: false, keyCode: keyCode(key)) else { return }
                    self?.window.firstResponder?.keyDown(with: e)
                }
                delay += 0.4
            }
            if turns > 0 {
                let meter = TurnMeter(self.view)
                for i in 0..<turns {
                    DispatchQueue.main.asyncAfter(deadline: .now() + delay + 0.5 + Double(i)) { [weak self] in
                        self?.view.turn(1)
                        meter.turned()
                    }
                }
                delay += 1 + Double(turns)
                DispatchQueue.main.asyncAfter(deadline: .now() + delay) { meter.report() }
            }
            DispatchQueue.main.asyncAfter(deadline: .now() + delay + 0.4) { [weak self] in
                if let png, let self {
                    let image = self.container.bitmapImageRepForCachingDisplay(in: self.container.bounds)
                    if let image {
                        self.container.cacheDisplay(in: self.container.bounds, to: image)
                        try? image.representation(using: .png, properties: [:])?.write(to: URL(fileURLWithPath: png))
                    }
                    self.view.savePNG(png + ".page.png")
                }
                NSApp.terminate(nil)
            }
        }
    }
}

/// The virtual key of a letter or digit, for `--keys`.
private func keyCode(_ c: Character) -> UInt16 {
    ["a": 0x00, "s": 0x01, "d": 0x02, "f": 0x03, "h": 0x04, "g": 0x05, "=": 0x18, "-": 0x1B, "0": 0x1D, "]": 0x1E, "o": 0x1F, "[": 0x21, "l": 0x25, "j": 0x26, "k": 0x28,
     "/": 0x2C, " ": 0x31, "\r": 0x24, "\u{1B}": 0x35][c] ?? 0x7F
}
