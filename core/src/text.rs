//! Plain text (`.txt`) and Markdown (`.md`, `.markdown`) books (`txt-parsing`, `markdown-parsing`): whether a file is
//! text, its encoding, Markdown or plain text by the marks in it, and plain text into paragraphs and chapters.
//!
//! A port of the Android app's `:core:format-text` (`TextParser`, `PlainTextReader`); the blocks must come out the
//! same, block for block and character for character, or synced reading places would not line up. The format is
//! decided by the bytes alone, never by the file's name.

use crate::model::{Block, Book, Chapter, Kind, Locator, StyledText, TocEntry};
use crate::{Error, typography};

/// A text file larger than this is not read (`Damaged`): the text is held whole, and a novel in TXT is 1–3 MB.
const MAX_BYTES: usize = 16 * 1024 * 1024;

/// Whether the start of a file looks like text: not a PDF or an HTML page, no zero byte in the first 4096 bytes (but
/// in UTF-16 with its byte order mark). Zip archives and FB2 are told apart before.
pub(crate) fn is_text(bytes: &[u8]) -> bool {
    if bytes.is_empty() || bytes.starts_with(b"PK") {
        return false;
    }
    if bytes.starts_with(&[0xFE, 0xFF]) || bytes.starts_with(&[0xFF, 0xFE]) {
        return true;
    }
    let n = bytes.len().min(4096);
    if bytes[..n].contains(&0) {
        return false;
    }
    let start: String = bytes[..n.min(64)].iter().map(|&b| b as char).collect();
    let start = start.strip_prefix("\u{ef}\u{bb}\u{bf}").unwrap_or(&start).trim_start().to_lowercase();
    !start.starts_with("%pdf") && !start.starts_with("<!doctype html") && !start.starts_with("<html")
}

/// Reads a text file (see [`is_text`]) as Markdown when it has Markdown's marks ([`crate::markdown::looks_like`]),
/// else as plain text.
pub(crate) fn parse(bytes: &[u8]) -> Result<Book, Error> {
    if bytes.len() > MAX_BYTES {
        return Err(Error::Damaged);
    }
    let text = decode(bytes).replace("\r\n", "\n").replace('\r', "\n");
    if text.trim().is_empty() {
        return Err(Error::NoText);
    }
    let book = if crate::markdown::looks_like(&text) { crate::markdown::read(&text) } else { read_plain(&text) };
    if book.chapters.iter().all(|c| c.blocks.is_empty()) {
        return Err(Error::NoText);
    }
    Ok(book)
}

/// The file's extension by its content, for a copy whose name gives none: `.md` or `.txt`.
pub(crate) fn extension(bytes: &[u8]) -> &'static str {
    let text = decode(&bytes[..bytes.len().min(MAX_BYTES)]).replace("\r\n", "\n").replace('\r', "\n");
    if crate::markdown::looks_like(&text) { ".md" } else { ".txt" }
}

/// The text: by the byte order mark (UTF-8, UTF-16), else UTF-8 when it is valid UTF-8 (strictly: no overlong forms,
/// no surrogates), else windows-1251 (old Russian TXT; KOI8-R is not guessed).
pub(crate) fn decode(bytes: &[u8]) -> String {
    if let Some((encoding, bom)) = encoding_rs::Encoding::for_bom(bytes) {
        return encoding.decode_without_bom_handling(&bytes[bom..]).0.into_owned();
    }
    match std::str::from_utf8(bytes) {
        Ok(text) => text.to_owned(),
        Err(_) => encoding_rs::WINDOWS_1251.decode_without_bom_handling(bytes).0.into_owned(),
    }
}

/// Java's `\s`: the ASCII whitespace of the Android app's regular expressions.
pub(crate) fn is_space(c: char) -> bool {
    matches!(c, ' ' | '\t' | '\n' | '\u{b}' | '\u{c}' | '\r')
}

/// Length in UTF-16 units, as the Android app counts.
pub(crate) fn utf16_len(s: &str) -> usize {
    s.chars().map(char::len_utf16).sum()
}

/// `text` with its typography normalized, no styles.
pub(crate) fn normalized(text: &str) -> StyledText {
    let mut chars: Vec<char> = text.chars().collect();
    typography::normalize(&mut chars);
    StyledText { text: chars.into_iter().collect(), styles: Vec::new() }
}

// region Plain text

/// How plain text is split into paragraphs, chosen over the whole file.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Mode {
    /// A paragraph starts at an indented line; lines without an indent continue it.
    Indent,
    /// A paragraph runs to a blank line.
    Blank,
    /// Every line is a paragraph.
    Line,
}

