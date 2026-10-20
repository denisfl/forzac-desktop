// Panels over the page, laid out as the Android app's (`SettingsSheet.kt`, `TocScreen.kt`, `NoteSheet.kt`; the design's
// `Settings.dc.html`, `Toc.dc.html`, `Footnote.dc.html`) with the same sizes as the GTK shell's: a bottom sheet for the
// settings and notes, a side panel for the contents. They follow the reader's theme and take the keys while open.

import AppKit

/// The widest bottom sheet, as on Android tablets.
let sheetWidth: CGFloat = 560

func label(_ text: String, _ font: NSFont, _ rgb: UInt32) -> NSTextField {
    let l = NSTextField(labelWithString: text)
    l.font = font
    l.textColor = nsColor(rgb)
    l.translatesAutoresizingMaskIntoConstraints = false
    return l
}

func fixed(_ view: NSView, width: CGFloat? = nil, height: CGFloat? = nil) {
    view.translatesAutoresizingMaskIntoConstraints = false
    if let width { view.widthAnchor.constraint(equalToConstant: width).isActive = true }
    if let height { view.heightAnchor.constraint(equalToConstant: height).isActive = true }
}

/// A view whose layer holds its look; y down.
class LayerView: NSView {
    override var isFlipped: Bool { true }

    init() {
        super.init(frame: .zero)
        wantsLayer = true
        translatesAutoresizingMaskIntoConstraints = false
    }

    required init?(coder: NSCoder) { fatalError() }
}

/// A bottom sheet: the panel colour, rounded top corners, a shadow, a grabber.
class SheetView: LayerView {
    let stack = NSStackView()

    init(spacing: CGFloat, insets: NSEdgeInsets) {
        super.init()
        layer!.cornerRadius = 22
        layer!.maskedCorners = [.layerMinXMinYCorner, .layerMaxXMinYCorner] // the top corners (the view is flipped)
        layer!.shadowOpacity = 1
        layer!.shadowRadius = 16
        layer!.shadowOffset = CGSize(width: 0, height: 6)
        stack.orientation = .vertical
        stack.alignment = .leading
        stack.spacing = spacing
        stack.edgeInsets = insets
        stack.translatesAutoresizingMaskIntoConstraints = false
        addSubview(stack)
        NSLayoutConstraint.activate([
            stack.topAnchor.constraint(equalTo: topAnchor), stack.leadingAnchor.constraint(equalTo: leadingAnchor),
            stack.trailingAnchor.constraint(equalTo: trailingAnchor), stack.bottomAnchor.constraint(equalTo: bottomAnchor),
        ])
    }

    required init?(coder: NSCoder) { fatalError() }

    let grabber = LayerView()

    func addGrabber() {
        fixed(grabber, width: 36, height: 4)
        grabber.layer!.cornerRadius = 2
        stack.addArrangedSubview(grabber)
        grabber.centerXAnchor.constraint(equalTo: stack.centerXAnchor).isActive = true
    }

    func colour(_ p: Palette) {
        layer!.backgroundColor = color(p.sheet)
        layer!.shadowColor = color(0x000000, alpha: 0.12)
        grabber.layer!.backgroundColor = color(p.line)
    }

    /// Pins the sheet to the bottom of `container`, centred, at most `sheetWidth` wide.
    func pin(in container: NSView) {
        container.addSubview(self)
        // As wide as allowed: just below the window's own size (500), so a sheet never resizes the window, and above
        // the content's hugging, so it does not shrink to its content.
        let wide = widthAnchor.constraint(equalToConstant: sheetWidth)
        wide.priority = NSLayoutConstraint.Priority(490)
        NSLayoutConstraint.activate([
            bottomAnchor.constraint(equalTo: container.bottomAnchor), centerXAnchor.constraint(equalTo: container.centerXAnchor),
            widthAnchor.constraint(lessThanOrEqualToConstant: sheetWidth), widthAnchor.constraint(lessThanOrEqualTo: container.widthAnchor), wide,
        ])
    }
}

/// A flat control: a label or an icon; filled with ink when selected.
final class FlatButton: LayerView {
    let content: NSView
    var onClick: (() -> Void)?
    var selected = false { didSet { tint() } }
    var enabled = true { didSet { alphaValue = enabled ? 1 : 0.35 } }
    private var palette = themes[0]

    init(_ content: NSView) {
        self.content = content
        super.init()
        content.translatesAutoresizingMaskIntoConstraints = false
        addSubview(content)
        NSLayoutConstraint.activate([content.centerXAnchor.constraint(equalTo: centerXAnchor), content.centerYAnchor.constraint(equalTo: centerYAnchor)])
    }

    required init?(coder: NSCoder) { fatalError() }

    override func mouseDown(with e: NSEvent) {}

    override func mouseUp(with e: NSEvent) {
        if enabled, bounds.contains(convert(e.locationInWindow, from: nil)) { onClick?() }
    }

    func colour(_ p: Palette) {
        palette = p
        tint()
    }

    private func tint() {
        layer!.backgroundColor = selected ? color(palette.ink) : nil
        let ink = nsColor(selected ? palette.sheet : palette.ink)
        (content as? NSTextField)?.textColor = ink
        (content as? IconView)?.tint = ink
    }
}

/// Options side by side in a rounded frame.
final class Segmented: LayerView {
    let buttons: [FlatButton]
    var dividers: [LayerView] = []

    /// Options `width` wide, else of equal widths filling the frame (unless `equal` is false: the caller sizes them).
    init(_ buttons: [FlatButton], width: CGFloat? = nil, equal: Bool = true) {
        self.buttons = buttons
        super.init()
        layer!.cornerRadius = 12
        layer!.borderWidth = 1
        layer!.masksToBounds = true
        fixed(self, height: 44)
        var previous: NSView?
        for b in buttons {
            addSubview(b)
            NSLayoutConstraint.activate([b.topAnchor.constraint(equalTo: topAnchor), b.bottomAnchor.constraint(equalTo: bottomAnchor)])
            b.leadingAnchor.constraint(equalTo: previous?.trailingAnchor ?? leadingAnchor).isActive = true
            if let width { b.widthAnchor.constraint(equalToConstant: width).isActive = true } else if equal, let first = buttons.first, b !== first {
                b.widthAnchor.constraint(equalTo: first.widthAnchor).isActive = true
            }
            previous = b
        }
        previous?.trailingAnchor.constraint(equalTo: trailingAnchor).isActive = true
    }

    required init?(coder: NSCoder) { fatalError() }

    func select(_ i: Int) {
        for (k, b) in buttons.enumerated() { b.selected = k == i }
    }

    func colour(_ p: Palette) {
        layer!.borderColor = color(p.line)
        for b in buttons { b.colour(p) }
        for d in dividers { d.layer!.backgroundColor = color(p.line) }
    }
}

/// A caption above a control.
func captioned(_ text: String, _ control: NSView) -> (NSStackView, NSTextField) {
    let caption = label(text, interfaceFont(12), 0)
    let column = NSStackView(views: [caption, control])
    column.orientation = .vertical
    column.alignment = .leading
    column.spacing = 6
    control.widthAnchor.constraint(equalTo: column.widthAnchor).isActive = true
    return (column, caption)
}

// MARK: - Settings

