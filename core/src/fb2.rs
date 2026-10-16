//! FictionBook 2 parser (`.fb2`, `.fb2.zip`): one streaming pass over the XML into the [`Book`] model.
//!
//! Ports the behaviour of the Android app's `Fb2Reader`: chapters with parts opened up, contents, notes, links, images,
//! tables. The encoding comes from the BOM or the XML declaration (FB2 files are often `windows-1251`).

use std::collections::HashMap;
use std::io::{Cursor, Read};
use std::sync::Arc;

use base64::Engine;
use quick_xml::Reader;
use quick_xml::events::Event;

use crate::model::{
    Block, Book, Chapter, Image, Kind, LinkTarget, Locator, Note, Style, StyleRange, StyledText, Table, TableCell, TocEntry,
};
use crate::typography;

#[derive(Debug)]
pub enum Error {
    /// Not a FictionBook document (or a zip without one), or an archive that unpacks too large.
    NotABook,
    /// A well-formed book without any text.
    NoText,
}

/// Unpacked FB2 larger than this is treated as a zip bomb.
const MAX_UNPACKED: u64 = 256 * 1024 * 1024;

/// Parses `.fb2` or `.fb2.zip` content.
pub fn parse(bytes: &[u8]) -> Result<Book, Error> {
    if bytes.starts_with(b"PK") {
        let mut zip = zip::ZipArchive::new(Cursor::new(bytes)).map_err(|_| Error::NotABook)?;
        let name = zip
            .file_names()
            .filter_map(Result::ok)
            .find(|n| n.to_lowercase().ends_with(".fb2"))
            .map(|n| n.into_owned())
            .ok_or(Error::NotABook)?;
        let mut xml = Vec::new();
        let entry = zip.by_name(&name).map_err(|_| Error::NotABook)?;
        entry.take(MAX_UNPACKED + 1).read_to_end(&mut xml).map_err(|_| Error::NotABook)?;
        if xml.len() as u64 > MAX_UNPACKED {
            return Err(Error::NotABook);
        }
        return Fb2Reader::new(&to_utf8(&xml)).read();
    }
    Fb2Reader::new(&to_utf8(bytes)).read()
}

/// The document as UTF-8: the encoding comes from the byte order mark, else from the XML declaration
/// (`encoding="windows-1251"`), else UTF-8. The parser reads UTF-8 only and ignores the declared label afterwards.
fn to_utf8(bytes: &[u8]) -> std::borrow::Cow<'_, [u8]> {
    use std::borrow::Cow;
    if let Some((encoding, bom)) = encoding_rs::Encoding::for_bom(bytes) {
        let (text, _) = encoding.decode_without_bom_handling(&bytes[bom..]);
        return Cow::Owned(text.into_owned().into_bytes());
    }
    let head = String::from_utf8_lossy(&bytes[..bytes.len().min(256)]);
    let label = head.find("encoding=").and_then(|at| {
        let rest = &head[at + "encoding=".len()..];
        let quote = rest.chars().next().filter(|c| *c == '"' || *c == '\'')?;
        rest[1..].split(quote).next()
    });
    match label.and_then(|l| encoding_rs::Encoding::for_label(l.trim().as_bytes())) {
        Some(encoding) if encoding != encoding_rs::UTF_8 => {
            let (text, _) = encoding.decode_without_bom_handling(bytes);
            Cow::Owned(text.into_owned().into_bytes())
        }
        _ => Cow::Borrowed(bytes),
    }
}

/// XML parsing went wrong past the root element: the caller keeps what was read.
struct Broken;

type Res<T> = Result<T, Broken>;

enum Ev {
    Start(String, Vec<(String, String)>),
    End,
    Text(String),
    Eof,
}

/// Every block gets a sequence number when it is read; ids map to sequence numbers, and after chapters are laid out
/// sequence numbers map to Locators (Android uses object identity for the same purpose).
type Seq = u32;

