// The view of an image or a table filling the window (`viewer`): opened from the page (`Tab` and `Return`, or a click),
// it zooms (`+`, `−`, `0`, `⌘`+wheel, a pinch) and pans (arrows, `h`/`j`/`k`/`l`, the wheel, the trackpad, a drag)
// within the edges, and closes (`Esc`, `q`, the close button) back to the unchanged page. Core Animation moves what is
// drawn: the image is one layer, a table bitmap tiles a window big in a layer that moves; nothing is drawn while idle.

import AppKit
import QuartzCore

/// Zoom factor of a key press or a wheel step; the limits for images (times the fitted size) and tables (times the
/// text size, in steps of 5 %).
private let step: CGFloat = 1.25
private let imageZoom: ClosedRange<CGFloat> = 1...5
private let tableZoom: ClosedRange<CGFloat> = 0.5...3
/// Space around a table: at the sides and below, and above (under the close button).
private let pad: CGFloat = 24
private let padTop: CGFloat = 56

final class Viewer: NSView {
    enum Subject {
        case image(String)
        /// A table, by its first row.
        case table(ForzacLocator)
    }

    private weak var page: PageView?
    private let subject: Subject
    /// The image, or the table's tiles in a layer as big as the table.
    private let content = CALayer()
    private var image: CGImage?
    /// The table typeset for the view at `typesetZoom` in `typesetTheme`: the core's handle, its size and areas.
    private var table: OpaquePointer?
    private var tablePage = ForzacPage()
    private var tableSize = CGSize.zero
    private var tableAreas: [Area] = []
    private var typesetZoom: CGFloat = 1
    private var typesetTheme = -1
    private var tiles: [Tile: CALayer] = [:]
    private let focusLayer = CALayer()
    /// An image: times the fitted size; a table: times the text size.
    private var zoom: CGFloat = 1
    /// The content's top left corner, y down from the view's top.
    private var pos = CGPoint.zero
    /// The view's size when the content was placed; zero before the first layout.
    private var placedFor = CGSize.zero
    /// The focused marker or link of a table (`Tab`), an index into its stops.
    private var focus: Int?
    private var mouseDownAt: CGPoint?
    private var dragFrom = CGPoint.zero
    private let closeCircle = LayerView()
    private let close = FlatButton(IconView(Icons.x, size: 22))
    private var closing = false
    /// Called after the view has faded out.
    var onClose: (() -> Void)?

    private struct Tile: Hashable {
        let column: Int, row: Int
    }

    private var isImage: Bool { if case .image = subject { return true } else { return false } }

    /// Opens `subject` of `page`'s book; nil when it cannot be shown (an image that cannot be decoded, not a table).
    init?(page: PageView, subject: Subject) {
        self.page = page
        self.subject = subject
        super.init(frame: .zero)
        switch subject {
        case .image(let id):
            guard let image = page.picture(id) else { return nil }
            self.image = image
        case .table:
            guard typeset(1) else { return nil }
        }
        wantsLayer = true
        layerContentsRedrawPolicy = .never
        content.actions = ["contents": NSNull()]
        content.magnificationFilter = .linear
        content.minificationFilter = .trilinear
        layer!.addSublayer(content)
        focusLayer.actions = ["sublayers": NSNull()]
        content.addSublayer(focusLayer)
        closeCircle.layer!.cornerRadius = 22
        closeCircle.layer!.shadowOpacity = 0.12
        closeCircle.layer!.shadowRadius = 4
        closeCircle.layer!.shadowOffset = CGSize(width: 0, height: -1)
        closeCircle.addSubview(close)
        close.translatesAutoresizingMaskIntoConstraints = false
        addSubview(closeCircle)
        NSLayoutConstraint.activate([
            closeCircle.widthAnchor.constraint(equalToConstant: 44), closeCircle.heightAnchor.constraint(equalToConstant: 44),
            closeCircle.topAnchor.constraint(equalTo: topAnchor, constant: 8), closeCircle.trailingAnchor.constraint(equalTo: trailingAnchor, constant: -8),
            close.widthAnchor.constraint(equalToConstant: 44), close.heightAnchor.constraint(equalToConstant: 44),
            close.centerXAnchor.constraint(equalTo: closeCircle.centerXAnchor), close.centerYAnchor.constraint(equalTo: closeCircle.centerYAnchor),
        ])
        close.onClick = { [weak self] in self?.dismiss() }
        recolour()
        alphaValue = 0
    }

