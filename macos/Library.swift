// The library (`library`): the Books folder's books and the reading state's as the web reader's shelf of covers, with
// the Android app's "Continue reading" card, generated covers, search and a question before a book (its file) goes to
// the Trash. Laid out as the GTK shell's `library.rs`. Cover thumbnails are cached in ~/Library/Caches/Forzac/covers, one file
// per book id (empty: the book has none).

import AppKit
import ImageIO

// MARK: Cover cache

/// Cached thumbnails fit this box: 176 points at 2×.
private let thumbMax = 528

private let coverCache: URL = FileManager.default.urls(for: .cachesDirectory, in: .userDomainMask)[0].appendingPathComponent("Forzac/covers")

func thumbnailURL(_ id: String) -> URL { coverCache.appendingPathComponent(id) }

/// Writes the thumbnail of a cover (`data`), or the empty marker for none; on a background queue.
func writeThumbnail(id: String, data: Data?) {
    try? FileManager.default.createDirectory(at: coverCache, withIntermediateDirectories: true)
    let url = thumbnailURL(id)
    let tmp = url.appendingPathExtension("tmp")
    var written = false
    if let data, let source = CGImageSourceCreateWithData(data as CFData, nil),
       let image = CGImageSourceCreateThumbnailAtIndex(source, 0, [kCGImageSourceCreateThumbnailFromImageAlways: true, kCGImageSourceThumbnailMaxPixelSize: thumbMax,
                                                                   kCGImageSourceCreateThumbnailWithTransform: true] as CFDictionary) {
        // Scaled to fit 352 × 528: a cover narrower than 2 : 3 is limited by its width.
        let scale = min(1, CGFloat(thumbMax * 2 / 3) / CGFloat(image.width), CGFloat(thumbMax) / CGFloat(image.height))
        let fitted = scale < 1 ? resized(image, scale) ?? image : image
        let alpha = ![.none, .noneSkipLast, .noneSkipFirst].contains(fitted.alphaInfo)
        if let dest = CGImageDestinationCreateWithURL(tmp as CFURL, (alpha ? "public.png" : "public.jpeg") as CFString, 1, nil) {
            CGImageDestinationAddImage(dest, fitted, [kCGImageDestinationLossyCompressionQuality: 0.85] as CFDictionary)
            written = CGImageDestinationFinalize(dest)
        }
    } else {
        written = FileManager.default.createFile(atPath: tmp.path, contents: Data())
    }
    if written {
        try? FileManager.default.removeItem(at: url)
        try? FileManager.default.moveItem(at: tmp, to: url)
    }
}

private func resized(_ image: CGImage, _ scale: CGFloat) -> CGImage? {
    let (w, h) = (max(1, Int(CGFloat(image.width) * scale)), max(1, Int(CGFloat(image.height) * scale)))
    guard let ctx = CGContext(data: nil, width: w, height: h, bitsPerComponent: 8, bytesPerRow: 0, space: CGColorSpace(name: CGColorSpace.sRGB)!,
                              bitmapInfo: CGImageAlphaInfo.premultipliedLast.rawValue) else { return nil }
    ctx.interpolationQuality = .high
    ctx.draw(image, in: CGRect(x: 0, y: 0, width: w, height: h))
    return ctx.makeImage()
}

/// Caches the cover of an opened book, if it is not cached yet.
func cacheCover(_ doc: OpaquePointer) {
    let id = String(cString: forzac_book_id(doc))
    guard !FileManager.default.fileExists(atPath: thumbnailURL(id).path) else { return }
    var len = 0
    let data = forzac_cover(doc, &len).map { Data(bytes: $0, count: len) }
    DispatchQueue.global(qos: .utility).async { writeThumbnail(id: id, data: data) }
}

// MARK: Covers

/// What a cover shows: its picture being loaded or made (the line colour, as the web's `.cover`), the picture, or a
/// generated cover.
enum Thumb {
    case waiting
    case picture(CGImage)
    case generated
}

/// A book of the library, from the Books folder and the reading state.
struct LibraryBook {
    let id, path, title, authors: String
    let progress: Float
    /// False for a book in the folder never opened.
    let opened: Bool
    /// Empty when not in a series.
    var series = ""
    /// The number in the series; negative when none.
    var number: Float = -1
    /// The reader's shelves it is on (`library-shelves`).
    var shelves: [String] = []

    /// The library's books (`forzac_library_books`): reads the Books folder, so on a background queue with a state
    /// handle of its own.
    static func list(_ state: OpaquePointer) -> [LibraryBook] {
        var n: UInt32 = 0
        let list = forzac_library_books(state, &n)!
        return (0..<Int(n)).map { i in
            let path = String(cString: list[i].path)
            let title = String(cString: list[i].title)
            return LibraryBook(id: String(cString: list[i].id), path: path, title: title.isEmpty ? URL(fileURLWithPath: path).lastPathComponent : title,
                               authors: String(cString: list[i].authors), progress: list[i].progress, opened: list[i].opened,
                               series: String(cString: list[i].series), number: list[i].number,
                               shelves: String(cString: list[i].shelves).split(separator: "\n").map(String.init))
        }
    }
}

/// A book's cover: its picture filling the box, or a generated one (the Android app's `GeneratedCover`). The view
/// is larger than the cover by `margin` on every side, for the shadow, the selection ring and the lift under the
/// pointer.
///
/// On screen the cover is drawn once, at its size, into an image (`face`); the shadows and the ring are layers and the
/// lift moves them, so scrolling and the pointer only move layers on the GPU. AppKit drops the drawing of views
/// scrolled out of sight: a cover drawn in `draw(_:)` was drawn again, picture scaling and blurred shadows on the CPU,
/// each time it scrolled back in. `draw(_:)` stays for `cacheDisplay` (`--png`).
final class CoverView: NSView {
    static let margin: CGFloat = 8
    /// The web's shadows, `0 1px 2px` at 12 % and `0 6px 18px` at 10 %: offset down, blur, opacity.
    private static let shadows: [(CGFloat, CGFloat, CGFloat)] = [(1, 2, 0.12), (6, 18, 0.10)]
    let book: LibraryBook
    let style: ForzacCoverStyle
    var palette: Palette
    var thumb: Thumb { didSet { face = nil; needsLayout = true } }
    var marble: CGImage? { didSet { face = nil; needsLayout = true } }
    var selected = false { didSet { if selected != oldValue { needsLayout = true } } }
    var hover = false { didSet { if hover != oldValue { needsLayout = true } } }

    private var face: CGImage?
    private let faceLayer = CALayer()
    private let shadowLayers = CoverView.shadows.map { _ in CALayer() }
    private let ring = CAShapeLayer()

    init(book: LibraryBook, thumb: Thumb, palette: Palette) {
        self.book = book
        self.thumb = thumb
        self.palette = palette
        style = forzac_cover_style(book.id, book.title)
        super.init(frame: .zero)
        wantsLayer = true
        for (l, (dy, blur, alpha)) in zip(shadowLayers, CoverView.shadows) {
            l.shadowColor = .black
            l.shadowOpacity = Float(alpha)
            l.shadowRadius = blur / 2
            l.shadowOffset = CGSize(width: 0, height: dy) // y down: the view is flipped
            layer!.addSublayer(l)
        }
        layer!.addSublayer(faceLayer)
        ring.fillColor = nil
        ring.lineWidth = 2
        layer!.addSublayer(ring)
    }

    required init?(coder: NSCoder) { fatalError() }

    override var isFlipped: Bool { true }
    override var wantsUpdateLayer: Bool { true }
    override func updateLayer() {} // the sublayers hold everything

    override func setFrameSize(_ newSize: NSSize) {
        if newSize != frame.size { face = nil }
        super.setFrameSize(newSize)
        needsLayout = true
    }

    override func viewDidChangeBackingProperties() {
        super.viewDidChangeBackingProperties()
        face = nil
        needsLayout = true
    }

    override func layout() {
        super.layout()
        CATransaction.begin()
        CATransaction.setDisableActions(true)
        var r = cover
        if hover { r.origin.y -= 3 } // the web's `li:hover .cover`
        let scale = window?.backingScaleFactor ?? 2
        if face == nil { face = render(r.size, scale: scale) }
        for l in shadowLayers {
            l.frame = r
            l.shadowPath = shape(CGRect(origin: .zero, size: r.size))
        }
        faceLayer.frame = r
        faceLayer.contents = face
        faceLayer.contentsScale = scale
        ring.path = shape(r.insetBy(dx: -4, dy: -4), grow: 4)
        ring.strokeColor = color(palette.ink)
        ring.isHidden = !selected
        CATransaction.commit()
    }

    /// The cover's face as an image `size` points large: drawn as in the view, y down.
    private func render(_ size: CGSize, scale: CGFloat) -> CGImage? {
        let (w, h) = (Int((size.width * scale).rounded(.up)), Int((size.height * scale).rounded(.up)))
        guard w > 0, h > 0,
              let ctx = CGContext(data: nil, width: w, height: h, bitsPerComponent: 8, bytesPerRow: 0, space: CGColorSpace(name: CGColorSpace.sRGB)!,
                                  bitmapInfo: CGImageAlphaInfo.premultipliedFirst.rawValue | CGBitmapInfo.byteOrder32Little.rawValue) else { return nil }
        ctx.scaleBy(x: scale, y: scale)
        ctx.translateBy(x: 0, y: size.height)
        ctx.scaleBy(x: 1, y: -1)
        NSGraphicsContext.saveGraphicsState()
        NSGraphicsContext.current = NSGraphicsContext(cgContext: ctx, flipped: true)
        drawFace(ctx, CGRect(origin: .zero, size: size))
        NSGraphicsContext.restoreGraphicsState()
        return ctx.makeImage()
    }

    /// The cover's own rectangle inside the view.
    var cover: CGRect { bounds.insetBy(dx: CoverView.margin, dy: CoverView.margin) }

    /// Android's `BookShape`: nearly square at the spine, rounded at the edge.
    private func shape(_ r: CGRect, grow: CGFloat = 0) -> CGPath {
        let (spine, edge) = (2 + grow, r.width * 0.06 + grow)
        let p = CGMutablePath()
        p.move(to: CGPoint(x: r.minX + spine, y: r.minY))
        p.addArc(tangent1End: CGPoint(x: r.maxX, y: r.minY), tangent2End: CGPoint(x: r.maxX, y: r.maxY), radius: edge)
        p.addArc(tangent1End: CGPoint(x: r.maxX, y: r.maxY), tangent2End: CGPoint(x: r.minX, y: r.maxY), radius: edge)
        p.addArc(tangent1End: CGPoint(x: r.minX, y: r.maxY), tangent2End: CGPoint(x: r.minX, y: r.minY), radius: spine)
        p.addArc(tangent1End: CGPoint(x: r.minX, y: r.minY), tangent2End: CGPoint(x: r.maxX, y: r.minY), radius: spine)
        p.closeSubpath()
        return p
    }

