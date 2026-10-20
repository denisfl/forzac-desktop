//! Sending a book to a Kindle (`send-to-kindle`): the book as a MOBI 6 file, laid out as Calibre's "old" MOBI output
//! that the maintainer's Kindle reads, and the copy onto a Kindle connected as a disk.
//!
//! The markup is Mobipocket HTML (`<mbp:pagebreak/>`, `<p height width align>`, `<img recindex>`, `<a filepos>`).
//! Links are written as ten-digit placeholders while the targets' byte offsets are noted, then filled in.

use std::collections::HashMap;
use std::fmt::Write as _;
use std::path::{Path, PathBuf};
use std::sync::Arc;

use crate::model::{Block, Book, Kind, LinkTarget, Style, StyledText, Table};
use crate::state::Edits;

/// Bytes of markup in a text record.
const RECORD_SIZE: usize = 4096;

/// A place a link can lead to.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
enum Target {
    /// A block of a chapter.
    Block(usize, usize),
    /// A note's heading in the notes.
    Note(String),
    /// A note's first marker in the text.
    Marker(String),
    Contents,
    /// The first chapter: where the Kindle opens the book.
    Start,
}

/// The book as Mobipocket HTML, its image ids in record order and the start offset.
struct Markup {
    text: String,
    /// Image ids in `recindex` order (1-based in the markup); the cover first when there is one.
    images: Vec<String>,
    start: usize,
    cover: bool,
    /// The contents' entries: where each starts in the text, and its title.
    toc: Vec<(usize, String)>,
    /// Where the chapters end (the notes and the contents page follow).
    body_end: usize,
}

struct Writer<'b> {
    book: &'b Book,
    out: String,
    /// Where each `filepos=` placeholder's digits are, and what it leads to.
    slots: Vec<(usize, Target)>,
    places: HashMap<Target, usize>,
    images: Vec<String>,
    /// Note ids in the order of their first marker.
    notes: Vec<String>,
}