/// An open inline element that makes a style range: the style and its target (note id or link key).
type StyleTag = (Style, Option<String>);

/// A section in document order: its own blocks and nested sections.
#[derive(Default)]
struct Section {
    id: Option<String>,
    items: Vec<Item>,
}

enum Item {
    Block(Seq, Block),
    Section(Section),
}

impl Section {
    fn seq_range(&self) -> Option<(Seq, Seq)> {
        let mut range: Option<(Seq, Seq)> = None;
        for item in &self.items {
            let r = match item {
                Item::Block(s, _) => Some((*s, *s)),
                Item::Section(child) => child.seq_range(),
            };
            if let Some((a, b)) = r {
                range = Some(range.map_or((a, b), |(x, y)| (x.min(a), y.max(b))));
            }
        }
        range
    }
}

struct Fb2Reader<'a> {
    xml: Reader<&'a [u8]>,
    buf: Vec<u8>,
    book: Book,
    main_body_seen: bool,
    /// Blocks of the main body before its first section (title page, book epigraph): a chapter of their own.
    prologue: Vec<Item>,
    next_seq: Seq,
    /// Sequence numbers of each chapter's blocks, parallel to `book.chapters`.
    chapter_seqs: Vec<Vec<Seq>>,
    /// First and last block of every element and section with an id.
    ids: HashMap<String, (Seq, Seq)>,
    /// Images inside the paragraph being read: placed after it as image blocks.
    inline_images: Vec<String>,
}

impl<'a> Fb2Reader<'a> {
    fn new(data: &'a [u8]) -> Self {
        let mut xml = Reader::from_reader(data);
        let config = xml.config_mut();
        config.expand_empty_elements = true;
        config.check_end_names = false; // broken books are common; nesting is tracked by depth
        Self {
            xml,
            buf: Vec::new(),
            book: Book::default(),
            main_body_seen: false,
            prologue: Vec::new(),
            next_seq: 0,
            chapter_seqs: Vec::new(),
            ids: HashMap::new(),
            inline_images: Vec::new(),
        }
    }

    fn read(mut self) -> Result<Book, Error> {
        let mut root_seen = false;
        let result: Res<()> = (|| {
            loop {
                match self.next()? {
                    Ev::Eof => return Ok(()),
                    Ev::Start(name, attrs) => {
                        if !root_seen {
                            if name != "FictionBook" {
                                return Err(Broken);
                            }
                            root_seen = true;
                            continue;
                        }
                        match name.as_str() {
                            "title-info" => self.read_title_info()?,
                            "document-info" | "publish-info" | "custom-info" | "stylesheet" => self.skip()?,
                            "body" => self.read_body(attr(&attrs, "name"))?,
                            "binary" => self.read_binary(&attrs)?,
                            _ => {} // descend: `description` holds `title-info`
                        }
                    }
                    _ => {}
                }
            }
        })();
        if !root_seen {
            return Err(Error::NotABook);
        }
        let _ = result; // past the root a broken file still yields what was read
        self.flush_prologue();
        if self.book.chapters.iter().all(|c| c.blocks.is_empty()) {
            return Err(Error::NoText);
        }
        self.resolve_links();
        Ok(self.book)
    }

    // region events

    fn next(&mut self) -> Res<Ev> {
        loop {
            self.buf.clear();
            let ev = match self.xml.read_event_into(&mut self.buf).map_err(|_| Broken)? {
                Event::Start(e) => {
                    let name = e.local_name().as_ref().to_owned();
                    let attrs = e
                        .attributes()
                        .flatten()
                        // Raw value: the attributes read here (`type`, `name`, `id`, `href`) carry no entities.
                        .map(|a| (a.key.local_name().as_ref().to_owned(), a.value.into_owned()))
                        .collect();
                    Ev::Start(name, attrs)
                }
                Event::End(_) => Ev::End,
                Event::Text(e) => Ev::Text(e.xml10_content().into_owned()),
                Event::CData(e) => Ev::Text(e.into_inner().into_owned()),
                Event::GeneralRef(r) => match r.resolve_char_ref() {
                    Ok(Some(c)) => Ev::Text(c.to_string()),
                    _ => Ev::Text(entity(&r.into_inner()).to_owned()),
                },
                Event::Eof => Ev::Eof,
                _ => continue,
            };
            return Ok(ev);
        }
    }