    required init?(coder: NSCoder) { fatalError() }

    deinit {
        if let table { forzac_table_free(table) }
    }

    override var acceptsFirstResponder: Bool { true }

    /// Fades in, as on Android (200 ms).
    func show() {
        NSAnimationContext.runAnimationGroup { ctx in
            ctx.duration = 0.2
            animator().alphaValue = 1
        }
    }

    /// Fades out (150 ms) and removes itself; the page gets the keys back.
    func dismiss() {
        guard !closing else { return }
        closing = true
        if let page { window?.makeFirstResponder(page) }
        NSAnimationContext.runAnimationGroup({ ctx in
            ctx.duration = 0.15
            animator().alphaValue = 0
        }, completionHandler: { [weak self] in
            self?.removeFromSuperview()
            self?.onClose?()
        })
    }

    /// The page's theme: the paper behind, the ink in the table (typeset again), the close button.
    func recolour() {
        guard let page else { return }
        let p = page.palette
        layer!.backgroundColor = color(p.paper)
        closeCircle.layer!.backgroundColor = color(p.sheet)
        close.colour(p)
        if !isImage, typesetTheme != page.theme {
            _ = typeset(typesetZoom)
            place(animated: false)
        }
    }

    override func layout() {
        super.layout()
        guard bounds.width > 0, bounds.height > 0, bounds.size != placedFor else { return }
        if placedFor == .zero {
            placedFor = bounds.size
            fit(animated: false)
        } else {
            // A new size keeps the content's point at the view's centre; tiles are a window big, so new ones.
            let size = contentSize(zoom)
            let centre = CGPoint(x: (placedFor.width / 2 - pos.x) / max(size.width, 1), y: (placedFor.height / 2 - pos.y) / max(size.height, 1))
            placedFor = bounds.size
            dropTiles()
            let new = contentSize(zoom)
            pos = CGPoint(x: bounds.width / 2 - centre.x * new.width, y: bounds.height / 2 - centre.y * new.height)
            place(animated: false)
        }
    }

    override func setFrameSize(_ size: NSSize) {
        super.setFrameSize(size)
        needsLayout = true
    }

    // MARK: Geometry

    /// The content's size at `zoom`: an image fitted to the view, then zoomed; a table as typeset (scaled for a zoom it
    /// has not been typeset at yet).
    private func contentSize(_ zoom: CGFloat) -> CGSize {
        if let image {
            let (iw, ih) = (CGFloat(max(image.width, 1)), CGFloat(max(image.height, 1)))
            let fit = min(bounds.width / iw, bounds.height / ih)
            return CGSize(width: iw * fit * zoom, height: ih * fit * zoom)
        }
        return CGSize(width: tableSize.width * zoom / typesetZoom, height: tableSize.height * zoom / typesetZoom)
    }

    /// `p` for content of `size` kept within the edges: centred in a direction where it is smaller than the view (less
    /// the padding), else no gap between its edge and the padding.
    private func clamp(_ p: CGPoint, _ size: CGSize) -> CGPoint {
        let (l, t, r, b) = isImage ? (0, 0, 0, 0) : (pad, padTop, pad, pad)
        func axis(_ p: CGFloat, _ c: CGFloat, _ view: CGFloat, _ lo: CGFloat, _ hi: CGFloat) -> CGFloat {
            c + lo + hi <= view ? lo + (view - lo - hi - c) / 2 : min(max(p, view - hi - c), lo)
        }
        return CGPoint(x: axis(p.x, size.width, bounds.width, l, r), y: axis(p.y, size.height, bounds.height, t, b))
    }