impl Writer<'_> {
    fn place(&mut self, target: Target) {
        self.places.entry(target).or_insert(self.out.len());
    }

    fn link(&mut self, target: Target) {
        self.out.push_str("<a filepos=");
        self.slots.push((self.out.len(), target));
        self.out.push_str("0000000000>");
    }

    /// `recindex` of image `id`, added on first use; none for an image the Kindle cannot show.
    fn image(&mut self, id: &str) -> Option<usize> {
        let image = self.book.images.get(id)?;
        if !matches!(image.content_type.as_str(), "image/jpeg" | "image/png" | "image/gif") {
            return None;
        }
        let at = self.images.iter().position(|i| i == id).unwrap_or_else(|| {
            self.images.push(id.to_owned());
            self.images.len() - 1
        });
        Some(at + 1)
    }

    fn text(&mut self, text: &str) {
        for c in text.chars() {
            match c {
                '&' => self.out.push_str("&amp;"),
                '<' => self.out.push_str("&lt;"),
                '>' => self.out.push_str("&gt;"),
                '\u{ad}' => {} // soft hyphens: MOBI readers may show them
                c => self.out.push(c),
            }
        }
    }

    /// Styled text: each run between style boundaries opens and closes its own tags, so overlapping ranges nest.
    fn styled(&mut self, t: &StyledText) {
        let mut cuts: Vec<usize> = vec![0, t.text.len()];
        cuts.extend(t.styles.iter().flat_map(|s| [s.start, s.end]));
        cuts.sort_unstable();
        cuts.dedup();
        for pair in cuts.windows(2) {
            let (a, b) = (pair[0], pair[1]);
            if a >= b || !t.text.is_char_boundary(a) || !t.text.is_char_boundary(b) {
                continue;
            }
            let active: Vec<_> = t.styles.iter().filter(|s| s.start <= a && s.end >= b).collect();
            let mut closing = Vec::new();
            let link = active.iter().find(|s| matches!(s.style, Style::Link | Style::NoteRef));
            if let Some(s) = link {
                let target = s.target.clone().unwrap_or_default();
                match (s.style, self.book.links.get(&target)) {
                    (Style::NoteRef, _) | (_, Some(LinkTarget::Note(_))) => {
                        let id = match self.book.links.get(&target) {
                            Some(LinkTarget::Note(id)) => id.clone(),
                            _ => target,
                        };
                        if self.book.notes.contains_key(&id) {
                            if !self.notes.contains(&id) {
                                self.notes.push(id.clone());
                            }
                            self.place(Target::Marker(id.clone()));
                            self.link(Target::Note(id));
                            closing.push("</a>");
                        }
                    }
                    (_, Some(LinkTarget::Place { start, .. })) => {
                        self.link(Target::Block(start.chapter, start.block));
                        closing.push("</a>");
                    }
                    (_, Some(LinkTarget::External(url))) => {
                        self.out.push_str("<a href=\"");
                        self.text(&url.replace('"', "%22"));
                        self.out.push_str("\">");
                        closing.push("</a>");
                    }
                    (_, None) => {}
                }
            }
            for (style, open, close) in [
                (Style::Strong, "<b>", "</b>"),
                (Style::Emphasis, "<i>", "</i>"),
                (Style::NoteRef, "<sup>", "</sup>"),
                (Style::Superscript, "<sup>", "</sup>"),
                (Style::Subscript, "<sub>", "</sub>"),
                (Style::Strikethrough, "<s>", "</s>"),
                (Style::Code, "<tt>", "</tt>"),
            ] {
                if active.iter().any(|s| s.style == style) {
                    self.out.push_str(open);
                    closing.push(close);
                }
            }
            self.text(&t.text[a..b]);
            for close in closing.into_iter().rev() {
                self.out.push_str(close);
            }
        }
    }

    fn paragraph(&mut self, attrs: &str, t: &StyledText, wrap: (&str, &str)) {
        let _ = write!(self.out, "<p {attrs}>{}", wrap.0);
        self.styled(t);
        let _ = write!(self.out, "{}</p>", wrap.1);
    }

    fn block(&mut self, block: &Block, table: &mut Option<Arc<Table>>) {
        // A table's rows are blocks of their own; the table closes before anything else.
        let row_of = |b: &Block| if let Block::TableRow { table, .. } = b { Some(table.clone()) } else { None };
        if table.is_some() && row_of(block).as_ref().map(Arc::as_ptr) != table.as_ref().map(Arc::as_ptr) {
            self.out.push_str("</table>");
            *table = None;
        }
        match block {
            Block::Heading { text, level } => {
                let size = match level {
                    0 => 7,
                    1 => 6,
                    2 => 5,
                    _ => 4,
                };
                let _ = write!(self.out, "<p height=\"2em\" width=\"0pt\" align=\"center\"><font size=\"{size}\"><b>");
                self.styled(text);
                self.out.push_str("</b></font></p>");
            }
            Block::Paragraph { text, kind } => match kind {
                Kind::Body => self.paragraph("height=\"0pt\" width=\"2em\" align=\"justify\"", text, ("", "")),
                Kind::Epigraph => {
                    self.out.push_str("<blockquote height=\"0pt\" width=\"0pt\" align=\"left\"><i>");
                    self.styled(text);
                    self.out.push_str("</i></blockquote>");
                }
                Kind::Verse => {
                    self.out.push_str("<blockquote height=\"0pt\" width=\"0pt\" align=\"left\">");
                    self.styled(text);
                    self.out.push_str("</blockquote>");
                }
                Kind::Cite => {
                    self.out.push_str("<blockquote height=\"0pt\" width=\"2em\" align=\"justify\">");
                    self.styled(text);
                    self.out.push_str("</blockquote>");
                }
                Kind::Subtitle => self.paragraph("height=\"1em\" width=\"0pt\" align=\"center\"", text, ("<b>", "</b>")),
                Kind::TextAuthor => self.paragraph("height=\"0pt\" width=\"0pt\" align=\"right\"", text, ("<i>", "</i>")),
                Kind::Code => {
                    self.out.push_str("<p height=\"0pt\" width=\"0pt\" align=\"left\"><tt>");
                    for (i, line) in text.text.split('\n').enumerate() {
                        if i > 0 {
                            self.out.push_str("<br/>");
                        }
                        self.text(line);
                    }
                    self.out.push_str("</tt></p>");
                }
            },
            Block::EmptyLine => self.out.push_str("<p height=\"1em\" width=\"0pt\"> </p>"),
            Block::Image { id } => {
                if let Some(n) = self.image(id) {
                    let _ = write!(
                        self.out,
                        "<p height=\"1em\" width=\"0pt\" align=\"center\"><img recindex=\"{n:05}\" align=\"baseline\"></img></p>"
                    );
                }
            }
            Block::TableRow { table: t, row } => {
                if table.is_none() {
                    self.out.push_str("<table border=\"1\">");
                    *table = Some(t.clone());
                }
                self.out.push_str("<tr>");
                for cell in &t.rows[*row] {
                    let tag = if cell.header { "th" } else { "td" };
                    let span = if cell.col_span > 1 { format!(" colspan=\"{}\"", cell.col_span) } else { String::new() };
                    let _ = write!(self.out, "<{tag}{span}>");
                    self.styled(&cell.text);
                    let _ = write!(self.out, "</{tag}>");
                }
                self.out.push_str("</tr>");
            }
        }
    }
}

