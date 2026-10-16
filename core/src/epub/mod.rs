//! EPUB 2 and 3 (a port of the Android app's `EpubReader`): the package document, spine documents into chapters,
//! contents, notes and links. Fixed layout and MathML are read as ordinary content.
//!
//! Keys in the model: images and documents by their path in the archive, notes and places by `path#fragment`,
//! external links by their address.

mod xhtml;

use std::collections::{HashMap, HashSet};

use xhtml::{Anchor, Css, Doc};

use crate::Error;
use crate::archive::{normalize_path, percent_decode};
use crate::model::{Block, Book, Chapter, Image, LinkTarget, Locator, Note, Style, TocEntry};
use crate::xml::{Ev, Xml, attr, collapse};

pub(crate) const CONTAINER: &str = "META-INF/container.xml";

/// Spine document types read as text; `RASTER` are images placed directly in the spine.
const TEXT_DOCS: &[&str] = &["application/xhtml+xml", "text/html", "application/xml", "image/svg+xml"];
const RASTER: &[&str] = &["image/jpeg", "image/png", "image/gif", "image/webp"];

struct Item {
    id: String,
    path: String,
    media_type: String,
    properties: String,
}

struct Opf {
    title: String,
    authors: Vec<String>,
    language: Option<String>,
    description: Option<String>,
    items: Vec<Item>,
    /// Spine documents and whether they are in the reading flow (`linear` not "no").
    spine: Vec<(String, bool)>,
    ncx: Option<String>,
    cover: Option<String>,
}

/// The part of a document from block `doc_start` on lies in `chapter` from block `offset`.
struct Segment {
    doc_start: usize,
    chapter: usize,
    offset: usize,
}

/// A contents entry before it is placed in the chapters.
struct RawToc {
    title: String,
    level: u32,
    path: String,
    fragment: Option<String>,
}