    /// Consumes the rest of the current element.
    fn skip(&mut self) -> Res<()> {
        let mut depth = 0;
        loop {
            match self.next()? {
                Ev::Start(..) => depth += 1,
                Ev::End if depth == 0 => return Ok(()),
                Ev::End => depth -= 1,
                Ev::Eof => return Err(Broken),
                Ev::Text(_) => {}
            }
        }
    }

    /// Calls `f` for every child element of the current element; `f` must consume the child.
    fn for_each_child(&mut self, mut f: impl FnMut(&mut Self, &str, &[(String, String)]) -> Res<()>) -> Res<()> {
        loop {
            match self.next()? {
                Ev::Start(name, attrs) => f(self, &name, &attrs)?,
                Ev::End => return Ok(()),
                Ev::Eof => return Err(Broken),
                Ev::Text(_) => {}
            }
        }
    }

    /// All text of the current element, as is.
    fn read_raw_text(&mut self) -> Res<String> {
        let mut out = String::new();
        let mut depth = 0;
        loop {
            match self.next()? {
                Ev::Text(t) => out.push_str(&t),
                Ev::Start(..) => depth += 1,
                Ev::End if depth == 0 => return Ok(out),
                Ev::End => depth -= 1,
                Ev::Eof => return Err(Broken),
            }
        }
    }

    fn read_plain_text(&mut self) -> Res<String> {
        Ok(self.read_raw_text()?.split_ascii_whitespace().collect::<Vec<_>>().join(" "))
    }

    // endregion

    // region description

    fn read_title_info(&mut self) -> Res<()> {
        self.for_each_child(|r, name, _| {
            match name {
                "book-title" => r.book.title = r.read_plain_text()?,
                "author" => {
                    let author = r.read_author()?;
                    if !author.is_empty() {
                        r.book.authors.push(author);
                    }
                }
                "lang" => r.book.language = Some(r.read_plain_text()?).filter(|l| !l.is_empty()),
                "coverpage" => r.for_each_child(|r, name, attrs| {
                    if name == "image" && r.book.cover.is_none() {
                        r.book.cover = image_id(attrs);
                    }
                    r.skip()
                })?,
                "annotation" => {
                    let mut lines = Vec::new();
                    r.for_each_child(|r, name, _| {
                        if name == "p" {
                            lines.push(r.read_plain_text()?);
                            Ok(())
                        } else {
                            r.skip()
                        }
                    })?;
                    let text = lines.into_iter().filter(|l| !l.is_empty()).collect::<Vec<_>>().join("\n");
                    r.book.annotation = (!text.is_empty()).then_some(text);
                }
                _ => r.skip()?,
            }
            Ok(())
        })
    }

    fn read_author(&mut self) -> Res<String> {
        let mut parts: Vec<(String, String)> = Vec::new();
        self.for_each_child(|r, name, _| {
            let text = r.read_plain_text()?;
            parts.push((name.to_owned(), text));
            Ok(())
        })?;
        let get = |key: &str| parts.iter().find(|(k, _)| k == key).map(|(_, v)| v.as_str()).unwrap_or("");
        let full = [get("first-name"), get("middle-name"), get("last-name")]
            .iter()
            .filter(|s| !s.is_empty())
            .copied()
            .collect::<Vec<_>>()
            .join(" ");
        Ok(if full.is_empty() { get("nickname").to_owned() } else { full })
    }

    // endregion

    // region body

    fn read_body(&mut self, name: Option<&str>) -> Res<()> {
        match name {
            Some("notes" | "comments") => self.read_notes(),
            None if !self.main_body_seen => {
                self.main_body_seen = true;
                self.read_main_body()
            }
            _ => self.skip(),
        }
    }