/// The words of the generated pages in the book's language.
fn words(language: Option<&str>) -> (&'static str, &'static str) {
    if language.is_some_and(|l| l.to_lowercase().starts_with("ru")) {
        ("Примечания", "Содержание")
    } else {
        ("Notes", "Contents")
    }
}

/// `book` as Mobipocket HTML with its title and authors as `title` and `authors`.
fn markup(book: &Book, title: &str, authors: &str) -> Markup {
    let mut w = Writer { book, out: String::new(), slots: Vec::new(), places: HashMap::new(), images: Vec::new(), notes: Vec::new() };
    let (notes_title, contents_title) = words(book.language.as_deref());
    w.out.push_str("<html><head><guide><reference type=\"toc\" title=\"");
    w.text(contents_title);
    w.out.push_str("\" filepos=");
    w.slots.push((w.out.len(), Target::Contents));
    w.out.push_str("0000000000 /></guide></head><body>");
    let cover = book.cover.as_deref().and_then(|c| w.image(c)).is_some();
    if cover {
        w.out.push_str(
            "<p height=\"0pt\" width=\"0pt\" align=\"center\"><img recindex=\"00001\" align=\"baseline\"></img></p><mbp:pagebreak/>",
        );
    }
    w.out.push_str("<p height=\"2em\" width=\"0pt\" align=\"center\"><font size=\"7\"><b>");
    w.text(title);
    w.out.push_str("</b></font></p>");
    if !authors.is_empty() {
        w.out.push_str("<p height=\"1em\" width=\"0pt\" align=\"center\"><font size=\"5\">");
        w.text(authors);
        w.out.push_str("</font></p>");
    }
    for (c, chapter) in book.chapters.iter().enumerate() {
        w.out.push_str("<mbp:pagebreak/>");
        if c == 0 {
            w.place(Target::Start);
        }
        let mut table = None;
        for (b, block) in chapter.blocks.iter().enumerate() {
            w.place(Target::Block(c, b));
            w.block(block, &mut table);
        }
        if table.is_some() {
            w.out.push_str("</table>");
        }
    }
    let body_end = w.out.len();
    // The notes: those with markers in their order, then the others.
    let mut rest: Vec<String> = book.notes.keys().filter(|id| !w.notes.contains(id)).cloned().collect();
    rest.sort();
    let notes: Vec<String> = w.notes.iter().cloned().chain(rest).collect();
    if !notes.is_empty() {
        w.out.push_str("<mbp:pagebreak/><p height=\"2em\" width=\"0pt\" align=\"center\"><font size=\"6\"><b>");
        w.text(notes_title);
        w.out.push_str("</b></font></p>");
        for (n, id) in notes.iter().enumerate() {
            let note = &book.notes[id];
            w.place(Target::Note(id.clone()));
            w.out.push_str("<p height=\"1em\" width=\"0pt\" align=\"left\"><b>");
            let label = note.title.clone().unwrap_or_else(|| (n + 1).to_string());
            let back = w.notes.contains(id);
            if back {
                w.link(Target::Marker(id.clone()));
            }
            w.text(&label);
            if back {
                w.out.push_str("</a>");
            }
            w.out.push_str("</b></p>");
            let mut table = None;
            for block in &note.blocks {
                w.block(block, &mut table);
            }
            if table.is_some() {
                w.out.push_str("</table>");
            }
        }
    }
    w.out.push_str("<mbp:pagebreak/>");
    w.place(Target::Contents);
    w.out.push_str("<p height=\"1em\" width=\"0pt\" align=\"center\"><font size=\"6\"><b>");
    w.text(contents_title);
    w.out.push_str("</b></font></p>");
    for entry in book.contents() {
        let indent = entry.level.saturating_sub(1) * 2;
        let _ = write!(w.out, "<p height=\"0pt\" width=\"{indent}em\" align=\"left\">");
        w.link(Target::Block(entry.target.chapter, entry.target.block));
        w.text(&entry.title);
        w.out.push_str("</a></p>");
    }
    w.out.push_str("<mbp:pagebreak/></body></html>");
    // The positions, now that every target has its place; a target that is not there leads to the start.
    let start = w.places.get(&Target::Start).copied().unwrap_or(0);
    let toc = book
        .contents()
        .into_iter()
        .filter_map(|e| w.places.get(&Target::Block(e.target.chapter, e.target.block)).map(|&at| (at, e.title)))
        .collect();
    let mut text = w.out.into_bytes();
    for (at, target) in &w.slots {
        let pos = w.places.get(target).copied().unwrap_or(start);
        text[*at..*at + 10].copy_from_slice(format!("{pos:010}").as_bytes());
    }
    Markup { text: String::from_utf8(text).unwrap_or_default(), images: w.images, start, cover, toc, body_end }
}