    /// Puts the content where `pos` and `zoom` say: smoothly in 150 ms (ease-out) or at once.
    private func place(animated: Bool) {
        let size = contentSize(zoom)
        pos = clamp(pos, size)
        let h = bounds.height
        let frame = CGRect(x: pos.x, y: h - pos.y - size.height, width: size.width, height: size.height)
        if !isImage { buildTiles(around: frame) }
        CATransaction.begin()
        if animated {
            CATransaction.setAnimationDuration(0.15)
            CATransaction.setAnimationTimingFunction(CAMediaTimingFunction(controlPoints: 0.215, 0.61, 0.355, 1))
        } else {
            CATransaction.setDisableActions(true)
        }
        if let image, content.contents == nil {
            content.contents = image
        }
        content.frame = frame
        CATransaction.commit()
        drawFocus()
    }

    private func pan(_ dx: CGFloat, _ dy: CGFloat, animated: Bool) {
        // An infinite step goes to the edge: the clamp stops it there.
        func far(_ p: CGFloat, _ d: CGFloat) -> CGFloat { d.isInfinite ? -(d.sign == .minus ? -1 : 1) * 1e9 : p - d }
        pos = CGPoint(x: far(pos.x, dx), y: far(pos.y, dy))
        place(animated: animated)
    }

    private func panPart(_ x: CGFloat, _ y: CGFloat) { pan(x * bounds.width, y * bounds.height, animated: true) }

    /// Zooms to `target` (within the limits; a table in 5 % steps, typeset again) keeping the content's point at `at`
    /// (y down; the view's centre when nil).
    private func zoom(to target: CGFloat, at: CGPoint?, animated: Bool) {
        let new = isImage ? min(max(target, imageZoom.lowerBound), imageZoom.upperBound)
                          : min(max((target * 20).rounded() / 20, tableZoom.lowerBound), tableZoom.upperBound)
        guard new != zoom else { return }
        let at = at ?? CGPoint(x: bounds.width / 2, y: bounds.height / 2)
        let before = contentSize(zoom)
        if !isImage { _ = typeset(new) }
        zoom = new
        let k = contentSize(new).width / max(before.width, 1)
        pos = CGPoint(x: at.x - (at.x - pos.x) * k, y: at.y - (at.y - pos.y) * k)
        // A table typeset at a new size cannot morph from the old one: it moves at once.
        place(animated: animated && isImage)
    }

    /// The whole image fitted and centred again; a table at the text size from its top left corner.
    private func fit(animated: Bool) {
        if !isImage { _ = typeset(1) }
        zoom = 1
        pos = CGPoint(x: pad, y: padTop)
        place(animated: animated && isImage)
    }

    // MARK: Table

    /// Typesets the table at `zoom` in the page's theme; false when it is not a table.
    private func typeset(_ zoom: CGFloat) -> Bool {
        guard let page, case .table(let at) = subject else { return false }
        if table != nil, typesetZoom == zoom, typesetTheme == page.theme { return true }
        guard let new = withUnsafePointer(to: page.settings, { forzac_table(page.doc, at, Float(zoom), $0) }) else { return false }
        if let table { forzac_table_free(table) }
        table = new
        var w: Float = 0, h: Float = 0
        tablePage = forzac_table_page(new, &w, &h)
        tableSize = CGSize(width: CGFloat(w), height: CGFloat(h))
        tableAreas = areas(of: tablePage)
        typesetZoom = zoom
        typesetTheme = page.theme
        dropTiles()
        return true
    }

    private func dropTiles() {
        for t in tiles.values { t.removeFromSuperlayer() }
        tiles = [:]
    }

