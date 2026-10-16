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
        layer!.maskedCorners = [.layerMinXMaxYCorner, .layerMaxXMaxYCorner] // the top corners (layers are y up)
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
        let full = widthAnchor.constraint(equalTo: container.widthAnchor)
        full.priority = .defaultHigh
        NSLayoutConstraint.activate([
            bottomAnchor.constraint(equalTo: container.bottomAnchor), centerXAnchor.constraint(equalTo: container.centerXAnchor),
            widthAnchor.constraint(lessThanOrEqualToConstant: sheetWidth), full,
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
        pages = Segmented(["Slide", "Scroll"].map { FlatButton(label($0, interfaceFont(14), 0)) })
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
        let (spacingColumn, c1) = captioned("Line spacing", spacing)
        let (marginsColumn, c2) = captioned("Margins", margins)
        let pair = NSStackView(views: [spacingColumn, marginsColumn])
        pair.spacing = 16
        pair.distribution = .fillEqually
        stack.addArrangedSubview(pair)
        pair.widthAnchor.constraint(equalTo: stack.widthAnchor, constant: -40).isActive = true
        let (pagesColumn, c3) = captioned("Pages", pages)
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
        keysRing.frame = layerFrame(of: controls[current].0).insetBy(dx: -4, dy: -4)
        CATransaction.commit()
    }

    /// A subview's frame in this view's layer (y down, like the view: AppKit flips the layer of a flipped view).
    private func layerFrame(of v: NSView) -> CGRect { v.convert(v.bounds, to: self) }

    override func keyDown(with e: NSEvent) {
        switch e.keyCode {
        case 0x26, 0x7D: current = min(controls.count - 1, current + 1); placeRings()  // j ↓
        case 0x28, 0x7E: current = max(0, current - 1); placeRings()                    // k ↑
        case 0x25, 0x7C: change(controls[current].1, 1)                                 // l →
        case 0x04, 0x7B: change(controls[current].1, -1)                                // h ←
        case 0x35, 0x01, 0x0C: close()                                                  // Esc s q
        default: super.keyDown(with: e)
        }
    }

    func open() {
        isHidden = false
        show()
        window?.makeFirstResponder(self)
    }

    func close() {
        isHidden = true
        if let view { window?.makeFirstResponder(view) }
    }
}

// MARK: - Contents

/// The contents: a title with a close button, a filter field, the book, then the entries with their position in the
/// book; the current one bold and marked. `j`/`k` move, `Enter` jumps (remembered for `⌃O`), `/` filters, `Esc` or
/// `o` closes.
final class ContentsPanel: LayerView, NSTableViewDataSource, NSTableViewDelegate, NSTextFieldDelegate {
    private weak var view: PageView?
    private let entries: [(title: String, level: UInt32, at: ForzacLocator, position: Float)]
    private var shown: [Int] = []
    private var currentEntry = 0
    private let title = label("Contents", interfaceFont(17, .semibold), 0)
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
        title.setContentHuggingPriority(.defaultLow, for: .horizontal)
        let header = NSStackView(views: [title, close])
        header.distribution = .fill
        header.edgeInsets = NSEdgeInsets(top: 12, left: 20, bottom: 0, right: 8)

        fixed(field, height: 44)
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