    fn read_main_body(&mut self) -> Res<()> {
        self.for_each_child(|r, name, attrs| {
            match name {
                "section" => {
                    r.flush_prologue();
                    let mut section = Section { id: attr(attrs, "id").map(str::to_owned), items: Vec::new() };
                    // Emitted even when reading fails: a truncated book keeps the section it broke off in.
                    let read = r.read_section(&mut section, 1);
                    r.emit_section(section, 1, Vec::new());
                    read?;
                }
                "title" => {
                    let mut out = std::mem::take(&mut r.prologue);
                    let read = r.read_title(0, &mut out);
                    r.prologue = out;
                    read?;
                }
                _ => {
                    let mut out = std::mem::take(&mut r.prologue);
                    let read = r.read_element(name, attrs, Kind::Body, 0, &mut out);
                    r.prologue = out;
                    read?;
                }
            }
            Ok(())
        })
    }

    fn flush_prologue(&mut self) {
        if self.prologue.is_empty() {
            return;
        }
        let items = std::mem::take(&mut self.prologue);
        let mut blocks = Vec::new();
        flatten(items, &mut blocks);
        self.push_chapter(blocks);
    }

    fn push_chapter(&mut self, blocks: Vec<(Seq, Block)>) {
        let title = heading_title(blocks.iter().map(|(_, b)| b));
        let (seqs, blocks): (Vec<Seq>, Vec<Block>) = blocks.into_iter().unzip();
        self.book.chapters.push(Chapter { title, blocks });
        self.chapter_seqs.push(seqs);
    }

    fn read_section(&mut self, section: &mut Section, depth: u32) -> Res<()> {
        let result = self.for_each_child(|r, name, attrs| {
            if name == "section" {
                section.items.push(Item::Section(Section { id: attr(attrs, "id").map(str::to_owned), items: Vec::new() }));
                let Some(Item::Section(child)) = section.items.last_mut() else { unreachable!() };
                r.read_section(child, depth + 1)
            } else {
                r.read_element(name, attrs, Kind::Body, depth, &mut section.items)
            }
        });
        if let (Some(id), Some(range)) = (&section.id, section.seq_range()) {
            self.ids.entry(id.clone()).or_insert(range);
        }
        result
    }

    /// Lays out a top-level section as chapters. A "part" — subsections and nothing of its own but a title, epigraph
    /// and the like — is opened up: its subsections become chapters and its own blocks go to the start of the first one.
    /// A section with text of its own is one chapter together with all its subsections.
    fn emit_section(&mut self, section: Section, level: u32, preamble: Vec<(Seq, Block)>) {
        if let Some(title) = section_title(&section) {
            let target = Locator { chapter: self.book.chapters.len(), block: preamble.len(), offset: 0 };
            self.book.toc.push(TocEntry { title, level, target });
        }
        let has_subsections = section.items.iter().any(|i| matches!(i, Item::Section(_)));
        let is_part = has_subsections
            && section.items.iter().all(|i| matches!(i, Item::Section(_)) || matches!(i, Item::Block(_, b) if is_preamble(b)));
        if is_part {
            let mut pending = preamble;
            for item in section.items {
                match item {
                    Item::Section(child) => self.emit_section(child, level + 1, std::mem::take(&mut pending)),
                    Item::Block(s, b) => pending.push((s, b)),
                }
            }
            if !pending.is_empty()
                && let (Some(last), Some(seqs)) = (self.book.chapters.last_mut(), self.chapter_seqs.last_mut())
            {
                for (s, b) in pending {
                    last.blocks.push(b);
                    seqs.push(s);
                }
            }
        } else {
            let chapter = self.book.chapters.len();
            let mut blocks = preamble;
            for item in section.items {
                match item {
                    Item::Block(s, b) => blocks.push((s, b)),
                    Item::Section(child) => self.flatten_section(child, level + 1, chapter, &mut blocks),
                }
            }
            self.push_chapter(blocks);
        }
    }

