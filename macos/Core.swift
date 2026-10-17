// What the Mac shell's views share: the palette, fonts from the core's ids, drawing a typeset page, the icons.

import AppKit
import CoreText
import QuartzCore

/// A theme's colours (the Android app's `docs/design.md`): the page's paper, ink and link ink; the panels' secondary
/// text, lines and background.
struct Palette {
    let paper, ink, muted, line, sheet, link: UInt32
}

/// The six themes, in the core's order.
let themes: [Palette] = [
    Palette(paper: 0xFAFAFA, ink: 0x1C1C1C, muted: 0x6E6E6E, line: 0xE5E5E5, sheet: 0xFFFFFF, link: 0x94621F),
    Palette(paper: 0xF4ECD8, ink: 0x5B4636, muted: 0x7A6650, line: 0xE2D6BC, sheet: 0xFBF5E8, link: 0x8A5A1C),
    Palette(paper: 0x121212, ink: 0xCFCFCF, muted: 0x8E8E8E, line: 0x333333, sheet: 0x1E1E1E, link: 0xC49A52),
    Palette(paper: 0xFAF4ED, ink: 0x575279, muted: 0x6E6A86, line: 0xE6DDD6, sheet: 0xFFFAF3, link: 0x8F5E1D),
    Palette(paper: 0x1F1F28, ink: 0xDCD7BA, muted: 0x9A9585, line: 0x363646, sheet: 0x2A2A37, link: 0xDCA561),
    Palette(paper: 0x2D353B, ink: 0xD3C6AA, muted: 0x9DA9A0, line: 0x475258, sheet: 0x343F44, link: 0xDBBC7F),
]
let themeNames = ["Light", "Sepia", "Dark", "Dawn", "Wave", "Forest"]

/// The book families, in the core's order: their names and the family names Core Text knows them by.
let familyNames = ["PT Serif", "Literata", "Source Serif", "PT Sans"]
let familyFonts = ["PT Serif", "Literata", "Source Serif 4", "PT Sans"]

/// Opacity of the ink for secondary text (the page number) and for table rules by strength.
let secondaryAlpha: CGFloat = 0.6
let ruleAlpha: [CGFloat] = [0, 0.35, 0.6, 0.9]

func color(_ rgb: UInt32, alpha: CGFloat = 1) -> CGColor {
    CGColor(srgbRed: CGFloat((rgb >> 16) & 0xFF) / 255, green: CGFloat((rgb >> 8) & 0xFF) / 255, blue: CGFloat(rgb & 0xFF) / 255, alpha: alpha)
}

func nsColor(_ rgb: UInt32, alpha: CGFloat = 1) -> NSColor { NSColor(cgColor: color(rgb, alpha: alpha))! }

