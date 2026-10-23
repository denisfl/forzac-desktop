// The help (`help`), as the GTK shell's: a card over the dimmed page with every key by section, in the reader's theme,
// the sizes of `openspec/changes/help-card/design.md`. `?`, Esc, the close button and a click beside the card hide it;
// while it is open it has the keys.

import AppKit
import QuartzCore

private let column: CGFloat = 416, keysWidth: CGFloat = 168, gap: CGFloat = 12, columnGap: CGFloat = 40
private let padding: CGFloat = 28, margin: CGFloat = 24, line: CGFloat = 40
private let supportURL = URL(string: "https://forzac.app/#support")!

/// The help from the core: the title, sections of (keys, meaning), the support line; a book's keys or the library's.
private struct Text {
    var title = "", sections: [(title: String, rows: [(keys: [String], meaning: String)])] = [], support = ""

    init(library: Bool) {
        var lines = String(cString: library ? forzac_library_help(1) : forzac_help(1)).components(separatedBy: "\n")
        title = lines.removeFirst()
        support = lines.removeLast()
        for l in lines {
            if l.hasPrefix("# ") {
                sections.append((String(l.dropFirst(2)), []))
            } else {
                let parts = l.components(separatedBy: "\t")
                let keys = parts[0].split(separator: " ").map(String.init)
                sections[sections.count - 1].rows.append((keys, parts.count > 1 ? parts[1] : ""))
            }
        }
    }
}

/// A view with y down that swallows clicks.
private class Flipped: NSView {
    override var isFlipped: Bool { true }
    override func mouseDown(with e: NSEvent) {}
}

/// The support line: a click on it opens the site's support section, the pointing hand over it.
private final class Link: NSTextField {
    override func resetCursorRects() { addCursorRect(bounds, cursor: .pointingHand) }
    override func mouseDown(with e: NSEvent) {}
    override func mouseUp(with e: NSEvent) {
        if bounds.contains(convert(e.locationInWindow, from: nil)) { NSWorkspace.shared.open(supportURL) }
    }
}

/// The round close button: the × in the ink, the ink at 6 % under the pointer.
private final class CloseButton: Flipped {
    let icon = IconView(Icons.x, size: 20)
    var action: (() -> Void)?
    var tint: UInt32 = 0

    override init(frame: NSRect) {
        super.init(frame: frame)
        wantsLayer = true
        layer?.cornerRadius = 16
        addSubview(icon)
    }

    required init?(coder: NSCoder) { fatalError() }

    override func layout() {
        super.layout()
        icon.frame = CGRect(x: (bounds.width - 20) / 2, y: (bounds.height - 20) / 2, width: 20, height: 20)
    }

    override func updateTrackingAreas() {
        super.updateTrackingAreas()
        trackingAreas.forEach(removeTrackingArea)
        addTrackingArea(NSTrackingArea(rect: bounds, options: [.mouseEnteredAndExited, .activeInKeyWindow], owner: self))
    }

    override func mouseEntered(with e: NSEvent) { layer?.backgroundColor = color(tint, alpha: 0.06) }
    override func mouseExited(with e: NSEvent) { layer?.backgroundColor = nil }
    override func mouseUp(with e: NSEvent) {
        layer?.backgroundColor = nil
        if bounds.contains(convert(e.locationInWindow, from: nil)) { action?() }
    }
}

final class HelpView: NSView {
    private let text: Text
    private let card = Flipped(), close = CloseButton(), rule = Flipped()
    private let title: NSTextField, support: Link
    private let scroll = NSScrollView(), document = Flipped()
    /// Per section: its title, and per row its caps and its meaning.
    private var sections: [(title: NSTextField, rows: [(caps: [NSTextField], meaning: NSTextField)])] = []
    /// The caps' frames, drawn under their labels: one layer per cap.
    private var frames: [NSTextField: CALayer] = [:]
    private var palette = themes[0]
    /// What had the keys before the help.
    private weak var previous: NSResponder?
    private(set) var isOpen = false