    /// Nested sections of a chapter: their blocks in order, their titles in the contents.
    fn flatten_section(&mut self, section: Section, level: u32, chapter: usize, out: &mut Vec<(Seq, Block)>) {
        if let Some(title) = section_title(&section) {
            self.book.toc.push(TocEntry { title, level, target: Locator { chapter, block: out.len(), offset: 0 } });
        }
        for item in section.items {
            match item {
                Item::Block(s, b) => out.push((s, b)),
                Item::Section(child) => self.flatten_section(child, level + 1, chapter, out),
            }
        }
    }

    /// Sections with an id are notes; sections without one are searched for nested notes.
    fn read_notes(&mut self) -> Res<()> {
        self.for_each_child(|r, name, attrs| if name == "section" { r.read_note_section(attrs) } else { r.skip() })
    }

    fn read_note_section(&mut self, attrs: &[(String, String)]) -> Res<()> {
        let Some(id) = attr(attrs, "id").map(str::to_owned) else { return self.read_notes() };
        let mut title = None;
        let mut items = Vec::new();
        self.for_each_child(|r, name, attrs| {
            if name == "title" {
                let text = r.read_plain_text()?;
                title = (!text.is_empty()).then_some(text);
                Ok(())
            } else {
                r.read_element(name, attrs, Kind::Body, 2, &mut items)
            }
        })?;
        let mut blocks = Vec::new();
        flatten(items, &mut blocks);
        self.book.notes.insert(id, Note { title, blocks: blocks.into_iter().map(|(_, b)| b).collect() });
        Ok(())
    }

    fn push_block(&mut self, out: &mut Vec<Item>, block: Block) {
        out.push(Item::Block(self.next_seq, block));
        self.next_seq += 1;
    }

    fn read_flow(&mut self, kind: Kind, depth: u32, out: &mut Vec<Item>) -> Res<()> {
        self.for_each_child(|r, name, attrs| r.read_element(name, attrs, kind, depth, out))
    }

    /// One element of a flow; records the element's id, if any, over the blocks it produced.
    fn read_element(&mut self, name: &str, attrs: &[(String, String)], kind: Kind, depth: u32, out: &mut Vec<Item>) -> Res<()> {
        let first = self.next_seq;
        let result = self.read_element_blocks(name, attrs, kind, depth, out);
        if let Some(id) = attr(attrs, "id")
            && self.next_seq > first
        {
            self.ids.entry(id.to_owned()).or_insert((first, self.next_seq - 1));
        }
        result
    }

    fn read_element_blocks(&mut self, name: &str, attrs: &[(String, String)], kind: Kind, depth: u32, out: &mut Vec<Item>) -> Res<()> {
        match name {
            "p" | "v" | "subtitle" | "text-author" | "date" => {
                let kind = match name {
                    "v" => Kind::Verse,
                    "subtitle" => Kind::Subtitle,
                    "text-author" | "date" => Kind::TextAuthor,
                    _ => kind,
                };
                self.inline_images.clear();
                let text = self.read_styled_text()?;
                if !text.text.is_empty() {
                    self.push_block(out, Block::Paragraph { text, kind });
                }
                for id in std::mem::take(&mut self.inline_images) {
                    self.push_block(out, Block::Image { id });
                }
            }
            "empty-line" => {
                self.push_block(out, Block::EmptyLine);
                self.skip()?;
            }
            "image" => {
                if let Some(id) = image_id(attrs) {
                    self.push_block(out, Block::Image { id });
                }
                self.skip()?;
            }
            "title" if kind == Kind::Body => {
                let mut blocks = Vec::new();
                let read = self.read_title(depth, &mut blocks);
                out.extend(blocks);
                read?;
            }
            "title" => self.read_flow(Kind::Subtitle, depth, out)?,
            "epigraph" => self.read_flow(Kind::Epigraph, depth, out)?,
            "cite" | "annotation" => self.read_flow(Kind::Cite, depth, out)?,
            "poem" => self.read_flow(kind, depth, out)?,
            "stanza" => {
                if matches!(out.last(), Some(Item::Block(_, Block::Paragraph { kind: Kind::Verse, .. }))) {
                    self.push_block(out, Block::EmptyLine);
                }
                self.read_flow(kind, depth, out)?;
            }
            "table" => self.read_table(out)?,
            "section" => self.read_flow(Kind::Body, depth + 1, out)?,
            _ => self.skip()?,
        }
        Ok(())
    }