// region the file

/// The text in records of `RECORD_SIZE` bytes, each followed by the rest of a character it splits and that count
/// (extra data flag 1), then, with an NCX index, the record's trailing byte sequence (flag 2), as Calibre writes them.
fn text_records(text: &[u8], ncx: Option<&[NcxEntry]>) -> Vec<Vec<u8>> {
    text.chunks(RECORD_SIZE)
        .enumerate()
        .map(|(i, chunk)| {
            let end = i * RECORD_SIZE + chunk.len();
            // The bytes after the record that continue its last character.
            let overlap = text[end..].iter().take(3).take_while(|b| (**b & 0xC0) == 0x80).count();
            let mut record = chunk.to_vec();
            record.extend_from_slice(&text[end..end + overlap]);
            record.push(overlap as u8);
            if let Some(entries) = ncx {
                let mut tbs = tbs(entries, i * RECORD_SIZE, end - 1);
                tbs.push(0x80 | (tbs.len() as u8 + 1));
                record.extend(tbs);
            }
            record
        })
        .collect()
}

/// An entry of the NCX index: the place of a contents entry in the text, up to the next one.
#[derive(Debug, Clone, PartialEq)]
struct NcxEntry {
    offset: usize,
    length: usize,
    title: String,
}

/// The NCX index's entries (Calibre's `create_book_index`): flat, by offset, one per place, each up to the next or to
/// `body_end`; empty ones dropped.
fn ncx_entries(toc: &[(usize, String)], body_end: usize) -> Vec<NcxEntry> {
    let mut places: Vec<(usize, String)> = toc.iter().filter(|(at, _)| *at < body_end).cloned().collect();
    places.sort_by_key(|(at, _)| *at);
    places.dedup_by_key(|(at, _)| *at);
    let ends: Vec<usize> = places.iter().skip(1).map(|(at, _)| *at).chain([body_end]).collect();
    places
        .into_iter()
        .zip(ends)
        .map(|((offset, title), end)| NcxEntry { offset, length: end - offset, title })
        .filter(|e| e.length > 0)
        .collect()
}

/// A forward variable-width integer: 7 bits a byte, the last byte marked.
fn encint(mut value: usize) -> Vec<u8> {
    let mut out = Vec::new();
    loop {
        out.push((value & 0x7F) as u8);
        value >>= 7;
        if value == 0 {
            break;
        }
    }
    out[0] |= 0x80;
    out.reverse();
    out
}

/// The trailing byte sequence of the text record `[start, end]` for a book's flat NCX (Calibre's `book_tbs`).
fn tbs(entries: &[NcxEntry], start: usize, end: usize) -> Vec<u8> {
    let last = |e: &NcxEntry| e.offset + e.length - 1;
    let starts: Vec<usize> = (0..entries.len()).filter(|&i| (start..=end).contains(&entries[i].offset)).collect();
    let ends: Vec<usize> =
        (0..entries.len()).filter(|&i| entries[i].offset < start && (start..=end).contains(&last(&entries[i]))).collect();
    let spans = (0..entries.len()).find(|&i| entries[i].offset < start && last(&entries[i]) > end);
    // `encode_tbs`: the index shifted past four flag bits, then 0 for flag 0b0010, the count for 0b0100, 0 for 0b0001.
    let encode = |index: usize, flags: usize, count: Option<usize>| {
        let mut out = encint((index << 4) | flags);
        out.extend(encint(0));
        if let Some(count) = count {
            out.push(count as u8);
        }
        if flags & 1 != 0 {
            out.extend(encint(0));
        }
        out
    };
    let completes = starts.iter().filter(|&&i| last(&entries[i]) <= end).count();
    let touching: Vec<usize> = ends.iter().chain(&starts).copied().collect();
    if !ends.is_empty() || completes > 0 {
        if completes == 0 && ((starts.len() == 1 && ends.is_empty()) || (ends.len() == 1 && starts.is_empty())) {
            encode(touching[0], 0b0010, None)
        } else {
            let first = touching.iter().copied().min().unwrap_or(0);
            encode(first, 0b0110, Some(touching.len()))
        }
    } else if let Some(&first) = starts.first() {
        encode(first, 0b0010, None)
    } else if let Some(i) = spans {
        encode(i, 0b0011, None)
    } else {
        Vec::new()
    }
}