pub(crate) fn read(archive: &HashMap<String, Vec<u8>>) -> Result<Book, Error> {
    let files: HashMap<&str, &[u8]> = archive.iter().map(|(name, data)| (name.trim_start_matches('/'), data.as_slice())).collect();
    check_drm(&files)?;
    let opf = read_opf(&files)?;
    let css = read_css(&files, &opf);
    let types: HashMap<&str, &str> = opf.items.iter().map(|i| (i.path.as_str(), i.media_type.as_str())).collect();
    let docs: Vec<Doc> = opf
        .spine
        .iter()
        .filter_map(|(path, _)| {
            let bytes = files.get(path.as_str())?;
            let kind = types.get(path.as_str()).copied().unwrap_or_default();
            if RASTER.contains(&kind) {
                return Some(Doc::image(path));
            }
            if !kind.is_empty() && !TEXT_DOCS.contains(&kind) {
                return None;
            }
            Some(xhtml::read(path, bytes, &css))
        })
        .collect();
    if docs.is_empty() {
        return Err(Error::NoText);
    }
    let linear: HashSet<&str> = opf.spine.iter().filter(|(_, l)| *l).map(|(p, _)| p.as_str()).collect();
    let by_path: HashMap<&str, &Doc> = docs.iter().map(|d| (d.path.as_str(), d)).collect();
    let anchor = |path: &str, fragment: &str| by_path.get(path).and_then(|d| d.anchors.get(fragment)).copied();

    let raw_toc = read_nav(&files, &opf).or_else(|| read_ncx(&files, &opf)).unwrap_or_default();
    let (notes, note_ranges) = collect_notes(&docs, &by_path);
    let notes_docs: HashSet<&str> = docs.iter().filter(|d| is_notes_doc(d, &note_ranges)).map(|d| d.path.as_str()).collect();

    // Chapters are spine documents; a document with no heading at its start and no contents entry there continues the
    // previous one (converters cut long chapters into files). Inside a document a chapter starts at a heading of the
    // book's top level that a top-level contents entry points to (publishers put several chapters in one file). Both
    // signs are needed: converters give every subheading the same class, and publishers mix up contents levels.
    let toc_starts: HashSet<&str> = raw_toc
        .iter()
        .filter(|e| e.fragment.as_deref().is_none_or(|f| anchor(&e.path, f).is_some_and(|a| a.start == 0)))
        .map(|e| e.path.as_str())
        .collect();
    let toc_top = raw_toc.iter().map(|e| e.level).min();
    let top_rank = docs
        .iter()
        .filter(|d| linear.contains(d.path.as_str()) && !notes_docs.contains(d.path.as_str()))
        .flat_map(|d| &d.main.blocks)
        .filter_map(heading_level)
        .min();
    let mut chapters: Vec<Vec<Block>> = Vec::new();
    let mut placement: HashMap<&str, Vec<Segment>> = HashMap::new();
    let mut first_doc = true;
    for (index, doc) in docs.iter().enumerate() {
        let path = doc.path.as_str();
        let blocks = &doc.main.blocks;
        if !linear.contains(path) || notes_docs.contains(path) || blocks.is_empty() {
            continue;
        }
        // The cover page is exactly one image; a document of several images (a scroll, a gallery) is part of the book.
        let content: Vec<&Block> = blocks.iter().filter(|b| **b != Block::EmptyLine).collect();
        if first_doc && index < 3 && content.len() == 1 && matches!(content[0], Block::Image { .. }) {
            continue;
        }
        let mut splits: Vec<usize> = raw_toc
            .iter()
            .filter(|e| e.path == path && Some(e.level) == toc_top)
            .filter_map(|e| anchor(path, e.fragment.as_deref()?).filter(|a| !a.hidden).map(|a| a.start))
            .filter(|&at| at > 0 && blocks.get(at).and_then(heading_level) == top_rank && heading_level(&blocks[at - 1]).is_none())
            .collect();
        splits.sort_unstable();
        splits.dedup();
        let starts: Vec<usize> = std::iter::once(0).chain(splits).collect();
        for (i, &from) in starts.iter().enumerate() {
            let to = starts.get(i + 1).copied().unwrap_or(blocks.len());
            if i > 0 || first_doc || heading_level(&blocks[0]).is_some() || toc_starts.contains(path) {
                chapters.push(Vec::new());
            }
            let chapter = chapters.len() - 1;
            let offset = chapters[chapter].len();
            placement.entry(path).or_default().push(Segment { doc_start: from, chapter, offset });
            chapters[chapter].extend(blocks[from..to].iter().map(|b| without_broken_refs(b, &notes)));
        }
        first_doc = false;
    }
    if !chapters.iter().flatten().any(|b| !matches!(b, Block::Image { .. } | Block::EmptyLine)) {
        return Err(Error::NoText);
    }

    // Heading levels from 1 for the book's top level; a chapter's title is its leading headings.
    let shift = chapters.iter().flatten().filter_map(heading_level).min().unwrap_or(1) - 1;
    let chapters: Vec<Chapter> = chapters
        .into_iter()
        .map(|blocks| {
            let blocks: Vec<Block> = blocks
                .into_iter()
                .map(|b| match b {
                    Block::Heading { text, level } => Block::Heading { text, level: (level - shift).clamp(1, 6) },
                    b => b,
                })
                .collect();
            let title = blocks
                .iter()
                .map_while(|b| if let Block::Heading { text, .. } = b { Some(text.text.as_str()) } else { None })
                .collect::<Vec<_>>()
                .join(". ");
            Chapter { title: (!title.trim().is_empty()).then_some(title), blocks }
        })
        .collect();

    // A block of a document → its place in the chapters: through the segments the document became.
    let locate = |path: &str, block: usize| -> Option<Locator> {
        let s = placement.get(path)?.iter().rev().find(|s| s.doc_start <= block)?;
        Some(Locator { chapter: s.chapter, block: s.offset + block - s.doc_start, offset: 0 })
    };
    let toc = raw_toc
        .iter()
        .filter_map(|e| {
            let block = e.fragment.as_deref().and_then(|f| anchor(&e.path, f)).filter(|a| !a.hidden).map_or(0, |a| a.start);
            Some(TocEntry { title: e.title.clone(), level: e.level, target: locate(&e.path, block)? })
        })
        .collect();

    let images = opf
        .items
        .iter()
        .filter(|i| i.media_type.starts_with("image/") && i.media_type != "image/svg+xml")
        .filter_map(|i| {
            let bytes = files.get(i.path.as_str()).filter(|b| !b.is_empty())?;
            Some((i.path.clone(), Image { content_type: i.media_type.clone(), bytes: bytes.to_vec() }))
        })
        .collect();

    let mut book = Book {
        title: opf.title,
        authors: opf.authors,
        language: opf.language,
        annotation: opf.description,
        cover: opf.cover.filter(|c| files.contains_key(c.as_str())),
        chapters,
        notes,
        toc,
        images,
        ..Book::default()
    };
    resolve_links(&mut book, &|path, fragment| {
        let a = if fragment.is_empty() { Anchor { hidden: false, start: 0, end: 0 } } else { anchor(path, fragment)? };
        if a.hidden {
            return None;
        }
        let start = locate(path, a.start)?;
        let end = (a.end > a.start)
            .then(|| locate(path, a.end - 1))
            .flatten()
            .filter(|l| l.chapter == start.chapter)
            .map(|l| Locator { block: l.block + 1, ..l });
        Some(LinkTarget::Place { start, end })
    });
    Ok(book)
}