    fn read_title(&mut self, level: u32, out: &mut Vec<Item>) -> Res<()> {
        self.for_each_child(|r, name, _| {
            if name == "p" {
                let text = r.read_styled_text()?;
                if !text.text.is_empty() {
                    r.push_block(out, Block::Heading { text, level });
                }
                Ok(())
            } else {
                r.skip()
            }
        })
    }

    /// Rows `tr` of cells `td`/`th` with `colspan`; one-column tables (layout wrappers) become paragraphs.
    fn read_table(&mut self, out: &mut Vec<Item>) -> Res<()> {
        let mut rows: Vec<Vec<TableCell>> = Vec::new();
        self.for_each_child(|r, name, _| {
            if name != "tr" {
                return r.skip();
            }
            let mut cells = Vec::new();
            r.for_each_child(|r, name, attrs| {
                if name != "td" && name != "th" {
                    return r.skip();
                }
                let col_span = attr(attrs, "colspan").and_then(|s| s.trim().parse().ok()).unwrap_or(1usize).max(1);
                let text = r.read_styled_text()?;
                cells.push(TableCell { text, header: name == "th", col_span });
                Ok(())
            })?;
            if cells.iter().any(|c| !c.text.text.is_empty()) {
                rows.push(cells);
            }
            Ok(())
        })?;
        if rows.is_empty() {
            return Ok(());
        }
        if rows.iter().all(|row| row.len() == 1 && row[0].col_span == 1) {
            for row in rows {
                if let Some(cell) = row.into_iter().next() {
                    self.push_block(out, Block::Paragraph { text: cell.text, kind: Kind::Body });
                }
            }
            return Ok(());
        }
        let table = Arc::new(Table { rows: Table::with_bold_header(rows) });
        for row in 0..table.rows.len() {
            self.push_block(out, Block::TableRow { table: table.clone(), row });
        }
        Ok(())
    }

    fn read_binary(&mut self, attrs: &[(String, String)]) -> Res<()> {
        let id = attr(attrs, "id").map(str::to_owned);
        let content_type = attr(attrs, "content-type").unwrap_or_default().to_owned();
        let text = self.read_raw_text()?;
        let Some(id) = id else { return Ok(()) };
        let clean: Vec<u8> = text.bytes().filter(|b| !b.is_ascii_whitespace()).collect();
        // Padding is often missing in the wild; an undecodable image is skipped.
        let engine = base64::engine::GeneralPurpose::new(
            &base64::alphabet::STANDARD,
            base64::engine::GeneralPurposeConfig::new().with_decode_padding_mode(base64::engine::DecodePaddingMode::Indifferent),
        );
        if let Ok(bytes) = engine.decode(clean)
            && !bytes.is_empty()
        {
            self.book.images.insert(id, Image { content_type, bytes });
        }
        Ok(())
    }

    // endregion

