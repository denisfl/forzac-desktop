//! One XHTML document of an EPUB into blocks (a port of the Android app's `XhtmlReader`).
//!
//! A flat streaming walk: inline elements add text to the current paragraph, block elements close it. That survives
//! invalid nesting (`span` around `div`) and text directly in `div`. A paragraph's kind comes from the nearest ancestor
//! with meaning, a tag or a class.

use std::collections::{HashMap, HashSet};
use std::sync::Arc;

use super::{fragment_of, is_external, resolve_path};
use crate::model::{Block, Kind, Style, StyleRange, StyledText, Table, TableCell};
use crate::typography;
use crate::xml::{Ev, Res, Xml, attr};

/// Blocks, and which of them are a paragraph that is only a link (in notes: "Back").
#[derive(Default)]
pub(super) struct Sink {
    pub hidden: bool,
    pub blocks: Vec<Block>,
    pub link_only: HashSet<usize>,
}

/// An element with an id: its first block and the block after its last; for an inline element `end == start`.
#[derive(Debug, Clone, Copy)]
pub(super) struct Anchor {
    pub hidden: bool,
    pub start: usize,
    pub end: usize,
}

/// CSS classes that make text italic, bold or small capitals; `small_caps_tags` and `normal_weight_tags` are the rules
/// of `b` and `strong` themselves (Standard Ebooks set `b` in small capitals of the normal weight).
#[derive(Default)]
pub(super) struct Css {
    pub italic: HashSet<String>,
    pub bold: HashSet<String>,
    pub small_caps: HashSet<String>,
    pub small_caps_tags: HashSet<String>,
    pub normal_weight_tags: HashSet<String>,
}

/// Signs the book fonts lack, replaced as the "compatible" EPUBs of Standard Ebooks replace them: the two- and
/// three-em dashes and the care-of sign.
pub(super) fn without_rare_signs(text: &str) -> String {
    let mut out = String::with_capacity(text.len());
    for ch in text.chars() {
        match rare_sign(ch) {
            Some(s) => out.push_str(s),
            None => out.push(ch),
        }
    }
    out
}

fn rare_sign(ch: char) -> Option<&'static str> {
    match ch {
        '\u{2E3A}' => Some("——"),
        '\u{2E3B}' => Some("———"),
        '\u{2105}' => Some("c/o"),
        _ => None,
    }
}

pub(super) struct Doc {
    pub path: String,
    pub main: Sink,
    /// Footnotes taken out of the reading flow (`aside epub:type="footnote"` and the like).
    pub hidden: Sink,
    pub anchors: HashMap<String, Anchor>,
    /// Note reference targets in this document: `path#id`.
    pub note_refs: HashSet<String>,
}

impl Doc {
    /// A raster image listed in the spine: one image in its place in the reading order.
    pub fn image(path: &str) -> Doc {
        let main = Sink { blocks: vec![Block::Image { id: path.to_owned() }], ..Sink::default() };
        Doc {
            path: path.to_owned(),
            main,
            hidden: Sink { hidden: true, ..Sink::default() },
            anchors: HashMap::new(),
            note_refs: HashSet::new(),
        }
    }
}

const MAIN: usize = 0;
const HIDDEN: usize = 1;

const SKIP: &[&str] =
    &["head", "script", "style", "nav", "audio", "video", "object", "iframe", "noscript", "rt", "rp", "annotation", "annotation-xml"];
const BLOCK: &[&str] = &[
    "html",
    "body",
    "p",
    "div",
    "h1",
    "h2",
    "h3",
    "h4",
    "h5",
    "h6",
    "li",
    "ul",
    "ol",
    "dl",
    "dt",
    "dd",
    "blockquote",
    "section",
    "article",
    "aside",
    "header",
    "footer",
    "main",
    "figure",
    "figcaption",
    "pre",
    "table",
    "thead",
    "tbody",
    "tfoot",
    "tr",
    "caption",
    "center",
    "address",
    "hgroup",
    "details",
    "summary",
];
const EMPHASIS: &[&str] = &["em", "i", "cite", "dfn", "var"];
const STRONG: &[&str] = &["strong", "b"];
const STRIKE: &[&str] = &["s", "strike", "del"];
const CODE: &[&str] = &["code", "kbd", "samp", "tt"];
const VERSE: &[&str] = &["poem", "stanza", "v", "verse"];
/// `epub:type` of verse and of a diary (the DAISY `z3998` vocabulary of Standard Ebooks).
const VERSE_TYPES: &[&str] = &["z3998:poem", "z3998:verse", "z3998:song", "z3998:hymn"];
const DIARY_TYPES: &[&str] = &["z3998:diary", "z3998:diary-entry"];
const NOTE_TYPES: &[&str] = &["footnote", "endnote", "rearnote"];