    override func draw(_ dirtyRect: NSRect) {
        guard let ctx = NSGraphicsContext.current?.cgContext else { return }
        var r = cover
        if hover { r.origin.y -= 3 } // the web's `li:hover .cover`
        let path = shape(r)
        // The web's shadow (y down here, so the offset is positive).
        for (dy, blur, alpha) in CoverView.shadows {
            ctx.saveGState()
            ctx.setShadow(offset: CGSize(width: 0, height: -dy), blur: blur, color: color(0x000000, alpha: alpha))
            ctx.addPath(path)
            ctx.setFillColor(color(palette.line))
            ctx.fillPath()
            ctx.restoreGState()
        }
        if selected {
            ctx.addPath(shape(r.insetBy(dx: -4, dy: -4), grow: 4))
            ctx.setStrokeColor(color(palette.ink))
            ctx.setLineWidth(2)
            ctx.strokePath()
        }
        drawFace(ctx, r)
    }

    /// The cover in `r`, clipped to the book's shape: the picture filling it, or the generated one.
    private func drawFace(_ ctx: CGContext, _ r: CGRect) {
        ctx.saveGState()
        ctx.addPath(shape(r))
        ctx.clip()
        switch thumb {
        case .picture(let image):
            let scale = max(r.width / CGFloat(image.width), r.height / CGFloat(image.height))
            let (w, h) = (CGFloat(image.width) * scale, CGFloat(image.height) * scale)
            ctx.interpolationQuality = .high
            drawImage(ctx, image, CGRect(x: r.midX - w / 2, y: r.midY - h / 2, width: w, height: h))
        case .waiting:
            ctx.setFillColor(color(palette.line))
            ctx.fill(r)
        case .generated:
            drawGenerated(ctx, r)
        }
        ctx.restoreGState()
    }

    /// A CGImage drawn upright in this flipped view.
    private func drawImage(_ ctx: CGContext, _ image: CGImage, _ r: CGRect) {
        ctx.saveGState()
        ctx.translateBy(x: r.minX, y: r.maxY)
        ctx.scaleBy(x: 1, y: -1)
        ctx.draw(image, in: CGRect(origin: .zero, size: r.size))
        ctx.restoreGState()
    }

    /// Cloth or marbled paper, a darker spine and the paper label with the title and the authors (`Covers.kt`).
    private func drawGenerated(_ ctx: CGContext, _ r: CGRect) {
        let w = r.width
        ctx.setFillColor(color(style.cloth))
        ctx.fill(r)
        if let marble {
            ctx.interpolationQuality = .medium
            drawImage(ctx, marble, r)
        }
        ctx.setFillColor(color(style.spine))
        ctx.fill(CGRect(x: r.minX, y: r.minY, width: w * 0.09, height: r.height))

        let base = w * 0.115
        let textWidth = w * 0.65
        // The longest word fits a line whole: a hyphen in a short title looks accidental. Down to 0.6 of the size.
        let longest = book.title.components(separatedBy: " ").max { $0.count < $1.count } ?? ""
        let probe = NSAttributedString(string: longest, attributes: [.font: bookFont("PT Serif", base * CGFloat(style.title_scale), bold: true)])
        let room = textWidth * 0.92
        let fit = probe.size().width > room ? max(room / probe.size().width, 0.6) : 1
        let centred = NSMutableParagraphStyle()
        centred.alignment = .center
        centred.lineBreakMode = .byWordWrapping
        let titleFont = bookFont("PT Serif", base * CGFloat(style.title_scale) * fit, bold: true)
        let title = NSAttributedString(string: book.title, attributes: [.font: titleFont, .foregroundColor: nsColor(0x231F1C), .paragraphStyle: centred])
        let options: NSString.DrawingOptions = [.usesLineFragmentOrigin, .truncatesLastVisibleLine]
        let titleHeight = min(title.boundingRect(with: CGSize(width: textWidth, height: 1e5), options: options).height,
                              ceil(titleFont.ascender - titleFont.descender + titleFont.leading) * CGFloat(style.title_lines))
        let authorsText = book.authors.trimmingCharacters(in: .whitespaces).uppercased()
        let authorsFont = interfaceFont(base * 0.46)
        let authors = NSAttributedString(string: authorsText, attributes: [.font: authorsFont, .foregroundColor: nsColor(0x6E655B), .kern: base * 0.05,
                                                                          .paragraphStyle: centred])
        let authorsHeight = authorsText.isEmpty ? 0 : min(authors.boundingRect(with: CGSize(width: textWidth, height: 1e5), options: options).height,
                                                          ceil(authorsFont.ascender - authorsFont.descender + authorsFont.leading) * 2)
        let (padX, padY) = (w * 0.05, w * 0.06)
        let label = CGRect(x: r.minX + w * 0.17, y: r.minY + r.height * 0.2, width: w * 0.75,
                           height: padY * 2 + titleHeight + (authorsText.isEmpty ? 0 : w * 0.035 + authorsHeight))
        ctx.setFillColor(color(0xF1EADB))
        ctx.fill(label)
        let frame = w * 0.008 + 0.5
        ctx.setStrokeColor(color(0xC49A52))
        ctx.setLineWidth(frame)
        ctx.stroke(label.insetBy(dx: frame / 2, dy: frame / 2))
        title.draw(with: CGRect(x: label.minX + padX, y: label.minY + padY, width: textWidth, height: titleHeight), options: options)
        if !authorsText.isEmpty {
            authors.draw(with: CGRect(x: label.minX + padX, y: label.minY + padY + titleHeight + w * 0.035, width: textWidth, height: authorsHeight), options: options)
        }
    }
}

/// The empty state's illustration: a book page over a second one turned slightly (the web's `.empty-book`).
final class SamplePageView: NSView {
    static let size = CGSize(width: 300, height: 433)
    let palette: Palette

    init(palette: Palette) {
        self.palette = palette
        super.init(frame: CGRect(origin: .zero, size: CGSize(width: SamplePageView.size.width + 24, height: SamplePageView.size.height + 48)))
    }

    required init?(coder: NSCoder) { fatalError() }

    override var isFlipped: Bool { true }

    override func draw(_ dirtyRect: NSRect) {
        guard let ctx = NSGraphicsContext.current?.cgContext else { return }
        let (w, h) = (SamplePageView.size.width, SamplePageView.size.height)
        let page = CGRect(x: 0, y: 0, width: w, height: h)
        let path = CGPath(roundedRect: page, cornerWidth: 6, cornerHeight: 6, transform: nil)
        ctx.translateBy(x: 4, y: 16)
        // The page under it: turned 3.2° about its centre, shifted, faded.
        ctx.saveGState()
        ctx.translateBy(x: w / 2 + w * 0.04, y: h / 2 + h * 0.01)
        ctx.rotate(by: 3.2 * .pi / 180)
        ctx.translateBy(x: -w / 2, y: -h / 2)
        ctx.setAlpha(0.7)
        ctx.setShadow(offset: CGSize(width: 0, height: -18), blur: 48, color: color(0x000000, alpha: 0.12))
        ctx.addPath(path)
        ctx.setFillColor(color(palette.sheet))
        ctx.fillPath()
        ctx.restoreGState()
        for (dy, blur, alpha) in [(1.0, 2.0, 0.06), (18.0, 48.0, 0.12)] as [(CGFloat, CGFloat, CGFloat)] {
            ctx.saveGState()
            ctx.setShadow(offset: CGSize(width: 0, height: -dy), blur: blur, color: color(0x000000, alpha: alpha))
            ctx.addPath(path)
            ctx.setFillColor(color(palette.sheet))
            ctx.fillPath()
            ctx.restoreGState()
        }
        ctx.addPath(path)
        ctx.clip()
        // Lines: the running head, the chapter title, its subtitle, the folio, the paragraphs (with soft hyphens).
        let lines = String(cString: forzac_sample_page()).components(separatedBy: "\n")
        guard lines.count > 4 else { return }
        let size = w * 0.0355
        let (left, inner) = (w * 0.09, w * 0.82)
        func style(_ align: NSTextAlignment, indent: CGFloat = 0) -> NSParagraphStyle {
            let s = NSMutableParagraphStyle()
            s.alignment = align
            s.firstLineHeadIndent = indent
            if align == .justified { (s.minimumLineHeight, s.maximumLineHeight) = (1.6 * size, 1.6 * size) }
            return s
        }
        // The web's padding `9% 9% 7%`: percentages of the width, also above and below.
        var y = w * 0.09
        func put(_ text: String, _ font: NSFont, _ rgb: UInt32, _ align: NSTextAlignment, indent: CGFloat = 0, after: CGFloat) {
            let s = NSAttributedString(string: text, attributes: [.font: font, .foregroundColor: nsColor(rgb), .paragraphStyle: style(align, indent: indent)])
            let height = ceil(s.boundingRect(with: CGSize(width: inner, height: 1e4), options: .usesLineFragmentOrigin).height)
            s.draw(with: CGRect(x: left, y: y, width: inner, height: height), options: .usesLineFragmentOrigin)
            y += height + after
        }
        put(lines[0], interfaceFont(size * 0.78), palette.muted, .center, after: size * 2.2)
        put(lines[1], bookFont("Literata", size * 1.15, bold: true), palette.ink, .center, after: lines[2].isEmpty ? size * 1.4 : 0)
        if !lines[2].isEmpty {
            let italic = NSFontManager.shared.convert(bookFont("Literata", size), toHaveTrait: .italicFontMask)
            put(lines[2], italic, palette.ink, .center, after: size * 1.4)
        }
        let folio = NSAttributedString(string: lines[3], attributes: [.font: interfaceFont(size * 0.72), .foregroundColor: nsColor(palette.muted),
                                                                       .paragraphStyle: style(.center)])
        let folioHeight = folio.size().height
        // The text stops above the folio, as the web's page hides what does not fit.
        ctx.saveGState()
        // At a line's edge: the paragraphs' lines are 1.6 of the size apart from the first one on.
        let room = h - w * 0.07 - folioHeight - size - y
        ctx.clip(to: CGRect(x: 0, y: 0, width: w, height: y + (room / (size * 1.6)).rounded(.down) * size * 1.6))
        for p in lines[4...] { put(p, bookFont("Literata", size), palette.ink, .justified, indent: size * 1.5, after: 0) }
        ctx.restoreGState()
        folio.draw(with: CGRect(x: left, y: h - w * 0.07 - folioHeight, width: inner, height: folioHeight), options: .usesLineFragmentOrigin)
    }
}