        for v in [header, field, card, rule, scroll] as [NSView] {
            v.translatesAutoresizingMaskIntoConstraints = false
            addSubview(v)
        }
        NSLayoutConstraint.activate([
            header.topAnchor.constraint(equalTo: topAnchor), header.leadingAnchor.constraint(equalTo: leadingAnchor), header.trailingAnchor.constraint(equalTo: trailingAnchor),
            field.topAnchor.constraint(equalTo: header.bottomAnchor, constant: 8), field.leadingAnchor.constraint(equalTo: leadingAnchor, constant: 20),
            field.trailingAnchor.constraint(equalTo: trailingAnchor, constant: -20),
            card.topAnchor.constraint(equalTo: field.bottomAnchor, constant: 12), card.leadingAnchor.constraint(equalTo: leadingAnchor, constant: 20),
            card.trailingAnchor.constraint(lessThanOrEqualTo: trailingAnchor, constant: -20),
            rule.topAnchor.constraint(equalTo: card.bottomAnchor, constant: 22), rule.leadingAnchor.constraint(equalTo: leadingAnchor), rule.trailingAnchor.constraint(equalTo: trailingAnchor),
            scroll.topAnchor.constraint(equalTo: rule.bottomAnchor, constant: 6), scroll.leadingAnchor.constraint(equalTo: leadingAnchor),
            scroll.trailingAnchor.constraint(equalTo: trailingAnchor), scroll.bottomAnchor.constraint(equalTo: bottomAnchor),
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
        title.textColor = nsColor(p.ink)
        close.colour(p)
        field.layer!.backgroundColor = color(p.sheet)
        field.layer!.borderColor = color(p.line)
        searchIcon.tint = nsColor(p.muted)
        filter.textColor = nsColor(p.ink)
        filter.placeholderAttributedString = NSAttributedString(string: "Filter contents", attributes: [.foregroundColor: nsColor(p.muted), .font: interfaceFont(15)])
        bookTitle.textColor = nsColor(p.ink)
        meta.textColor = nsColor(p.muted)
        rule.layer!.backgroundColor = color(p.line)
        table.selectionColor = color(p.ink, alpha: 0.07)
        table.reloadData()
    }

    /// Shows the panel with the entry of the current place selected.
    func open() {
        guard let view else { return }
        let here = view.locator
        let before: (ForzacLocator) -> Bool = { ($0.chapter, $0.block, $0.offset) <= (here.chapter, here.block, here.offset) }
        currentEntry = entries.lastIndex { before($0.at) } ?? 0
        let read = Int((forzac_progress(view.doc, here) * 100).rounded())
        let author = String(cString: forzac_authors(view.doc)).components(separatedBy: ", ").first ?? ""
        meta.stringValue = [author, "\(read)% read"].filter { !$0.isEmpty }.joined(separator: " · ")
        filter.stringValue = ""
        refilter()
        colour(view.palette)
        isHidden = false
        select(shown.firstIndex(of: currentEntry) ?? 0)
        window?.makeFirstResponder(table)
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
        guard !shown.isEmpty else { return }
        let row = min(max(0, row), shown.count - 1)
        table.selectRowIndexes([row], byExtendingSelection: false)
        table.scrollRowToVisible(row)
    }

    @objc func jump() {
        guard let view, table.selectedRow >= 0, table.selectedRow < shown.count else { return }
        let at = entries[shown[table.selectedRow]].at
        dismiss()
        view.go(to: at)
    }

    func focusFilter() { window?.makeFirstResponder(filter) }

    func numberOfRows(in tableView: NSTableView) -> Int { shown.count }

    /// The room for an entry's title: the panel less the margins, the indent, the dot and the position.
    private func titleWidth(level: UInt32) -> CGFloat {
        let width = table.bounds.width > 0 ? table.bounds.width : 400
        return max(40, width - 40 - 16 * CGFloat(min(4, max(0, Int(level) - 1))) - 18 - 44)
    }

    func tableView(_ tableView: NSTableView, heightOfRow row: Int) -> CGFloat {
        let e = entries[shown[row]]
        let font = bookFont("PT Serif", 16, bold: shown[row] == currentEntry)
        let h = (e.title as NSString).boundingRect(with: NSSize(width: titleWidth(level: e.level), height: 1000), options: [.usesLineFragmentOrigin], attributes: [.font: font]).height
        let line = ceil(font.ascender - font.descender + font.leading)
        return max(48, min(h, 2 * line) + 16) // at most two lines, 8 above and below
    }

    func tableView(_ tableView: NSTableView, viewFor tableColumn: NSTableColumn?, row: Int) -> NSView? {
        let i = shown[row]
        let e = entries[i]
        let isCurrent = i == currentEntry
        let cell = NSView()
        let t = label(e.title, bookFont("PT Serif", 16, bold: isCurrent), palette.ink)
        t.maximumNumberOfLines = 2
        t.lineBreakMode = .byWordWrapping
        t.cell?.truncatesLastVisibleLine = true
        t.preferredMaxLayoutWidth = titleWidth(level: e.level)
        let p = label("\(Int((e.position * 100).rounded()))%", interfaceFont(13), palette.muted)
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

/// The contents list: `j`/`k` move, `Enter` jumps, `/` filters, `Esc` and `o` close.
final class ContentsTable: NSTableView {
    weak var panel: ContentsPanel?
    var selectionColor = color(0x000000, alpha: 0.07)

    override func keyDown(with e: NSEvent) {
        switch e.keyCode {
        case 0x26: panel?.select(selectedRow + 1)        // j
        case 0x28: panel?.select(selectedRow - 1)        // k
        case 0x24, 0x4C: panel?.jump()                    // Return
        case 0x2C where !e.modifierFlags.contains(.shift): panel?.focusFilter() // /
        case 0x35, 0x1F, 0x0C: panel?.dismiss()           // Esc o q
        default: super.keyDown(with: e)
        }
    }
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

    init?(view: PageView, id: String, windowSize: CGSize) {
        self.view = view
        let width = min(sheetWidth, windowSize.width) - 48
        var page = ForzacPage()
        let height = withUnsafePointer(to: view.settings) { forzac_note(view.doc, id, Float(width), $0, &page) }
        guard height > 0 else { return nil }
        let p = view.palette
        let scale = NSScreen.main?.backingScaleFactor ?? 2
        var pictures: [UnsafeRawPointer: CGImage] = [:]
        let image = bitmap(CGSize(width: width, height: CGFloat(height)), scale: scale, background: nil) { ctx in
            drawPage(page, in: ctx, height: CGFloat(height), palette: p, fonts: view.fonts, pictures: &pictures)
        }
        super.init(spacing: 12, insets: NSEdgeInsets(top: 10, left: 24, bottom: 36, right: 24))
        addGrabber()
        let title = forzac_note_title(view.doc, id).map { String(cString: $0) } ?? ""
        let caption = label(["Note", title].filter { !$0.isEmpty }.joined(separator: " ").uppercased(), interfaceFont(12, .medium), p.muted)
        caption.attributedStringValue = NSAttributedString(string: caption.stringValue, attributes: [.kern: 0.72, .font: interfaceFont(12, .medium), .foregroundColor: nsColor(p.muted)])
        stack.addArrangedSubview(caption)
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
        switch e.keyCode {
        case 0x35, 0x24, 0x4C, 0x31, 0x0C: dismiss() // Esc Return Space q
        default: super.keyDown(with: e)
        }
    }

    func dismiss() {
        removeFromSuperview()
        if let view { view.window?.makeFirstResponder(view) }
    }
}