struct Frame {
    name: String,
    block: bool,
    sink: usize,
    id: Option<String>,
    anchor_start: usize,
    kind: Option<Kind>,
    /// Raw heading level: `h2` → 2, class `title1` → 1.
    heading: Option<u32>,
    pre: bool,
    style: Option<Style>,
    /// A second style: bold small capitals (`b` whose CSS keeps the weight), or a small-capitals class on an element
    /// that has another style.
    extra_style: Option<Style>,
    /// Where the element's text starts in the current paragraph (chars).
    start: usize,
    href: Option<String>,
    note_type: bool,
    ordered: bool,
    counter: usize,
    col_span: usize,
    /// `epub:type="titlepage"`: with an image in it, its text is a caption for screen readers, and the page is the image.
    titlepage: bool,
    /// For `hgroup`: the raw level of the group's last heading; a `p` after it is a heading one level lower.
    group_heading: Option<u32>,
}

impl Frame {
    fn styles(&self) -> impl Iterator<Item = Style> {
        self.style.into_iter().chain(self.extra_style)
    }
}

/// The top-level table being read: rows collect and become blocks at the end of `table`.
struct TableBuilder {
    sink: usize,
    rows: Vec<Vec<TableCell>>,
    row: Option<Vec<TableCell>>,
    /// Blocks of each cell: for a one-column table, which is a wrapper (a sidebar, a layout) shown as ordinary blocks.
    blocks: Vec<Vec<Block>>,
}

pub(super) fn read(path: &str, bytes: &[u8], css: &Css) -> Doc {
    let data = crate::fb2::to_utf8(bytes);
    let mut r = Reader {
        path,
        css,
        sinks: vec![Sink::default(), Sink { hidden: true, ..Sink::default() }],
        cells: HashSet::new(),
        anchors: HashMap::new(),
        note_refs: HashSet::new(),
        frames: Vec::new(),
        table: None,
        nested_tables: 0,
        sb: Vec::new(),
        styles: Vec::new(),
        links: Vec::new(),
    };
    let mut xml = Xml::new(&data).with_attributes(&["class", "type", "id", "name", "src", "href", "start", "colspan", "hidden"]);
    // A break or gross markup error past the start keeps what was read.
    let _ = r.walk(&mut xml);
    r.flush();
    let mut sinks = r.sinks.into_iter();
    let main = sinks.next().unwrap_or_default();
    let hidden = sinks.next().unwrap_or_default();
    Doc { path: path.to_owned(), main, hidden, anchors: r.anchors, note_refs: r.note_refs }
}

struct Reader<'a> {
    path: &'a str,
    css: &'a Css,
    /// `MAIN`, `HIDDEN`, then one per table cell.
    sinks: Vec<Sink>,
    /// Sinks of the cells being read: their paragraphs join into the cell's text at its end.
    cells: HashSet<usize>,
    anchors: HashMap<String, Anchor>,
    note_refs: HashSet<String>,
    frames: Vec<Frame>,
    table: Option<TableBuilder>,
    /// Tables inside the table: read as text of its cell.
    nested_tables: usize,
    /// The current paragraph, as chars for `typography::normalize`; style ranges in chars.
    sb: Vec<char>,
    styles: Vec<(usize, usize, Style, Option<String>)>,
    links: Vec<(usize, usize)>,
}