    /// The keys of a book window, or of the library window (`library`).
    init(library: Bool = false) {
        text = Text(library: library)
        title = label(text.title, interfaceFont(20, .semibold), 0)
        support = Link(wrappingLabelWithString: text.support)
        super.init(frame: .zero)
        wantsLayer = true
        isHidden = true
        autoresizingMask = [.width, .height]
        card.wantsLayer = true
        card.layer?.cornerRadius = 22
        card.layer?.shadowOpacity = 0.16
        card.layer?.shadowRadius = 16
        card.layer?.shadowOffset = CGSize(width: 0, height: -12)
        card.layer?.borderWidth = 1
        rule.wantsLayer = true
        support.font = interfaceFont(13)
        support.isSelectable = false
        close.action = { [weak self] in self?.hide() }
        for v: NSView in [title, support] { v.translatesAutoresizingMaskIntoConstraints = true }
        scroll.drawsBackground = false
        scroll.hasVerticalScroller = true
        scroll.autohidesScrollers = true
        scroll.documentView = document
        document.wantsLayer = true
        addSubview(card)
        [title, close, scroll, rule, support].forEach(card.addSubview)
        for s in text.sections {
            let t = label(s.title.uppercased(), interfaceFont(12, .medium), 0)
            t.attributedStringValue = NSAttributedString(string: s.title.uppercased(), attributes: [.font: interfaceFont(12, .medium), .kern: 0.72])
            let rows = s.rows.map { row -> ([NSTextField], NSTextField) in
                let caps = row.keys.map { key -> NSTextField in
                    let cap = label(key, interfaceFont(13, .medium), 0)
                    cap.alignment = .center
                    return cap
                }
                let meaning = NSTextField(wrappingLabelWithString: row.meaning)
                meaning.font = interfaceFont(14)
                return (caps, meaning)
            }
            sections.append((t, rows))
            document.addSubview(t)
            for (caps, meaning) in rows {
                for cap in caps {
                    let frame = CALayer()
                    frame.cornerRadius = 6
                    frame.borderWidth = 1
                    frames[cap] = frame
                    document.addSubview(cap)
                }
                document.addSubview(meaning)
            }
        }
        for v in document.subviews { v.translatesAutoresizingMaskIntoConstraints = true }
    }

    required init?(coder: NSCoder) { fatalError() }

    override var isFlipped: Bool { true }
    override var acceptsFirstResponder: Bool { true }

    /// Shows the help over everything in `container`, in `palette`, and gives it the keys.
    func show(in container: NSView, palette: Palette) {
        guard !isOpen else { return }
        isOpen = true
        self.palette = palette
        frame = container.bounds
        container.addSubview(self, positioned: .above, relativeTo: nil) // over the panels and the menu
        recolour()
        needsLayout = true
        layoutSubtreeIfNeeded()
        document.scroll(.zero)
        previous = window?.firstResponder
        window?.makeFirstResponder(self)
        alphaValue = 0
        isHidden = false
        NSAnimationContext.runAnimationGroup { c in
            c.duration = 0.2
            c.timingFunction = CAMediaTimingFunction(name: .easeOut)
            animator().alphaValue = 1
        }
    }

    func hide() {
        guard isOpen else { return }
        isOpen = false
        if window?.firstResponder === self { window?.makeFirstResponder(previous) }
        NSAnimationContext.runAnimationGroup({ c in
            c.duration = 0.15
            c.timingFunction = CAMediaTimingFunction(name: .easeIn)
            animator().alphaValue = 0
        }, completionHandler: { [weak self] in
            if self?.isOpen == false { self?.isHidden = true }
        })
    }

    private func recolour() {
        let p = palette
        layer?.backgroundColor = CGColor(gray: 0, alpha: 0.24)
        card.layer?.backgroundColor = color(p.sheet)
        card.layer?.borderColor = color(p.line)
        card.layer?.shadowColor = .black
        rule.layer?.backgroundColor = color(p.line)
        title.textColor = nsColor(p.ink)
        // The line in the muted colour, its thanks in the link's, underlined.
        let words = NSMutableAttributedString(string: text.support, attributes: [.font: interfaceFont(13), .foregroundColor: nsColor(p.muted)])
        if let thanks = text.support.range(of: tr("your thanks")) {
            words.addAttributes([.foregroundColor: nsColor(p.link), .underlineStyle: NSUnderlineStyle.single.rawValue],
                               range: NSRange(thanks, in: text.support))
        }
        support.attributedStringValue = words
        close.tint = p.ink
        close.icon.tint = nsColor(p.ink)
        for s in sections {
            s.title.textColor = nsColor(p.muted)
            for (caps, meaning) in s.rows {
                meaning.textColor = nsColor(p.ink)
                for cap in caps {
                    cap.textColor = nsColor(p.ink)
                    frames[cap]?.backgroundColor = color(p.paper)
                    frames[cap]?.borderColor = color(p.line)
                }
            }
        }
    }