fn heading_level(block: &Block) -> Option<u32> {
    if let Block::Heading { level, .. } = block { Some(*level) } else { None }
}

// region package

/// Encryption of anything but fonts (those are "obfuscated" legally) is DRM.
fn check_drm(files: &HashMap<&str, &[u8]>) -> Result<(), Error> {
    let Some(xml) = files.get("META-INF/encryption.xml") else { return Ok(()) };
    let xml = String::from_utf8_lossy(xml);
    for rest in xml.split("CipherReference").skip(1) {
        let rest = &rest[..rest.find('>').unwrap_or(rest.len())];
        let Some(at) = rest.find("URI") else { continue };
        let value = rest[at + 3..].trim_start().trim_start_matches('=').trim_start();
        let Some(quote) = value.chars().next().filter(|q| *q == '"' || *q == '\'') else { continue };
        let uri = value[1..].split(quote).next().unwrap_or("").to_lowercase();
        if ![".otf", ".ttf", ".woff", ".woff2"].iter().any(|ext| uri.ends_with(ext)) {
            return Err(Error::Protected);
        }
    }
    Ok(())
}

fn read_opf(files: &HashMap<&str, &[u8]>) -> Result<Opf, Error> {
    let mut opf_path = None;
    walk(files[CONTAINER], |_, name, attrs| {
        if name == "rootfile" && opf_path.is_none() {
            opf_path = attr(attrs, "full-path").map(|p| p.trim_start_matches('/').to_owned());
        }
    });
    let path = opf_path.ok_or(Error::NotABook)?;
    let bytes = files.get(path.as_str()).ok_or(Error::NotABook)?;

    let mut title = String::new();
    let mut creators: Vec<(String, Option<String>)> = Vec::new(); // name, role
    let mut creator_ids: HashMap<String, usize> = HashMap::new();
    let mut roles: HashMap<String, String> = HashMap::new(); // creator id → role from `<meta refines>` (EPUB 3)
    let mut language = None;
    let mut description = None;
    let mut cover_id = None;
    let mut items: Vec<Item> = Vec::new();
    let mut spine = Vec::new();
    let mut ncx_id = None;
    let mut guide_cover = None;
    walk(bytes, |xml, name, attrs| match name {
        "title" if title.is_empty() => title = xml.text().unwrap_or_default(),
        "creator" => {
            if let Some(id) = attr(attrs, "id") {
                creator_ids.insert(id.to_owned(), creators.len());
            }
            let role = attr(attrs, "role").map(str::to_owned);
            creators.push((xml.text().unwrap_or_default(), role));
        }
        "language" if language.is_none() => language = Some(xml.text().unwrap_or_default()).filter(|l| !l.is_empty()),
        "description" if description.is_none() => description = Some(strip_tags(&xml.text().unwrap_or_default())).filter(|d| !d.is_empty()),
        "meta" => {
            if attr(attrs, "name") == Some("cover") {
                cover_id = attr(attrs, "content").map(str::to_owned);
            }
            if let (Some(refines), Some("role")) = (attr(attrs, "refines"), attr(attrs, "property")) {
                roles.insert(refines.trim_start_matches('#').to_owned(), xml.text().unwrap_or_default());
            }
        }
        "item" => {
            if let (Some(id), Some(href)) = (attr(attrs, "id"), attr(attrs, "href")) {
                items.push(Item {
                    id: id.to_owned(),
                    path: resolve_path(&path, href),
                    media_type: attr(attrs, "media-type").unwrap_or_default().to_owned(),
                    properties: attr(attrs, "properties").unwrap_or_default().to_owned(),
                });
            }
        }
        "spine" => ncx_id = attr(attrs, "toc").map(str::to_owned),
        "itemref" => {
            if let Some(item) = items.iter().find(|i| Some(i.id.as_str()) == attr(attrs, "idref")) {
                spine.push((item.path.clone(), attr(attrs, "linear") != Some("no")));
            }
        }
        "reference" if attr(attrs, "type") == Some("cover") => guide_cover = attr(attrs, "href").map(|h| resolve_path(&path, h)),
        _ => {}
    });
    for (id, role) in roles {
        if let Some(&i) = creator_ids.get(&id) {
            creators[i].1 = Some(role);
        }
    }
    let authored: Vec<&(String, Option<String>)> = creators.iter().filter(|(_, r)| r.as_deref().is_none_or(|r| r == "aut")).collect();
    let chosen = if authored.is_empty() { creators.iter().collect() } else { authored };
    let mut authors: Vec<String> = Vec::new();
    for (name, _) in chosen {
        let name = collapse(name);
        if !name.is_empty() && !authors.contains(&name) {
            authors.push(name);
        }
    }

    let images = || items.iter().filter(|i| i.media_type.starts_with("image/"));
    let cover = images()
        .find(|i| i.properties.split_whitespace().any(|p| p == "cover-image"))
        .or_else(|| images().find(|i| Some(&i.id) == cover_id.as_ref()))
        .map(|i| i.path.clone())
        .or_else(|| guide_cover.and_then(|doc| first_image_in(&doc, files.get(doc.as_str())?)))
        .or_else(|| {
            images().find(|i| i.id.to_lowercase().contains("cover") || i.path.to_lowercase().contains("cover")).map(|i| i.path.clone())
        });
    let ncx = items
        .iter()
        .find(|i| Some(&i.id) == ncx_id.as_ref())
        .or_else(|| items.iter().find(|i| i.media_type == "application/x-dtbncx+xml"))
        .map(|i| i.path.clone());
    Ok(Opf { title: collapse(&title), authors, language: language.map(|l| l.trim().to_owned()), description, items, spine, ncx, cover })
}