    /// Draws the tiles (a view big) of the table the view shows at `frame` (y up) and where it shows now, so a movement
    /// between them only moves; drops tiles far away.
    private func buildTiles(around frame: CGRect) {
        guard let page, bounds.width > 0, bounds.height > 0 else { return }
        let (tw, th) = (bounds.width.rounded(.up), bounds.height.rounded(.up))
        let h = bounds.height
        let size = frame.size
        // The visible parts of the table, y down from its top, now and after the move.
        var wanted = Set<Tile>()
        for f in [content.frame, frame] where f.width > 0 {
            let top = h - f.maxY
            let visible = CGRect(x: -f.minX, y: -top, width: bounds.width, height: bounds.height).intersection(CGRect(origin: .zero, size: size))
            guard !visible.isNull else { continue }
            for c in Int(visible.minX / tw)...Int(max(visible.maxX - 1, 0) / tw) {
                for r in Int(visible.minY / th)...Int(max(visible.maxY - 1, 0) / th) { wanted.insert(Tile(column: c, row: r)) }
            }
        }
        let scale = window?.backingScaleFactor ?? 2
        let palette = page.palette
        CATransaction.begin()
        CATransaction.setDisableActions(true)
        for tile in wanted where tiles[tile] == nil {
            let (tx, ty) = (CGFloat(tile.column) * tw, CGFloat(tile.row) * th)
            let l = CALayer()
            l.actions = ["position": NSNull(), "bounds": NSNull(), "contents": NSNull()]
            l.contents = bitmap(CGSize(width: tw, height: th), scale: scale, background: nil) { ctx in
                ctx.translateBy(x: -tx, y: 0)
                var pictures: [UnsafeRawPointer: CGImage] = [:]
                drawPage(tablePage, in: ctx, height: th + ty, palette: palette, fonts: page.fonts, pictures: &pictures)
            }
            l.contentsScale = scale
            content.insertSublayer(l, below: focusLayer)
            tiles[tile] = l
        }
        // Tiles sit in the table's layer, y up from its bottom; the layer is as big as the table at the zoom typeset.
        for (tile, l) in tiles {
            let (tx, ty) = (CGFloat(tile.column) * tw, CGFloat(tile.row) * th)
            l.frame = CGRect(x: tx, y: tableSize.height - ty - th, width: tw, height: th)
        }
        if tiles.count > 24 {
            for (tile, l) in tiles where !wanted.contains(tile) {
                l.removeFromSuperlayer()
                tiles[tile] = nil
            }
        }
        CATransaction.commit()
    }

    private func drawFocus() {
        CATransaction.begin()
        CATransaction.setDisableActions(true)
        focusLayer.sublayers = nil
        if let page, let f = focus, let stop = stops(tableAreas)[safe: f] {
            focusLayer.frame = CGRect(origin: .zero, size: tableSize)
            for l in focusLayers(tableAreas, stop, palette: page.palette, height: tableSize.height) { focusLayer.addSublayer(l) }
        }
        CATransaction.commit()
    }

    /// `Tab` in a table: the next or previous marker or link, panned into the view.
    private func focusNext(_ delta: Int) {
        let all = stops(tableAreas)
        guard !all.isEmpty else { return }
        focus = focus.map { (($0 + delta) % all.count + all.count) % all.count } ?? (delta > 0 ? 0 : all.count - 1)
        let r = tableAreas[all[focus!]].map(\.rect).reduce(tableAreas[all[focus!].lowerBound].rect) { $0.union($1) }
        let k = zoom / typesetZoom
        // The least pan that brings the stop inside the view, less the padding.
        func into(_ start: CGFloat, _ length: CGFloat, _ lo: CGFloat, _ hi: CGFloat) -> CGFloat {
            if start < lo { return start - lo }
            if start + length > hi { return min(start + length - hi, start - lo) }
            return 0
        }
        let dx = into(pos.x + r.minX * k, r.width * k, pad, bounds.width - pad)
        let dy = into(pos.y + r.minY * k, r.height * k, padTop, bounds.height - pad)
        if dx != 0 || dy != 0 { pan(dx, dy, animated: true) } else { drawFocus() }
    }

    private func openFocused() {
        guard let f = focus, let stop = stops(tableAreas)[safe: f] else { return }
        activate(tableAreas[stop.lowerBound])
    }