/// The settings sheet: theme circles; text size and alignment; font chips; line spacing and margins; pages.
final class SettingsSheet: SheetView {
    private weak var view: PageView?
    private var circles: [LayerView] = []
    private var circleLabels: [NSTextField] = []
    private let ring = CALayer()
    private let size: Segmented
    private let sizeLabel = label("", interfaceFont(14), 0)
    private let align: Segmented
    private var chips: [FlatButton] = []
    private let spacing: Segmented
    private let margins: Segmented
    private let pages: Segmented
    private var captions: [NSTextField] = []
    private let themesRow = NSStackView()
    private let fontsRow = NSStackView()
    /// The controls in reading order and the core's setting each changes.
    private var controls: [(NSView, UInt32)] = []
    private var current = 0
    private let keysRing = CALayer()

    init(view: PageView) {
        self.view = view
        let smaller = FlatButton(label("A", bookFont("PT Serif", 14), 0))
        let larger = FlatButton(label("A", bookFont("PT Serif", 22), 0))
        let number = FlatButton(sizeLabel)
        size = Segmented([smaller, number, larger], equal: false)
        align = Segmented([Icons.alignJustify, Icons.alignLeft].map { FlatButton(IconView($0, size: 20)) }, width: 48)
        spacing = Segmented(Icons.spacing.map { FlatButton(IconView($0, size: 20)) })
        margins = Segmented(Icons.margins.map { FlatButton(IconView($0, size: 20)) })
        pages = Segmented([tr("Slide"), tr("Scroll")].map { FlatButton(label($0, interfaceFont(14), 0)) })
        super.init(spacing: 18, insets: NSEdgeInsets(top: 10, left: 20, bottom: 24, right: 20))
        addGrabber()

        // Themes: a circle in each theme's colours, the name below.
        themesRow.distribution = .equalSpacing
        for (i, name) in themeNames.enumerated() {
            let p = themes[i]
            let circle = LayerView()
            fixed(circle, width: 44, height: 44)
            circle.layer!.cornerRadius = 22
            circle.layer!.backgroundColor = color(p.paper)
            circle.layer!.borderWidth = 1
            circle.layer!.borderColor = color(p.line)
            let aa = label("Aa", bookFont("PT Serif", 16), p.ink)
            circle.addSubview(aa)
            NSLayoutConstraint.activate([aa.centerXAnchor.constraint(equalTo: circle.centerXAnchor), aa.centerYAnchor.constraint(equalTo: circle.centerYAnchor)])
            let name = label(name, interfaceFont(11), 0)
            let cell = NSStackView(views: [circle, name])
            cell.orientation = .vertical
            cell.spacing = 8
            fixed(cell, width: 56)
            let click = NSClickGestureRecognizer(target: self, action: #selector(pickTheme(_:)))
            cell.addGestureRecognizer(click)
            themesRow.addArrangedSubview(cell)
            circles.append(circle)
            circleLabels.append(name)
        }
        ring.borderWidth = 2
        ring.cornerRadius = 28
        layer!.addSublayer(ring)
        stack.addArrangedSubview(themesRow)
        themesRow.widthAnchor.constraint(equalTo: stack.widthAnchor, constant: -40).isActive = true

        // Text size and alignment.
        smaller.onClick = { [weak self] in self?.change(2, -1) }
        larger.onClick = { [weak self] in self?.change(2, 1) }
        number.widthAnchor.constraint(equalToConstant: 44).isActive = true
        smaller.widthAnchor.constraint(equalTo: larger.widthAnchor).isActive = true
        for b in [smaller, number] {
            let d = LayerView()
            fixed(d, width: 1)
            size.addSubview(d)
            NSLayoutConstraint.activate([d.topAnchor.constraint(equalTo: size.topAnchor), d.bottomAnchor.constraint(equalTo: size.bottomAnchor), d.leadingAnchor.constraint(equalTo: b.trailingAnchor)])
            size.dividers.append(d)
        }
        let row = NSStackView(views: [size, align])
        row.spacing = 12
        size.setContentHuggingPriority(.defaultLow, for: .horizontal)
        align.setContentHuggingPriority(.required, for: .horizontal)
        stack.addArrangedSubview(row)
        row.widthAnchor.constraint(equalTo: stack.widthAnchor, constant: -40).isActive = true

        // Fonts, each name in its own font.
        fontsRow.spacing = 8
        for (i, name) in familyNames.enumerated() {
            let chip = FlatButton(label(name, bookFont(familyFonts[i], 15), 0))
            fixed(chip, height: 40)
            chip.layer!.cornerRadius = 20
            chip.layer!.borderWidth = 1
            chip.widthAnchor.constraint(equalTo: chip.content.widthAnchor, constant: 32).isActive = true
            chip.onClick = { [weak self] in self?.set { $0.family = UInt32(i) } }
            fontsRow.addArrangedSubview(chip)
            chips.append(chip)
        }
        stack.addArrangedSubview(fontsRow)

        // Line spacing and margins; pages.
        let (spacingColumn, c1) = captioned(tr("Line spacing"), spacing)
        let (marginsColumn, c2) = captioned(tr("Margins"), margins)
        let pair = NSStackView(views: [spacingColumn, marginsColumn])
        pair.spacing = 16
        pair.distribution = .fillEqually
        stack.addArrangedSubview(pair)
        pair.widthAnchor.constraint(equalTo: stack.widthAnchor, constant: -40).isActive = true
        let (pagesColumn, c3) = captioned(tr("Pages"), pages)
        stack.addArrangedSubview(pagesColumn)
        pagesColumn.widthAnchor.constraint(equalTo: stack.widthAnchor, constant: -40).isActive = true
        captions = [c1, c2, c3]

        for (i, b) in align.buttons.enumerated() { b.onClick = { [weak self] in self?.set { $0.align = UInt32(i) } } }
        for (i, b) in spacing.buttons.enumerated() { b.onClick = { [weak self] in self?.set { $0.spacing = UInt32(i) } } }
        for (i, b) in margins.buttons.enumerated() { b.onClick = { [weak self] in self?.set { $0.margins = UInt32(i) } } }
        for (i, b) in pages.buttons.enumerated() { b.onClick = { [weak self] in self?.set { $0.turning = UInt32(i) } } }
        controls = [(themesRow, 0), (size, 2), (align, 4), (fontsRow, 1), (spacing, 3), (margins, 5), (pages, 6)]
        keysRing.borderWidth = 2
        keysRing.cornerRadius = 12
        layer!.addSublayer(keysRing)
    }

    required init?(coder: NSCoder) { fatalError() }

    override var acceptsFirstResponder: Bool { true }

    @objc private func pickTheme(_ g: NSClickGestureRecognizer) {
        guard let i = themesRow.arrangedSubviews.firstIndex(where: { $0 === g.view }) else { return }
        set { $0.theme = Int32(i) }
    }

    private func set(_ change: (inout ForzacSettings) -> Void) {
        guard let view else { return }
        var s = view.settings
        change(&s)
        view.apply(s)
        show()
    }

    private func change(_ setting: UInt32, _ delta: Int32) {
        view?.step(setting, delta)
        show()
    }

    /// Marks the current values in the current theme.
    func show() {
        guard let view else { return }
        let s = view.settings
        let p = view.palette
        colour(p)
        for control in [size, align, spacing, margins, pages] { control.colour(p) }
        sizeLabel.stringValue = String(format: "%g", withUnsafePointer(to: s) { forzac_settings_text_size($0) })
        sizeLabel.textColor = nsColor(p.muted)
        size.buttons[0].enabled = s.size > 0
        size.buttons[2].enabled = s.size + 1 < forzac_settings_size_count()
        align.select(Int(s.align))
        spacing.select(Int(s.spacing))
        margins.select(Int(s.margins))
        pages.select(Int(s.turning))
        for (i, chip) in chips.enumerated() {
            chip.colour(p)
            chip.selected = i == Int(s.family)
            chip.layer!.borderColor = color(chip.selected ? p.ink : p.line)
        }
        for l in circleLabels + captions { l.textColor = nsColor(p.muted) }
        ring.borderColor = color(p.ink)
        keysRing.borderColor = color(p.ink, alpha: 0.55)
        layoutSubtreeIfNeeded()
        placeRings()
    }

    override func layout() {
        super.layout()
        placeRings()
    }

    /// The ring around the shown theme (a 4 px gap, a 2 px ring) and the outline of the control the keys change.
    private func placeRings() {
        guard let view, view.theme < circles.count else { return }
        CATransaction.begin()
        CATransaction.setDisableActions(true)
        let circle = circles[view.theme]
        ring.frame = layerFrame(of: circle).insetBy(dx: -6, dy: -6)
        // Around the theme circles with room for the shown theme's ring (6 px beyond its circle).
        let room: CGFloat = controls[current].0 === themesRow ? -10 : -4
        keysRing.frame = layerFrame(of: controls[current].0).insetBy(dx: room, dy: room)
        CATransaction.commit()
    }

    /// A subview's frame in this view's layer (y down, like the view: AppKit flips the layer of a flipped view).
    private func layerFrame(of v: NSView) -> CGRect { v.convert(v.bounds, to: self) }

    override func keyDown(with e: NSEvent) {
        switch e.shortcutKey {
        case 0x26, 0x7D: keysUsed(); current = min(controls.count - 1, current + 1); placeRings() // j ↓
        case 0x28, 0x7E: keysUsed(); current = max(0, current - 1); placeRings()                    // k ↑
        case 0x25, 0x7C: keysUsed(); change(controls[current].1, 1)                                 // l →
        case 0x04, 0x7B: keysUsed(); change(controls[current].1, -1)                                // h ←
        case 0x35, 0x01, 0x0C: close()                                                  // Esc s q
        default: super.keyDown(with: e)
        }
    }

    func open() {
        isHidden = false
        keysRing.isHidden = true
        show()
        window?.makeFirstResponder(self)
    }

    /// The outline of the control the keys change shows once the keys are used, not for the mouse.
    private func keysUsed() {
        keysRing.isHidden = false
    }

    func close() {
        isHidden = true
        if let view { window?.makeFirstResponder(view) }
    }
}

// MARK: - Contents

/// The tabs of the contents panel.
enum ContentsTab: Int, CaseIterable {
    case contents, bookmarks, highlights
    var title: String { [tr("Contents"), tr("Bookmarks"), tr("Highlights")][rawValue] }
}

/// The contents: the tabs with a close button, a filter field, the book, then the entries with their position in the
/// book (the current one bold and marked), or the bookmarks. `j`/`k` move, `Enter` jumps (remembered for `⌃O`), `/`
/// filters, `←`/`→` switch tabs, `d` removes a bookmark, `Esc` or `o` closes.
final class ContentsPanel: LayerView, NSTableViewDataSource, NSTableViewDelegate, NSTextFieldDelegate {
    private weak var view: PageView?
    private let entries: [(title: String, level: UInt32, at: ForzacLocator, position: Float)]
    private var shown: [Int] = []
    private var currentEntry = 0
    private(set) var tab = ContentsTab.contents
    private let tabs = ContentsTab.allCases.map { FlatButton(label($0.title, interfaceFont(17, .medium), 0)) }
    /// The bookmarks as listed, and the hint shown when there are none.
    private var marks: [(at: ForzacLocator, section: String, position: Float, snippet: String)] = []
    /// The highlights as listed: range, text and "section · N%".
    private var quotes: [(range: LocRange, text: String, place: String)] = []
    /// Shares all highlights (the header, on the "Highlights" tab when there are any).
    private let share = FlatButton(IconView(Icons.share, size: 20))
    private let empty = NSStackView()
    private let emptyTitle = label("", bookFont("PT Serif", 20), 0)
    private let emptyBody = label("", interfaceFont(14), 0)
    private var fieldHeight: NSLayoutConstraint!
    private var cardTop: NSLayoutConstraint!
    private let close = FlatButton(IconView(Icons.x, size: 22))
    private let field = LayerView()
    private let searchIcon = IconView(Icons.search, size: 18)
    private let filter = NSTextField()
    private let bookTitle: NSTextField
    private let meta = label("", interfaceFont(13), 0)
    private let rule = LayerView()
    private let table = ContentsTable()
    private let scroll = NSScrollView()
    private var palette = themes[0]