/// The first image a cover document shows (or the document itself, when the guide points to an image).
fn first_image_in(doc: &str, bytes: &[u8]) -> Option<String> {
    let ext = doc.rsplit('.').next().unwrap_or("").to_lowercase();
    if ["jpg", "jpeg", "png", "gif", "webp"].contains(&ext.as_str()) {
        return Some(doc.to_owned());
    }
    let mut image = None;
    walk(bytes, |_, name, attrs| {
        if image.is_none() && (name == "img" || name == "image") {
            image = attr(attrs, "src").or_else(|| attr(attrs, "href")).map(|s| resolve_path(doc, s));
        }
    });
    image
}

/// Classes that make text italic or bold, from the book's CSS. Only simple selectors: `.c` and `tag.c`.
fn read_css(files: &HashMap<&str, &[u8]>, opf: &Opf) -> Css {
    let mut css = Css::default();
    for item in opf.items.iter().filter(|i| i.media_type == "text/css") {
        let Some(bytes) = files.get(item.path.as_str()) else { continue };
        let mut text = String::from_utf8_lossy(bytes).into_owned();
        while let Some(start) = text.find("/*") {
            let end = text[start..].find("*/").map_or(text.len(), |e| start + e + 2);
            text.replace_range(start..end, "");
        }
        for rule in text.split('}') {
            let Some((selectors, body)) = rule.split_once('{') else { continue };
            let body: String = body.to_lowercase().chars().filter(|c| !c.is_whitespace()).collect();
            let italic = body.contains("font-style:italic") || body.contains("font-style:oblique");
            let bold = ["bold", "bolder", "600", "700", "800", "900"].iter().any(|w| body.contains(&format!("font-weight:{w}")));
            if !italic && !bold {
                continue;
            }
            for class in selectors.split(',').filter_map(|s| simple_class(s.trim())) {
                if italic {
                    css.italic.insert(class.clone());
                }
                if bold {
                    css.bold.insert(class);
                }
            }
        }
    }
    css
}