    /// Opens a marker or link of the table: a note or a short target in the sheet over the view, a place in the book
    /// after closing the view.
    private func activate(_ area: Area) {
        guard let page else { return }
        if page.jumps(area) { dismiss() }
        page.open(area)
    }

    // MARK: Input

    /// A point of the event, y down.
    private func point(_ e: NSEvent) -> CGPoint {
        let p = convert(e.locationInWindow, from: nil)
        return CGPoint(x: p.x, y: bounds.height - p.y)
    }

    override func mouseDown(with e: NSEvent) {
        mouseDownAt = point(e)
        dragFrom = pos
    }

    override func mouseDragged(with e: NSEvent) {
        guard let down = mouseDownAt else { return }
        let p = point(e)
        pos = CGPoint(x: dragFrom.x + p.x - down.x, y: dragFrom.y + p.y - down.y)
        place(animated: false)
    }

    override func mouseUp(with e: NSEvent) {
        let p = point(e)
        defer { mouseDownAt = nil }
        guard let down = mouseDownAt, hypot(p.x - down.x, p.y - down.y) <= 4 else { return } // a drag, not a click
        // A click on an image closes the view, as a tap on Android; on a table it opens the marker or link under it.
        if isImage { return dismiss() }
        let k = zoom / typesetZoom
        let at = CGPoint(x: (p.x - pos.x) / k, y: (p.y - pos.y) / k)
        if let a = tableAreas.first(where: { $0.rect.insetBy(dx: -4, dy: -4).contains(at) }) { activate(a) }
    }

    override func scrollWheel(with e: NSEvent) {
        if e.modifierFlags.contains(.command) {
            let factor = e.hasPreciseScrollingDeltas ? exp(e.scrollingDeltaY / 200) : pow(step, e.scrollingDeltaY > 0 ? 1 : -1)
            if e.scrollingDeltaY != 0 { zoom(to: zoom * factor, at: point(e), animated: false) }
            return
        }
        if e.hasPreciseScrollingDeltas {
            pan(-e.scrollingDeltaX, -e.scrollingDeltaY, animated: false)
        } else {
            let lines = 3 * (page?.lineHeight ?? 30)
            pan(-e.scrollingDeltaX * lines, -e.scrollingDeltaY * lines, animated: true)
        }
    }

    override func magnify(with e: NSEvent) {
        zoom(to: zoom * (1 + e.magnification), at: point(e), animated: false)
    }

    /// Keys by physical key code, as the page's.
    override func keyDown(with e: NSEvent) {
        guard !closing else { return }
        let shift = e.modifierFlags.contains(.shift)
        switch e.shortcutKey {
        case 0x35, 0x0C: dismiss()                                                   // Esc q
        case 0x18, 0x45: zoom(to: zoom * step, at: nil, animated: true)              // = + (keypad +)
        case 0x1B, 0x4E: zoom(to: zoom / step, at: nil, animated: true)              // - (keypad -)
        case 0x1D, 0x52: fit(animated: true)                                         // 0
        case 0x04, 0x7B: panPart(-0.125, 0)                                          // h ←
        case 0x25, 0x7C: panPart(0.125, 0)                                           // l →
        case 0x28, 0x7E: panPart(0, -0.125)                                          // k ↑
        case 0x26, 0x7D: panPart(0, 0.125)                                           // j ↓
        case 0x31: panPart(0, shift ? -0.9 : 0.9)                                    // Space Shift+Space
        case 0x79: panPart(0, 0.9)                                                   // PgDn
        case 0x74: panPart(0, -0.9)                                                  // PgUp
        case 0x05 where shift, 0x77: pan(0, .infinity, animated: true)               // G End
        case 0x05, 0x73: pan(0, -.infinity, animated: true)                          // g Home
        case 0x30: focusNext(shift ? -1 : 1)                                         // Tab
        case 0x24, 0x4C: openFocused()                                               // Return
        default: super.keyDown(with: e)
        }
    }
}