/// The web's ink pill.
final class PillButton: LayerView {
    let title: NSTextField
    var onClick: (() -> Void)?

    init(_ text: String, big: Bool, palette: Palette) {
        title = label(text, interfaceFont(big ? 17 : 15, .medium), palette.paper)
        super.init()
        translatesAutoresizingMaskIntoConstraints = true
        layer!.backgroundColor = color(palette.ink)
        title.translatesAutoresizingMaskIntoConstraints = true
        addSubview(title)
        let pad = big ? CGSize(width: 30, height: 16) : CGSize(width: 20, height: 11)
        let text = title.intrinsicContentSize
        frame.size = CGSize(width: ceil(text.width) + pad.width * 2, height: ceil(text.height) + pad.height * 2)
        title.frame = CGRect(x: pad.width, y: pad.height, width: ceil(text.width), height: ceil(text.height))
        layer!.cornerRadius = frame.height / 2
    }

    required init?(coder: NSCoder) { fatalError() }

    override func mouseDown(with e: NSEvent) { alphaValue = 0.88 }

    override func mouseUp(with e: NSEvent) {
        alphaValue = 1
        if bounds.contains(convert(e.locationInWindow, from: nil)) { onClick?() }
    }
}

/// A book on the window: the card or a shelf item, with its labels; a click opens it, a right click or its "⋯" opens
/// its menu; under the pointer its cover lifts and its "⋯" shows.
final class LibraryItem: NSView {
    let index: Int
    let cover: CoverView
    let labels: [NSTextField]
    let more: NSTextField
    let card: Bool
    let bar: LayerView?
    let fill: LayerView?
    let progress: Float
    var onOpen: (() -> Void)?
    var onMore: (() -> Void)?
    /// The selected book keeps its "⋯".
    var isSelected = false { didSet { more.alphaValue = isSelected ? 1 : 0 } }

    init(index: Int, cover: CoverView, labels: [NSTextField], card: Bool, progress: Float, palette: Palette) {
        self.index = index
        self.cover = cover
        self.labels = labels
        self.card = card
        self.progress = progress
        more = label("⋯", interfaceFont(18), palette.muted)
        more.toolTip = tr("More")
        more.alphaValue = 0
        if card {
            bar = LayerView()
            fill = LayerView()
            bar!.layer!.backgroundColor = color(palette.line)
            fill!.layer!.backgroundColor = color(palette.ink)
        } else {
            bar = nil
            fill = nil
        }
        super.init(frame: .zero)
        for v in [cover as NSView, more] + labels + [bar, fill].compactMap({ $0 }) {
            v.translatesAutoresizingMaskIntoConstraints = true
            addSubview(v)
        }
        addTrackingArea(NSTrackingArea(rect: .zero, options: [.mouseEnteredAndExited, .activeInKeyWindow, .inVisibleRect], owner: self))
    }

    required init?(coder: NSCoder) { fatalError() }

    override var isFlipped: Bool { true }

    override func mouseEntered(with e: NSEvent) {
        cover.hover = true
        more.alphaValue = 1
    }

    override func mouseExited(with e: NSEvent) {
        cover.hover = false
        more.alphaValue = isSelected ? 1 : 0
    }

    override func mouseDown(with e: NSEvent) {}

    override func mouseUp(with e: NSEvent) {
        let at = convert(e.locationInWindow, from: nil)
        if more.frame.insetBy(dx: -6, dy: -6).contains(at) { onMore?() } else if bounds.contains(at) { onOpen?() }
    }

    override func rightMouseDown(with e: NSEvent) {
        more.alphaValue = 1
        onMore?()
    }

    override func resetCursorRects() { addCursorRect(bounds, cursor: .pointingHand) }

    /// Lays the item out `width` wide; returns its height.
    @discardableResult
    func layout(width: CGFloat) -> CGFloat {
        let m = CoverView.margin
        if card {
            // Android's card: the cover 92 × 138, then the caption, title, authors, bar and share.
            cover.frame = CGRect(x: -m, y: -m, width: 92 + 2 * m, height: 138 + 2 * m)
            let x: CGFloat = 92 + 18
            let textWidth = width - x
            var heights: [CGFloat] = []
            for (i, l) in labels.enumerated() {
                let w = i == 1 ? textWidth - 24 : textWidth
                heights.append(ceil(l.cell!.cellSize(forBounds: CGRect(x: 0, y: 0, width: w, height: 1e4)).height))
            }
            let total = heights.reduce(0, +) + CGFloat(labels.count) * 6 + 6
            var y = max(0, (138 - total) / 2)
            for (i, l) in labels.enumerated() {
                if i == labels.count - 1 {
                    bar!.frame = CGRect(x: x, y: y + 4, width: textWidth, height: 2)
                    fill!.frame = CGRect(x: x, y: y + 4, width: textWidth * CGFloat(min(max(progress, 0), 1)), height: 2)
                    y += 12
                }
                l.frame = CGRect(x: x, y: y, width: i == 1 ? textWidth - 24 : textWidth, height: heights[i])
                if i == 1 { more.frame = CGRect(x: width - 18, y: y, width: 18, height: 22) }
                y += heights[i] + 6
            }
            return max(138, y)
        }
        let h = (width * 1.5).rounded()
        cover.frame = CGRect(x: -m, y: -m, width: width + 2 * m, height: h + 2 * m)
        var y = h + 12
        for (i, l) in labels.enumerated() {
            let w = i == 0 ? width - 20 : width
            let height = ceil(l.cell!.cellSize(forBounds: CGRect(x: 0, y: 0, width: w, height: 1e4)).height)
            l.frame = CGRect(x: 0, y: y, width: w, height: height)
            if i == 0 { more.frame = CGRect(x: width - 16, y: y - 3, width: 16, height: 22) }
            y += height + (i == 0 ? 2 : 4)
        }
        return y
    }
}

// MARK: The window

/// The library window, laid out as the web reader's `#library` in the reader's theme.
final class LibraryWindow: NSObject, NSTextFieldDelegate, NSWindowDelegate {
    let window: NSWindow
    let palette: Palette
    private let state: OpaquePointer
    private let open: (String) -> Void
    /// The Books folder, when it could be made.
    private var folder: String?
    private let remembersFrame: Bool
    private static let frameName = "Library"
    private var books: [LibraryBook]
    private var thumbs: [String: Thumb] = [:]
    private var marbles: [Int32: CGImage] = [:]
    private var items: [LibraryItem] = []
    private var hasCard = false
    private var columns = 1
    private(set) var selected = 0
    private var searching = false
    private var asking: (view: NSView, book: Int)?
    private var keys: Any?
    /// Covers being loaded or made; `onSettled` runs when none is left.
    private var pending = 0
    var onSettled: (() -> Void)? { didSet { settle() } }
    /// Adds a dropped folder (`folder-import`); the app runs it and reports through `addingStarted` and the rest.
    var onAddFolder: ((String) -> Void)?
    /// Folders being added; "Adding books… N of M" stands in place of the search button meanwhile.
    private var adding = 0
    /// The view (`library-metadata`: 0 Recent, 1 Title, 2 Author, 3 Series, 4 Shelves), its tabs under the head, and the shown
    /// groups as (books, columns) for moving the selection.
    private var view: UInt32
    private var tabs: [ClickLabel] = []
    private let tabRule = LayerView()
    private var groups: [(count: Int, heading: NSTextField?)] = []
    /// "About the book" while it shows.
    private var about: (view: AboutView, book: Int)?
    private let progressText: NSTextField

    private let scroll = NSScrollView()
    private let content = FlippedView()
    private let container: DropView
    private let brandIcon = NSImageView()
    private let brandName: NSTextField
    private let searchField = NSTextField()
    private let searchButton = FlatButton(IconView(Icons.search, size: 20))
    /// The head's "⋯": the Books folder and adding a folder (`books-folder-setting`).
    private let headMenuButton = FlatButton(IconView(Icons.ellipsis, size: 20))
    /// Changes the Books folder (the app runs it): the chosen path and whether to move the books.
    var onChangeFolder: ((String, Bool) -> Void)?
    /// Sends a book to the Kindle (the app runs it): its path and title.
    var onSendToKindle: ((String, String) -> Void)?
    private let closeButton = FlatButton(IconView(Icons.x, size: 20))
    private let openButton: PillButton
    /// "Version N is out — download ×" (`update-check`), in the head when it fits.
    private let updateText = ClickLabel()
    private let updateClose = ClickLabel()
    private var hasUpdate = false
    private var body: [NSView] = []
    private var bodyLayout: ((CGFloat, CGFloat) -> CGFloat)?