/// The class of a selector `.c` or `tag.c`, lower case; anything more complex is `None`.
fn simple_class(selector: &str) -> Option<String> {
    let (tag, class) = selector.split_once('.')?;
    let ok_tag = tag.chars().all(|c| c.is_ascii_alphanumeric());
    let ok_class = !class.is_empty() && class.chars().all(|c| c.is_alphanumeric() || c == '_' || c == '-');
    (ok_tag && ok_class).then(|| class.to_lowercase())
}

// endregion

// region contents

/// EPUB 3: `<nav epub:type="toc">` with nested `ol/li/a`.
fn read_nav(files: &HashMap<&str, &[u8]>, opf: &Opf) -> Option<Vec<RawToc>> {
    let nav = opf.items.iter().find(|i| i.properties.split_whitespace().any(|p| p == "nav"))?;
    let mut xml = Xml::new(files.get(nav.path.as_str())?);
    let mut entries = Vec::new();
    let mut stack: Vec<String> = Vec::new();
    let mut toc_depth = None; // depth of the toc `nav`
    let mut ol = 0u32;
    while let Ok(ev) = xml.next() {
        match ev {
            Ev::Eof => break,
            Ev::Start(name, attrs) => {
                if name == "nav" && toc_depth.is_none() && attr(&attrs, "type").is_some_and(|t| t.contains("toc")) {
                    toc_depth = Some(stack.len());
                }
                if toc_depth.is_some() && name == "a" {
                    let href = attr(&attrs, "href").map(str::to_owned);
                    let title = xml.text().unwrap_or_default(); // consumes the element
                    if let Some(href) = href.filter(|_| !title.is_empty()) {
                        entries.push(RawToc {
                            title,
                            level: ol.max(1),
                            path: resolve_path(&nav.path, &href),
                            fragment: fragment_of(&href),
                        });
                    }
                    continue;
                }
                if toc_depth.is_some() && name == "ol" {
                    ol += 1;
                }
                stack.push(name);
            }
            Ev::End => {
                let Some(name) = stack.pop() else { continue };
                if toc_depth.is_some() && name == "ol" {
                    ol = ol.saturating_sub(1);
                }
                if name == "nav" && toc_depth == Some(stack.len()) {
                    break;
                }
            }
            Ev::Text(_) => {}
        }
    }
    (!entries.is_empty()).then_some(entries)
}