    override func layout() {
        super.layout()
        // Two columns when they fit beside each other with the padding and the margins, else one.
        let two = bounds.width >= 2 * column + columnGap + 2 * padding + 2 * margin
        let width = two ? 2 * column + columnGap : column
        let split = two ? splitIndex() : sections.count
        var heights: [CGFloat] = [0, 0]
        for (i, s) in sections.enumerated() {
            let c = i < split ? 0 : 1
            let x = CGFloat(c) * (column + columnGap)
            var y = heights[c] + (heights[c] > 0 ? 16 : 0)
            s.title.sizeToFit()
            s.title.frame.origin = CGPoint(x: x, y: y)
            y += s.title.frame.height + 6
            for (caps, meaning) in s.rows {
                let capsHeight = place(caps, x: x, y: y)
                let left = caps.isEmpty ? x : x + keysWidth + gap
                let w = caps.isEmpty ? column : column - keysWidth - gap
                meaning.preferredMaxLayoutWidth = w
                let size = meaning.sizeThatFits(NSSize(width: w, height: .greatestFiniteMagnitude))
                meaning.frame = CGRect(x: left, y: y + (caps.isEmpty ? 0 : 1), width: w, height: ceil(size.height))
                y += max(capsHeight, meaning.frame.height) + 4
            }
            heights[c] = y - 4
        }
        let contentHeight = heights.max() ?? 0
        document.frame = CGRect(x: 0, y: 0, width: width, height: contentHeight)

        title.sizeToFit()
        support.preferredMaxLayoutWidth = width
        let supportHeight = ceil(support.sizeThatFits(NSSize(width: width, height: .greatestFiniteMagnitude)).height)
        let head = max(32, title.frame.height)
        let foot = 20 + 1 + 16 + supportHeight
        let available = bounds.height - 2 * margin - 2 * 20 - head - 16 - foot
        let shown = max(0, min(contentHeight, available))
        let cardSize = CGSize(width: width + 2 * padding, height: 20 + head + 16 + shown + foot + 20)
        card.frame = CGRect(x: ((bounds.width - cardSize.width) / 2).rounded(), y: ((bounds.height - cardSize.height) / 2).rounded(),
                            width: cardSize.width, height: cardSize.height)
        card.layer?.shadowPath = CGPath(roundedRect: card.bounds, cornerWidth: 22, cornerHeight: 22, transform: nil)
        title.frame.origin = CGPoint(x: padding, y: 20 + (head - title.frame.height) / 2)
        close.frame = CGRect(x: cardSize.width - padding - 32 + 6, y: 20 + (head - 32) / 2, width: 32, height: 32)
        scroll.frame = CGRect(x: padding, y: 20 + head + 16, width: width, height: shown)
        rule.frame = CGRect(x: padding, y: scroll.frame.maxY + 20, width: width, height: 1)
        support.frame = CGRect(x: padding, y: rule.frame.maxY + 16, width: width, height: supportHeight)
    }

    /// The caps from (x, y) in lines no wider than the key area; their height.
    private func place(_ caps: [NSTextField], x: CGFloat, y: CGFloat) -> CGFloat {
        var at = CGPoint(x: x, y: y), height: CGFloat = 0
        for cap in caps {
            // The text's own size, 7 points of padding at the sides and 2 above and below, as GTK's.
            let text = cap.attributedStringValue.size()
            let size = CGSize(width: max(24, ceil(text.width) + 16), height: ceil(text.height) + 6)
            if at.x > x, at.x + size.width > x + keysWidth {
                at = CGPoint(x: x, y: at.y + size.height + 4)
            }
            let box = CGRect(origin: at, size: size)
            frames[cap]?.frame = box
            if let frame = frames[cap], frame.superlayer == nil { document.layer?.insertSublayer(frame, at: 0) }
            cap.frame = CGRect(x: box.minX - 4, y: box.minY + 3, width: box.width + 8, height: ceil(text.height))
            at.x += size.width + 4
            height = at.y + size.height - y
        }
        return height
    }

    /// The split between two sections that makes the taller column shortest.
    private func splitIndex() -> Int {
        let rows = sections.map { $0.rows.count }
        let total = rows.reduce(0, +)
        return (1..<max(rows.count, 2)).min { a, b in
            let fa = rows[..<a].reduce(0, +), fb = rows[..<b].reduce(0, +)
            return max(fa, total - fa) < max(fb, total - fb)
        } ?? rows.count
    }

    /// A click beside the card hides the help when the button comes up (then the page under it gets neither half).
    override func mouseDown(with e: NSEvent) {}
    override func mouseUp(with e: NSEvent) {
        if !card.frame.contains(convert(e.locationInWindow, from: nil)) { hide() }
    }

    override func scrollWheel(with e: NSEvent) {
        if card.frame.contains(convert(e.locationInWindow, from: nil)) { scroll.scrollWheel(with: e) }
    }

    /// `?` and Esc hide the help; j k ↓ ↑ scroll a line, Space PgDn PgUp a screen; the other keys do nothing.
    override func keyDown(with e: NSEvent) {
        let page = max(line, scroll.frame.height - line)
        // By key code (any layout), or by character for `--keys`, whose letters have none.
        let char = e.charactersIgnoringModifiers ?? ""
        var step: CGFloat
        switch (e.keyCode, char) {
        case (53, _): return hide()
        case (0x26, _), (125, _), (_, "j"): step = line
        case (0x28, _), (126, _), (_, "k"): step = -line
        case (49, _), (_, " "): step = e.modifierFlags.contains(.shift) ? -page : page
        case (121, _): step = page
        case (116, _): step = -page
        default:
            if e.characters == "?" { hide() }
            return
        }
        let clip = scroll.contentView
        let maxY = max(0, document.frame.height - clip.bounds.height)
        clip.scroll(to: CGPoint(x: 0, y: min(maxY, max(0, clip.bounds.origin.y + step))))
        scroll.reflectScrolledClipView(clip)
    }
}
