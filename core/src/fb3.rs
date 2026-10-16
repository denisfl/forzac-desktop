//! FictionBook 3 (`.fb3`, LitRes): an OPC zip package. `_rels/.rels` leads to the book description and the package
//! properties; the description's relationships lead to the body; the body's relationships to its images. The body is
//! read by the FB2 reader. Part names compare case-insensitively after `%XX` decoding; image ids in the model are part
//! names.

use std::collections::HashMap;

use crate::archive::{normalize_path, percent_decode};
use crate::model::{Book, Image};
use crate::xml::{Ev, Xml, attr, collapse};
use crate::{Error, fb2};

const BOOK: &str = "fictionbook3/relationships/book";

/// A package whose root relationships name an FB3 book.
pub(crate) fn is_fb3(files: &HashMap<String, Vec<u8>>) -> bool {
    files.iter().any(|(name, data)| part_name(name) == "_rels/.rels" && String::from_utf8_lossy(data).to_lowercase().contains(BOOK))
}

pub(crate) fn read(files: &HashMap<String, Vec<u8>>) -> Result<Book, Error> {
    let parts: HashMap<String, &[u8]> = files.iter().map(|(name, data)| (part_name(name), data.as_slice())).collect();
    let root = relations(&parts, "");
    let target = |rels: &[Relation], kind: &str| {
        rels.iter().find(|r| r.kind.ends_with(kind) && parts.contains_key(&r.target)).map(|r| r.target.clone())
    };
    let description = target(&root, BOOK).ok_or(Error::NotABook)?;
    let body = target(&relations(&parts, &description), "/relationships/body").ok_or(Error::NotABook)?;
    let images: HashMap<String, String> = relations(&parts, &body)
        .into_iter()
        .filter(|r| r.kind.ends_with("/relationships/image") && parts.contains_key(&r.target))
        .map(|r| (r.id, r.target))
        .collect();

    let mut book = fb2::read_fb3_body(parts[&body], &images)?;
    let cover = target(&root, "/metadata/thumbnail").filter(|c| is_raster(c));
    for part in images.values().chain(cover.iter()) {
        book.images.insert(part.clone(), Image { content_type: content_type(part).to_owned(), bytes: parts[part].to_vec() });
    }
    book.cover = cover;
    let core = target(&root, "/metadata/core-properties").map(|c| parts[&c]);
    read_meta(&mut book, parts[&description], core);
    Ok(book)
}

struct Relation {
    id: String,
    kind: String,
    target: String,
}

/// Relationships of part `source` (`""` for the package), from `<folder>/_rels/<name>.rels`; targets as part names.
fn relations(parts: &HashMap<String, &[u8]>, source: &str) -> Vec<Relation> {
    let (folder, name) = source.rsplit_once('/').unwrap_or(("", source));
    let rels = if folder.is_empty() { format!("_rels/{name}.rels") } else { format!("{folder}/_rels/{name}.rels") };
    let Some(data) = parts.get(&rels) else { return Vec::new() };
    let mut xml = Xml::new(data);
    let mut out = Vec::new();
    while let Ok(ev) = xml.next() {
        match ev {
            Ev::Eof => break,
            Ev::Start(name, attrs) if name == "Relationship" => {
                let Some(target) = attr(&attrs, "Target") else { continue };
                if attr(&attrs, "TargetMode").is_some_and(|m| m.eq_ignore_ascii_case("External")) {
                    continue;
                }
                let path = if let Some(abs) = target.strip_prefix('/') { abs.to_owned() } else { format!("{folder}/{target}") };
                out.push(Relation {
                    id: attr(&attrs, "Id").unwrap_or_default().to_owned(),
                    kind: attr(&attrs, "Type").unwrap_or_default().to_lowercase(),
                    target: normalize_path(&part_name(&path)),
                });
            }
            _ => {}
        }
    }
    out
}

/// A part name for comparison: `%XX` decoded, lower case, no leading `/`.
fn part_name(name: &str) -> String {
    percent_decode(name).to_lowercase().trim_start_matches('/').to_owned()
}

fn content_type(part: &str) -> &'static str {
    match part.rsplit('.').next().unwrap_or("") {
        "jpg" | "jpeg" => "image/jpeg",
        "png" => "image/png",
        "gif" => "image/gif",
        "svg" => "image/svg+xml",
        "webp" => "image/webp",
        _ => "application/octet-stream",
    }
}