/// `block` padded with zeros to a multiple of four bytes.
fn align(mut block: Vec<u8>) -> Vec<u8> {
    while !block.len().is_multiple_of(4) {
        block.push(0);
    }
    block
}

/// The NCX index records (Calibre's MOBI 6 `flat_book` index): the header record, one index record, the CNCX record.
/// None when the index would not fit one record.
fn ncx_records(entries: &[NcxEntry]) -> Option<Vec<Vec<u8>>> {
    if entries.is_empty() {
        return None;
    }
    let ident = |i: usize| {
        let mut hex = format!("{i:X}");
        if hex.len() % 2 != 0 {
            hex.insert(0, '0');
        }
        let mut out = vec![hex.len() as u8];
        out.extend(hex.into_bytes());
        out
    };
    // CNCX: each title once, its length then its bytes.
    let mut cncx = Vec::new();
    let mut labels: HashMap<&str, usize> = HashMap::new();
    for e in entries {
        if !labels.contains_key(e.title.as_str()) {
            labels.insert(&e.title, cncx.len());
            let bytes: Vec<u8> = e.title.chars().take(500).collect::<String>().into_bytes();
            cncx.extend(encint(bytes.len()));
            cncx.extend(bytes);
        }
    }
    if cncx.len() > 0xFBF8 {
        return None;
    }
    // The index record.
    let mut block = Vec::new();
    let mut offsets = Vec::new();
    for (i, e) in entries.iter().enumerate() {
        offsets.push(block.len());
        block.extend(ident(i));
        block.push(0b1111); // offset, length, label, depth
        for v in [e.offset, e.length, labels[e.title.as_str()], 0] {
            block.extend(encint(v));
        }
    }
    let block = align(block);
    let mut idxt = b"IDXT".to_vec();
    for o in &offsets {
        idxt.extend_from_slice(&((192 + o) as u16).to_be_bytes());
    }
    let idxt = align(idxt);
    let be32 = |n: usize| (n as u32).to_be_bytes();
    let mut record = b"INDX".to_vec();
    record.extend(be32(192));
    record.extend([0; 4]);
    record.extend(be32(1));
    record.extend([0; 4]);
    record.extend(be32(192 + block.len()));
    record.extend(be32(entries.len()));
    record.extend([0xFF; 8]);
    record.extend([0; 156]);
    record.extend(block);
    record.extend(idxt);
    if record.len() > 0x10000 {
        return None;
    }
    // The header record with TAGX, the last entry's ident and the count.
    let mut header = b"INDX".to_vec();
    header.extend(be32(192));
    header.extend([0; 8]);
    header.extend(be32(2)); // index type
    header.extend(be32(0)); // the IDXT offset, below
    header.extend(be32(1)); // index records
    header.extend(be32(65001));
    header.extend([0xFF; 4]);
    header.extend(be32(entries.len()));
    header.extend([0; 12]); // ORDT, LIGT
    header.extend(be32(1)); // CNCX records
    header.extend([0; 124]);
    header.extend(be32(192)); // TAGX offset
    header.extend([0; 8]);
    header.extend_from_slice(b"TAGX");
    header.extend(be32(32));
    header.extend(be32(1)); // control bytes
    header.extend_from_slice(&[1, 1, 1, 0, 2, 1, 2, 0, 3, 1, 4, 0, 4, 1, 8, 0, 0, 0, 0, 1]);
    header.extend(ident(entries.len() - 1));
    header.extend_from_slice(&(entries.len() as u16).to_be_bytes());
    let mut header = align(header);
    let idxt_at = header.len();
    header.extend_from_slice(b"IDXT");
    header.extend_from_slice(&((192 + 32) as u16).to_be_bytes());
    header.push(0);
    header[20..24].copy_from_slice(&be32(idxt_at));
    Some(vec![align(header), record, align(cncx)])
}