/// Plain text into the book: paragraphs by [`mode`], chapters at headings ([`is_heading`]), scene breaks as empty
/// lines. The title is left empty: the library shows the file's name.
fn read_plain(text: &str) -> Book {
    let lines: Vec<String> = text.split('\n').map(|l| l.replace('\t', " ").trim_end().to_owned()).collect();
    let headings: Vec<bool> = (0..lines.len()).map(|i| is_heading(&lines, i)).collect();
    let mode = mode(&lines, &headings);
    let mut book = Book::default();
    let mut title: Option<String> = None;
    let mut blocks: Vec<Block> = Vec::new();
    let mut paragraph = String::new();
    let flush = |paragraph: &mut String, blocks: &mut Vec<Block>| {
        if !paragraph.is_empty() {
            blocks.push(Block::Paragraph { text: normalized(paragraph), kind: Kind::Body });
            paragraph.clear();
        }
    };
    for (i, line) in lines.iter().enumerate() {
        let trimmed = line.trim();
        if headings[i] {
            flush(&mut paragraph, &mut blocks);
            if !blocks.is_empty() {
                book.chapters.push(Chapter { title: title.clone(), blocks: std::mem::take(&mut blocks) });
            }
            let heading = normalized(trimmed);
            let target = Locator { chapter: book.chapters.len(), ..Locator::default() };
            book.toc.push(TocEntry { title: heading.text.clone(), level: 1, target });
            title = Some(heading.text.clone());
            blocks.push(Block::Heading { text: heading, level: 1 });
        } else if trimmed.is_empty() {
            flush(&mut paragraph, &mut blocks);
        } else if is_scene_break(trimmed) {
            flush(&mut paragraph, &mut blocks);
            blocks.push(Block::EmptyLine);
        } else {
            let starts = match mode {
                Mode::Indent => line.starts_with(' '),
                Mode::Blank => false,
                Mode::Line => true,
            };
            if starts {
                flush(&mut paragraph, &mut blocks);
            }
            append(&mut paragraph, trimmed);
        }
    }
    flush(&mut paragraph, &mut blocks);
    if !blocks.is_empty() {
        book.chapters.push(Chapter { title, blocks });
    }
    book
}

/// A continuation line joins with a space, its runs of spaces as one; a word broken with `-` at the end of a line
/// joins the next line without the hyphen when that goes on in lower case.
fn append(paragraph: &mut String, line: &str) {
    let mut text = String::with_capacity(line.len());
    for c in line.chars() {
        if c != ' ' || !text.ends_with(' ') {
            text.push(c);
        }
    }
    if paragraph.is_empty() {
        paragraph.push_str(&text);
        return;
    }
    let mut end = paragraph.chars().rev();
    let hyphenated = end.next() == Some('-') && end.next().is_some_and(char::is_alphabetic);
    if hyphenated && text.starts_with(char::is_lowercase) {
        paragraph.pop();
    } else {
        paragraph.push(' ');
    }
    paragraph.push_str(&text);
}

/// Only `*`, `-`, `_`, `~` and spaces, three of the marks or more.
fn is_scene_break(trimmed: &str) -> bool {
    trimmed.starts_with(['*', '-', '_', '~'])
        && trimmed.chars().all(|c| matches!(c, '*' | '-' | '_' | '~') || is_space(c))
        && trimmed.chars().filter(|c| !is_space(*c)).count() >= 3
}

/// Words that start a chapter's heading.
const CHAPTER_WORDS: [&str; 8] = ["глава", "часть", "пролог", "эпилог", "chapter", "part", "prologue", "epilogue"];

/// A chapter's heading: after the start or a blank line, at most 80 characters, and either "Глава", "Часть",
/// "Пролог", "Эпилог", "Chapter", "Part", "Prologue", "Epilogue" (any case) alone or followed by a space, a dot, a colon,
/// a comma or a dash and anything, or a number alone: up to three digits or a Roman number, with an optional dot.
pub(crate) fn is_heading(lines: &[String], i: usize) -> bool {
    let line = lines[i].trim();
    if line.is_empty() || utf16_len(line) > 80 || (i > 0 && !lines[i - 1].trim().is_empty()) {
        return false;
    }
    let lower = line.to_lowercase();
    let word = CHAPTER_WORDS
        .iter()
        .any(|w| lower.strip_prefix(w).is_some_and(|rest| rest.chars().next().is_none_or(|c| is_space(c) || ".:,-–—".contains(c))));
    let bare = lower.strip_suffix('.').unwrap_or(&lower);
    let digits = (1..=3).contains(&bare.len()) && bare.bytes().all(|b| b.is_ascii_digit());
    let roman = (1..=8).contains(&bare.len()) && bare.bytes().all(|b| b"ivxlcdm".contains(&b));
    word || digits || roman
}

/// The paragraph rule over the lines of text (not headings or scene breaks): indents when at least a third of them
/// are indented but not all; blank lines when there are some between them and more than one line per paragraph on
/// average; else a line per paragraph.
fn mode(lines: &[String], headings: &[bool]) -> Mode {
    let text: Vec<usize> =
        (0..lines.len()).filter(|&i| !lines[i].trim().is_empty() && !headings[i] && !is_scene_break(lines[i].trim())).collect();
    if text.is_empty() {
        return Mode::Line;
    }
    let indented = text.iter().filter(|&&i| lines[i].starts_with(' ')).count();
    if indented * 3 >= text.len() && indented < text.len() {
        return Mode::Indent;
    }
    let mut groups = 0;
    let mut previous: Option<usize> = None;
    for &i in &text {
        if previous.is_none_or(|p| i != p + 1) {
            groups += 1;
        }
        previous = Some(i);
    }
    if groups > 1 && text.len() > groups { Mode::Blank } else { Mode::Line }
}

// endregion

#[cfg(test)]
mod tests;
