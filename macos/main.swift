// Forzac for macOS: an AppKit shell over forzac-core.
//
// The core typesets pages into glyph runs; this shell draws them with Core Text into a bitmap at the window's backing
// scale and slides pages with Core Animation, which runs in the window server: no app work per frame, ProMotion and
// Low Power Mode handled by the system, nothing drawn while idle.
//
// Spike flags: --measure (time to the first presented page), --png out.png (first page), --chapter N.

import AppKit
import CoreText
import QuartzCore

let processStart = Date()

/// Text size presets, logical pixels: steps, not a slider.
let textSizes: [Float] = [16, 18, 20, 22, 24, 27, 30]

/// Background and ink of the six themes (the Android app's palettes).
let themes: [(String, UInt32, UInt32)] = [
    ("Light", 0xFAFAFA, 0x1C1C1C), ("Sepia", 0xF4ECD8, 0x5B4636), ("Dark", 0x121212, 0xCFCFCF),
    ("Dawn", 0xFAF4ED, 0x575279), ("Wave", 0x1F1F28, 0xDCD7BA), ("Forest", 0x2D353B, 0xD3C6AA),
]

func color(_ rgb: UInt32) -> CGColor {
    CGColor(srgbRed: CGFloat((rgb >> 16) & 0xFF) / 255, green: CGFloat((rgb >> 8) & 0xFF) / 255, blue: CGFloat(rgb & 0xFF) / 255, alpha: 1)
}

/// The bundled faces, created from the very bytes the core shaped with: glyph ids match by construction.
final class Fonts {
    private let graphics: [CGFont]
    private var cache: [Int: CTFont] = [:]

    init() {
        graphics = (0..<4).map { face in
            var len = 0
            let ptr = forzac_font_data(UInt8(face), &len)!
            let data = Data(bytesNoCopy: UnsafeMutableRawPointer(mutating: ptr), count: len, deallocator: .none)
            return CGFont(CGDataProvider(data: data as CFData)!)!
        }
    }

    func font(face: UInt8, size: Float) -> CTFont {
        let key = Int(face) << 24 | Int(size * 64)
        if let f = cache[key] { return f }
        let f = CTFontCreateWithGraphicsFont(graphics[Int(face)], CGFloat(size), nil, nil)
        cache[key] = f
        return f
    }
}

final class PageView: NSView {
    let doc: OpaquePointer
    let fonts = Fonts()
    var chapter: UInt32
    var pageCount: UInt32 = 0
    var page: UInt32 = 0
    var locator: ForzacLocator
    var toLast = false
    var sizeStep = 2
    var theme = 0
    var laidOutFor: (CGSize, Int, UInt32)?
    var images: [UInt32: CGImage] = [:]
    let current = CALayer()
    let previous = CALayer()
    var onFirstFrame: ((PageView) -> Void)?

    init(doc: OpaquePointer, chapter: UInt32) {
        self.doc = doc
        self.chapter = chapter
        self.locator = ForzacLocator(chapter: chapter, block: 0, offset: 0)
        super.init(frame: .zero)
        wantsLayer = true
        layerContentsRedrawPolicy = .never
        for l in [previous, current] {
            l.contentsGravity = .resize
            l.actions = ["contents": NSNull(), "position": NSNull(), "bounds": NSNull()]
            layer!.addSublayer(l)
        }
    }

    required init?(coder: NSCoder) { fatalError() }

    override var acceptsFirstResponder: Bool { true }

    override func layout() {
        super.layout()
        relayout()
        show(animated: 0)
    }

    override func viewDidChangeBackingProperties() {
        images = [:]
        show(animated: 0)
    }

    var scale: CGFloat { window?.backingScaleFactor ?? 2 }

    /// Typesets the chapter when the size, text size or chapter changed; keeps the reading position.
    func relayout() {
        let size = bounds.size
        guard size.width > 0, size.height > 0 else { return }
        if let l = laidOutFor, l.0 == size, l.1 == sizeStep, l.2 == chapter { return }
        var style = ForzacStyle(width: Float(size.width), height: Float(size.height), text_size: textSizes[sizeStep])
        pageCount = forzac_paginate(doc, chapter, &style)
        page = toLast ? pageCount - 1 : forzac_page_of(doc, locator)
        toLast = false
        laidOutFor = (size, sizeStep, chapter)
        images = [:]
    }