    /// Element text with styles. XML whitespace collapses to single spaces while the text is built, so style ranges
    /// point into the final string. Built as chars for [`typography::normalize`], then converted to byte offsets.
    fn read_styled_text(&mut self) -> Res<StyledText> {
        let mut chars: Vec<char> = Vec::new();
        let mut ranges: Vec<(usize, usize, Style, Option<String>)> = Vec::new();
        let mut open: Vec<(Option<StyleTag>, usize)> = Vec::new();
        loop {
            match self.next()? {
                Ev::Text(t) => {
                    if matches!(open.last(), Some((Some((Style::NoteRef, _)), _))) {
                        // Footnote marker: "[12]" → "12".
                        chars.extend(t.chars().filter(|c| !"[]{}".contains(*c) && !c.is_ascii_whitespace()));
                    } else {
                        for c in t.chars() {
                            // XML whitespace only: a no-break space from `&nbsp;` must survive.
                            if c.is_ascii_whitespace() {
                                if chars.last().is_some_and(|l| *l != ' ') {
                                    chars.push(' ');
                                }
                            } else {
                                chars.push(c);
                            }
                        }
                    }
                }
                Ev::Start(name, attrs) => {
                    let style = match name.as_str() {
                        "emphasis" => Some((Style::Emphasis, None)),
                        "strong" => Some((Style::Strong, None)),
                        "sup" => Some((Style::Superscript, None)),
                        "sub" => Some((Style::Subscript, None)),
                        "strikethrough" => Some((Style::Strikethrough, None)),
                        "code" => Some((Style::Code, None)),
                        "a" if attr(&attrs, "type") == Some("note") => Some((Style::NoteRef, image_id(&attrs))),
                        "a" => link_key(&attrs).map(|key| (Style::Link, Some(key))),
                        "image" => {
                            if let Some(id) = image_id(&attrs) {
                                self.inline_images.push(id);
                            }
                            None
                        }
                        _ => None,
                    };
                    open.push((style, chars.len()));
                }
                Ev::End => match open.pop() {
                    None => break,
                    Some((Some((style, target)), start)) => ranges.push((start, chars.len(), style, target)),
                    Some((None, _)) => {}
                },
                Ev::Eof => return Err(Broken),
            }
        }
        if chars.last() == Some(&' ') {
            chars.pop();
        }
        typography::normalize(&mut chars);
        // Char index → byte offset.
        let mut byte_at = Vec::with_capacity(chars.len() + 1);
        let mut text = String::with_capacity(chars.len());
        for c in &chars {
            byte_at.push(text.len());
            text.push(*c);
        }
        byte_at.push(text.len());
        let styles = ranges
            .into_iter()
            .map(|(s, e, style, target)| StyleRange { start: byte_at[s.min(chars.len())], end: byte_at[e.min(chars.len())], style, target })
            .filter(|r| r.start < r.end)
            .collect();
        Ok(StyledText { text, styles })
    }

    /// Link targets: external addresses, notes, places in the book. Links to ids nowhere in the book become plain text.
    fn resolve_links(&mut self) {
        let mut place_of: HashMap<Seq, Locator> = HashMap::new();
        for (chapter, seqs) in self.chapter_seqs.iter().enumerate() {
            for (block, seq) in seqs.iter().enumerate() {
                place_of.insert(*seq, Locator { chapter, block, offset: 0 });
            }
        }
        let place = |id: &str| -> Option<LinkTarget> {
            let (first, last) = self.ids.get(id)?;
            let start = *place_of.get(first)?;
            let end = place_of.get(last).filter(|l| l.chapter == start.chapter).map(|l| Locator { block: l.block + 1, ..*l });
            Some(LinkTarget::Place { start, end })
        };
        let mut links = HashMap::new();
        for chapter in &self.book.chapters {
            for block in &chapter.blocks {
                for range in block.text().map(|t| t.styles.as_slice()).unwrap_or_default() {
                    let (Style::Link, Some(key)) = (range.style, &range.target) else { continue };
                    if links.contains_key(key) {
                        continue;
                    }
                    let target = match key.strip_prefix('#') {
                        None => Some(LinkTarget::External(key.clone())),
                        Some(id) if self.book.notes.contains_key(id) => Some(LinkTarget::Note(id.to_owned())),
                        Some(id) => place(id),
                    };
                    if let Some(t) = target {
                        links.insert(key.clone(), t);
                    }
                }
            }
        }
        for chapter in &mut self.book.chapters {
            for block in &mut chapter.blocks {
                if let Block::Heading { text, .. } | Block::Paragraph { text, .. } = block {
                    text.styles.retain(|r| r.style != Style::Link || r.target.as_ref().is_some_and(|k| links.contains_key(k)));
                }
            }
        }
        self.book.links = links;
    }
}

