//! FictionBook 2 parser (`.fb2`, `.fb2.zip`): one streaming pass over the XML into the [`Book`] model.
//!
//! A port of the Android app's `Fb2Reader`, trimmed for the spike: no notes bodies, images, tables or links yet.
//! The encoding comes from the XML declaration (FB2 files are often `windows-1251`).

use std::io::{Cursor, Read};

use quick_xml::Reader;
use quick_xml::events::Event;

use crate::model::{Block, Book, Chapter, Kind, Style, StyleRange, StyledText};
use crate::typography;

#[derive(Debug)]
pub enum Error {
    /// Not a FictionBook document (or a zip without one).
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
        return Fb2Reader::new(&xml).read();
    }
    Fb2Reader::new(bytes).read()
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

/// A section in document order: its own blocks and nested sections.
#[derive(Default)]
struct Section {
    items: Vec<Item>,
}

enum Item {
    Block(Block),
    Section(Section),
}

struct Fb2Reader<'a> {
    xml: Reader<&'a [u8]>,
    buf: Vec<u8>,
    book: Book,
    main_body_seen: bool,
    /// Blocks of the main body before its first section (title page, book epigraph): a chapter of their own.
    prologue: Vec<Block>,
}

impl<'a> Fb2Reader<'a> {
    fn new(data: &'a [u8]) -> Self {
        let mut xml = Reader::from_reader(data);
        let config = xml.config_mut();
        config.expand_empty_elements = true;
        config.check_end_names = false; // broken books are common; nesting is tracked by depth
        Self { xml, buf: Vec::new(), book: Book::default(), main_body_seen: false, prologue: Vec::new() }
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
                            "document-info" | "publish-info" | "custom-info" | "stylesheet" | "binary" => self.skip()?,
                            "body" => self.read_body(attr(&attrs, "name"))?,
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
        // A truncated file keeps everything read so far.
        self.flush_prologue();
        if self.book.chapters.iter().all(|c| c.blocks.is_empty()) {
            return Err(Error::NoText);
        }
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
                        .map(|a| {
                            // Raw value: the attributes read here (`type`, `name`) carry no entities.
                            (a.key.local_name().as_ref().to_owned(), a.value.into_owned())
                        })
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

    fn read_plain_text(&mut self) -> Res<String> {
        let mut out = String::new();
        let mut depth = 0;
        loop {
            match self.next()? {
                Ev::Text(t) => out.push_str(&t),
                Ev::Start(..) => depth += 1,
                Ev::End if depth == 0 => break,
                Ev::End => depth -= 1,
                Ev::Eof => return Err(Broken),
            }
        }
        Ok(out.split_ascii_whitespace().collect::<Vec<_>>().join(" "))
    }

    // endregion

    // region body

    fn read_body(&mut self, name: Option<&str>) -> Res<()> {
        if name.is_none() && !self.main_body_seen {
            self.main_body_seen = true;
            return self.read_main_body();
        }
        self.skip() // notes and comments: not in the spike
    }

    fn read_main_body(&mut self) -> Res<()> {
        self.for_each_child(|r, name, _| {
            match name {
                "section" => {
                    r.flush_prologue();
                    let mut section = Section::default();
                    let read = r.read_section(&mut section, 1);
                    r.emit_section(section, Vec::new());
                    read?;
                }
                "title" => {
                    let mut out = std::mem::take(&mut r.prologue);
                    let read = r.read_title(0, &mut out);
                    r.prologue = out;
                    read?;
                }
                _ => {
                    let mut out: Vec<Item> = r.prologue.drain(..).map(Item::Block).collect();
                    let read = r.read_element(name, Kind::Body, 0, &mut out);
                    r.prologue = out.into_iter().filter_map(|i| if let Item::Block(b) = i { Some(b) } else { None }).collect();
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
        let blocks = std::mem::take(&mut self.prologue);
        let title = heading_title(blocks.iter());
        self.book.chapters.push(Chapter { title, blocks });
    }

    fn read_section(&mut self, section: &mut Section, depth: u32) -> Res<()> {
        self.for_each_child(|r, name, _| {
            if name == "section" {
                section.items.push(Item::Section(Section::default()));
                let Some(Item::Section(child)) = section.items.last_mut() else { unreachable!() };
                r.read_section(child, depth + 1)
            } else {
                r.read_element(name, Kind::Body, depth, &mut section.items)
            }
        })
    }

    /// Lays a top-level section out as chapters. A "part" — subsections and nothing of its own but a title, epigraph
    /// and the like — is opened up: its subsections become chapters and its own blocks go to the start of the first one.
    /// A section with text of its own is one chapter together with all its subsections.
    fn emit_section(&mut self, section: Section, preamble: Vec<Block>) {
        let has_subsections = section.items.iter().any(|i| matches!(i, Item::Section(_)));
        let is_part =
            has_subsections && section.items.iter().all(|i| matches!(i, Item::Section(_)) || matches!(i, Item::Block(b) if is_preamble(b)));
        if is_part {
            let mut pending = preamble;
            for item in section.items {
                match item {
                    Item::Section(child) => self.emit_section(child, std::mem::take(&mut pending)),
                    Item::Block(b) => pending.push(b),
                }
            }
            if !pending.is_empty()
                && let Some(last) = self.book.chapters.last_mut()
            {
                last.blocks.extend(pending);
            }
        } else {
            let mut blocks = preamble;
            flatten(section, &mut blocks);
            let title = heading_title(blocks.iter());
            self.book.chapters.push(Chapter { title, blocks });
        }
    }

    fn read_flow(&mut self, kind: Kind, depth: u32, out: &mut Vec<Item>) -> Res<()> {
        self.for_each_child(|r, name, _| r.read_element(name, kind, depth, out))
    }

    fn read_element(&mut self, name: &str, kind: Kind, depth: u32, out: &mut Vec<Item>) -> Res<()> {
        match name {
            "p" => {
                let text = self.read_styled_text()?;
                push_paragraph(out, text, kind);
            }
            "v" => {
                let text = self.read_styled_text()?;
                push_paragraph(out, text, Kind::Verse);
            }
            "subtitle" => {
                let text = self.read_styled_text()?;
                push_paragraph(out, text, Kind::Subtitle);
            }
            "text-author" | "date" => {
                let text = self.read_styled_text()?;
                push_paragraph(out, text, Kind::TextAuthor);
            }
            "empty-line" => {
                out.push(Item::Block(Block::EmptyLine));
                self.skip()?;
            }
            "title" if kind == Kind::Body => {
                let mut blocks = Vec::new();
                let read = self.read_title(depth, &mut blocks);
                out.extend(blocks.into_iter().map(Item::Block));
                read?;
            }
            "title" => self.read_flow(Kind::Subtitle, depth, out)?,
            "epigraph" => self.read_flow(Kind::Epigraph, depth, out)?,
            "cite" | "annotation" => self.read_flow(Kind::Cite, depth, out)?,
            "poem" => self.read_flow(kind, depth, out)?,
            "stanza" => {
                if matches!(out.last(), Some(Item::Block(Block::Paragraph { kind: Kind::Verse, .. }))) {
                    out.push(Item::Block(Block::EmptyLine));
                }
                self.read_flow(kind, depth, out)?;
            }
            "section" => self.read_flow(Kind::Body, depth + 1, out)?,
            _ => self.skip()?, // images, tables: not in the spike
        }
        Ok(())
    }

    fn read_title(&mut self, level: u32, out: &mut Vec<Block>) -> Res<()> {
        self.for_each_child(|r, name, _| {
            if name == "p" {
                let text = r.read_styled_text()?;
                if !text.text.is_empty() {
                    out.push(Block::Heading { text, level });
                }
                Ok(())
            } else {
                r.skip()
            }
        })
    }

    // endregion

    /// Element text with styles. XML whitespace collapses to single spaces while the text is built, so style ranges
    /// point into the final string. Built as chars for [`typography::normalize`], then converted to byte offsets.
    fn read_styled_text(&mut self) -> Res<StyledText> {
        let mut chars: Vec<char> = Vec::new();
        let mut ranges: Vec<(usize, usize, Style)> = Vec::new();
        let mut open: Vec<(Option<Style>, usize)> = Vec::new();
        loop {
            match self.next()? {
                Ev::Text(t) => {
                    if matches!(open.last(), Some((Some(Style::NoteRef), _))) {
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
                        "emphasis" => Some(Style::Emphasis),
                        "strong" => Some(Style::Strong),
                        "sup" => Some(Style::Superscript),
                        "sub" => Some(Style::Subscript),
                        "strikethrough" => Some(Style::Strikethrough),
                        "code" => Some(Style::Code),
                        "a" if attr(&attrs, "type") == Some("note") => Some(Style::NoteRef),
                        _ => None,
                    };
                    open.push((style, chars.len()));
                }
                Ev::End => match open.pop() {
                    None => break,
                    Some((Some(style), start)) => ranges.push((start, chars.len(), style)),
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
            .map(|(s, e, style)| StyleRange { start: byte_at[s.min(chars.len())], end: byte_at[e.min(chars.len())], style })
            .filter(|r| r.start < r.end)
            .collect();
        Ok(StyledText { text, styles })
    }
}

fn attr<'v>(attrs: &'v [(String, String)], name: &str) -> Option<&'v str> {
    attrs.iter().find(|(k, _)| k == name).map(|(_, v)| v.as_str())
}

fn push_paragraph(out: &mut Vec<Item>, text: StyledText, kind: Kind) {
    if !text.text.is_empty() {
        out.push(Item::Block(Block::Paragraph { text, kind }));
    }
}

fn flatten(section: Section, out: &mut Vec<Block>) {
    for item in section.items {
        match item {
            Item::Block(b) => out.push(b),
            Item::Section(child) => flatten(child, out),
        }
    }
}

fn is_preamble(block: &Block) -> bool {
    match block {
        Block::Heading { .. } | Block::EmptyLine => true,
        Block::Paragraph { kind, .. } => *kind != Kind::Body && *kind != Kind::Verse,
    }
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
        "laquo" => "«",
        "raquo" => "»",
        "bdquo" => "„",
        "ldquo" => "“",
        "rdquo" => "”",
        "lsquo" => "‘",
        "rsquo" => "’",
        "copy" => "©",
        "deg" => "°",
        "times" => "×",
        "sect" => "§",
        "numero" => "№",
        _ => "",
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const BOOK: &str = r##"<?xml version="1.0" encoding="utf-8"?>
<FictionBook xmlns="http://www.gribuser.ru/xml/fictionbook/2.0" xmlns:l="http://www.w3.org/1999/xlink">
 <description><title-info><author><first-name>Анна</first-name><last-name>Ахматова</last-name></author>
  <book-title>Книга</book-title><lang>ru</lang></title-info></description>
 <body>
  <title><p>Книга</p></title>
  <section><title><p>Часть первая</p></title>
   <section><title><p>Глава 1</p></title><p>Текст <emphasis>с курсивом</emphasis>&nbsp;и сноской<a l:href="#n1" type="note">[1]</a>.</p></section>
   <section><title><p>Глава 2</p></title><poem><stanza><v>Строка</v></stanza><stanza><v>Ещё</v></stanza></poem></section>
  </section>
 </body>
 <body name="notes"><section id="n1"><p>Сноска</p></section></body>
</FictionBook>"##;

    #[test]
    fn parses_meta_parts_and_styles() {
        let book = parse(BOOK.as_bytes()).unwrap();
        assert_eq!(book.title, "Книга");
        assert_eq!(book.authors, ["Анна Ахматова"]);
        assert_eq!(book.language.as_deref(), Some("ru"));
        // Prologue, then the part opened up into two chapters; the part title goes to the first one.
        let titles: Vec<_> = book.chapters.iter().map(|c| c.title.clone().unwrap_or_default()).collect();
        assert_eq!(titles, ["Книга", "Часть первая. Глава 1", "Глава 2"]);
        let Block::Paragraph { text, .. } = &book.chapters[1].blocks[2] else { panic!() };
        assert_eq!(text.text, "Текст с\u{a0}курсивом\u{a0}и\u{a0}сноской1.");
        let em = text.styles.iter().find(|s| s.style == Style::Emphasis).unwrap();
        assert_eq!(&text.text[em.start..em.end], "с\u{a0}курсивом");
        let note = text.styles.iter().find(|s| s.style == Style::NoteRef).unwrap();
        assert_eq!(&text.text[note.start..note.end], "1");
        assert_eq!(book.chapters[2].blocks.iter().filter(|b| **b == Block::EmptyLine).count(), 1);
    }

    #[test]
    fn rejects_non_books() {
        assert!(matches!(parse(b"<html><body/></html>"), Err(Error::NotABook)));
        assert!(matches!(parse(b"plain text"), Err(Error::NotABook)));
    }
}