impl Reader<'_> {
    fn walk(&mut self, xml: &mut Xml) -> Res<()> {
        loop {
            match xml.next()? {
                Ev::Start(mut name, attrs) => {
                    name.make_ascii_lowercase();
                    self.start(xml, name, &attrs)?
                }
                Ev::End => self.end(),
                Ev::Text(t) => self.text(&t),
                Ev::Eof => return Ok(()),
            }
        }
    }

    fn sink(&self) -> usize {
        self.frames.last().map_or(MAIN, |f| f.sink)
    }

    fn in_cell(&self) -> bool {
        !self.cells.is_empty() && self.frames.iter().any(|f| self.cells.contains(&f.sink))
    }

    fn start(&mut self, xml: &mut Xml, owned_name: String, attrs: &[(String, String)]) -> Res<()> {
        let name = owned_name.as_str();
        // `hidden` is not shown in HTML (Standard Ebooks: captions for screen readers).
        if SKIP.contains(&name) || attr(attrs, "hidden").is_some() {
            return xml.skip();
        }
        let class = attr(attrs, "class").unwrap_or_default().to_lowercase();
        let classes: Vec<&str> = class.split_whitespace().collect();
        let epub_type = attr(attrs, "type").unwrap_or_default();
        let id = attr(attrs, "id").or_else(|| if name == "a" { attr(attrs, "name") } else { None }).map(str::to_owned);

        match name {
            "br" => {
                if !self.in_pre() && self.sb.last() == Some(&' ') {
                    self.sb.pop();
                }
                self.sb.push('\n');
            }
            "hr" => {
                self.flush();
                let sink = self.sink();
                self.sinks[sink].blocks.push(Block::EmptyLine);
            }
            "img" | "image" => {
                self.flush();
                let src = if name == "img" { attr(attrs, "src") } else { attr(attrs, "href") };
                if let Some(src) = src.filter(|s| !s.trim().is_empty()) {
                    let sink = self.sink();
                    self.sinks[sink].blocks.push(Block::Image { id: resolve_path(self.path, src) });
                }
            }
            "td" | "th" if self.table.is_none() || self.nested_tables > 0 => {
                if self.sb.iter().any(|c| !c.is_whitespace()) {
                    self.sb.extend(" — ".chars());
                }
            }
            "table" => {
                if self.table.is_none() {
                    self.flush();
                    self.table = Some(TableBuilder { sink: self.sink(), rows: Vec::new(), row: None, blocks: Vec::new() });
                } else {
                    self.nested_tables += 1;
                }
            }
            "tr" if self.nested_tables == 0 => {
                if let Some(t) = &mut self.table {
                    t.row = Some(Vec::new());
                }
            }
            _ => {}
        }
        if name == "p" && classes.contains(&"empty-line") {
            self.flush();
            let sink = self.sink();
            self.sinks[sink].blocks.push(Block::EmptyLine);
        }

        let cell = matches!(name, "td" | "th") && self.table.is_some() && self.nested_tables == 0;
        let block = BLOCK.contains(&name) || cell;
        if block {
            self.flush();
        }
        let parent = self.sink();
        let footnote = !epub_type.is_empty() && NOTE_TYPES.iter().any(|t| epub_type.contains(t));
        let sink = if cell {
            self.sinks.push(Sink::default());
            self.cells.insert(self.sinks.len() - 1);
            self.sinks.len() - 1
        } else if footnote {
            HIDDEN
        } else {
            parent
        };
        if footnote && sink != parent {
            self.flush();
        }

        if name == "li"
            && let Some(list) = self.frames.iter_mut().rev().find(|f| f.name == "ol" || f.name == "ul")
        {
            list.counter += 1;
            let marker = if list.ordered { format!("{}. ", list.counter) } else { "• ".to_owned() };
            self.sb.extend(marker.chars());
        }

        let css = self.css;
        let small_caps_tag = STRONG.contains(&name) && css.small_caps_tags.contains(name);
        let small_caps_class = classes.iter().any(|c| css.small_caps.contains(*c));
        let style = if EMPHASIS.contains(&name) || classes.iter().any(|c| css.italic.contains(*c) || *c == "italic") {
            Some(Style::Emphasis)
        } else if small_caps_tag {
            Some(Style::SmallCaps)
        } else if STRONG.contains(&name) || classes.iter().any(|c| css.bold.contains(*c) || *c == "bold") {
            Some(Style::Strong)
        } else if small_caps_class {
            Some(Style::SmallCaps)
        } else {
            match name {
                "sup" => Some(Style::Superscript),
                "sub" => Some(Style::Subscript),
                _ if STRIKE.contains(&name) => Some(Style::Strikethrough),
                _ if CODE.contains(&name) => Some(Style::Code),
                _ => None,
            }
        };
        let extra_style = if small_caps_tag && !css.normal_weight_tags.contains(name) {
            Some(Style::Strong)
        } else if small_caps_class && style != Some(Style::SmallCaps) {
            Some(Style::SmallCaps)
        } else {
            None
        };
        let mut heading = heading_level(name).or_else(|| classes.iter().find_map(|c| title_level(c)));
        // A `p` in `hgroup` is a heading one level below the group's heading before it (a chapter's title under its
        // number); without one, a subtitle.
        let group = self.frames.last_mut().filter(|f| f.name == "hgroup");
        let mut group_subtitle = false;
        if let Some(group) = group {
            if heading.is_some() {
                group.group_heading = heading;
            } else if name == "p" {
                heading = group.group_heading.map(|h| (h + 1).min(6));
                group_subtitle = heading.is_none();
            }
        }
        // A verse line's indent (`i1`…`i9` at Standard Ebooks: that many em): em spaces at the line's start.
        if name == "span" && self.frames.iter().any(|f| f.kind == Some(Kind::Verse)) {
            let em = classes.iter().find_map(|c| c.strip_prefix('i').filter(|n| n.len() == 1).and_then(|n| n.parse::<usize>().ok()));
            if let Some(n) = em.filter(|n| (1..=9).contains(n)) {
                self.sb.extend(std::iter::repeat_n('\u{2003}', n));
            }
        }
        // An anchor inside a table is the table's start: its rows reach the table's sink only at its end.
        let anchor_start = if self.cells.contains(&sink) || self.in_cell() {
            self.table.as_ref().map_or(0, |t| self.sinks[t.sink].blocks.len())
        } else {
            self.sinks[sink].blocks.len()
        };
        if let Some(id) = &id {
            self.anchors.entry(id.clone()).or_insert(Anchor { hidden: self.sinks[sink].hidden, start: anchor_start, end: anchor_start });
        }
        let ordered = name == "ol";
        let pre = name == "pre";
        let kind = if group_subtitle { Some(Kind::Subtitle) } else { kind_of(name, &classes, epub_type) };
        let href = if name == "a" { attr(attrs, "href").map(str::to_owned) } else { None };
        self.frames.push(Frame {
            name: owned_name,
            block,
            sink,
            id,
            anchor_start,
            kind,
            heading,
            pre,
            style,
            extra_style,
            start: self.sb.len(),
            href,
            note_type: epub_type.contains("noteref"),
            ordered,
            counter: if ordered {
                attr(attrs, "start").and_then(|s| s.trim().parse::<usize>().ok()).map_or(0, |s| s.saturating_sub(1))
            } else {
                0
            },
            col_span: if cell { attr(attrs, "colspan").and_then(|s| s.trim().parse().ok()).unwrap_or(1usize).clamp(1, 64) } else { 1 },
            titlepage: epub_type.split_whitespace().any(|t| t == "titlepage"),
            group_heading: None,
        });
        Ok(())
    }

    fn end(&mut self) {
        let Some(frame) = self.frames.last() else { return };
        if frame.name == "a" && frame.href.is_some() {
            self.close_link();
        }
        let frame = self.frames.last().unwrap();
        if frame.name == "li" && self.marker_only() {
            self.sb.clear(); // an empty item: its marker goes with it
        }
        let frame = self.frames.last().unwrap();
        let parent = self.frames.len().checked_sub(2).map_or(MAIN, |i| self.frames[i].sink);
        if frame.block || frame.sink != parent {
            // The paragraph closes while the element is still open: its kind and style belong to it.
            self.flush();
            let frame = self.frames.pop().unwrap();
            if frame.titlepage {
                self.drop_title_text(&frame);
            }
            let in_cell = self.cells.contains(&frame.sink) || self.in_cell();
            self.end_table_part(&frame);
            if in_cell {
                return; // an anchor inside a table is the table's start
            }
            if let Some(id) = &frame.id
                && self.anchors.get(id).is_some_and(|a| a.start == frame.anchor_start)
            {
                let sink = &self.sinks[frame.sink];
                self.anchors.insert(id.clone(), Anchor { hidden: sink.hidden, start: frame.anchor_start, end: sink.blocks.len() });
            }
        } else {
            let frame = self.frames.pop().unwrap();
            if frame.start < self.sb.len() {
                let end = self.sb.len();
                self.styles.extend(frame.styles().map(|style| (frame.start, end, style, None)));
            }
        }
    }

    /// A title page with an image (Standard Ebooks): its title and author are a caption for screen readers, which the
    /// book's CSS hides, and the page is the image. The images stay, the text goes; anchors inside point to its start.
    fn drop_title_text(&mut self, frame: &Frame) {
        let sink = &mut self.sinks[frame.sink];
        let from = frame.anchor_start;
        if from > sink.blocks.len() || !sink.blocks[from..].iter().any(|b| matches!(b, Block::Image { .. })) {
            return;
        }
        let tail = sink.blocks.split_off(from);
        sink.blocks.extend(tail.into_iter().filter(|b| matches!(b, Block::Image { .. })));
        sink.link_only.retain(|&i| i < from);
        let (hidden, end) = (sink.hidden, sink.blocks.len());
        for a in self.anchors.values_mut().filter(|a| a.hidden == hidden && a.start >= from) {
            a.start = a.start.min(end);
            a.end = a.end.min(end);
        }
    }

    /// The end of a cell, a row or a top-level table.
    fn end_table_part(&mut self, frame: &Frame) {
        let Some(builder) = &mut self.table else { return };
        match frame.name.as_str() {
            "td" | "th" if self.cells.contains(&frame.sink) => {
                self.cells.remove(&frame.sink);
                let blocks = std::mem::take(&mut self.sinks[frame.sink].blocks);
                let header = frame.name == "th" || self.frames.iter().any(|f| f.name == "thead");
                builder.row.get_or_insert_with(Vec::new).push(TableCell { text: cell_text(&blocks), header, col_span: frame.col_span });
                builder.blocks.push(blocks);
            }
            "tr" if self.nested_tables == 0 => {
                if let Some(row) = builder.row.take().filter(|row| row.iter().any(|c| !c.text.text.is_empty())) {
                    builder.rows.push(row);
                }
            }
            "table" if self.nested_tables > 0 => self.nested_tables -= 1,
            "table" => {
                if let Some(row) = builder.row.take().filter(|row| row.iter().any(|c| !c.text.text.is_empty())) {
                    builder.rows.push(row);
                }
                let builder = self.table.take().unwrap();
                let sink = &mut self.sinks[builder.sink].blocks;
                if builder.rows.iter().all(|r| r.len() == 1 && r[0].col_span == 1) {
                    sink.extend(builder.blocks.into_iter().flatten());
                } else {
                    let table = Arc::new(Table { rows: Table::with_bold_header(builder.rows) });
                    sink.extend((0..table.rows.len()).map(|row| Block::TableRow { table: table.clone(), row }));
                }
            }
            _ => {}
        }
    }

    /// A link has closed: a note marker ("[12]", "12", "*" in superscript) or an ordinary link.
    fn close_link(&mut self) {
        let frame = self.frames.last().unwrap();
        let (start, note_type) = (frame.start, frame.note_type);
        let href = frame.href.clone().unwrap_or_default();
        let text: String = self.sb[start.min(self.sb.len())..].iter().collect();
        let marker: String = text.chars().filter(|c| !"[]{}()".contains(*c) && !c.is_whitespace()).collect();
        let count = marker.chars().count();
        let looks_like_marker = (1..=4).contains(&count) && (marker.chars().all(char::is_numeric) || marker.chars().all(|c| c == '*'));
        let in_sup = self.frames.iter().any(|f| f.style == Some(Style::Superscript))
            || self.styles.iter().any(|s| s.2 == Style::Superscript && s.0 >= start);
        let bracketed = text.chars().any(|c| "[]{}".contains(c));
        match fragment_of(&href) {
            Some(fragment) if note_type || (looks_like_marker && (in_sup || bracketed)) => {
                let key = format!("{}#{fragment}", resolve_path(self.path, &href));
                self.styles.retain(|s| s.0 < start);
                self.sb.truncate(start);
                self.sb.extend(if marker.is_empty() { "*".chars().collect::<Vec<_>>() } else { marker.chars().collect() });
                self.styles.push((start, self.sb.len(), Style::NoteRef, Some(key.clone())));
                self.note_refs.insert(key);
            }
            _ if self.sb.len() > start => {
                self.links.push((start, self.sb.len()));
                if let Some(key) = self.link_key(&href) {
                    self.styles.push((start, self.sb.len(), Style::Link, Some(key)));
                }
            }
            _ => {}
        }
    }

    /// The key of an ordinary link: an external address or `path#fragment` (an empty fragment is the document's start).
    fn link_key(&self, href: &str) -> Option<String> {
        let href = href.trim();
        if is_external(href) {
            return Some(href.to_owned());
        }
        if href.is_empty() || href == "#" || has_scheme(href) {
            return None;
        }
        Some(format!("{}#{}", resolve_path(self.path, href), fragment_of(href).unwrap_or_default()))
    }

    fn text(&mut self, chunk: &str) {
        if self.in_pre() {
            for ch in chunk.chars() {
                match ch {
                    '\r' => {}
                    '\t' => self.sb.extend("    ".chars()),
                    _ => self.sb.push(ch),
                }
            }
            return;
        }
        for ch in chunk.chars() {
            if matches!(ch, ' ' | '\n' | '\r' | '\t') {
                if self.sb.last().is_some_and(|l| *l != ' ' && *l != '\n') {
                    self.sb.push(' ');
                }
            } else if let Some(s) = rare_sign(ch) {
                self.sb.extend(s.chars());
            } else {
                self.sb.push(ch);
            }
        }
    }

    /// The paragraph so far is only a list item's marker ("• ", "3. ") and the item is still open.
    fn marker_only(&self) -> bool {
        if !self.frames.iter().any(|f| f.name == "li") {
            return false;
        }
        let text: String = self.sb.iter().collect();
        let text = text.trim();
        text == "•" || (text.ends_with('.') && text.len() > 1 && text[..text.len() - 1].chars().all(|c| c.is_ascii_digit()))
    }

    fn in_pre(&self) -> bool {
        self.frames.iter().any(|f| f.pre)
    }

    /// Closes the current paragraph. Open styles go on in the next paragraph from its start.
    fn flush(&mut self) {
        if self.marker_only() {
            return; // `<li><p>…</p></li>`: the marker waits for the item's first paragraph
        }
        let len = self.sb.len();
        for f in &self.frames {
            if f.start < len {
                self.styles.extend(f.styles().map(|style| (f.start, len, style, None)));
            }
        }
        self.emit();
        self.sb.clear();
        self.styles.clear();
        self.links.clear();
        self.frames.iter_mut().for_each(|f| f.start = 0);
    }

    fn emit(&mut self) {
        let pre = self.in_pre();
        let sb = &self.sb;
        let mut lead = 0;
        while lead < sb.len() && (sb[lead] == '\n' || (!pre && sb[lead] == ' ')) {
            lead += 1;
        }
        let mut end = sb.len();
        while end > lead && sb[end - 1].is_whitespace() {
            end -= 1;
        }
        if lead >= end {
            return;
        }
        let mut chars = sb[lead..end].to_vec();
        if !pre {
            typography::normalize(&mut chars);
        }
        let n = chars.len();
        let mut byte_at = Vec::with_capacity(n + 1);
        let mut text = String::with_capacity(n);
        for c in &chars {
            byte_at.push(text.len());
            text.push(*c);
        }
        byte_at.push(text.len());
        let mut styles: Vec<StyleRange> = self
            .styles
            .iter()
            .map(|(s, e, style, target)| (s.saturating_sub(lead).min(n), e.saturating_sub(lead).min(n), *style, target))
            .filter(|(s, e, ..)| s < e)
            .map(|(s, e, style, target)| StyleRange { start: byte_at[s], end: byte_at[e], style, target: target.clone() })
            .collect();
        // Nested elements of one style repeat a range; sorted, so a paragraph of thousands of spans stays linear.
        styles.sort_by(|a, b| (a.start, a.end, a.style as u8, &a.target).cmp(&(b.start, b.end, b.style as u8, &b.target)));
        styles.dedup();
        let text = StyledText { text, styles };
        let owner = self.frames.iter().rev().find(|f| f.pre || f.heading.is_some() || f.kind.is_some());
        let block = match owner {
            None => Block::Paragraph { text, kind: Kind::Body },
            Some(f) if f.pre => Block::Paragraph { text, kind: Kind::Code },
            Some(Frame { heading: Some(level), .. }) => Block::Heading { text, level: *level },
            Some(f) => Block::Paragraph { text, kind: f.kind.unwrap_or(Kind::Body) },
        };
        let link_only = self.links.iter().any(|(a, b)| *a <= lead && *b >= end);
        let index = self.sink();
        let sink = &mut self.sinks[index];
        if link_only {
            sink.link_only.insert(sink.blocks.len());
        }
        sink.blocks.push(block);
    }
}