    init(view: PageView) {
        self.view = view
        var n: UInt32 = 0
        let list = forzac_contents(view.doc, &n)!
        entries = (0..<Int(n)).map { (String(cString: list[$0].title), list[$0].level, list[$0].at, list[$0].position) }
        bookTitle = label(String(cString: forzac_title(view.doc)), bookFont("PT Serif", 18, bold: true), 0)
        super.init()
        layer!.shadowOpacity = 1
        layer!.shadowRadius = 16

        fixed(close, width: 44, height: 44)
        close.onClick = { [weak self] in self?.dismiss() }
        let spacer = NSView()
        spacer.setContentHuggingPriority(.defaultLow, for: .horizontal)
        for (t, button) in zip(ContentsTab.allCases, tabs) {
            button.layer!.cornerRadius = 8
            button.widthAnchor.constraint(equalTo: button.content.widthAnchor, constant: 24).isActive = true
            button.heightAnchor.constraint(equalToConstant: 44).isActive = true
            button.onClick = { [weak self] in self?.show(t) }
        }
        fixed(share, width: 44, height: 44)
        share.onClick = { [weak self] in self?.shareAll() }
        share.isHidden = true
        let header = NSStackView(views: tabs + [spacer, share, close])
        header.spacing = 0
        header.distribution = .fill
        header.edgeInsets = NSEdgeInsets(top: 4, left: 8, bottom: 0, right: 8)

        fieldHeight = field.heightAnchor.constraint(equalToConstant: 44)
        fieldHeight.isActive = true
        field.layer!.cornerRadius = 22
        field.layer!.borderWidth = 1
        filter.isBordered = false
        filter.drawsBackground = false
        filter.focusRingType = .none
        filter.font = interfaceFont(15)
        filter.delegate = self
        filter.translatesAutoresizingMaskIntoConstraints = false
        searchIcon.translatesAutoresizingMaskIntoConstraints = false
        field.addSubview(searchIcon)
        field.addSubview(filter)
        NSLayoutConstraint.activate([
            searchIcon.leadingAnchor.constraint(equalTo: field.leadingAnchor, constant: 14), searchIcon.centerYAnchor.constraint(equalTo: field.centerYAnchor),
            searchIcon.widthAnchor.constraint(equalToConstant: 18), searchIcon.heightAnchor.constraint(equalToConstant: 18),
            filter.leadingAnchor.constraint(equalTo: searchIcon.trailingAnchor, constant: 10), filter.trailingAnchor.constraint(equalTo: field.trailingAnchor, constant: -14),
            filter.centerYAnchor.constraint(equalTo: field.centerYAnchor),
        ])

        bookTitle.maximumNumberOfLines = 2
        bookTitle.lineBreakMode = .byTruncatingTail
        bookTitle.cell?.wraps = true
        bookTitle.setContentCompressionResistancePriority(.defaultLow, for: .horizontal)
        let about = NSStackView(views: [bookTitle, meta])
        about.orientation = .vertical
        about.alignment = .leading
        about.spacing = 3
        let card = NSStackView(views: [cover(of: view), about])
        card.spacing = 14
        card.alignment = .centerY

        fixed(rule, height: 1)
        table.addTableColumn(NSTableColumn(identifier: .init("entry")))
        table.headerView = nil
        table.backgroundColor = .clear
        table.intercellSpacing = .zero
        table.selectionHighlightStyle = .regular
        table.dataSource = self
        table.delegate = self
        table.target = self
        table.doubleAction = #selector(jump)
        table.panel = self
        scroll.documentView = table
        scroll.drawsBackground = false
        scroll.hasVerticalScroller = true
        scroll.translatesAutoresizingMaskIntoConstraints = false

        emptyBody.alignment = .center
        emptyBody.maximumNumberOfLines = 0
        emptyBody.lineBreakMode = .byWordWrapping
        emptyBody.cell?.wraps = true
        emptyBody.preferredMaxLayoutWidth = 240
        empty.orientation = .vertical
        empty.spacing = 8
        empty.addArrangedSubview(emptyTitle)
        empty.addArrangedSubview(emptyBody)
        empty.isHidden = true
        for v in [header, field, card, rule, scroll, empty] as [NSView] {
            v.translatesAutoresizingMaskIntoConstraints = false
            addSubview(v)
        }
        cardTop = card.topAnchor.constraint(equalTo: field.bottomAnchor, constant: 12)
        NSLayoutConstraint.activate([
            header.topAnchor.constraint(equalTo: topAnchor), header.leadingAnchor.constraint(equalTo: leadingAnchor), header.trailingAnchor.constraint(equalTo: trailingAnchor),
            field.topAnchor.constraint(equalTo: header.bottomAnchor, constant: 8), field.leadingAnchor.constraint(equalTo: leadingAnchor, constant: 20),
            field.trailingAnchor.constraint(equalTo: trailingAnchor, constant: -20),
            cardTop, card.leadingAnchor.constraint(equalTo: leadingAnchor, constant: 20),
            card.trailingAnchor.constraint(lessThanOrEqualTo: trailingAnchor, constant: -20),
            rule.topAnchor.constraint(equalTo: card.bottomAnchor, constant: 22), rule.leadingAnchor.constraint(equalTo: leadingAnchor), rule.trailingAnchor.constraint(equalTo: trailingAnchor),
            scroll.topAnchor.constraint(equalTo: rule.bottomAnchor, constant: 6), scroll.leadingAnchor.constraint(equalTo: leadingAnchor),
            scroll.trailingAnchor.constraint(equalTo: trailingAnchor), scroll.bottomAnchor.constraint(equalTo: bottomAnchor),
            empty.centerXAnchor.constraint(equalTo: scroll.centerXAnchor), empty.centerYAnchor.constraint(equalTo: scroll.centerYAnchor, constant: -40),
        ])
        refilter()
    }