    /// The page as a bitmap at the backing scale, drawn with Core Text.
    func image(_ index: UInt32) -> CGImage? {
        if let i = images[index] { return i }
        let (w, h) = (bounds.width, bounds.height)
        let s = scale
        guard let ctx = CGContext(data: nil, width: Int(w * s), height: Int(h * s), bitsPerComponent: 8, bytesPerRow: 0,
                                  space: CGColorSpace(name: CGColorSpace.sRGB)!, bitmapInfo: CGImageAlphaInfo.premultipliedFirst.rawValue) else { return nil }
        ctx.scaleBy(x: s, y: s)
        ctx.setFillColor(color(themes[theme].1))
        ctx.fill(CGRect(x: 0, y: 0, width: w, height: h))
        ctx.setFillColor(color(themes[theme].2))
        ctx.setAllowsFontSubpixelPositioning(true)
        ctx.setShouldSubpixelPositionFonts(true)
        var runs: UnsafePointer<ForzacRun>?
        let n = forzac_page_runs(doc, index, &runs)
        for r in 0..<Int(n) {
            let run = runs![r]
            let font = fonts.font(face: run.face, size: run.size)
            let count = Int(run.count)
            // Core Graphics is y up: flip the page's y-down coordinates.
            let points = (0..<count).map { i in CGPoint(x: CGFloat(run.positions[2 * i]), y: h - CGFloat(run.positions[2 * i + 1])) }
            CTFontDrawGlyphs(font, run.glyphs, points, count, ctx)
        }
        let image = ctx.makeImage()
        images[index] = image
        return image
    }

    /// Shows the current page; `animated` +1 slides it in from the right, −1 from the left.
    func show(animated dir: CGFloat) {
        guard let img = image(page) else { return }
        layer!.backgroundColor = color(themes[theme].1)
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
        if let f = onFirstFrame {
            onFirstFrame = nil
            CATransaction.setCompletionBlock { f(self) }
        }
    }

    func turn(_ delta: Int32) {
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
        show(animated: CGFloat(delta.signum()))
    }

    /// Keys by physical key code, so they work in any layout (Russian included).
    override func keyDown(with e: NSEvent) {
        let shift = e.modifierFlags.contains(.shift)
        switch e.keyCode {
        case 0x31 where shift: turn(-1)                                       // Shift+Space
        case 0x26, 0x25, 0x7C, 0x7D, 0x31, 0x79, 0x24: turn(1)                // j l → ↓ Space PgDn Return
        case 0x28, 0x04, 0x7B, 0x7E, 0x74, 0x33: turn(-1)                     // k h ← ↑ PgUp Backspace
        case 0x1E: jumpChapter(1)                                              // ]
        case 0x21: jumpChapter(-1)                                             // [
        case 0x05 where shift, 0x77: edge(end: true)                           // G End
        case 0x05, 0x73: edge(end: false)                                      // g Home
        case 0x18: resize(1)                                                   // = +
        case 0x1B: resize(-1)                                                  // -
        case 0x11: theme = (theme + (shift ? themes.count - 1 : 1)) % themes.count; images = [:]; show(animated: 0) // t T
        case 0x03: window?.toggleFullScreen(nil)                               // f
        case 0x2C where shift: (NSApp.delegate as? AppDelegate)?.toggleHelp()  // ?
        case 0x35: (NSApp.delegate as? AppDelegate)?.hideHelp()               // Esc
        case 0x0C: window?.close()                                             // q
        default: super.keyDown(with: e)
        }
    }

    func jumpChapter(_ delta: Int32) {
        let target = Int64(chapter) + Int64(delta)
        guard target >= 0 && target < Int64(forzac_chapter_count(doc)) else { return }
        chapter = UInt32(target)
        locator = ForzacLocator(chapter: chapter, block: 0, offset: 0)
        relayout()
        show(animated: 0)
    }

    func edge(end: Bool) {
        page = end ? pageCount - 1 : 0
        locator = forzac_page_start(doc, page)
        show(animated: 0)
    }

    func resize(_ delta: Int) {
        sizeStep = max(0, min(textSizes.count - 1, sizeStep + delta))
        relayout()
        show(animated: 0)
    }