    /// The library of `books` (`forzac_library_books`) in the Books `folder`; it opens where and as large as it was
    /// left when it `remembersFrame` (not in measurement runs).
    init(state: OpaquePointer, books: [LibraryBook], folder: String?, remembersFrame: Bool, open: @escaping (String) -> Void) {
        self.state = state
        self.remembersFrame = remembersFrame
        self.open = open
        self.folder = folder
        self.books = books
        var settings = forzac_state_settings(state)
        palette = themes[Int(forzac_settings_theme(&settings, systemIsDark()))]
        brandName = label(tr("Forzac"), bookFont("Literata", 22, bold: true), palette.ink)
        openButton = PillButton(tr("Open a book"), big: false, palette: palette)
        progressText = label("", interfaceFont(15), palette.muted)
        progressText.isHidden = true
        view = forzac_library_view(state)
        container = DropView(palette: palette)
        window = NSWindow(contentRect: NSRect(x: 0, y: 0, width: 1040, height: 760), styleMask: [.titled, .closable, .miniaturizable, .resizable], backing: .buffered, defer: false)
        super.init()
        window.title = tr("Forzac")
        window.isReleasedWhenClosed = false
        window.tabbingMode = .disallowed
        window.backgroundColor = nsColor(palette.paper)
        window.appearance = NSAppearance(named: palette.paper < 0x808080 ? .darkAqua : .aqua)
        window.delegate = self
        if let screen = NSScreen.main?.visibleFrame { window.setContentSize(CGSize(width: min(1040, screen.width * 0.85), height: min(760, screen.height * 0.85))) }

        // The app's icon; a development build without a bundle takes it from the source tree.
        let icns = Bundle.main.path(forResource: "Forzac", ofType: "icns")
            ?? URL(fileURLWithPath: #filePath).deletingLastPathComponent().appendingPathComponent("Forzac.icns").path
        brandIcon.image = NSImage(contentsOfFile: icns) ?? NSApp.applicationIconImage
        brandIcon.imageScaling = .scaleProportionallyUpOrDown
        searchField.isBordered = false
        searchField.drawsBackground = false
        searchField.focusRingType = .none
        searchField.font = bookFont("PT Serif", 26)
        searchField.textColor = nsColor(palette.ink)
        searchField.placeholderAttributedString = NSAttributedString(string: tr("Title, author or series"), attributes: [.font: bookFont("PT Serif", 26), .foregroundColor: nsColor(palette.muted)])
        searchField.delegate = self
        searchField.isHidden = true
        for b in [searchButton, closeButton, headMenuButton] {
            b.colour(palette)
            b.layer!.cornerRadius = 22
            b.layer!.borderWidth = 1
            b.layer!.borderColor = color(palette.line)
            b.translatesAutoresizingMaskIntoConstraints = true
        }
        searchButton.toolTip = tr("Search")
        closeButton.toolTip = tr("Close")
        closeButton.isHidden = true
        searchButton.onClick = { [weak self] in self?.setSearching(true) }
        headMenuButton.toolTip = tr("More")
        headMenuButton.onClick = { [weak self] in self?.headMenu() }
        closeButton.onClick = { [weak self] in self?.setSearching(false) }
        openButton.onClick = { [weak self] in self?.chooseFile() }
        // "install" (macos-auto-update) until it fails, then "download" from the site.
        updateText.onClick = {
            switch Installer.shared.phase {
            case .offered: Installer.shared.install()
            case .installing: break
            case .failed: NSWorkspace.shared.open(URL(string: String(cString: forzac_update_page()))!)
            }
        }
        Installer.shared.onChange = { [weak self] in self?.showUpdate() }
        updateClose.attributedStringValue = NSAttributedString(string: "×", attributes: [.font: interfaceFont(18), .foregroundColor: nsColor(palette.muted)])
        updateClose.sizeToFit()
        updateClose.onClick = { [weak self] in
            guard let self else { return }
            _ = forzac_update_dismiss(self.state)
            self.showUpdate()
        }
        updateNotice()
        tabRule.layer!.backgroundColor = color(palette.line)
        let viewNames = [tr("Recent"), tr("Title"), tr("Author"), tr("Series"), tr("Shelves")]
        for (i, name) in viewNames.enumerated() {
            let tab = ClickLabel()
            tab.stringValue = name
            tab.font = interfaceFont(15, .medium)
            tab.onClick = { [weak self] in self?.setView(UInt32(i)) }
            tabs.append(tab)
        }
        for v in [brandIcon, brandName, searchField, searchButton, closeButton, headMenuButton, openButton, progressText, updateText, updateClose, tabRule] + tabs as [NSView] {
            v.translatesAutoresizingMaskIntoConstraints = true
            content.addSubview(v)
        }
        scroll.documentView = content
        scroll.drawsBackground = false
        scroll.hasVerticalScroller = true
        scroll.autohidesScrollers = true
        scroll.autoresizingMask = [.width, .height]
        container.addSubview(scroll)
        container.onResize = { [weak self] in self?.relayout() }
        container.onDrop = { [weak self] urls in
            for url in urls {
                if (try? url.resourceValues(forKeys: [.isDirectoryKey]))?.isDirectory == true { self?.onAddFolder?(url.path) } else { self?.open(url.path) }
            }
        }
        window.contentView = container
        scroll.frame = container.bounds

        loadCovers()
        rebuild()
        if !remembersFrame || !window.setFrameUsingName(LibraryWindow.frameName) { window.center() }
        window.makeKeyAndOrderFront(nil)
        window.makeFirstResponder(nil)
        keys = NSEvent.addLocalMonitorForEvents(matching: .keyDown) { [weak self] e in
            guard let self, e.window === self.window else { return e }
            return self.key(e) ? nil : e
        }
    }

    func windowDidEndLiveResize(_ notification: Notification) { saveFrame() }
    func windowDidMove(_ notification: Notification) { saveFrame() }

    private func saveFrame() {
        if remembersFrame { window.saveFrame(usingName: LibraryWindow.frameName) }
    }

    func windowWillClose(_ notification: Notification) {
        saveFrame()
        if let keys { NSEvent.removeMonitor(keys) }
        keys = nil
    }

    var bookCount: Int { books.count }

    // MARK: Covers

    /// Loads every cached thumbnail on a background queue; books without one get it made, one at a time.
    private func loadCovers() {
        var missing: [LibraryBook] = []
        for b in books {
            let attributes = try? FileManager.default.attributesOfItem(atPath: thumbnailURL(b.id).path)
            if let size = attributes?[.size] as? Int {
                thumbs[b.id] = size == 0 ? .generated : .waiting
                if size > 0 { loadThumbnail(b.id) }
            } else {
                thumbs[b.id] = .waiting
                missing.append(b)
            }
        }
        guard !missing.isEmpty else { return }
        pending += missing.count
        let queue = DispatchQueue(label: "forzac.covers", qos: .utility)
        for b in missing {
            queue.async { [weak self] in
                guard self != nil else { return }
                var data: Data?
                if let doc = forzac_open(b.path, nil) {
                    var len = 0
                    data = forzac_cover(doc, &len).map { Data(bytes: $0, count: len) }
                    forzac_close(doc)
                }
                writeThumbnail(id: b.id, data: data)
                DispatchQueue.main.async {
                    guard let self else { return }
                    self.pending -= 1
                    let size = (try? FileManager.default.attributesOfItem(atPath: thumbnailURL(b.id).path))?[.size] as? Int ?? 0
                    if size > 0 { self.loadThumbnail(b.id) } else { self.setThumb(b.id, .generated) }
                    self.settle()
                }
            }
        }
    }

    private func loadThumbnail(_ id: String) {
        pending += 1
        DispatchQueue.global(qos: .userInitiated).async { [weak self] in
            // Decoded here, not when first drawn.
            let image = CGImageSourceCreateWithURL(thumbnailURL(id) as CFURL, nil).flatMap {
                CGImageSourceCreateImageAtIndex($0, 0, [kCGImageSourceShouldCacheImmediately: true] as CFDictionary)
            }
            DispatchQueue.main.async {
                guard let self else { return }
                self.pending -= 1
                self.setThumb(id, image.map { .picture($0) } ?? .generated)
                self.settle()
            }
        }
    }

    private func setThumb(_ id: String, _ thumb: Thumb) {
        thumbs[id] = thumb
        for item in items where item.cover.book.id == id {
            item.cover.thumb = thumb
            if case .generated = thumb { marble(for: item.cover) }
        }
    }

    /// Marbled paper for a generated cover that has it: made on a background queue, kept by its seed.
    private func marble(for cover: CoverView) {
        guard cover.style.marble else { return }
        let seed = cover.style.seed
        if let image = marbles[seed] { cover.marble = image; return }
        DispatchQueue.global(qos: .userInitiated).async { [weak self, weak cover] in
            let (w, h) = (88, 132)
            var pixels = [UInt8](repeating: 0, count: w * h * 4)
            forzac_marble(seed, UInt32(w), UInt32(h), &pixels)
            let image = CGImage(width: w, height: h, bitsPerComponent: 8, bitsPerPixel: 32, bytesPerRow: w * 4, space: CGColorSpace(name: CGColorSpace.sRGB)!,
                                bitmapInfo: CGBitmapInfo(rawValue: CGImageAlphaInfo.noneSkipLast.rawValue), provider: CGDataProvider(data: Data(pixels) as CFData)!,
                                decode: nil, shouldInterpolate: true, intent: .defaultIntent)
            DispatchQueue.main.async {
                guard let image else { return }
                self?.marbles[seed] = image
                for item in self?.items ?? [] where item.cover.style.marble && item.cover.style.seed == seed { item.cover.marble = image }
                cover?.marble = image
            }
        }
    }

    private func settle() {
        guard pending == 0, let done = onSettled else { return }
        onSettled = nil
        done()
    }

    private func coverView(_ book: Int) -> CoverView {
        let b = books[book]
        let view = CoverView(book: b, thumb: thumbs[b.id] ?? .waiting, palette: palette)
        if case .generated = view.thumb { marble(for: view) }
        return view
    }

    // MARK: Building

    /// Fills the window for the books and the search: the empty state, or the card and the shelf.
    private func rebuild() {
        body.forEach { $0.removeFromSuperview() }
        body = []
        items = []
        let empty = books.isEmpty
        searchButton.isHidden = empty || searching || adding > 0
        headMenuButton.isHidden = searching
        progressText.isHidden = adding == 0 || searching
        closeButton.isHidden = !searching
        openButton.isHidden = empty || searching
        brandIcon.isHidden = searching
        brandName.isHidden = searching
        searchField.isHidden = !searching
        let showTabs = !empty && !searching
        tabs.forEach { $0.isHidden = !showTabs }
        tabRule.isHidden = !showTabs
        if empty { buildEmpty(); relayout(); return }
        var card: Int?
        var arranged: [(heading: String?, books: [Int])]
        if searching {
            let query = searchField.stringValue
            arranged = [(nil, books.indices.compactMap { i -> (Int32, Int)? in
                let r = forzac_library_rank(query, books[i].title, books[i].authors, books[i].series)
                return r < 0 ? nil : (r, i)
            }.sorted { $0.0 < $1.0 }.map { $0.1 })]
        } else if view == 0 {
            card = books.firstIndex { $0.opened && $0.progress < 0.999 }
            arranged = [(nil, books.indices.filter { $0 != card })]
        } else {
            arranged = arrange()
        }
        let order = arranged.flatMap(\.books)
        if let c = card {
            let b = books[c]
            var labels = [label(tr("Continue reading").uppercased(), interfaceFont(12, .medium), palette.muted),
                          wrapping(b.title, bookFont("Literata", 21, bold: true), palette.ink, lines: 2)]
            if !b.authors.isEmpty { labels.append(wrapping(b.authors, interfaceFont(14), palette.muted, lines: 2)) }
            labels.append(label(share(b), interfaceFont(12), palette.muted))
            labels[0].attributedStringValue = NSAttributedString(string: labels[0].stringValue, attributes: [.font: interfaceFont(12, .medium), .kern: 0.7,
                                                                                                             .foregroundColor: nsColor(palette.muted)])
            items.append(item(c, labels: labels, card: true))
        }
        hasCard = card != nil
        let rule = LayerView()
        rule.layer!.backgroundColor = color(palette.line)
        let none = label(tr("No results"), interfaceFont(15), palette.muted)
        groups = []
        for group in arranged {
            let heading = group.heading.map { wrapping($0, bookFont("Literata", 19, bold: true), palette.ink, lines: 2) }
            groups.append((group.books.count, heading))
            if let heading { body.append(heading) }
            for i in group.books {
                let b = books[i]
                var labels = [wrapping(b.title, interfaceFont(14, .medium), palette.ink, lines: 3)]
                if !b.authors.isEmpty { labels.append(wrapping(b.authors, interfaceFont(13), palette.muted, lines: 2)) }
                if !b.series.isEmpty && (view != 3 || searching) {
                    labels.append(wrapping(seriesLabel(b), interfaceFont(13), palette.muted, lines: 1))
                }
                labels.append(label(share(b), interfaceFont(12), palette.muted))
                items.append(item(i, labels: labels, card: false))
            }
        }
        for v in items { body.append(v) }
        if hasCard && order.count > 0 { body.append(rule) }
        if searching && order.isEmpty { body.append(none) }
        body.forEach { $0.translatesAutoresizingMaskIntoConstraints = true; content.addSubview($0) }
        bodyLayout = { [weak self] x, width in
            guard let self else { return 0 }
            var y: CGFloat = 20 + 48 + 24 + (self.tabRule.isHidden ? 0 : self.tabRule.frame.maxY - 20 - 48)
            var shelf = self.items[...]
            if self.hasCard, let card = self.items.first {
                let h = card.layout(width: width)
                card.frame = CGRect(x: x, y: y, width: width, height: h)
                card.layout(width: width)
                y += h
                shelf = self.items.dropFirst()
                if !shelf.isEmpty {
                    rule.frame = CGRect(x: x, y: y + 28, width: width, height: 1)
                    y += 57
                }
            }
            if self.searching && shelf.isEmpty {
                none.sizeToFit()
                none.frame.origin = CGPoint(x: x + (width - none.frame.width) / 2, y: y + 48)
                y += 48 + none.frame.height
            }
            // The web's grid: columns `clamp(130, 14 %, 176)` wide, gaps 36 down and `clamp(20, 2.6 %, 32)` across.
            let gap = min(max(width * 0.026, 20), 32)
            let minColumn = min(max(width * 0.14, 130), 176)
            let columns = max(1, Int((width + gap) / (minColumn + gap)))
            let column = (width - gap * CGFloat(columns - 1)) / CGFloat(columns)
            self.columns = columns
            var shelfItems = Array(shelf)[...]
            for (n, group) in self.groups.enumerated() {
                if let heading = group.heading {
                    if n > 0 { y += 4 }
                    let h = ceil(heading.cell!.cellSize(forBounds: CGRect(x: 0, y: 0, width: width, height: 1e4)).height)
                    heading.frame = CGRect(x: x, y: y, width: width, height: h)
                    y += h + 16
                }
                let groupItems = Array(shelfItems.prefix(group.count))
                shelfItems = shelfItems.dropFirst(group.count)
                for start in stride(from: 0, to: groupItems.count, by: columns) {
                    let row = groupItems[start..<min(start + columns, groupItems.count)]
                    let heights = row.map { $0.layout(width: column) }
                    let rowHeight = heights.max() ?? 0
                    for (i, item) in row.enumerated() {
                        item.frame = CGRect(x: x + CGFloat(i) * (column + gap), y: y, width: column, height: rowHeight)
                        item.layout(width: column)
                    }
                    y += rowHeight + 36
                }
            }
            return y + 64 - 36
        }
        select(min(selected, max(items.count - 1, 0)))
        relayout()
    }

    private func item(_ book: Int, labels: [NSTextField], card: Bool) -> LibraryItem {
        let item = LibraryItem(index: book, cover: coverView(book), labels: labels, card: card, progress: books[book].progress, palette: palette)
        item.onOpen = { [weak self] in self?.openBook(book) }
        item.onMore = { [weak self, weak item] in if let item { self?.menu(book, at: item) } }
        return item
    }

    /// "Series · 2".
    private func seriesLabel(_ b: LibraryBook) -> String {
        guard b.number >= 0 else { return b.series }
        return b.series + " · " + (b.number.rounded() == b.number ? String(Int(b.number)) : String(b.number))
    }

    /// The books in the chosen view's groups (`forzac_library_arrange`).
    private func arrange() -> [(heading: String?, books: [Int])] {
        let strings = books.map { (strdup($0.title), strdup($0.authors), strdup($0.series), strdup($0.shelves.joined(separator: "\n"))) }
        defer { strings.forEach { free($0.0); free($0.1); free($0.2); free($0.3) } }
        let keys = zip(books, strings).map {
            ForzacBookKey(title: $1.0, authors: $1.1, series: $1.2, number: $0.number, progress: $0.progress, opened: $0.opened, shelves: $1.3)
        }
        guard let handle = forzac_library_arrange(keys, UInt32(keys.count), view) else { return [] }
        defer { forzac_groups_free(handle) }
        let g = forzac_groups(handle)!.pointee
        return (0..<Int(g.count)).map { i in
            let (start, end) = (Int(g.starts[i]), Int(g.starts[i + 1]))
            return (g.headings[i].map { String(cString: $0) }, (start..<end).map { Int(g.books[$0]) })
        }
    }

    /// Shows view `v` and remembers it.
    private func setView(_ v: UInt32) {
        view = v
        _ = forzac_set_library_view(state, v)
        selected = 0
        rebuild()
        content.scroll(.zero)
    }

    /// The book's menu at its "⋯": "About the book", "Remove from library".
    private func menu(_ book: Int, at item: LibraryItem) {
        let menu = NSMenu()
        let about = NSMenuItem(title: tr("About the book"), action: #selector(menuAbout(_:)), keyEquivalent: "")
        let edit = NSMenuItem(title: tr("Edit…"), action: #selector(menuEdit(_:)), keyEquivalent: "")
        // "Add to shelf": every shelf, checked when the book is on it, then "New shelf…".
        let shelves = NSMenu()
        let names = String(cString: forzac_shelf_names(state)).split(separator: "\n").map(String.init)
        for name in names {
            let on = books[book].shelves.contains { $0.lowercased() == name.lowercased() }
            let i = NSMenuItem(title: name, action: #selector(menuShelf(_:)), keyEquivalent: "")
            i.state = on ? .on : .off
            i.representedObject = (book, name, !on)
            i.target = self
            shelves.addItem(i)
        }
        if !names.isEmpty { shelves.addItem(.separator()) }
        let new = NSMenuItem(title: tr("New shelf…"), action: #selector(menuNewShelf(_:)), keyEquivalent: "")
        new.target = self
        new.tag = book
        shelves.addItem(new)
        let shelf = NSMenuItem(title: tr("Add to shelf"), action: nil, keyEquivalent: "")
        shelf.submenu = shelves
        let kindle = NSMenuItem(title: tr("Send to Kindle"), action: #selector(menuKindle(_:)), keyEquivalent: "")
        let remove = NSMenuItem(title: tr("Remove from library"), action: #selector(menuRemove(_:)), keyEquivalent: "")
        for i in [about, edit, shelf, kindle, remove] {
            i.target = i === shelf ? nil : self
            i.tag = book
            menu.addItem(i)
        }
        menu.popUp(positioning: nil, at: CGPoint(x: item.more.frame.minX, y: item.more.frame.maxY), in: item)
    }

    @objc private func menuAbout(_ sender: NSMenuItem) { showAbout(sender.tag) }
    @objc private func menuRemove(_ sender: NSMenuItem) { ask(sender.tag) }

    @objc private func menuKindle(_ sender: NSMenuItem) {
        guard sender.tag < books.count else { return }
        onSendToKindle?(books[sender.tag].path, books[sender.tag].title)
    }

    /// "Edit…": the file read on a background queue, then the form in an alert; the file is never changed.
    @objc private func menuEdit(_ sender: NSMenuItem) {
        let book = sender.tag
        guard book < books.count else { return }
        let path = books[book].path
        DispatchQueue.global(qos: .userInitiated).async { [weak self] in
            guard let doc = forzac_open(path, nil) else { return }
            DispatchQueue.main.async {
                guard let self else { forzac_close(doc); return }
                self.showEdit(book, doc)
            }
        }
    }

    /// The form for `doc` (`forzac_edit_fields`): Title, Authors, Series, Number, Genres; "Save", "Cancel", "From the
    /// book". Closes `doc` when answered.
    private func showEdit(_ book: Int, _ doc: OpaquePointer) {
        let handle = forzac_edit_fields(state, doc)!
        let f = forzac_fields_view(handle)!.pointee
        let values = [f.title, f.authors, f.series, f.number, f.genres].map { String(cString: $0!) }
        forzac_fields_free(handle)
        let names = [tr("Title"), tr("Authors"), tr("Series"), tr("Number"), tr("Genres")]
        let fields = values.map { value -> NSTextField in
            let field = NSTextField(string: value)
            field.widthAnchor.constraint(equalToConstant: 280).isActive = true
            return field
        }
        let grid = NSGridView(views: zip(names, fields).map { [NSTextField(labelWithString: $0), $1] })
        grid.rowSpacing = 8
        grid.column(at: 0).xPlacement = .trailing
        grid.frame.size = grid.fittingSize
        let alert = NSAlert()
        alert.messageText = tr("Edit the book")
        alert.accessoryView = grid
        alert.addButton(withTitle: tr("Save"))
        alert.addButton(withTitle: tr("Cancel"))
        alert.addButton(withTitle: tr("From the book"))
        alert.window.initialFirstResponder = fields[0]
        let id = books[book].id
        alert.beginSheetModal(for: window) { [weak self] response in
            defer { forzac_close(doc) }
            guard let self else { return }
            switch response {
            case .alertFirstButtonReturn:
                let c = fields.map { strdup($0.stringValue) }
                defer { c.forEach { free($0) } }
                _ = forzac_save_edits(self.state, doc, ForzacFields(title: c[0], authors: c[1], series: c[2], number: c[3], genres: c[4]))
            case .alertThirdButtonReturn:
                _ = forzac_reset_edits(self.state, id)
            default:
                return
            }
            // The library's book as it now reads.
            let shown = forzac_edit_fields(self.state, doc)!
            let v = forzac_fields_view(shown)!.pointee
            let (title, authors, series, number) = (String(cString: v.title), String(cString: v.authors), String(cString: v.series), String(cString: v.number))
            forzac_fields_free(shown)
            for i in self.books.indices where self.books[i].id == id {
                let b = self.books[i]
                var edited = LibraryBook(id: b.id, path: b.path, title: title, authors: authors, progress: b.progress, opened: b.opened)
                edited.series = series
                edited.number = Float(number) ?? -1
                edited.shelves = b.shelves
                self.books[i] = edited
            }
            self.rebuild()
        }
    }

    @objc private func menuShelf(_ sender: NSMenuItem) {
        guard let (book, name, on) = sender.representedObject as? (Int, String, Bool) else { return }
        setShelf(book, name, on)
    }

    /// "New shelf": the name asked in an alert; the book goes on that shelf.
    @objc private func menuNewShelf(_ sender: NSMenuItem) {
        let alert = NSAlert()
        alert.messageText = tr("New shelf")
        let field = NSTextField(frame: NSRect(x: 0, y: 0, width: 260, height: 24))
        field.placeholderString = tr("Shelf name")
        alert.accessoryView = field
        alert.addButton(withTitle: tr("Add"))
        alert.addButton(withTitle: tr("Cancel"))
        alert.window.initialFirstResponder = field
        let book = sender.tag
        alert.beginSheetModal(for: window) { [weak self] response in
            if response == .alertFirstButtonReturn { self?.setShelf(book, field.stringValue, true) }
        }
    }

    /// Puts `book` on the shelf `name` or takes it off (`forzac_set_shelf`), and shows it.
    private func setShelf(_ book: Int, _ name: String, _ on: Bool) {
        let name = name.split(whereSeparator: \.isWhitespace).joined(separator: " ")
        guard book < books.count, !name.isEmpty, forzac_set_shelf(state, books[book].id, name, on) else { return }
        let existing = String(cString: forzac_shelf_names(state)).split(separator: "\n").map(String.init)
        let spelt = existing.first { $0.lowercased() == name.lowercased() } ?? name
        let id = books[book].id
        for i in books.indices where books[i].id == id {
            books[i].shelves.removeAll { $0.lowercased() == name.lowercased() }
            if on { books[i].shelves.append(spelt) }
        }
        if view == 4 { rebuild() }
    }

    /// "About the book": the file read on a background queue, then a sheet over the dimmed library.
    private func showAbout(_ book: Int) {
        guard about == nil, asking == nil, book < books.count else { return }
        let path = books[book].path
        DispatchQueue.global(qos: .userInitiated).async { [weak self] in
            guard let doc = forzac_open(path, nil) else { return }
            DispatchQueue.main.async {
                defer { forzac_close(doc) }
                guard let self, self.about == nil, self.asking == nil else { return }
                // The reader's edits come from the main handle, so on the main queue.
                let handle = forzac_book_info(self.state, doc)!
                let i = forzac_book_info_view(handle)!.pointee
                let info = [i.title, i.authors, i.series, i.details, i.annotation].map { String(cString: $0!) }
                forzac_book_info_free(handle)
                let sheet = AboutView(palette: self.palette, cover: self.coverView(book), info: info)
                sheet.onAnswer = { [weak self] read in self?.closeAbout(read) }
                sheet.frame = self.container.bounds
                sheet.autoresizingMask = [.width, .height]
                self.container.addSubview(sheet)
                self.about = (sheet, book)
            }
        }
    }

    private func closeAbout(_ read: Bool) {
        guard let (view, book) = about else { return }
        view.removeFromSuperview()
        about = nil
        if read { openBook(book) }
    }

    private func share(_ b: LibraryBook) -> String {
        !b.opened ? tr("New") : b.progress >= 0.999 ? tr("Finished") : "\(max(Int(b.progress * 100), 1))%"
    }

    /// The web's empty library: the invitation, the button, the hint, the Books folder and the formats beside the
    /// sample page.
    private func buildEmpty() {
        let heading = wrapping(tr("Read books right on your computer"), bookFont("Literata", 44, bold: true), palette.ink)
        let lead = wrapping(tr("Open a book file — a copy goes into your Books folder, the original stays where it is."), bookFont("Literata", 19), palette.ink)
        let button = PillButton(tr("Open a book"), big: true, palette: palette)
        button.onClick = { [weak self] in self?.chooseFile() }
        let hint = label(tr("or drop a file into this window"), interfaceFont(15), palette.muted)
        let show = ClickLabel()
        show.attributedStringValue = NSAttributedString(string: tr("Show the Books folder"), attributes: [.font: interfaceFont(15), .foregroundColor: nsColor(palette.muted),
                                                                                                   .underlineStyle: NSUnderlineStyle.single.rawValue])
        show.sizeToFit()
        show.onClick = { [weak self] in self?.showFolder() }
        show.isHidden = folder == nil
        let rule = LayerView()
        rule.layer!.backgroundColor = color(palette.line)
        let formats = wrapping(tr("FB2, FB2.ZIP, FB3 and EPUB are supported."), interfaceFont(14), palette.muted)
        let page = SamplePageView(palette: palette)
        body = [heading, lead, button, hint, show, rule, formats, page]
        body.forEach { $0.translatesAutoresizingMaskIntoConstraints = true; content.addSubview($0) }
        bodyLayout = { [weak self] x, width in
            guard let self else { return 0 }
            let wide = width >= 860
            let gap = min(max(width * 0.07, 40), 112)
            let textWidth = wide ? width - gap - page.frame.width : width
            func height(_ l: NSTextField, _ w: CGFloat) -> CGFloat { ceil(l.cell!.cellSize(forBounds: CGRect(x: 0, y: 0, width: w, height: 1e4)).height) }
            let headingWidth = min(textWidth, 44 * 14 * 0.6 * 2)
            let parts: [CGFloat] = [height(heading, headingWidth), 24, height(lead, min(textWidth, 560)), 36, button.frame.height, 14, hint.frame.height + 4,
                                    show.isHidden ? 0 : 6 + show.frame.height, 42, 1 + 18, height(formats, min(textWidth, 480))]
            let textHeight = parts.reduce(0, +)
            let top: CGFloat = 20 + 48 + 32
            let available = max(self.scroll.contentSize.height - top - 32 - 64, 0)
            var y = top + (wide ? max((available - max(textHeight, page.frame.height)) / 2, 0) + max((page.frame.height - textHeight) / 2, 0) : 0)
            heading.frame = CGRect(x: x, y: y, width: headingWidth, height: parts[0])
            y += parts[0] + 24
            lead.frame = CGRect(x: x, y: y, width: min(textWidth, 560), height: parts[2])
            y += parts[2] + 36
            button.frame.origin = CGPoint(x: x, y: y)
            y += button.frame.height + 14
            hint.sizeToFit()
            hint.frame.origin = CGPoint(x: x, y: y)
            y += hint.frame.height + 4
            show.frame.origin = CGPoint(x: x, y: y + 6)
            y += parts[7] + 42
            rule.frame = CGRect(x: x, y: y, width: textWidth, height: 1)
            y += 19
            formats.frame = CGRect(x: x, y: y, width: min(textWidth, 480), height: parts[10])
            y += parts[10]
            if wide {
                page.frame.origin = CGPoint(x: x + width - page.frame.width, y: heading.frame.minY + (textHeight - page.frame.height) / 2)
                return max(y, page.frame.maxY) + 32 + 64
            }
            page.frame.origin = CGPoint(x: x + (width - page.frame.width) / 2, y: y + 40)
            return page.frame.maxY + 32 + 64
        }
    }

    /// Places everything for the window's width: the column at most 1120 wide with the web's side padding.
    private func relayout() {
        scroll.frame = container.bounds
        let width = scroll.contentSize.width
        let pad = min(max(width * 0.04, 20), 48)
        let inner = min(width - 2 * pad, 1120)
        let x = (width - inner) / 2
        // The head: the brand or the search field on the left, the buttons on the right.
        let headY: CGFloat = 20
        brandIcon.frame = CGRect(x: x, y: headY + 6, width: 36, height: 36)
        brandName.sizeToFit()
        brandName.frame.origin = CGPoint(x: x + 48, y: headY + (48 - brandName.frame.height) / 2)
        var right = x + inner
        if !openButton.isHidden {
            openButton.frame.origin = CGPoint(x: right - openButton.frame.width, y: headY + (48 - openButton.frame.height) / 2)
            right -= openButton.frame.width + 10
        }
        for b in [searchButton, closeButton, headMenuButton] where !b.isHidden {
            b.frame = CGRect(x: right - 44, y: headY + 2, width: 44, height: 44)
            right -= 54
        }
        if !progressText.isHidden {
            progressText.sizeToFit()
            progressText.frame.origin = CGPoint(x: right - progressText.frame.width, y: headY + (48 - progressText.frame.height) / 2)
            right -= progressText.frame.width + 16
        }
        searchField.frame = CGRect(x: x, y: headY + 5, width: max(right - x, 100), height: 38)
        // The update line next to the buttons, unless it would reach the brand.
        let fits = hasUpdate && !searching && right - 4 - updateClose.frame.width - 6 - updateText.frame.width >= brandName.frame.maxX + 24
        updateText.isHidden = !fits
        updateClose.isHidden = !fits || Installer.shared.phase == .installing
        if fits {
            updateClose.frame.origin = CGPoint(x: right - 4 - updateClose.frame.width, y: headY + (48 - updateClose.frame.height) / 2)
            updateText.frame.origin = CGPoint(x: updateClose.frame.minX - 6 - updateText.frame.width, y: headY + (48 - updateText.frame.height) / 2)
        }
        // The views' tabs under the head, the chosen one in ink and underlined.
        var tx = x
        for (i, tab) in tabs.enumerated() {
            tab.textColor = nsColor(UInt32(i) == view ? palette.ink : palette.muted)
            tab.sizeToFit()
            tab.frame.origin = CGPoint(x: tx, y: headY + 48 + 18)
            tx += tab.frame.width + 24
        }
        tabRule.frame = CGRect(x: x, y: headY + 48 + 18 + (tabs.first?.frame.height ?? 20) + 8, width: inner, height: 1)
        let height = bodyLayout?(x, inner) ?? 0
        content.frame = CGRect(x: 0, y: 0, width: width, height: max(height, scroll.contentSize.height))
        asking?.view.frame = container.bounds
        asking?.view.needsLayout = true
    }

    // MARK: Actions

    private func select(_ index: Int) {
        for (i, item) in items.enumerated() {
            item.cover.selected = i == index
            item.isSelected = i == index
        }
        selected = index
        guard index < items.count else { return }
        let f = items[index].frame.insetBy(dx: 0, dy: -24)
        content.scrollToVisible(f)
    }

    /// Moves the selection: `dx` along the books, `dy` rows through the card and the groups (`forzac_library_step`).
    private func step(dx: Int = 0, dy: Int = 0) {
        var sizes: [UInt32] = hasCard ? [1, 1] : []
        for g in groups { sizes += [UInt32(g.count), UInt32(columns)] }
        select(Int(forzac_library_step(sizes, UInt32(sizes.count / 2), UInt32(selected), Int32(dx), Int32(dy))))
    }

    private func openBook(_ book: Int) {
        guard book < books.count else { return }
        open(books[book].path)
    }

    private func chooseFile() { (NSApp.delegate as? AppDelegate)?.openDocument(nil) }

    /// The head's menu: "Show the Books folder", "Change the Books folder…", "Add a Folder…", then "Settings…".
    private func headMenu() {
        let menu = NSMenu()
        for (title, action) in [(tr("Show the Books folder"), #selector(menuShowFolder(_:))), (tr("Change the Books folder…"), #selector(menuChangeFolder(_:))),
                                (tr("Add a Folder…"), #selector(menuAddFolder(_:))), ("", nil), (tr("Settings…"), #selector(menuSettings(_:)))] {
            guard let action else {
                menu.addItem(.separator())
                continue
            }
            let item = NSMenuItem(title: title, action: action, keyEquivalent: "")
            item.target = self
            menu.addItem(item)
        }
        menu.popUp(positioning: nil, at: CGPoint(x: 0, y: headMenuButton.bounds.maxY + 4), in: headMenuButton)
    }

    @objc private func menuShowFolder(_ sender: Any?) { showFolder() }
    @objc private func menuChangeFolder(_ sender: Any?) { chooseBooksFolder() }
    @objc private func menuAddFolder(_ sender: Any?) { (NSApp.delegate as? AppDelegate)?.addFolder(nil) }
    @objc private func menuSettings(_ sender: Any?) { (NSApp.delegate as? AppDelegate)?.openSettings() }

    /// "Change the Books folder…": the folder panel, then "Move the books…?" (Move, Leave them, Cancel).
    func chooseBooksFolder() {
        let panel = NSOpenPanel()
        panel.canChooseFiles = false
        panel.canChooseDirectories = true
        panel.canCreateDirectories = true
        panel.message = tr("Change the Books folder…")
        guard panel.runModal() == .OK, let url = panel.url else { return }
        let new = url.path
        if let folder, URL(fileURLWithPath: folder).standardizedFileURL == url.standardizedFileURL { return }
        let alert = NSAlert()
        alert.messageText = fill(tr("Move the books to “{folder}”?"), ["folder": url.lastPathComponent])
        alert.informativeText = fill(tr("The books in “{folder}” go to the new folder; where you stopped in each stays."),
                                     ["folder": URL(fileURLWithPath: folder ?? "").lastPathComponent])
        alert.addButton(withTitle: tr("Move"))
        alert.addButton(withTitle: tr("Leave them"))
        alert.addButton(withTitle: tr("Cancel"))
        alert.beginSheetModal(for: window) { [weak self] response in
            switch response {
            case .alertFirstButtonReturn: self?.onChangeFolder?(new, true)
            case .alertSecondButtonReturn: self?.onChangeFolder?(new, false)
            default: break
            }
        }
    }

    /// The Books folder changed: the library shows `books` from `folder`.
    func folderChanged(_ folder: String?, books: [LibraryBook]) {
        self.folder = folder
        self.books = books
        selected = 0
        loadCovers()
        rebuild()
    }

    // MARK: Adding a folder (folder-import)

    func addingStarted() {
        adding += 1
        progressText.stringValue = ""
        rebuild()
    }

    func addingProgress(done: Int, found: Int) {
        progressText.stringValue = fill(tr("Adding books… {n} of {m}"), ["n": "\(done)", "m": "\(found)"])
        relayout()
    }

    /// One folder is added: the library shows `books` (the folder read again), the first selected.
    func addingEnded(books: [LibraryBook]) {
        adding -= 1
        self.books = books
        selected = 0
        loadCovers()
        rebuild()
    }

    /// Opens the Books folder in Finder.
    private func showFolder() {
        if let folder { NSWorkspace.shared.open(URL(fileURLWithPath: folder, isDirectory: true)) }
    }

    /// Shows the update line for the version the core offers, or hides it.
    func showUpdate() {
        updateNotice()
        relayout()
    }

    private func updateNotice() {
        guard let version = forzac_update_notice(state) else { hasUpdate = false; return }
        let muted: [NSAttributedString.Key: Any] = [.font: interfaceFont(13), .foregroundColor: nsColor(palette.muted)]
        let text: NSMutableAttributedString
        if Installer.shared.phase == .installing {
            text = NSMutableAttributedString(string: fill(tr("Installing version {version}…"), ["version": String(cString: version)]), attributes: muted)
        } else {
            text = NSMutableAttributedString(string: fill(tr("Version {version} is out"), ["version": String(cString: version)]) + " — ", attributes: muted)
            let link = Installer.shared.phase == .offered ? tr("install") : tr("download")
            text.append(NSAttributedString(string: link, attributes: [.font: interfaceFont(13), .foregroundColor: nsColor(palette.link)]))
        }
        updateText.attributedStringValue = text
        updateText.sizeToFit()
        hasUpdate = true
    }

    private func setSearching(_ on: Bool) {
        guard searching != on else { return }
        searching = on
        searchField.stringValue = ""
        selected = 0
        rebuild()
        if on { window.makeFirstResponder(searchField) } else { window.makeFirstResponder(nil) }
    }

    func controlTextDidChange(_ obj: Notification) {
        selected = 0
        rebuild()
    }

    /// Android's question before removing a book, as a sheet at the window's bottom.
    private func ask(_ book: Int) {
        guard asking == nil, book < books.count else { return }
        let b = books[book]
        let backdrop = QuestionView(palette: palette, title: fill(tr("Remove “{title}” from the library?"), ["title": b.title]), body: tr("The book file goes to the Trash."))
        backdrop.onAnswer = { [weak self] remove in self?.answer(remove) }
        backdrop.frame = container.bounds
        backdrop.autoresizingMask = [.width, .height]
        container.addSubview(backdrop)
        asking = (backdrop, book)
    }

    private func answer(_ remove: Bool) {
        guard let (view, book) = asking else { return }
        view.removeFromSuperview()
        asking = nil
        guard remove else { return }
        let b = books[book]
        // The file goes to the Trash first; the book stays when it cannot. A file outside the folder (one the move
        // could not copy) is left alone.
        if let folder, b.path.hasPrefix(folder + "/"), FileManager.default.fileExists(atPath: b.path) {
            do {
                try FileManager.default.trashItem(at: URL(fileURLWithPath: b.path), resultingItemURL: nil)
            } catch {
                fputs("cannot move \(b.path) to the Trash: \(error)\n", stderr)
                let alert = NSAlert()
                alert.messageText = fill(tr("Couldn't move “{title}” to the Trash"), ["title": b.title])
                alert.informativeText = error.localizedDescription
                alert.beginSheetModal(for: window)
                return
            }
        }
        let id = b.id
        guard forzac_state_forget(state, id) else { fputs("cannot save the reading state\n", stderr); return }
        try? FileManager.default.removeItem(at: thumbnailURL(id))
        books.remove(at: book)
        thumbs[id] = nil
        rebuild()
    }

    /// The library's keys, by key code so a Russian layout works: arrows and h j k l move, Return opens, Delete or d
    /// asks to remove, / searches, F shows the Books folder, q and Esc close (Esc first leaves the search or the
    /// question). True when handled.
    private func key(_ e: NSEvent) -> Bool {
        guard e.modifierFlags.intersection([.command, .control, .option]).isEmpty else { return false }
        let typing = window.firstResponder is NSTextView && searching
        let code = e.keyCode == 0x4C ? 0x24 : e.keyCode
        if about != nil {
            switch code {
            case 0x24: closeAbout(true)
            case 0x35: closeAbout(false)
            default: break
            }
            return true
        }
        if asking != nil {
            switch code {
            case 0x24: answer(true)
            case 0x35: answer(false)
            default: break
            }
            return true
        }
        switch (code, typing) {
        case (0x7B, false): step(dx: -1)
        case (0x7C, false): step(dx: 1)
        case (0x7D, _): step(dy: 1)
        case (0x7E, _): step(dy: -1)
        case (0x73, false): select(0)
        case (0x77, false): select(max(items.count - 1, 0))
        case (0x24, _): if selected < items.count { openBook(items[selected].index) }
        case (0x33, false), (0x75, false): if selected < items.count { ask(items[selected].index) }
        case (0x35, _): if searching { setSearching(false) } else { window.close() }
        case (0x30, false): if !books.isEmpty { setView((view + (e.modifierFlags.contains(.shift) ? 4 : 1)) % 5) }
        default:
            guard !typing else { return false }
            switch e.shortcutKey {
            case 0x04: step(dx: -1)
            case 0x25: step(dx: 1)
            case 0x26: step(dy: 1)
            case 0x28: step(dy: -1)
            case 0x02: if selected < items.count { ask(items[selected].index) }
            case 0x22: if selected < items.count { showAbout(items[selected].index) }
            case 0x2C: if !books.isEmpty && e.characters != "?" { setSearching(true) } else { return false }
            case 0x03 where e.modifierFlags.contains(.shift): showFolder()
            case 0x0C: window.close()
            default: return false
            }
        }
        return true
    }

    /// A key press as the keyboard would send it (`--keys`): letters, \r Return, \e Esc.
    func send(_ key: Character) {
        let codes: [Character: UInt16] = ["\r": 0x24, "\u{1b}": 0x35, "\u{7f}": 0x33]
        let text = String(key)
        guard let e = NSEvent.keyEvent(with: .keyDown, location: .zero, modifierFlags: [], timestamp: 0, windowNumber: window.windowNumber, context: nil,
                                       characters: text, charactersIgnoringModifiers: text, isARepeat: false, keyCode: codes[key] ?? 0xFFFF) else { return }
        if !self.key(e) { window.sendEvent(e) }
    }

    /// The window as drawn (`--png`).
    func savePNG(_ path: String) {
        guard let view = window.contentView, let rep = view.bitmapImageRepForCachingDisplay(in: view.bounds) else { return }
        view.cacheDisplay(in: view.bounds, to: rep)
        try? rep.representation(using: .png, properties: [:])?.write(to: URL(fileURLWithPath: path))
        print("saved \(path)")
    }
}

/// A view with y down.
final class FlippedView: NSView {
    override var isFlipped: Bool { true }
}

/// The window's content: tells the library to lay out again when it resizes, and opens files dropped on it with the
/// web's dashed frame "Release the file to open it" while they are over it.
final class DropView: NSView {
    var onResize: (() -> Void)?
    var onDrop: (([URL]) -> Void)?
    private let frameView = LayerView()

    init(palette: Palette) {
        super.init(frame: .zero)
        wantsLayer = true
        layer!.backgroundColor = color(palette.paper)
        registerForDraggedTypes([.fileURL])
        frameView.translatesAutoresizingMaskIntoConstraints = true
        frameView.layer!.cornerRadius = 20
        frameView.layer!.backgroundColor = color(palette.paper, alpha: 0.88)
        let border = CAShapeLayer()
        border.strokeColor = color(palette.muted)
        border.fillColor = nil
        border.lineWidth = 2
        border.lineDashPattern = [6, 4]
        frameView.layer!.addSublayer(border)
        let text = label(tr("Release the file to open it"), interfaceFont(20, .medium), palette.ink)
        text.translatesAutoresizingMaskIntoConstraints = false
        frameView.addSubview(text)
        NSLayoutConstraint.activate([text.centerXAnchor.constraint(equalTo: frameView.centerXAnchor), text.centerYAnchor.constraint(equalTo: frameView.centerYAnchor)])
        frameView.isHidden = true
        addSubview(frameView)
    }

    required init?(coder: NSCoder) { fatalError() }

    override func resizeSubviews(withOldSize oldSize: NSSize) {
        super.resizeSubviews(withOldSize: oldSize)
        onResize?()
        frameView.frame = bounds.insetBy(dx: 16, dy: 16)
        if let border = frameView.layer?.sublayers?.first as? CAShapeLayer {
            border.path = CGPath(roundedRect: frameView.bounds.insetBy(dx: 1, dy: 1), cornerWidth: 19, cornerHeight: 19, transform: nil)
        }
    }

    override func draggingEntered(_ sender: NSDraggingInfo) -> NSDragOperation {
        frameView.frame = bounds.insetBy(dx: 16, dy: 16)
        (frameView.layer?.sublayers?.first as? CAShapeLayer)?.path = CGPath(roundedRect: frameView.bounds.insetBy(dx: 1, dy: 1), cornerWidth: 19, cornerHeight: 19, transform: nil)
        addSubview(frameView) // on top of the library
        frameView.isHidden = false
        return .copy
    }

    override func draggingExited(_ sender: NSDraggingInfo?) { frameView.isHidden = true }

    override func performDragOperation(_ sender: NSDraggingInfo) -> Bool {
        frameView.isHidden = true
        let urls = sender.draggingPasteboard.readObjects(forClasses: [NSURL.self], options: [.urlReadingFileURLsOnly: true]) as? [URL] ?? []
        onDrop?(urls)
        return !urls.isEmpty
    }
}

/// "About the book": a dimmed window with a sheet in the middle — the cover, the title, the authors, the series, the
/// details and the annotation, "Close" and "Read". A click beside the sheet closes it.
final class AboutView: NSView {
    var onAnswer: ((Bool) -> Void)?
    private let sheet = LayerView()
    private let cover: CoverView
    private let texts: [NSTextField]
    private let annotation = NSScrollView()
    private let annotationText: NSTextField
    private let buttons: [PillButton]

    /// `info`: title, authors, series, details, annotation; empty ones are left out.
    init(palette: Palette, cover: CoverView, info: [String]) {
        self.cover = cover
        let styles: [(NSFont, UInt32)] = [(bookFont("Literata", 22, bold: true), palette.ink), (interfaceFont(15), palette.ink),
                                          (interfaceFont(14), palette.muted), (interfaceFont(14), palette.muted)]
        texts = zip(info.prefix(4), styles).filter { !$0.0.isEmpty }.map { wrapping($0.0, $0.1.0, $0.1.1) }
        annotationText = wrapping(info[4], bookFont("Literata", 15), palette.ink)
        let close = PillButton(tr("Close"), big: false, palette: palette)
        let read = PillButton(tr("Read"), big: false, palette: palette)
        close.layer!.backgroundColor = nil
        close.layer!.borderWidth = 1
        close.layer!.borderColor = color(palette.line)
        close.title.textColor = nsColor(palette.ink)
        read.title.textColor = nsColor(palette.sheet)
        buttons = [close, read]
        super.init(frame: .zero)
        wantsLayer = true
        layer!.backgroundColor = color(0x000000, alpha: 0.2)
        sheet.translatesAutoresizingMaskIntoConstraints = true
        sheet.layer!.backgroundColor = color(palette.sheet)
        sheet.layer!.cornerRadius = 22
        addSubview(sheet)
        close.onClick = { [weak self] in self?.onAnswer?(false) }
        read.onClick = { [weak self] in self?.onAnswer?(true) }
        let document = FlippedView()
        document.addSubview(annotationText)
        annotationText.translatesAutoresizingMaskIntoConstraints = true
        annotation.documentView = document
        annotation.drawsBackground = false
        annotation.hasVerticalScroller = true
        annotation.autohidesScrollers = true
        annotation.isHidden = info[4].isEmpty
        for v in [cover as NSView, annotation] + texts + buttons as [NSView] {
            v.translatesAutoresizingMaskIntoConstraints = true
            sheet.addSubview(v)
        }
    }

    required init?(coder: NSCoder) { fatalError() }

    override var isFlipped: Bool { true }

    override func layout() {
        super.layout()
        let width = min(bounds.width - 48, 600)
        let inner = width - 56
        let m = CoverView.margin
        cover.frame = CGRect(x: 28 - m, y: 28 - m, width: 100 + 2 * m, height: 150 + 2 * m)
        let textX: CGFloat = 28 + 100 + 20
        let textWidth = width - textX - 28
        let heights = texts.map { ceil($0.cell!.cellSize(forBounds: CGRect(x: 0, y: 0, width: textWidth, height: 1e4)).height) }
        let textHeight: CGFloat = heights.reduce(0, +) + CGFloat(max(texts.count - 1, 0)) * 6
        var y: CGFloat = 28 + max(0, (150 - textHeight) / 2)
        for (t, h) in zip(texts, heights) {
            t.frame = CGRect(x: textX, y: y, width: textWidth, height: h)
            y += h + 6
        }
        y = max(28 + 150, y - 6) + 20
        if !annotation.isHidden {
            let h = ceil(annotationText.cell!.cellSize(forBounds: CGRect(x: 0, y: 0, width: inner - 16, height: 1e4)).height)
            annotationText.frame = CGRect(x: 0, y: 0, width: inner - 16, height: h)
            annotation.documentView!.frame = CGRect(x: 0, y: 0, width: inner, height: h)
            let shown = min(h, 260, max(bounds.height - y - 48 - 28 - 120, 60))
            annotation.frame = CGRect(x: 28, y: y, width: inner, height: shown)
            y += shown + 20
        }
        let half = (inner - 12) / 2
        for (i, button) in buttons.enumerated() {
            button.frame = CGRect(x: 28 + CGFloat(i) * (half + 12), y: y, width: half, height: 48)
            button.layer!.cornerRadius = 24
            button.title.sizeToFit()
            button.title.frame.origin = CGPoint(x: (half - button.title.frame.width) / 2, y: (48 - button.title.frame.height) / 2)
        }
        let height = y + 48 + 28
        sheet.frame = CGRect(x: (bounds.width - width) / 2, y: max((bounds.height - height) / 2, 16), width: width, height: height)
    }

    override func mouseDown(with e: NSEvent) {}

    override func mouseUp(with e: NSEvent) {
        if !sheet.frame.contains(convert(e.locationInWindow, from: nil)) { onAnswer?(false) }
    }
}

/// Android's `DeleteSheet`: a dimmed window with a sheet at the bottom, "Cancel" and "Remove". A click beside the
/// sheet cancels.
final class QuestionView: NSView {
    var onAnswer: ((Bool) -> Void)?
    private let sheet = LayerView()

    init(palette: Palette, title: String, body: String) {
        super.init(frame: .zero)
        wantsLayer = true
        layer!.backgroundColor = color(0x000000, alpha: 0.2)
        sheet.translatesAutoresizingMaskIntoConstraints = true
        sheet.layer!.backgroundColor = color(palette.sheet)
        sheet.layer!.cornerRadius = 22
        sheet.layer!.maskedCorners = [.layerMinXMinYCorner, .layerMaxXMinYCorner] // the top corners (the view is flipped)
        addSubview(sheet)
        let t = wrapping(title, bookFont("PT Serif", 19, bold: true), palette.ink)
        let b = label(body, interfaceFont(14), palette.muted)
        let cancel = PillButton(tr("Cancel"), big: false, palette: palette)
        let remove = PillButton(tr("Remove"), big: false, palette: palette)
        cancel.layer!.backgroundColor = nil
        cancel.layer!.borderWidth = 1
        cancel.layer!.borderColor = color(palette.line)
        cancel.title.textColor = nsColor(palette.ink)
        remove.title.textColor = nsColor(palette.sheet)
        cancel.onClick = { [weak self] in self?.onAnswer?(false) }
        remove.onClick = { [weak self] in self?.onAnswer?(true) }
        for v in [t, b, cancel, remove] as [NSView] {
            v.translatesAutoresizingMaskIntoConstraints = true
            sheet.addSubview(v)
        }
    }

    required init?(coder: NSCoder) { fatalError() }

    override var isFlipped: Bool { true }

    override func layout() {
        super.layout()
        let width = min(bounds.width, 560)
        let views = sheet.subviews
        guard views.count == 4, let t = views[0] as? NSTextField, let b = views[1] as? NSTextField else { return }
        let inner = width - 48
        let th = ceil(t.cell!.cellSize(forBounds: CGRect(x: 0, y: 0, width: inner, height: 1e4)).height)
        b.sizeToFit()
        t.frame = CGRect(x: 24, y: 24, width: inner, height: th)
        b.frame = CGRect(x: 24, y: 24 + th + 10, width: inner, height: b.frame.height)
        let y = b.frame.maxY + 18
        let half = (inner - 12) / 2
        for (i, button) in [views[2], views[3]].enumerated() {
            button.frame = CGRect(x: 24 + CGFloat(i) * (half + 12), y: y, width: half, height: 48)
            button.layer!.cornerRadius = 24
            if let title = (button as? PillButton)?.title {
                title.sizeToFit()
                title.frame.origin = CGPoint(x: (half - title.frame.width) / 2, y: (48 - title.frame.height) / 2)
            }
        }
        let height = y + 48 + 24
        sheet.frame = CGRect(x: (bounds.width - width) / 2, y: bounds.height - height, width: width, height: height)
    }

    override func mouseDown(with e: NSEvent) {}

    override func mouseUp(with e: NSEvent) {
        if !sheet.frame.contains(convert(e.locationInWindow, from: nil)) { onAnswer?(false) }
    }
}

/// A label that wraps, at most `lines` lines (0: any), the rest truncated.
/// A label that runs `onClick` when clicked, with the pointing hand over it.
final class ClickLabel: NSTextField {
    var onClick: (() -> Void)?

    convenience init() {
        self.init(labelWithString: "")
    }

    override func resetCursorRects() { addCursorRect(bounds, cursor: .pointingHand) }
    override func mouseDown(with e: NSEvent) {}
    override func mouseUp(with e: NSEvent) {
        if bounds.contains(convert(e.locationInWindow, from: nil)) { onClick?() }
    }
}

func wrapping(_ text: String, _ font: NSFont, _ rgb: UInt32, lines: Int = 0) -> NSTextField {
    let l = label(text, font, rgb)
    l.cell?.wraps = true
    l.cell?.truncatesLastVisibleLine = true
    l.lineBreakMode = .byWordWrapping
    l.maximumNumberOfLines = lines
    return l
}