    required init?(coder: NSCoder) { fatalError() }

    /// The book's cover, 44×66: its own picture, else a plain one with the title.
    private func cover(of view: PageView) -> NSView {
        let box = LayerView()
        fixed(box, width: 44, height: 66)
        box.layer!.cornerRadius = 3
        box.layer!.masksToBounds = true
        var len = 0
        if let bytes = forzac_cover(view.doc, &len), let source = CGImageSourceCreateWithData(Data(bytes: bytes, count: len) as CFData, nil),
           let image = CGImageSourceCreateImageAtIndex(source, 0, nil) {
            box.layer!.contents = image
            box.layer!.contentsGravity = .resizeAspectFill
        } else {
            box.layer!.backgroundColor = color(0x2F3E46)
            let t = label(String(cString: forzac_title(view.doc)), bookFont("PT Serif", 8, bold: true), 0xF3EFE6)
            t.maximumNumberOfLines = 5
            t.cell?.wraps = true
            t.preferredMaxLayoutWidth = 34
            box.addSubview(t)
            NSLayoutConstraint.activate([t.leadingAnchor.constraint(equalTo: box.leadingAnchor, constant: 5), t.trailingAnchor.constraint(equalTo: box.trailingAnchor, constant: -5),
                                         t.bottomAnchor.constraint(equalTo: box.bottomAnchor, constant: -5)])
        }
        return box
    }

    func colour(_ p: Palette) {
        palette = p
        layer!.backgroundColor = color(p.paper)
        layer!.shadowColor = color(0x000000, alpha: 0.08)
        layer!.borderWidth = 0
        for (t, button) in zip(ContentsTab.allCases, tabs) {
            let selected = t == tab
            (button.content as? NSTextField)?.font = interfaceFont(17, selected ? .semibold : .medium)
            button.colour(p)
            (button.content as? NSTextField)?.textColor = nsColor(selected ? p.ink : p.muted)
        }
        share.colour(p)
        emptyTitle.textColor = nsColor(p.ink)
        emptyBody.textColor = nsColor(p.muted)
        close.colour(p)
        field.layer!.backgroundColor = color(p.sheet)
        field.layer!.borderColor = color(p.line)
        searchIcon.tint = nsColor(p.muted)
        filter.textColor = nsColor(p.ink)
        filter.placeholderAttributedString = NSAttributedString(string: tr("Filter contents"), attributes: [.foregroundColor: nsColor(p.muted), .font: interfaceFont(15)])
        bookTitle.textColor = nsColor(p.ink)
        meta.textColor = nsColor(p.muted)
        rule.layer!.backgroundColor = color(p.line)
        table.selectionColor = color(p.ink, alpha: 0.07)
        table.reloadData()
    }

    /// Shows the panel on `tab`: the contents with the entry of the current place selected, or the bookmarks.
    func open(_ tab: ContentsTab) {
        guard let view else { return }
        let here = view.locator
        let before: (ForzacLocator) -> Bool = { ($0.chapter, $0.block, $0.offset) <= (here.chapter, here.block, here.offset) }
        currentEntry = entries.lastIndex { before($0.at) } ?? 0
        let read = Int((forzac_progress(view.doc, here) * 100).rounded())
        let author = String(cString: forzac_authors(view.doc)).components(separatedBy: ", ").first ?? ""
        meta.stringValue = [author, fill(tr("{n}% read"), ["n": "\(read)"])].filter { !$0.isEmpty }.joined(separator: " · ")
        filter.stringValue = ""
        refilter()
        isHidden = false
        show(tab)
    }

    /// Shows `tab` (the bookmarks read afresh) and gives it the keys.
    func show(_ tab: ContentsTab) {
        guard let view else { return }
        self.tab = tab
        let contents = tab == .contents
        field.isHidden = !contents
        fieldHeight.constant = contents ? 44 : 0
        cardTop.constant = contents ? 12 : 0
        colour(view.palette)
        share.isHidden = true
        if contents {
            empty.isHidden = true
            table.reloadData()
            select(shown.firstIndex(of: currentEntry) ?? 0)
        } else if tab == .bookmarks {
            emptyTitle.stringValue = tr("No bookmarks yet")
            emptyBody.stringValue = tr("Press b on a page — a ribbon marks the place.")
            reloadBookmarks(selecting: 0)
        } else {
            emptyTitle.stringValue = tr("No highlights yet")
            emptyBody.stringValue = tr("Select text with the mouse, or press v on a page, then choose Highlight.")
            reloadHighlights(selecting: 0)
        }
        window?.makeFirstResponder(table)
    }