/// The MOBI locale of a language tag (Calibre's `iana2mobi`, the common ones).
fn locale(language: Option<&str>) -> u32 {
    let primary = language.unwrap_or("en").split(['-', '_']).next().unwrap_or_default().to_lowercase();
    match primary.as_str() {
        "ru" => 0x19,
        "uk" => 0x22,
        "be" => 0x23,
        "de" => 0x07,
        "fr" => 0x0c,
        "es" => 0x0a,
        "it" => 0x10,
        "pl" => 0x15,
        "pt" => 0x16,
        "ja" => 0x11,
        "zh" => 0x04,
        _ => 0x09,
    }
}

fn exth(records: &[(u32, Vec<u8>)]) -> Vec<u8> {
    let body: usize = records.iter().map(|(_, d)| 8 + d.len()).sum();
    let mut out = Vec::with_capacity(12 + body + 4);
    out.extend_from_slice(b"EXTH");
    out.extend_from_slice(&((12 + body) as u32).to_be_bytes());
    out.extend_from_slice(&(records.len() as u32).to_be_bytes());
    for (kind, data) in records {
        out.extend_from_slice(&kind.to_be_bytes());
        out.extend_from_slice(&((8 + data.len()) as u32).to_be_bytes());
        out.extend_from_slice(data);
    }
    while out.len() % 4 != 0 {
        out.push(0);
    }
    out
}

const FLIS: &[u8] = b"FLIS\0\0\0\x08\0\x41\0\0\0\0\0\0\xff\xff\xff\xff\0\x01\0\x03\0\0\0\x03\0\0\0\x01\xff\xff\xff\xff";

fn fcis(text_length: usize) -> Vec<u8> {
    let mut out = b"FCIS\0\0\0\x14\0\0\0\x10\0\0\0\x01\0\0\0\0".to_vec();
    out.extend_from_slice(&(text_length as u32).to_be_bytes());
    out.extend_from_slice(b"\0\0\0\0\0\0\0\x20\0\0\0\x08\0\x01\0\x01\0\0\0\0");
    out
}