/// EPUB 2: the NCX, nested `navPoint`s.
fn read_ncx(files: &HashMap<&str, &[u8]>, opf: &Opf) -> Option<Vec<RawToc>> {
    let path = opf.ncx.as_deref()?;
    let mut xml = Xml::new(files.get(path)?);
    let mut entries = Vec::new();
    let mut depth = 0u32; // nesting of `navPoint`
    let mut stack: Vec<bool> = Vec::new(); // whether each open element is a `navPoint`
    let mut label: Option<String> = None;
    while let Ok(ev) = xml.next() {
        match ev {
            Ev::Eof => break,
            Ev::Start(name, attrs) => match name.as_str() {
                "text" => {
                    let text = xml.text().unwrap_or_default(); // consumes the element
                    label.get_or_insert(text);
                }
                _ => {
                    if name == "navPoint" {
                        depth += 1;
                        label = None;
                    }
                    if name == "content"
                        && let (Some(src), Some(title)) = (attr(&attrs, "src"), label.clone().filter(|l| !l.is_empty()))
                    {
                        entries.push(RawToc { title, level: depth.max(1), path: resolve_path(path, src), fragment: fragment_of(src) });
                    }
                    stack.push(name == "navPoint");
                }
            },
            Ev::End => {
                if stack.pop() == Some(true) {
                    depth -= 1;
                }
            }
            Ev::Text(_) => {}
        }
    }
    (!entries.is_empty()).then_some(entries)
}

// endregion

// region notes and links

type NoteRanges = HashMap<String, Vec<(usize, usize)>>;

/// Note texts by key, and per document the block ranges they take in the main flow. A note's text is the blocks of
/// the element with the target id; for an empty anchor, the blocks up to the next note target or heading. Paragraphs
/// that are only a link ("Back") are dropped; a leading heading ("12") becomes the title.
fn collect_notes(docs: &[Doc], by_path: &HashMap<&str, &Doc>) -> (HashMap<String, Note>, NoteRanges) {
    let targets: HashSet<&String> = docs.iter().flat_map(|d| &d.note_refs).collect();
    let mut by_doc: HashMap<&str, Vec<&str>> = HashMap::new();
    for key in &targets {
        if let Some((path, fragment)) = key.split_once('#') {
            by_doc.entry(path).or_default().push(fragment);
        }
    }
    let mut notes = HashMap::new();
    let mut ranges: NoteRanges = HashMap::new();
    for key in targets {
        let Some((path, fragment)) = key.split_once('#') else { continue };
        let Some(doc) = by_path.get(path) else { continue };
        let Some(anchor) = doc.anchors.get(fragment) else { continue };
        let sink = if anchor.hidden { &doc.hidden } else { &doc.main };
        let len = sink.blocks.len();
        let end = if anchor.end > anchor.start {
            anchor.end
        } else {
            let next_target = by_doc[path]
                .iter()
                .filter_map(|f| doc.anchors.get(*f))
                .filter(|a| a.hidden == anchor.hidden && a.start > anchor.start)
                .map(|a| a.start)
                .min()
                .unwrap_or(len);
            let next_heading = (anchor.start + 1..len).find(|&i| heading_level(&sink.blocks[i]).is_some()).unwrap_or(len);
            next_target.min(next_heading)
        };
        let (start, end) = (anchor.start, end.min(len));
        if start >= end {
            continue;
        }
        let blocks: Vec<&Block> = (start..end).filter(|i| !sink.link_only.contains(i)).map(|i| &sink.blocks[i]).collect();
        let title = blocks
            .iter()
            .map_while(|b| if let Block::Heading { text, .. } = b { Some(text.text.as_str()) } else { None })
            .collect::<Vec<_>>()
            .join(" ");
        let only = blocks.len() == 1;
        let body: Vec<Block> = blocks
            .iter()
            .skip_while(|b| heading_level(b).is_some())
            .filter(|b| !matches!(b, Block::Image { .. }) || only)
            .map(|b| (*b).clone())
            .collect();
        if body.is_empty() {
            continue;
        }
        notes.insert(key.clone(), Note { title: (!title.trim().is_empty()).then_some(title), blocks: body });
        if !anchor.hidden {
            ranges.entry(path.to_owned()).or_default().push((start, end));
        }
    }
    (notes, ranges)
}

/// A notes document: note texts take at least 60% of its content blocks.
fn is_notes_doc(doc: &Doc, ranges: &NoteRanges) -> bool {
    let Some(ranges) = ranges.get(&doc.path) else { return false };
    let blocks = &doc.main.blocks;
    let is_content = |i: usize| !matches!(blocks.get(i), None | Some(Block::Heading { .. } | Block::EmptyLine));
    let content = (0..blocks.len()).filter(|&i| !doc.main.link_only.contains(&i) && is_content(i)).count();
    let covered: HashSet<usize> = ranges.iter().flat_map(|&(a, b)| a..b).collect();
    content > 0 && covered.into_iter().filter(|&i| is_content(i)).count() as f64 >= content as f64 * 0.6
}