    private func reloadHighlights(selecting row: Int) {
        guard let view else { return }
        var n: UInt32 = 0
        let list = forzac_quotes(view.doc, &n)
        quotes = (0..<Int(n)).map { i in
            let q = list![i]
            return (LocRange(start: q.start, end: q.end), String(cString: q.text), sectionPlace(view, q.section.map { String(cString: $0) }, q.start, q.position))
        }
        empty.isHidden = !quotes.isEmpty
        share.isHidden = quotes.isEmpty
        table.reloadData()
        select(row)
    }

    /// `d`: deletes the selected highlight; the one after it is selected.
    func deleteHighlight() {
        guard let view, tab == .highlights, quotes.indices.contains(table.selectedRow) else { return }
        let row = table.selectedRow
        view.deleteHighlight(quotes[row].range)
        reloadHighlights(selecting: row)
    }

    /// `c`: copies the selected highlight's text.
    func copyHighlight() {
        guard tab == .highlights, quotes.indices.contains(table.selectedRow) else { return }
        NSPasteboard.general.clearContents()
        NSPasteboard.general.setString(quotes[table.selectedRow].text, forType: .string)
    }

    /// `⌘C`: all highlights in the export format.
    func copyAll() {
        guard let view, tab == .highlights, !quotes.isEmpty else { return }
        NSPasteboard.general.clearContents()
        NSPasteboard.general.setString(String(cString: forzac_quotes_export(view.doc)), forType: .string)
    }

    private func shareAll() {
        guard let view, !quotes.isEmpty else { return }
        let text = String(cString: forzac_quotes_export(view.doc))
        NSSharingServicePicker(items: [text]).show(relativeTo: share.bounds, of: share, preferredEdge: .minY)
    }

    /// The next (`delta` 1) or previous (−1) tab.
    func switchTab(_ delta: Int) {
        let n = ContentsTab.allCases.count
        show(ContentsTab(rawValue: ((tab.rawValue + delta) % n + n) % n)!)
    }

    private func reloadBookmarks(selecting row: Int) {
        guard let view else { return }
        var n: UInt32 = 0
        let list = forzac_bookmarks(view.doc, &n)
        marks = (0..<Int(n)).map { i in
            let b = list![i]
            return (b.at, b.section.map { String(cString: $0) } ?? "", b.position, String(cString: b.snippet))
        }
        empty.isHidden = !marks.isEmpty
        table.reloadData()
        select(row)
    }

    /// `d`: removes the selected bookmark; the one after it is selected.
    func removeBookmark() {
        guard let view, tab == .bookmarks, marks.indices.contains(table.selectedRow) else { return }
        let row = table.selectedRow
        view.removeBookmark(marks[row].at)
        reloadBookmarks(selecting: row)
    }

    func dismiss() {
        isHidden = true
        if let view { window?.makeFirstResponder(view) }
    }

    private func refilter() {
        let q = fold(filter.stringValue)
        shown = entries.indices.filter { q.isEmpty || fold(entries[$0].title).contains(q) }
        table.reloadData()
    }

    func select(_ row: Int) {
        let count = numberOfRows(in: table)
        guard count > 0 else { return }
        let row = min(max(0, row), count - 1)
        table.selectRowIndexes([row], byExtendingSelection: false)
        table.scrollRowToVisible(row)
    }

    @objc func jump() {
        guard let view, table.selectedRow >= 0, table.selectedRow < numberOfRows(in: table) else { return }
        let at = tab == .contents ? entries[shown[table.selectedRow]].at : tab == .bookmarks ? marks[table.selectedRow].at : quotes[table.selectedRow].range.start
        dismiss()
        view.go(to: at)
    }

    func focusFilter() { window?.makeFirstResponder(filter) }

    func numberOfRows(in tableView: NSTableView) -> Int { tab == .contents ? shown.count : tab == .bookmarks ? marks.count : quotes.count }

    private func quoteText(_ text: String) -> NSAttributedString {
        let style = NSMutableParagraphStyle()
        style.minimumLineHeight = 23
        style.maximumLineHeight = 23
        style.lineBreakMode = .byWordWrapping
        return NSAttributedString(string: text, attributes: [.font: bookFont("PT Serif", 16), .foregroundColor: nsColor(palette.ink), .paragraphStyle: style])
    }

    /// A highlight as Android lists it: an ochre rule, the text (six lines at most), "section · N%" below.
    private func highlightCell(_ row: Int) -> NSView {
        let q = quotes[row]
        let rule = LayerView()
        rule.layer!.backgroundColor = color(0xC49A52)
        rule.layer!.cornerRadius = 1.5
        let text = NSTextField(labelWithAttributedString: quoteText(q.text))
        text.maximumNumberOfLines = 6
        text.cell?.wraps = true
        text.cell?.truncatesLastVisibleLine = true
        text.preferredMaxLayoutWidth = markWidth - 17
        let place = label(q.place, interfaceFont(13), palette.muted)
        place.lineBreakMode = .byTruncatingTail
        let column = NSStackView(views: [text, place])
        column.orientation = .vertical
        column.alignment = .leading
        column.spacing = 6
        let cell = NSView()
        for v in [rule, column] as [NSView] {
            v.translatesAutoresizingMaskIntoConstraints = false
            cell.addSubview(v)
        }
        NSLayoutConstraint.activate([
            rule.leadingAnchor.constraint(equalTo: cell.leadingAnchor, constant: 20), rule.widthAnchor.constraint(equalToConstant: 3),
            rule.topAnchor.constraint(equalTo: cell.topAnchor, constant: 14), rule.bottomAnchor.constraint(equalTo: cell.bottomAnchor, constant: -14),
            column.leadingAnchor.constraint(equalTo: rule.trailingAnchor, constant: 14), column.trailingAnchor.constraint(equalTo: cell.trailingAnchor, constant: -20),
            column.topAnchor.constraint(equalTo: cell.topAnchor, constant: 14),
            text.widthAnchor.constraint(equalTo: column.widthAnchor), place.widthAnchor.constraint(equalTo: column.widthAnchor),
        ])
        return cell
    }

    /// The room for a bookmark's text: the panel less the margins.
    private var markWidth: CGFloat { max(100, (table.bounds.width > 0 ? table.bounds.width : 400) - 40) }

    private func snippet(_ text: String) -> NSAttributedString {
        let style = NSMutableParagraphStyle()
        style.minimumLineHeight = 18
        style.maximumLineHeight = 18
        style.lineBreakMode = .byWordWrapping
        return NSAttributedString(string: text, attributes: [.font: interfaceFont(13), .foregroundColor: nsColor(palette.muted), .paragraphStyle: style])
    }

    /// A bookmark as Android lists it: the section with the position on the right, two lines of text below.
    private func bookmarkCell(_ row: Int) -> NSView {
        let m = marks[row]
        let section = label(m.section, bookFont("PT Serif", 16), palette.ink)
        section.lineBreakMode = .byTruncatingTail
        section.setContentCompressionResistancePriority(.defaultLow, for: .horizontal)
        let p = label(view.map { pageOrPercent($0, m.at, m.position) } ?? "", interfaceFont(13), palette.muted)
        p.alignment = .right
        fixed(p, width: 44)
        let top = NSStackView(views: [section, p])
        top.distribution = .fill
        let text = NSTextField(labelWithAttributedString: snippet(m.snippet))
        text.maximumNumberOfLines = 2
        text.cell?.wraps = true
        text.cell?.truncatesLastVisibleLine = true
        text.preferredMaxLayoutWidth = markWidth
        let column = NSStackView(views: [top, text])
        column.orientation = .vertical
        column.alignment = .leading
        column.spacing = 4
        column.edgeInsets = NSEdgeInsets(top: 12, left: 20, bottom: 12, right: 20)
        for v in [top, text] { v.widthAnchor.constraint(equalTo: column.widthAnchor, constant: -40).isActive = true }
        return column
    }