    func savePNG(_ path: String) {
        guard let img = image(page), let dest = CGImageDestinationCreateWithURL(URL(fileURLWithPath: path) as CFURL, "public.png" as CFString, 1, nil) else { return }
        CGImageDestinationAddImage(dest, img, nil)
        CGImageDestinationFinalize(dest)
        print("saved \(path) (\(img.width)×\(img.height))")
    }
}

let helpText = """
j  l  →  ↓  Space  PgDn    next page
k  h  ←  ↑  Shift+Space  PgUp    previous page
]  [    next / previous chapter
g  G  Home  End    chapter start / end
+  −    text size
t  T    theme
f  ⌃⌘F    full screen
?    this help
q  ⌘W    close

Forzac is free. Support it: forzac.truenrth.app
"""

final class AppDelegate: NSObject, NSApplicationDelegate {
    var window: NSWindow!
    var help: NSTextField!
    let args = CommandLine.arguments

    func value(after flag: String) -> String? {
        guard let i = args.firstIndex(of: flag), i + 1 < args.count else { return nil }
        return args[i + 1]
    }

    func applicationDidFinishLaunching(_ note: Notification) {
        let png = value(after: "--png")
        let chapter = UInt32(value(after: "--chapter") ?? "0") ?? 0
        guard let path = args.dropFirst().first(where: { !$0.hasPrefix("--") && $0 != png && UInt32($0) == nil }),
              let doc = forzac_open(path) else {
            fputs("usage: forzac <book.fb2|book.fb2.zip>\n", stderr)
            exit(2)
        }
        let parsed = Date().timeIntervalSince(processStart)
        let title = String(cString: forzac_title(doc))
        let view = PageView(doc: doc, chapter: chapter)
        window = NSWindow(contentRect: NSRect(x: 0, y: 0, width: 760, height: 980), styleMask: [.titled, .closable, .miniaturizable, .resizable], backing: .buffered, defer: false)
        window.title = title.isEmpty ? "Forzac" : "\(title) — Forzac"
        window.contentView = view
        help = NSTextField(labelWithString: helpText)
        help.font = .systemFont(ofSize: 13)
        help.isHidden = true
        help.wantsLayer = true
        help.layer?.backgroundColor = NSColor.windowBackgroundColor.withAlphaComponent(0.95).cgColor
        help.layer?.cornerRadius = 12
        view.addSubview(help)
        window.center()
        window.makeKeyAndOrderFront(nil)
        window.makeFirstResponder(view)
        NSApp.activate(ignoringOtherApps: true)
        if args.contains("--measure") || png != nil {
            view.onFirstFrame = { v in
                if self.args.contains("--measure") {
                    print(String(format: "startup: parsed %.1f ms, first page presented %.1f ms, pages in chapter %d", parsed * 1000, Date().timeIntervalSince(processStart) * 1000, v.pageCount))
                }
                if let png { v.savePNG(png) }
                NSApp.terminate(nil)
            }
        }
    }

    func toggleHelp() {
        help.sizeToFit()
        if let v = window.contentView { help.setFrameOrigin(NSPoint(x: (v.bounds.width - help.frame.width) / 2, y: (v.bounds.height - help.frame.height) / 2)) }
        help.isHidden.toggle()
    }

    func hideHelp() { help.isHidden = true }

    func applicationShouldTerminateAfterLastWindowClosed(_ app: NSApplication) -> Bool { true }
}

// Standard menu: ⌘Q, ⌘W, ⌃⌘F come from AppKit for free.
let app = NSApplication.shared
app.setActivationPolicy(.regular)
let menu = NSMenu()
let appItem = NSMenuItem()
menu.addItem(appItem)
let appMenu = NSMenu()
appMenu.addItem(withTitle: "Quit Forzac", action: #selector(NSApplication.terminate(_:)), keyEquivalent: "q")
appItem.submenu = appMenu
let windowItem = NSMenuItem()
menu.addItem(windowItem)
let windowMenu = NSMenu(title: "Window")
windowMenu.addItem(withTitle: "Close", action: #selector(NSWindow.performClose(_:)), keyEquivalent: "w")
let fullScreen = windowMenu.addItem(withTitle: "Enter Full Screen", action: #selector(NSWindow.toggleFullScreen(_:)), keyEquivalent: "f")
fullScreen.keyEquivalentModifierMask = [.control, .command]
windowItem.submenu = windowMenu
app.mainMenu = menu
let delegate = AppDelegate()
app.delegate = delegate
app.run()