/// A note marker whose note was not found becomes plain text.
fn without_broken_refs(block: &Block, notes: &HashMap<String, Note>) -> Block {
    let mut block = block.clone();
    if let Block::Paragraph { text, .. } | Block::Heading { text, .. } = &mut block {
        text.styles.retain(|s| s.style != Style::NoteRef || s.target.as_ref().is_some_and(|t| notes.contains_key(t)));
    }
    block
}

/// Link targets; links whose target is not in the book (or not in the reading flow, and not a note) become plain text.
fn resolve_links(book: &mut Book, place: &dyn Fn(&str, &str) -> Option<LinkTarget>) {
    let mut links = HashMap::new();
    for block in book.chapters.iter().flat_map(|c| &c.blocks) {
        for range in block.text().map(|t| t.styles.as_slice()).unwrap_or_default() {
            let (Style::Link, Some(key)) = (range.style, &range.target) else { continue };
            if links.contains_key(key) {
                continue;
            }
            let target = if is_external(key) {
                Some(LinkTarget::External(key.clone()))
            } else if book.notes.contains_key(key) {
                Some(LinkTarget::Note(key.clone()))
            } else {
                key.split_once('#').and_then(|(path, fragment)| place(path, fragment))
            };
            if let Some(t) = target {
                links.insert(key.clone(), t);
            }
        }
    }
    for block in book.chapters.iter_mut().flat_map(|c| &mut c.blocks) {
        if let Block::Paragraph { text, .. } | Block::Heading { text, .. } = block {
            text.styles.retain(|r| r.style != Style::Link || r.target.as_ref().is_some_and(|k| links.contains_key(k)));
        }
    }
    book.links = links;
}

// endregion

// region paths and XML

/// The archive path for link `href` from document `base`; the fragment and query are dropped; empty means `base`.
fn resolve_path(base: &str, href: &str) -> String {
    let raw = href.split('#').next().unwrap_or("").split('?').next().unwrap_or("");
    if raw.is_empty() {
        return base.to_owned();
    }
    let decoded = percent_decode(raw);
    let joined = match (decoded.strip_prefix('/'), base.rsplit_once('/')) {
        (Some(abs), _) => abs.to_owned(),
        (None, Some((dir, _))) => format!("{dir}/{decoded}"),
        (None, None) => decoded,
    };
    normalize_path(&joined)
}

/// The fragment of a link (`#id`), if any.
fn fragment_of(href: &str) -> Option<String> {
    href.split_once('#').map(|(_, f)| f.to_owned()).filter(|f| !f.is_empty())
}

/// A link outside the book: `http`, `https`, `mailto`.
fn is_external(href: &str) -> bool {
    let lower = href.trim_start().to_ascii_lowercase();
    ["http:", "https:", "mailto:"].iter().any(|s| lower.starts_with(s))
}

/// Calls `f` for every element; `f` may consume the element (read its text) or leave it to the walk.
fn walk(data: &[u8], mut f: impl FnMut(&mut Xml, &str, &[(String, String)])) {
    let mut xml = Xml::new(data);
    while let Ok(ev) = xml.next() {
        match ev {
            Ev::Eof => break,
            Ev::Start(name, attrs) => f(&mut xml, &name, &attrs),
            _ => {}
        }
    }
}

/// Text without markup: `<p>About</p>` → `About`.
fn strip_tags(s: &str) -> String {
    let mut out = String::with_capacity(s.len());
    let mut in_tag = false;
    for c in s.chars() {
        match c {
            '<' => in_tag = true,
            '>' if in_tag => {
                in_tag = false;
                out.push(' ');
            }
            _ if !in_tag => out.push(c),
            _ => {}
        }
    }
    collapse(&out)
}

// endregion

#[cfg(test)]
mod tests;