/// Paragraphs of a cell joined by line breaks, styles shifted; images and empty lines in a cell are not shown.
fn cell_text(blocks: &[Block]) -> StyledText {
    let mut out = StyledText::default();
    for text in blocks.iter().filter_map(Block::text) {
        if !out.text.is_empty() {
            out.text.push('\n');
        }
        let at = out.text.len();
        out.text.push_str(&text.text);
        out.styles.extend(text.styles.iter().map(|s| StyleRange { start: s.start + at, end: s.end + at, ..s.clone() }));
    }
    out
}

/// The kind of an element's paragraphs, by its classes and `epub:type`.
fn kind_of(name: &str, classes: &[&str], epub_type: &str) -> Option<Kind> {
    let types = || epub_type.split_whitespace();
    if classes.contains(&"epigraph") {
        Some(Kind::Epigraph)
    } else if classes.contains(&"text-author") {
        Some(Kind::TextAuthor)
    } else if classes.contains(&"subtitle") {
        Some(Kind::Subtitle)
    } else if classes.iter().any(|c| VERSE.contains(c)) || types().any(|t| VERSE_TYPES.contains(&t)) {
        Some(Kind::Verse)
    } else if types().any(|t| DIARY_TYPES.contains(&t)) {
        None // a diary is the book's own text, not a quotation in it
    } else if name == "blockquote" || classes.contains(&"cite") {
        Some(Kind::Cite)
    } else if name == "caption" {
        Some(Kind::Subtitle)
    } else {
        None
    }
}

fn heading_level(name: &str) -> Option<u32> {
    let b = name.as_bytes();
    (b.len() == 2 && b[0] == b'h' && (b'1'..=b'6').contains(&b[1])).then(|| (b[1] - b'0') as u32)
}

/// Heading classes of FB2 converters: `title` and `title1`…`title9`.
fn title_level(class: &str) -> Option<u32> {
    match class.strip_prefix("title")? {
        "" => Some(1),
        n if n.len() == 1 => n.parse().ok().filter(|n| (1..=9).contains(n)),
        _ => None,
    }
}

/// `javascript:`, `tel:` and other schemes: not a place in the book.
fn has_scheme(href: &str) -> bool {
    let Some((scheme, _)) = href.split_once(':') else { return false };
    let mut chars = scheme.chars();
    chars.next().is_some_and(|c| c.is_ascii_alphabetic()) && chars.all(|c| c.is_ascii_alphanumeric() || "+.-".contains(c))
}