/// Where the bundled assets are: the app's resources, else the source tree (a development build).
let assetsDir: String = {
    if let resources = Bundle.main.resourcePath, FileManager.default.fileExists(atPath: resources + "/fonts") { return resources }
    return URL(fileURLWithPath: #filePath).deletingLastPathComponent().deletingLastPathComponent().appendingPathComponent("assets").path
}()

/// The interface font (Golos Text) and the book families, usable by name in the panels: registered for this process
/// when the first panel is built, so the start does not read them.
let registerInterfaceFonts: Void = {
    for family in ["golos-text", "pt-serif", "literata", "source-serif", "pt-sans", "pt-mono"] {
        let dir = URL(fileURLWithPath: assetsDir).appendingPathComponent("fonts/\(family)")
        let files = (try? FileManager.default.contentsOfDirectory(at: dir, includingPropertiesForKeys: nil)) ?? []
        for file in files where file.pathExtension == "ttf" { CTFontManagerRegisterFontsForURL(file as CFURL, .process, nil) }
    }
}()

/// Golos Text at `size` and `weight`; the system font until it is registered or when it is missing.
func interfaceFont(_ size: CGFloat, _ weight: NSFont.Weight = .regular) -> NSFont {
    _ = registerInterfaceFonts
    let descriptor = NSFontDescriptor(fontAttributes: [.family: "Golos Text", .traits: [NSFontDescriptor.TraitKey.weight: weight]])
    return NSFont(descriptor: descriptor, size: size) ?? .systemFont(ofSize: size, weight: weight)
}

/// A book family's regular face at `size`, by its family name (registered with the interface fonts).
func bookFont(_ family: String, _ size: CGFloat, bold: Bool = false) -> NSFont {
    _ = registerInterfaceFonts
    var traits: [NSFontDescriptor.TraitKey: Any] = [:]
    if bold { traits[.weight] = NSFont.Weight.bold }
    let descriptor = NSFontDescriptor(fontAttributes: [.family: family, .traits: traits])
    return NSFont(descriptor: descriptor, size: size) ?? .systemFont(ofSize: size)
}

/// The bundled PT Serif face, created from the very bytes the core shaped with: glyph ids match by construction.
func bundled(_ font: UInt16) -> CGFont {
    var len = 0
    let ptr = forzac_font_data(font, &len)!
    let data = Data(bytesNoCopy: UnsafeMutableRawPointer(mutating: ptr), count: len, deallocator: .none)
    return CGFont(CGDataProvider(data: data as CFData)!)!
}

/// The face at `index` of a font file (a collection holds several).
func fontDescriptor(path: String, index: Int) -> CTFontDescriptor? {
    let all = CTFontManagerCreateFontDescriptorsFromURL(URL(fileURLWithPath: path) as CFURL) as? [CTFontDescriptor] ?? []
    return index < all.count ? all[index] : all.first
}

/// The core's fallback provider: the font Core Text cascades to for the character, as a file and face index.
let fallbackBase = CTFontCreateWithGraphicsFont(bundled(0), 12, nil, nil)
let fallback: ForzacFallback = { ch, path, capacity, index in
    guard let scalar = Unicode.Scalar(ch) else { return false }
    let s = String(Character(scalar)) as CFString
    let font = CTFontCreateForString(fallbackBase, s, CFRange(location: 0, length: CFStringGetLength(s)))
    var chars = Array(String(Character(scalar)).utf16)
    var glyphs = [CGGlyph](repeating: 0, count: chars.count)
    guard CTFontGetGlyphsForCharacters(font, &chars, &glyphs, chars.count),
          let url = CTFontCopyAttribute(font, kCTFontURLAttribute) as? URL else { return false }
    let file = url.path
    let name = CTFontCopyPostScriptName(font) as String
    let faces = CTFontManagerCreateFontDescriptorsFromURL(url as CFURL) as? [CTFontDescriptor] ?? []
    let i = faces.firstIndex { (CTFontDescriptorCopyAttribute($0, kCTFontNameAttribute) as? String) == name } ?? 0
    let bytes = Array(file.utf8)
    guard bytes.count < capacity else { return false }
    for (k, b) in bytes.enumerated() { path![k] = CChar(bitPattern: b) }
    path![bytes.count] = 0
    index!.pointee = UInt32(i)
    return true
}

/// Fonts by the core's id: PT Serif from its bytes; other families and fallbacks from the files the core shaped with,
/// at the same variation-axis values.
final class Fonts {
    private var graphics: [UInt16: CGFont] = [:]
    private var descriptors: [UInt16: CTFontDescriptor] = [:]
    private var cache: [Int: CTFont] = [:]

    func font(_ id: UInt16, size: Float) -> CTFont? {
        let key = Int(id) << 24 | Int(size * 64)
        if let f = cache[key] { return f }
        var f: CTFont?
        if id < 4 {
            let g = graphics[id] ?? bundled(id)
            graphics[id] = g
            f = CTFontCreateWithGraphicsFont(g, CGFloat(size), nil, nil)
        } else if let d = descriptor(of: id) {
            f = CTFontCreateWithFontDescriptor(d, CGFloat(size), nil)
        }
        cache[key] = f
        return f
    }

    private func descriptor(of id: UInt16) -> CTFontDescriptor? {
        if let d = descriptors[id] { return d }
        var buf = [CChar](repeating: 0, count: 4096)
        var index: UInt32 = 0
        guard forzac_font_file(id, &buf, buf.count, &index), var d = fontDescriptor(path: String(cString: buf), index: Int(index)) else { return nil }
        var tags = [UInt32](repeating: 0, count: 8)
        var values = [Float](repeating: 0, count: 8)
        let n = Int(forzac_font_variations(id, &tags, &values, 8))
        for i in 0..<n { d = CTFontDescriptorCreateCopyWithVariation(d, NSNumber(value: tags[i]), CGFloat(values[i])) }
        descriptors[id] = d
        return d
    }
}

/// Draws a typeset page (or a slice of a column, or a note) into `ctx`, whose height is `h`: images, table rules and
/// glyph runs in `palette`. `pictures` caches decoded book images by the address of their bytes.
func drawPage(_ page: ForzacPage, in ctx: CGContext, height h: CGFloat, palette: Palette, fonts: Fonts, pictures: inout [UnsafeRawPointer: CGImage]) {
    ctx.setAllowsFontSubpixelPositioning(true)
    ctx.setShouldSubpixelPositionFonts(true)
    ctx.interpolationQuality = .high
    // Core Graphics is y up: flip the page's y-down rectangles.
    func flipped(_ r: ForzacRect) -> CGRect { CGRect(x: CGFloat(r.x), y: h - CGFloat(r.y + r.h), width: CGFloat(r.w), height: CGFloat(r.h)) }
    for i in 0..<Int(page.image_count) {
        let image = page.images[i]
        let key = UnsafeRawPointer(image.bytes!)
        if pictures[key] == nil {
            let data = Data(bytesNoCopy: UnsafeMutableRawPointer(mutating: key), count: image.len, deallocator: .none)
            if let source = CGImageSourceCreateWithData(data as CFData, nil) { pictures[key] = CGImageSourceCreateImageAtIndex(source, 0, nil) }
        }
        if let picture = pictures[key] { ctx.draw(picture, in: flipped(image.rect)) }
    }
    for i in 0..<Int(page.rule_count) {
        let rule = page.rules[i]
        ctx.setFillColor(color(palette.ink, alpha: ruleAlpha[Int(rule.strength)]))
        ctx.fill(flipped(rule.rect))
    }
    // Lines through struck-out text, in their run's ink.
    for i in 0..<Int(page.stroke_count) {
        let stroke = page.strokes[i]
        ctx.setFillColor(stroke.ink == 1 ? color(palette.link) : color(palette.ink, alpha: stroke.ink == 2 ? secondaryAlpha : 1))
        ctx.fill(flipped(stroke.rect))
    }
    for r in 0..<Int(page.run_count) {
        let run = page.runs[r]
        guard let font = fonts.font(run.font, size: run.size) else { continue }
        ctx.setFillColor(run.ink == 1 ? color(palette.link) : color(palette.ink, alpha: run.ink == 2 ? secondaryAlpha : 1))
        let count = Int(run.count)
        let points = (0..<count).map { i in CGPoint(x: CGFloat(run.positions[2 * i]), y: h - CGFloat(run.positions[2 * i + 1])) }
        CTFontDrawGlyphs(font, run.glyphs, points, count, ctx)
    }
}

/// A bitmap `size` big at `scale`, filled with `background`, drawn into by `body` (y up, logical pixels).
func bitmap(_ size: CGSize, scale: CGFloat, background: UInt32?, _ body: (CGContext) -> Void) -> CGImage? {
    guard size.width > 0, size.height > 0,
          let ctx = CGContext(data: nil, width: Int((size.width * scale).rounded(.up)), height: Int((size.height * scale).rounded(.up)), bitsPerComponent: 8,
                              bytesPerRow: 0, space: CGColorSpace(name: CGColorSpace.sRGB)!, bitmapInfo: CGImageAlphaInfo.premultipliedFirst.rawValue) else { return nil }
    ctx.scaleBy(x: scale, y: scale)
    if let background {
        ctx.setFillColor(color(background))
        ctx.fill(CGRect(origin: .zero, size: size))
    }
    body(ctx)
    return ctx.makeImage()
}

/// A note marker, link, image or table on a page: where it is (y down) and what it opens.
struct Area {
    let rect: CGRect
    /// 0 note marker, 1 link, 2 image, 3 table (the core's kinds), 4 a highlight (the shell's).
    let kind: UInt8
    /// The note id, link key or image id.
    let target: String
    /// A table's first row; a highlight's start.
    let at: ForzacLocator
    var isNote: Bool { kind == 0 }
    /// An image or a table: it opens in the view.
    var isViewable: Bool { kind == 2 || kind == 3 }

    func opensSame(_ o: Area) -> Bool {
        kind == o.kind && target == o.target && (kind < 3 || (at.chapter, at.block, at.offset) == (o.at.chapter, o.at.block, o.at.offset))
    }

    func offsetBy(dy: CGFloat) -> Area { Area(rect: rect.offsetBy(dx: 0, dy: dy), kind: kind, target: target, at: at) }
}

func areas(of page: ForzacPage, dy: CGFloat = 0) -> [Area] {
    (0..<Int(page.area_count)).map { i in
        let a = page.areas[i]
        return Area(rect: CGRect(x: CGFloat(a.rect.x), y: CGFloat(a.rect.y) + dy, width: CGFloat(a.rect.w), height: CGFloat(a.rect.h)),
                    kind: a.kind, target: String(cString: a.target), at: a.at)
    }
}

/// `Tab` stops: runs of consecutive areas with the same target (a link broken across lines is one stop). An image or a
/// table is one stop however many areas it has (a table cut by the "Scroll" tiles has one per tile).
func stops(_ areas: [Area]) -> [ClosedRange<Int>] {
    var out: [ClosedRange<Int>] = []
    for (i, a) in areas.enumerated() {
        if let last = out.last, areas[last.upperBound].opensSame(a) {
            out[out.count - 1] = last.lowerBound...i
        } else if !(a.isViewable && out.contains { areas[$0.lowerBound].opensSame(a) }) {
            out.append(i...i)
        }
    }
    return out
}

/// Layers (y up in a view `height` high) that mark the focused stop of `areas`: a marker or link as a soft box in the
/// link ink over each of its lines; an image or a table as a rounded outline in the link ink around all its areas.
func focusLayers(_ areas: [Area], _ stop: ClosedRange<Int>, palette: Palette, height h: CGFloat) -> [CALayer] {
    let first = areas[stop.lowerBound]
    if first.isViewable {
        let r = areas.filter { $0.opensSame(first) }.map(\.rect).reduce(first.rect) { $0.union($1) }
        let l = CALayer()
        l.borderColor = color(palette.link, alpha: 0.7)
        l.borderWidth = 2
        l.cornerRadius = 6
        l.frame = CGRect(x: r.minX - 6, y: h - r.maxY - 6, width: r.width + 12, height: r.height + 12)
        return [l]
    }
    return areas[stop].map { a in
        let l = CALayer()
        l.backgroundColor = color(palette.link, alpha: 0.22)
        l.cornerRadius = 4
        l.frame = CGRect(x: a.rect.minX - 3, y: h - a.rect.maxY - 2, width: a.rect.width + 6, height: a.rect.height + 4)
        return l
    }
}

/// [Lucide](https://lucide.dev) icons (ISC) as shapes on their 24×24 grid, stroke 1.75; the spacing and margin presets
/// in Lucide's style, as in the Android app.
enum Shape {
    case line(CGFloat, CGFloat, CGFloat, CGFloat)
    case rect(CGFloat, CGFloat, CGFloat, CGFloat)
    case circle(CGFloat, CGFloat, CGFloat)
}

enum Icons {
    static let x: [Shape] = [.line(18, 6, 6, 18), .line(6, 6, 18, 18)]
    static let search: [Shape] = [.circle(11, 11, 8), .line(21, 21, 16.7, 16.7)]
    static let share: [Shape] = [.circle(18, 5, 3), .circle(6, 12, 3), .circle(18, 19, 3), .line(8.59, 13.51, 15.42, 17.49), .line(15.41, 6.51, 8.59, 10.49)]
    static let alignJustify: [Shape] = [.line(3, 6, 21, 6), .line(3, 12, 21, 12), .line(3, 18, 21, 18)]
    static let alignLeft: [Shape] = [.line(3, 6, 21, 6), .line(3, 12, 15, 12), .line(3, 18, 18, 18)]
    static let spacing: [[Shape]] = [[8, 12, 16], [6, 12, 18], [4, 12, 20]].map { ys in ys.map { .line(4, $0, 20, $0) } }
    static let margins: [[Shape]] = [(6, 18), (8, 16), (10, 14)].map { (a, b) in [.rect(3, 3, 18, 18)] + [8, 12, 16].map { .line(a, $0, b, $0) } }
}

/// An icon drawn in `tint` at `size` points square.
final class IconView: NSView {
    let shapes: [Shape]
    let size: CGFloat
    var tint: NSColor = .labelColor { didSet { needsDisplay = true } }

    init(_ shapes: [Shape], size: CGFloat) {
        self.shapes = shapes
        self.size = size
        super.init(frame: NSRect(x: 0, y: 0, width: size, height: size))
    }

    required init?(coder: NSCoder) { fatalError() }

    override var isFlipped: Bool { true }
    override var intrinsicContentSize: NSSize { NSSize(width: size, height: size) }

    override func draw(_ dirtyRect: NSRect) {
        guard let ctx = NSGraphicsContext.current?.cgContext else { return }
        let s = size / 24
        ctx.scaleBy(x: s, y: s)
        ctx.setLineWidth(1.75)
        ctx.setLineCap(.round)
        ctx.setLineJoin(.round)
        ctx.setStrokeColor(tint.cgColor)
        for shape in shapes {
            switch shape {
            case let .line(x1, y1, x2, y2): ctx.move(to: CGPoint(x: x1, y: y1)); ctx.addLine(to: CGPoint(x: x2, y: y2))
            case let .rect(x, y, w, h): ctx.addRect(CGRect(x: x, y: y, width: w, height: h))
            case let .circle(x, y, r): ctx.addEllipse(in: CGRect(x: x - r, y: y - r, width: 2 * r, height: 2 * r))
            }
        }
        ctx.strokePath()
    }
}

/// Lower case and "ё" as "е", for filtering the contents.
func fold(_ s: String) -> String { s.lowercased().replacingOccurrences(of: "ё", with: "е") }

/// ANSI key codes of the characters the shortcuts use.
private let keyCodes: [Character: UInt16] = [
    "a": 0x00, "b": 0x0B, "c": 0x08, "v": 0x09, "s": 0x01, "d": 0x02, "f": 0x03, "h": 0x04, "g": 0x05, "t": 0x11, "o": 0x1F, "u": 0x20, "l": 0x25, "j": 0x26,
    "k": 0x28, "m": 0x2E, "n": 0x2D, "q": 0x0C, "0": 0x1D, "=": 0x18, "+": 0x18, "-": 0x1B, "]": 0x1E, "[": 0x21, "/": 0x2C, "?": 0x2C,
]

extension NSEvent {
    /// The key for shortcuts: the key code of a Latin character typed (so other Latin layouts follow their letters),
    /// else the physical key code (a Russian layout uses the keys of the Latin letters, like GTK).
    var shortcutKey: UInt16 {
        if let c = charactersIgnoringModifiers?.lowercased().first, let code = keyCodes[c] { return code }
        return keyCode
    }
}