    /// The room for an entry's title: the panel less the margins, the indent, the dot and the position.
    private func titleWidth(level: UInt32) -> CGFloat {
        let width = table.bounds.width > 0 ? table.bounds.width : 400
        return max(40, width - 40 - 16 * CGFloat(min(4, max(0, Int(level) - 1))) - 18 - 44)
    }

    func tableView(_ tableView: NSTableView, heightOfRow row: Int) -> CGFloat {
        if tab == .highlights {
            let h: CGFloat = quoteText(quotes[row].text).boundingRect(with: NSSize(width: markWidth - 17, height: 1000), options: [.usesLineFragmentOrigin]).height
            let text: CGFloat = min(ceil(h), 6 * 23)
            return text + 52 // 14 above, 6 and the 18 of "section · N%", 14 below
        }
        if tab == .bookmarks {
            let h = snippet(marks[row].snippet).boundingRect(with: NSSize(width: markWidth, height: 1000), options: [.usesLineFragmentOrigin]).height
            return 12 + 21 + 4 + min(ceil(h), 36) + 12
        }
        let e = entries[shown[row]]
        let font = bookFont("PT Serif", 16, bold: shown[row] == currentEntry)
        let h = (e.title as NSString).boundingRect(with: NSSize(width: titleWidth(level: e.level), height: 1000), options: [.usesLineFragmentOrigin], attributes: [.font: font]).height
        let line = ceil(font.ascender - font.descender + font.leading)
        return max(48, min(h, 2 * line) + 16) // at most two lines, 8 above and below
    }

    func tableView(_ tableView: NSTableView, viewFor tableColumn: NSTableColumn?, row: Int) -> NSView? {
        if tab == .bookmarks { return bookmarkCell(row) }
        if tab == .highlights { return highlightCell(row) }
        let i = shown[row]
        let e = entries[i]
        let isCurrent = i == currentEntry
        let cell = NSView()
        let t = label(e.title, bookFont("PT Serif", 16, bold: isCurrent), palette.ink)
        t.maximumNumberOfLines = 2
        t.lineBreakMode = .byWordWrapping
        t.cell?.truncatesLastVisibleLine = true
        t.preferredMaxLayoutWidth = titleWidth(level: e.level)
        let p = label(view.map { pageOrPercent($0, e.at, e.position) } ?? "", interfaceFont(13), palette.muted)
        p.alignment = .right
        cell.addSubview(t)
        cell.addSubview(p)
        let indent = 20 + 16 * CGFloat(min(4, max(0, Int(e.level) - 1)))
        NSLayoutConstraint.activate([
            t.leadingAnchor.constraint(equalTo: cell.leadingAnchor, constant: indent), t.centerYAnchor.constraint(equalTo: cell.centerYAnchor),
            t.widthAnchor.constraint(lessThanOrEqualToConstant: titleWidth(level: e.level)),
            p.trailingAnchor.constraint(equalTo: cell.trailingAnchor, constant: -20), p.centerYAnchor.constraint(equalTo: cell.centerYAnchor),
            p.widthAnchor.constraint(equalToConstant: 44),
        ])
        if isCurrent {
            let dot = LayerView()
            fixed(dot, width: 6, height: 6)
            dot.layer!.cornerRadius = 3
            dot.layer!.backgroundColor = color(palette.ink)
            cell.addSubview(dot)
            NSLayoutConstraint.activate([dot.trailingAnchor.constraint(equalTo: p.leadingAnchor), dot.centerYAnchor.constraint(equalTo: cell.centerYAnchor)])
        }
        return cell
    }

    func tableView(_ tableView: NSTableView, rowViewForRow row: Int) -> NSTableRowView? {
        let r = ContentsRow()
        r.selection = table.selectionColor
        return r
    }

    func controlTextDidChange(_ obj: Notification) {
        refilter()
        select(0)
    }

    func control(_ control: NSControl, textView: NSTextView, doCommandBy selector: Selector) -> Bool {
        switch selector {
        case #selector(NSResponder.cancelOperation(_:)):
            if filter.stringValue.isEmpty { window?.makeFirstResponder(table) } else { filter.stringValue = ""; refilter(); select(0) }
            return true
        case #selector(NSResponder.moveDown(_:)):
            window?.makeFirstResponder(table)
            return true
        case #selector(NSResponder.insertNewline(_:)):
            jump()
            return true
        default:
            return false
        }
    }
}

/// The contents list: `j`/`k` move, `Enter` jumps, `/` filters, `←`/`→` switch tabs, `d` removes a bookmark, `Esc`
/// and `o` close.
final class ContentsTable: NSTableView {
    weak var panel: ContentsPanel?
    var selectionColor = color(0x000000, alpha: 0.07)

    override func keyDown(with e: NSEvent) {
        let shift = e.modifierFlags.contains(.shift)
        let bookmarks = panel?.tab == .bookmarks
        let highlights = panel?.tab == .highlights
        switch e.shortcutKey {
        case 0x26: panel?.select(selectedRow + 1)        // j
        case 0x28: panel?.select(selectedRow - 1)        // k
        case 0x24, 0x4C: panel?.jump()                    // Return
        case 0x2C where !shift && panel?.tab == .contents: panel?.focusFilter() // /
        case 0x30: panel?.switchTab(shift ? -1 : 1)      // Tab
        case 0x7C, 0x25: panel?.switchTab(1)             // → l
        case 0x7B, 0x04: panel?.switchTab(-1)            // ← h
        case 0x02, 0x33, 0x75: if highlights { panel?.deleteHighlight() } else { panel?.removeBookmark() } // d Delete
        case 0x08 where highlights && !e.modifierFlags.contains(.command): panel?.copyHighlight() // c
        case 0x0B where shift: if bookmarks { panel?.dismiss() } else { panel?.show(.bookmarks) } // B
        case 0x35, 0x1F, 0x0C: panel?.dismiss()           // Esc o q
        default: super.keyDown(with: e)
        }
    }

    /// Edit → Copy (`⌘C`) on the "Highlights" tab: all highlights.
    @objc func copy(_ sender: Any?) { panel?.copyAll() }
}

final class ContentsRow: NSTableRowView {
    var selection = color(0x000000, alpha: 0.07)

    override func drawSelection(in dirtyRect: NSRect) {
        NSGraphicsContext.current?.cgContext.setFillColor(selection)
        NSGraphicsContext.current?.cgContext.fill(bounds)
    }

    override var isEmphasized: Bool { get { false } set {} }
}

// MARK: - Note

/// A note in a bottom sheet: a grabber, "NOTE" with the note's title, then the note typeset by the core.
/// `Esc`, `Enter`, `Space` or `q` closes it.
final class NoteSheet: SheetView {
    private weak var view: PageView?
    /// What gets the keys back on closing: the image or table view the note was opened from, else the page.
    weak var back: NSView?