/// The cover is shown only if the shells can decode it: an SVG thumbnail (seen in samples) gives no cover.
fn is_raster(part: &str) -> bool {
    matches!(content_type(part), "image/jpeg" | "image/png" | "image/gif" | "image/webp")
}

/// Title, authors, language and annotation from the description; what it lacks from the package's core properties.
fn read_meta(book: &mut Book, description: &[u8], core: Option<&[u8]>) {
    let mut xml = Xml::new(description);
    let mut depth = 0;
    let _ = (|| -> crate::xml::Res<()> {
        loop {
            match xml.next()? {
                Ev::Eof => return Ok(()),
                Ev::End => depth -= 1,
                Ev::Text(_) => {}
                Ev::Start(name, attrs) => {
                    depth += 1;
                    // Children of the root (depth 2) or of `fb3-relations` (subjects).
                    match name.as_str() {
                        "title" if depth == 2 => {
                            book.title = main_title(&mut xml)?;
                            depth -= 1;
                        }
                        "lang" if depth == 2 => {
                            book.language = Some(xml.text()?).filter(|l| !l.is_empty());
                            depth -= 1;
                        }
                        "annotation" if depth == 2 => {
                            book.annotation = paragraphs(&mut xml)?;
                            depth -= 1;
                        }
                        "subject" if attr(&attrs, "link") == Some("author") => {
                            if let Some(author) = author(&mut xml)? {
                                book.authors.push(author);
                            }
                            depth -= 1;
                        }
                        _ => {}
                    }
                }
            }
        }
    })();
    let Some(core) = core.filter(|_| book.title.trim().is_empty() || book.authors.is_empty()) else { return };
    let mut xml = Xml::new(core);
    while let Ok(ev) = xml.next() {
        match ev {
            Ev::Eof => break,
            Ev::Start(name, _) if name == "title" && book.title.trim().is_empty() => book.title = xml.text().unwrap_or_default(),
            Ev::Start(name, _) if name == "creator" && book.authors.is_empty() => {
                let text = xml.text().unwrap_or_default();
                book.authors = text.split([',', ';']).map(str::trim).filter(|a| !a.is_empty()).map(str::to_owned).collect();
            }
            _ => {}
        }
    }
}

/// `main` of the current element (a `title`); consumes the element.
fn main_title(xml: &mut Xml) -> crate::xml::Res<String> {
    let mut main = String::new();
    children(xml, |xml, name, _| {
        if name == "main" {
            main = xml.text()?;
            Ok(())
        } else {
            xml.skip()
        }
    })?;
    Ok(main)
}

/// An author: `title/main`, else first, middle and last names.
fn author(xml: &mut Xml) -> crate::xml::Res<Option<String>> {
    let mut main = String::new();
    let mut names: HashMap<String, String> = HashMap::new();
    children(xml, |xml, name, _| {
        match name {
            "title" => main = main_title(xml)?,
            "first-name" | "middle-name" | "last-name" => {
                names.insert(name.to_owned(), xml.text()?);
            }
            _ => xml.skip()?,
        }
        Ok(())
    })?;
    if main.trim().is_empty() {
        main = ["first-name", "middle-name", "last-name"]
            .iter()
            .filter_map(|k| names.get(*k))
            .filter(|n| !n.is_empty())
            .cloned()
            .collect::<Vec<_>>()
            .join(" ");
    }
    Ok((!main.trim().is_empty()).then(|| collapse(&main)))
}

fn paragraphs(xml: &mut Xml) -> crate::xml::Res<Option<String>> {
    let mut lines = Vec::new();
    children(xml, |xml, name, _| {
        if name == "p" {
            lines.push(xml.text()?);
            Ok(())
        } else {
            xml.skip()
        }
    })?;
    let text = lines.into_iter().filter(|l| !l.is_empty()).collect::<Vec<_>>().join("\n");
    Ok((!text.is_empty()).then_some(text))
}

/// Calls `f` for every child element; `f` must consume it.
fn children(xml: &mut Xml, mut f: impl FnMut(&mut Xml, &str, &[(String, String)]) -> crate::xml::Res<()>) -> crate::xml::Res<()> {
    loop {
        match xml.next()? {
            Ev::Start(name, attrs) => f(xml, &name, &attrs)?,
            Ev::End | Ev::Eof => return Ok(()),
            Ev::Text(_) => {}
        }
    }
}

#[cfg(test)]
mod tests;