fn attr<'v>(attrs: &'v [(String, String)], name: &str) -> Option<&'v str> {
    attrs.iter().find(|(k, _)| k == name).map(|(_, v)| v.as_str())
}

/// `l:href`, `xlink:href`: the namespace prefix varies, so only the local name counts.
fn href(attrs: &[(String, String)]) -> Option<&str> {
    attr(attrs, "href").map(str::trim)
}

/// The id an `image` or note reference points to: `href` without the `#`.
fn image_id(attrs: &[(String, String)]) -> Option<String> {
    href(attrs).map(|h| h.trim_start_matches('#').to_owned()).filter(|h| !h.is_empty())
}

/// The key of a link that is not a note reference: `#id` or an external address; anything else is not a link.
fn link_key(attrs: &[(String, String)]) -> Option<String> {
    let raw = href(attrs)?;
    let lower = raw.to_ascii_lowercase();
    let external = lower.starts_with("http://") || lower.starts_with("https://") || lower.starts_with("mailto:");
    (external || (raw.len() > 1 && raw.starts_with('#'))).then(|| raw.to_owned())
}

fn flatten(items: Vec<Item>, out: &mut Vec<(Seq, Block)>) {
    for item in items {
        match item {
            Item::Block(s, b) => out.push((s, b)),
            Item::Section(child) => flatten(child.items, out),
        }
    }
}

fn is_preamble(block: &Block) -> bool {
    match block {
        Block::Heading { .. } | Block::EmptyLine | Block::Image { .. } => true,
        Block::Paragraph { kind, .. } => *kind != Kind::Body && *kind != Kind::Verse,
        Block::TableRow { .. } => false,
    }
}

fn section_title(section: &Section) -> Option<String> {
    heading_title(section.items.iter().map_while(|i| if let Item::Block(_, b) = i { Some(b) } else { None }))
}

/// Leading headings joined: "Part One. Chapter 1".
fn heading_title<'b>(blocks: impl Iterator<Item = &'b Block>) -> Option<String> {
    let title = blocks
        .map_while(|b| if let Block::Heading { text, .. } = b { Some(text.text.as_str()) } else { None })
        .collect::<Vec<_>>()
        .join(". ");
    (!title.trim().is_empty()).then_some(title)
}

/// Named entities that books use without a DTD, plus the XML ones.
fn entity(name: &str) -> &'static str {
    match name {
        "amp" => "&",
        "lt" => "<",
        "gt" => ">",
        "quot" => "\"",
        "apos" => "'",
        "nbsp" => "\u{a0}",
        "shy" => "\u{ad}",
        "thinsp" => "\u{2009}",
        "ensp" => "\u{2002}",
        "emsp" => "\u{2003}",
        "mdash" => "—",
        "ndash" => "–",
        "minus" => "−",
        "hellip" => "…",
        "bull" => "•",
        "middot" => "·",
        "laquo" => "«",
        "raquo" => "»",
        "bdquo" => "„",
        "ldquo" => "“",
        "rdquo" => "”",
        "lsquo" => "‘",
        "rsquo" => "’",
        "sbquo" => "‚",
        "prime" => "′",
        "Prime" => "″",
        "copy" => "©",
        "reg" => "®",
        "trade" => "™",
        "deg" => "°",
        "times" => "×",
        "divide" => "÷",
        "plusmn" => "±",
        "sect" => "§",
        "para" => "¶",
        "euro" => "€",
        "numero" => "№",
        _ => "",
    }
}

#[cfg(test)]
mod tests;