    /// Note `id`, or with `preview` the short target of link `id` (a figure, a table, a paragraph) without a label.
    init?(view: PageView, id: String, preview: Bool = false, windowSize: CGSize) {
        self.view = view
        let width = min(sheetWidth, windowSize.width) - 48
        var page = ForzacPage()
        let height = withUnsafePointer(to: view.settings) {
            preview ? forzac_link_preview(view.doc, id, Float(width), $0, &page) : forzac_note(view.doc, id, Float(width), $0, &page)
        }
        guard height > 0 else { return nil }
        let p = view.palette
        let scale = NSScreen.main?.backingScaleFactor ?? 2
        var pictures: [UnsafeRawPointer: CGImage] = [:]
        let image = bitmap(CGSize(width: width, height: CGFloat(height)), scale: scale, background: nil) { ctx in
            drawPage(page, in: ctx, height: CGFloat(height), palette: p, fonts: view.fonts, pictures: &pictures)
        }
        super.init(spacing: 12, insets: NSEdgeInsets(top: 10, left: 24, bottom: 36, right: 24))
        addGrabber()
        if !preview {
            let title = forzac_note_title(view.doc, id).map { String(cString: $0) } ?? ""
            let caption = label([tr("Note"), title].filter { !$0.isEmpty }.joined(separator: " ").uppercased(), interfaceFont(12, .medium), p.muted)
            caption.attributedStringValue = NSAttributedString(string: caption.stringValue, attributes: [.kern: 0.72, .font: interfaceFont(12, .medium), .foregroundColor: nsColor(p.muted)])
            stack.addArrangedSubview(caption)
        }
        let text = LayerView()
        text.layer!.contents = image
        text.layer!.contentsScale = scale
        fixed(text, width: width, height: CGFloat(height))
        let scroll = NSScrollView()
        scroll.documentView = text
        scroll.drawsBackground = false
        scroll.hasVerticalScroller = CGFloat(height) > windowSize.height * 0.6 - 80
        scroll.translatesAutoresizingMaskIntoConstraints = false
        stack.addArrangedSubview(scroll)
        NSLayoutConstraint.activate([
            scroll.widthAnchor.constraint(equalToConstant: width),
            scroll.heightAnchor.constraint(equalToConstant: min(CGFloat(height), windowSize.height * 0.6 - 80)),
        ])
        colour(p)
    }

    required init?(coder: NSCoder) { fatalError() }

    override var acceptsFirstResponder: Bool { true }

    override func keyDown(with e: NSEvent) {
        switch e.shortcutKey {
        case 0x35, 0x24, 0x4C, 0x31, 0x0C: dismiss() // Esc Return Space q
        default: super.keyDown(with: e)
        }
    }

    func dismiss() {
        removeFromSuperview()
        if let next = back ?? view { next.window?.makeFirstResponder(next) }
    }
}

// MARK: - Search

/// The search panel, laid out like the contents: the field, the count, then the matches (a snippet with the match in
/// bold, the section and the position). The book is searched as the reader types, on a background queue. `Return` or a
/// double-click goes to a match (remembered for `⌃O`), `↓` moves into the list, `j`/`k` move in it, `Esc` clears the
/// field or closes the panel.
final class SearchPanel: LayerView, NSTableViewDataSource, NSTableViewDelegate, NSTextFieldDelegate {
    struct Hit {
        let start, end: ForzacLocator
        let snippet: NSAttributedString
        let section: String?
        let position: Float
    }

    private weak var view: PageView?
    private let book: OpaquePointer
    private var hits: [Hit] = []
    /// The query of the search running or shown: a slower older search does not overwrite a newer one.
    private var query = ""
    private var pending: DispatchWorkItem?
    private let field = LayerView()
    private let searchIcon = IconView(Icons.search, size: 18)
    private let input = NSTextField()
    private let clear = FlatButton(IconView(Icons.x, size: 18))
    private let count = label("", interfaceFont(13), 0)
    private let table = SearchTable()
    private let scroll = NSScrollView()
    private var palette = themes[0]

    init(view: PageView) {
        self.view = view
        book = forzac_book(view.doc)
        super.init()
        layer!.shadowOpacity = 1
        layer!.shadowRadius = 16

        fixed(field, height: 44)
        field.layer!.cornerRadius = 22
        field.layer!.borderWidth = 1
        input.isBordered = false
        input.drawsBackground = false
        input.focusRingType = .none
        input.font = interfaceFont(15)
        input.delegate = self
        fixed(clear, width: 28, height: 28)
        clear.onClick = { [weak self] in
            self?.input.stringValue = ""
            self?.changed()
        }
        for v in [searchIcon, input, clear] as [NSView] {
            v.translatesAutoresizingMaskIntoConstraints = false
            field.addSubview(v)
        }
        NSLayoutConstraint.activate([
            searchIcon.leadingAnchor.constraint(equalTo: field.leadingAnchor, constant: 14), searchIcon.centerYAnchor.constraint(equalTo: field.centerYAnchor),
            searchIcon.widthAnchor.constraint(equalToConstant: 18), searchIcon.heightAnchor.constraint(equalToConstant: 18),
            input.leadingAnchor.constraint(equalTo: searchIcon.trailingAnchor, constant: 10), input.trailingAnchor.constraint(equalTo: clear.leadingAnchor, constant: -6),
            input.centerYAnchor.constraint(equalTo: field.centerYAnchor),
            clear.trailingAnchor.constraint(equalTo: field.trailingAnchor, constant: -8), clear.centerYAnchor.constraint(equalTo: field.centerYAnchor),
        ])

        table.addTableColumn(NSTableColumn(identifier: .init("hit")))
        table.headerView = nil
        table.backgroundColor = .clear
        table.intercellSpacing = .zero
        table.dataSource = self
        table.delegate = self
        table.target = self
        table.doubleAction = #selector(pick)
        table.panel = self
        scroll.documentView = table
        scroll.drawsBackground = false
        scroll.hasVerticalScroller = true

        for v in [field, count, scroll] as [NSView] {
            v.translatesAutoresizingMaskIntoConstraints = false
            addSubview(v)
        }
        NSLayoutConstraint.activate([
            field.topAnchor.constraint(equalTo: topAnchor, constant: 16), field.leadingAnchor.constraint(equalTo: leadingAnchor, constant: 20),
            field.trailingAnchor.constraint(equalTo: trailingAnchor, constant: -20),
            count.topAnchor.constraint(equalTo: field.bottomAnchor, constant: 14), count.leadingAnchor.constraint(equalTo: leadingAnchor, constant: 20),
            scroll.topAnchor.constraint(equalTo: count.bottomAnchor, constant: 6), scroll.leadingAnchor.constraint(equalTo: leadingAnchor),
            scroll.trailingAnchor.constraint(equalTo: trailingAnchor), scroll.bottomAnchor.constraint(equalTo: bottomAnchor),
        ])
    }

    required init?(coder: NSCoder) { fatalError() }

    deinit { forzac_book_release(book) }

    func colour(_ p: Palette) {
        palette = p
        layer!.backgroundColor = color(p.paper)
        layer!.shadowColor = color(0x000000, alpha: 0.08)
        field.layer!.backgroundColor = color(p.sheet)
        field.layer!.borderColor = color(p.line)
        searchIcon.tint = nsColor(p.muted)
        clear.colour(p)
        input.textColor = nsColor(p.ink)
        input.placeholderAttributedString = NSAttributedString(string: tr("Search in book"), attributes: [.foregroundColor: nsColor(p.muted), .font: interfaceFont(15)])
        count.textColor = nsColor(p.muted)
        table.selectionColor = color(p.ink, alpha: 0.07)
        if !query.isEmpty { search(query) } // the snippets carry the theme's ink
    }