/// `book` as a MOBI 6 file, with the reader's `edits` to title, authors and genres.
pub fn mobi(book: &Book, edits: &Edits) -> Vec<u8> {
    let title = edits.title.clone().unwrap_or_else(|| book.title.clone());
    let authors: Vec<String> = edits.authors.as_ref().map_or_else(|| book.authors.clone(), |a| a.split(", ").map(str::to_owned).collect());
    let genres = edits.genres.clone().unwrap_or_else(|| book.genres.clone());
    let markup = markup(book, &title, &authors.join(", "));
    let text = markup.text.as_bytes();

    let entries = ncx_entries(&markup.toc, markup.body_end);
    let index = ncx_records(&entries);
    let mut records: Vec<Vec<u8>> = vec![Vec::new()]; // record 0 comes last
    let texts = text_records(text, index.as_ref().map(|_| entries.as_slice()));
    let text_count = texts.len();
    let size: usize = texts.iter().map(Vec::len).sum();
    records.extend(texts);
    if !size.is_multiple_of(4) {
        records.push(vec![0; 4 - size % 4]);
    }
    let first_non_text = records.len();
    let ncx_at = index.as_ref().map(|_| records.len());
    records.extend(index.into_iter().flatten());
    let first_image = records.len();
    for id in &markup.images {
        records.push(book.images[id].bytes.clone());
    }
    let last_content = records.len() - 1;
    let flis = records.len();
    records.push(FLIS.to_vec());
    let fcis_at = records.len();
    records.push(fcis(text.len()));
    records.push(b"\xe9\x8e\r\n".to_vec());

    // EXTH
    let number = |n: u32| n.to_be_bytes().to_vec();
    let mut ex: Vec<(u32, Vec<u8>)> = authors.iter().filter(|a| !a.is_empty()).map(|a| (100, a.as_bytes().to_vec())).collect();
    if let Some(annotation) = book.annotation.as_deref().filter(|a| !a.is_empty()) {
        ex.push((103, annotation.as_bytes().to_vec()));
    }
    ex.extend(genres.iter().map(|g| (105, crate::i18n::genre_name(g).as_bytes().to_vec())));
    if let Some(year) = book.year {
        ex.push((106, year.to_string().into_bytes()));
    }
    ex.push((108, b"Forzac".to_vec()));
    ex.push((113, book.id.as_bytes().to_vec()));
    ex.push((116, number(markup.start as u32)));
    if markup.cover {
        ex.extend([(201, number(0)), (202, number(0)), (203, number(0))]);
    }
    ex.extend([(204, number(201)), (205, number(1)), (206, number(2)), (207, number(33307))]);
    ex.push((501, b"EBOK".to_vec()));
    ex.push((503, title.as_bytes().to_vec()));
    if let Some(language) = book.language.as_deref() {
        ex.push((524, language.as_bytes().to_vec()));
    }
    let exth = exth(&ex);

    // Record 0: the PalmDOC header, the MOBI header (Calibre's `writer2`, 232 bytes), EXTH, the full title.
    let mut r0 = Vec::with_capacity(16 + 232 + exth.len() + title.len() + 8192);
    let be32 = |v: &mut Vec<u8>, n: u32| v.extend_from_slice(&n.to_be_bytes());
    r0.extend_from_slice(&1u16.to_be_bytes()); // no compression
    r0.extend_from_slice(&[0, 0]);
    be32(&mut r0, text.len() as u32);
    r0.extend_from_slice(&(text_count as u16).to_be_bytes());
    r0.extend_from_slice(&(RECORD_SIZE as u16).to_be_bytes());
    r0.extend_from_slice(&[0; 4]);
    r0.extend_from_slice(b"MOBI");
    for n in [0xE8, 2, 65001, uid(&book.id), 6] {
        be32(&mut r0, n);
    }
    r0.extend_from_slice(&[0xFF; 8]);
    be32(&mut r0, u32::MAX); // secondary index
    r0.extend_from_slice(&[0xFF; 28]);
    be32(&mut r0, first_non_text as u32);
    be32(&mut r0, (16 + 0xE8 + exth.len()) as u32); // the full title's offset
    be32(&mut r0, title.len() as u32);
    be32(&mut r0, locale(book.language.as_deref()));
    r0.extend_from_slice(&[0; 8]);
    be32(&mut r0, 6); // min version
    be32(&mut r0, if markup.images.is_empty() { flis } else { first_image } as u32);
    r0.extend_from_slice(&[0; 16]); // huffman
    be32(&mut r0, 0b101_0000); // has EXTH
    r0.extend_from_slice(&[0; 32]);
    be32(&mut r0, u32::MAX);
    for n in [u32::MAX, 0, 0, 0] {
        be32(&mut r0, n); // DRM
    }
    r0.extend_from_slice(&[0; 8]);
    r0.extend_from_slice(&1u16.to_be_bytes()); // first content record
    r0.extend_from_slice(&(last_content as u16).to_be_bytes());
    be32(&mut r0, 1);
    for n in [fcis_at as u32, 1, flis as u32, 1] {
        be32(&mut r0, n);
    }
    r0.extend_from_slice(&[0; 8]);
    for n in [u32::MAX, 0, u32::MAX, u32::MAX] {
        be32(&mut r0, n);
    }
    be32(&mut r0, if ncx_at.is_some() { 0b11 } else { 0b1 }); // extra data: multibyte overlap, the NCX's TBS
    be32(&mut r0, ncx_at.map_or(u32::MAX, |at| at as u32)); // the NCX index
    r0.extend_from_slice(&exth);
    r0.extend_from_slice(title.as_bytes());
    r0.extend_from_slice(&[0; 8192]);
    while r0.len() % 4 != 0 {
        r0.push(0);
    }
    records[0] = r0;
    palm_db(&title, &records)
}

/// A stable MOBI unique id from the book's id.
fn uid(id: &str) -> u32 {
    u32::from_str_radix(id.get(..8).unwrap_or("0"), 16).unwrap_or(0)
}