    /// Shows the panel with the field focused and the last query selected.
    func open() {
        guard let view else { return }
        colour(view.palette)
        isHidden = false
        window?.makeFirstResponder(input)
        input.currentEditor()?.selectAll(nil)
    }

    func dismiss() {
        isHidden = true
        if let view { window?.makeFirstResponder(view) }
    }

    func focusField() { window?.makeFirstResponder(input) }

    /// A moment after the last key: the book is searched for the field's text.
    private func changed() {
        pending?.cancel()
        let q = input.stringValue
        let work = DispatchWorkItem { [weak self] in self?.search(q) }
        pending = work
        DispatchQueue.main.asyncAfter(deadline: .now() + 0.15, execute: work)
    }

    private func search(_ q: String) {
        query = q
        let book = UInt(bitPattern: book)
        let ink = palette.ink
        DispatchQueue.global(qos: .userInitiated).async { [weak self] in
            guard let search = forzac_search(OpaquePointer(bitPattern: book), q) else { return }
            var n: UInt32 = 0
            let list = forzac_search_hits(search, &n)!
            let hits = (0..<Int(n)).map { SearchPanel.hit(list[$0], ink: ink) }
            forzac_search_free(search)
            DispatchQueue.main.async {
                guard let self, self.query == q else { return }
                self.show(hits, searched: q.trimmingCharacters(in: .whitespaces).count >= 2)
            }
        }
    }

    /// A match as shown: the snippet in the book serif at 78 % ink with the match in full ink and bold.
    private static func hit(_ h: ForzacHit, ink: UInt32) -> Hit {
        let bytes = Array(UnsafeBufferPointer(start: UnsafeRawPointer(h.snippet).assumingMemoryBound(to: UInt8.self), count: strlen(h.snippet)))
        let (a, b) = (Int(h.match_start), Int(h.match_end))
        let part = { (r: Range<Int>) in String(decoding: bytes[r], as: UTF8.self) }
        let style = NSMutableParagraphStyle()
        style.minimumLineHeight = 23
        style.maximumLineHeight = 23
        let plain: [NSAttributedString.Key: Any] = [.font: bookFont("PT Serif", 16), .foregroundColor: nsColor(ink, alpha: 0.78), .paragraphStyle: style]
        let bold: [NSAttributedString.Key: Any] = [.font: bookFont("PT Serif", 16, bold: true), .foregroundColor: nsColor(ink), .paragraphStyle: style]
        let text = NSMutableAttributedString(string: part(0..<a), attributes: plain)
        text.append(NSAttributedString(string: part(a..<b), attributes: bold))
        text.append(NSAttributedString(string: part(b..<bytes.count), attributes: plain))
        return Hit(start: h.start, end: h.end, snippet: text, section: h.section.map { String(cString: $0) }, position: h.position)
    }

    private func show(_ found: [Hit], searched: Bool) {
        hits = found
        count.stringValue = !searched ? "" : found.isEmpty ? tr("No matches") : trCount(found.count, "{n} match", "{n} matches")
        table.reloadData()
        if !hits.isEmpty { select(0) }
    }

    func select(_ row: Int) {
        guard !hits.isEmpty else { return }
        let row = min(max(0, row), hits.count - 1)
        table.selectRowIndexes([row], byExtendingSelection: false)
        table.scrollRowToVisible(row)
    }

    @objc func pick() {
        guard let view, !hits.isEmpty else { return }
        let i = min(max(0, table.selectedRow), hits.count - 1)
        dismiss()
        view.goToMatch(hits.map { ($0.start, $0.end) }, i)
    }

    func moveIntoList() {
        window?.makeFirstResponder(table)
        select(max(0, table.selectedRow))
    }

    func numberOfRows(in tableView: NSTableView) -> Int { hits.count }

    private var textWidth: CGFloat { max(100, (table.bounds.width > 0 ? table.bounds.width : 400) - 40) }

    func tableView(_ tableView: NSTableView, heightOfRow row: Int) -> CGFloat {
        let h = hits[row].snippet.boundingRect(with: NSSize(width: textWidth, height: 1000), options: [.usesLineFragmentOrigin]).height
        return ceil(h) + 4 + 18 + 24
    }

    func tableView(_ tableView: NSTableView, viewFor tableColumn: NSTableColumn?, row: Int) -> NSView? {
        let hit = hits[row]
        let snippet = NSTextField(labelWithAttributedString: hit.snippet)
        snippet.maximumNumberOfLines = 0
        snippet.cell?.wraps = true
        snippet.preferredMaxLayoutWidth = textWidth
        let place = label(view.map { sectionPlace($0, hit.section, hit.start, hit.position) } ?? "", interfaceFont(13), palette.muted)
        place.lineBreakMode = .byTruncatingTail
        let column = NSStackView(views: [snippet, place])
        column.orientation = .vertical
        column.alignment = .leading
        column.spacing = 4
        column.edgeInsets = NSEdgeInsets(top: 12, left: 20, bottom: 12, right: 20)
        return column
    }

    func tableView(_ tableView: NSTableView, rowViewForRow row: Int) -> NSTableRowView? {
        let r = ContentsRow()
        r.selection = table.selectionColor
        return r
    }

    func controlTextDidChange(_ obj: Notification) { changed() }

    func control(_ control: NSControl, textView: NSTextView, doCommandBy selector: Selector) -> Bool {
        switch selector {
        case #selector(NSResponder.cancelOperation(_:)):
            if input.stringValue.isEmpty { dismiss() } else { input.stringValue = ""; changed() }
            return true
        case #selector(NSResponder.moveDown(_:)):
            moveIntoList()
            return true
        case #selector(NSResponder.insertNewline(_:)):
            pick()
            return true
        default:
            return false
        }
    }
}

/// The list of matches: `j`/`k` move, `Return` goes, `/` back to the field, `Esc` closes.
final class SearchTable: NSTableView {
    weak var panel: SearchPanel?
    var selectionColor = color(0x000000, alpha: 0.07)

    override func keyDown(with e: NSEvent) {
        switch e.shortcutKey {
        case 0x26: panel?.select(selectedRow + 1)                         // j
        case 0x28: panel?.select(selectedRow - 1)                         // k
        case 0x24, 0x4C: panel?.pick()                                     // Return
        case 0x2C where !e.modifierFlags.contains(.shift): panel?.focusField() // /
        case 0x35, 0x0C: panel?.dismiss()                                  // Esc q
        default: super.keyDown(with: e)
        }
    }
}

/// Where `at` is in the book for a list: its page once the book's pages are counted, else its share in percent.
func pageOrPercent(_ view: PageView, _ at: ForzacLocator, _ position: Float) -> String {
    forzac_book_pages(view.doc) > 0 ? "\(forzac_book_page_of(view.doc, at) + 1)" : "\(Int((position * 100).rounded()))%"
}

/// "section · p. N" (or "section · N%" before the count) for the highlights and the matches.
func sectionPlace(_ view: PageView, _ section: String?, _ at: ForzacLocator, _ position: Float) -> String {
    let place = forzac_book_pages(view.doc) > 0 ? fill(tr("p. {n}"), ["n": "\(forzac_book_page_of(view.doc, at) + 1)"]) : "\(Int((position * 100).rounded()))%"
    return section.map { "\($0) · \(place)" } ?? place
}