/// The PalmDB around `records`: the header, the record list and the records.
fn palm_db(title: &str, records: &[Vec<u8>]) -> Vec<u8> {
    let mut name: Vec<u8> = title.chars().map(|c| if c.is_ascii_alphanumeric() || c == ' ' { c as u8 } else { b'_' }).take(31).collect();
    name.resize(32, 0);
    let now = std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).map_or(0, |d| d.as_secs() as u32);
    let n = records.len();
    let mut out = Vec::new();
    out.extend_from_slice(&name);
    out.extend_from_slice(&[0; 4]); // attributes, version
    out.extend_from_slice(&now.to_be_bytes());
    out.extend_from_slice(&now.to_be_bytes());
    out.extend_from_slice(&[0; 16]); // backup date, modification number, app info, sort info
    out.extend_from_slice(b"BOOKMOBI");
    out.extend_from_slice(&((2 * n - 1) as u32).to_be_bytes());
    out.extend_from_slice(&[0; 4]);
    out.extend_from_slice(&(n as u16).to_be_bytes());
    let mut offset = 78 + 8 * n + 2;
    for (i, r) in records.iter().enumerate() {
        out.extend_from_slice(&(offset as u32).to_be_bytes());
        out.extend_from_slice(&((2 * i) as u32).to_be_bytes()); // attributes 0, unique id 2i
        offset += r.len();
    }
    out.extend_from_slice(&[0, 0]);
    for r in records {
        out.extend_from_slice(r);
    }
    out
}

// endregion

// region the Kindle

/// The roots where disks are mounted.
fn mount_roots() -> Vec<PathBuf> {
    if cfg!(target_os = "macos") {
        return vec![PathBuf::from("/Volumes")];
    }
    let user = std::env::var("USER").unwrap_or_default();
    vec![PathBuf::from(format!("/run/media/{user}")), PathBuf::from(format!("/media/{user}")), PathBuf::from("/media")]
}

/// The first Kindle connected as a disk under `roots`: a folder with `documents` and `system` folders.
pub fn find_in(roots: &[PathBuf]) -> Option<PathBuf> {
    let mut found: Vec<PathBuf> = roots
        .iter()
        .flat_map(|root| std::fs::read_dir(root).into_iter().flatten().flatten().map(|e| e.path()))
        .filter(|p| p.join("documents").is_dir() && p.join("system").is_dir())
        .collect();
    found.sort();
    found.into_iter().next()
}

/// The connected Kindle, if any.
pub fn find() -> Option<PathBuf> {
    find_in(&mount_roots())
}

/// Converts the book at `path` (with `edits`) and writes it into `kindle`'s `documents` as `Author — Title.mobi`,
/// replacing an earlier copy of that name; a temporary file is renamed into place. Returns the written file.
pub fn send(path: &Path, edits: &Edits, kindle: &Path) -> Result<PathBuf, crate::Error> {
    let book = crate::open(path)?;
    let title = edits.title.clone().unwrap_or_else(|| book.title.clone());
    let authors: Vec<String> = edits.authors.as_ref().map_or_else(|| book.authors.clone(), |a| a.split(", ").map(str::to_owned).collect());
    let bytes = mobi(&book, edits);
    let documents = kindle.join("documents");
    let name = crate::library::file_name(&title, &authors, "book.mobi", ".mobi", |_| false);
    let (part, target) = (documents.join(format!(".{name}.part")), documents.join(&name));
    let written = std::fs::write(&part, &bytes).and_then(|_| std::fs::rename(&part, &target));
    if written.is_err() {
        let _ = std::fs::remove_file(&part);
        return Err(crate::Error::Unavailable);
    }
    // The Kindle shows a sideloaded book's cover from its thumbnail, named by the ASIN (EXTH 113, the book's id), as
    // Calibre's Kindle driver writes it; a JPEG cover only.
    let cover = book.cover.as_ref().and_then(|c| book.images.get(c)).filter(|i| i.content_type == "image/jpeg");
    if let Some(cover) = cover {
        let thumbnails = kindle.join("system/thumbnails");
        let _ = std::fs::create_dir_all(&thumbnails);
        let _ = std::fs::write(thumbnails.join(format!("thumbnail_{}_EBOK_portrait.jpg", book.id)), &cover.bytes);
    }
    Ok(target)
}

/// "Send to Kindle": the book at `path` (titled `title` in the library, with `edits`) onto the connected Kindle; the
/// message for the reader.
pub fn send_to_kindle(path: &Path, edits: &Edits, title: &str) -> String {
    use crate::i18n::{fill, tr};
    let Some(kindle) = find() else { return tr("Connect your Kindle with a cable and try again.").to_owned() };
    match send(path, edits, &kindle) {
        Ok(_) => fill(tr("“{title}” is on your Kindle. Eject it before reading."), &[("title", title)]),
        Err(_) => tr("Couldn't write to the Kindle.").to_owned(),
    }
}

// endregion

#[cfg(test)]
mod tests;
