//! Markdown (`markdown-parsing`): our own reader of the subset that long answers and notes use — headings, paragraphs,
//! quotes, code, lists, rules, GitHub tables, inline marks and links — into the book model. Not CommonMark: lazy
//! continuations, reference links, footnotes, HTML entities and formulas stay text.
//!
//! A port of the Android app's `MarkdownReader` and `MarkdownInline`, rule for rule (their regular expressions as
//! code): the blocks must come out the same, block for block and character for character, or synced reading places
//! would not line up.

use std::collections::HashMap;
use std::sync::Arc;

use crate::model::{Block, Book, Chapter, Kind, LinkTarget, Locator, Style, StyleRange, StyledText, Table, TableCell, TocEntry};
use crate::text::{is_space, normalized};
use crate::typography;

// region Markdown or not

/// Markdown when one line is a heading (`#`…`######`, a space and text), a code fence (three or more `` ` `` or `~`
/// indented by at most three) or a table's header over its delimiter row, or when at least two different weaker marks
/// are there: a link `[text](address)`, bold `**text**` or `__text__`, three or more list lines (`- `, `* `, `+ `,
/// `1. `), a quote line (`>`).
pub(crate) fn looks_like(text: &str) -> bool {
    let lines: Vec<&str> = text.split('\n').collect();
    let (mut list_lines, mut quote) = (0, false);
    for (i, line) in lines.iter().enumerate() {
        if atx(line).is_some() && !line.trim_start().trim_start_matches('#').trim().is_empty() {
            return true;
        }
        if fence(line).is_some_and(|f| f.indent <= 3) {
            return true;
        }
        if line.contains('|') && lines.get(i + 1).is_some_and(|next| next.contains('|') && is_delimiter_row(next) && next.contains('-')) {
            return true;
        }
        if !is_rule(line) && list(line).is_some_and(|l| !l.content.trim().is_empty()) {
            list_lines += 1;
        }
        if quote_rest(line).is_some_and(|rest| !rest.trim().is_empty()) {
            quote = true;
        }
    }
    let weak = [has_link(text), has_bold(text), list_lines >= 3, quote];
    weak.iter().filter(|w| **w).count() >= 2
}

/// `[text](address)`: text without `]` or a line break, an address without `)` or spaces.
fn has_link(text: &str) -> bool {
    let c: Vec<char> = text.chars().collect();
    (0..c.len()).filter(|&p| c[p] == '[').any(|p| {
        let mut j = p + 1;
        while j < c.len() && c[j] != ']' && c[j] != '\n' {
            j += 1;
        }
        if j == p + 1 || c.get(j) != Some(&']') || c.get(j + 1) != Some(&'(') {
            return false;
        }
        let mut k = j + 2;
        while k < c.len() && c[k] != ')' && !is_space(c[k]) {
            k += 1;
        }
        k > j + 2 && c.get(k) == Some(&')')
    })
}

/// `**text**` or `__text__` within a line, the text starting with no space.
fn has_bold(text: &str) -> bool {
    let c: Vec<char> = text.chars().collect();
    ['*', '_'].into_iter().any(|m| {
        (0..c.len().saturating_sub(1)).filter(|&p| c[p] == m && c[p + 1] == m).any(|p| {
            if c.get(p + 2).is_none_or(|x| is_space(*x)) {
                return false;
            }
            let mut j = p + 3;
            while j < c.len() && c[j] != m && c[j] != '\n' {
                j += 1;
            }
            c.get(j) == Some(&m) && c.get(j + 1) == Some(&m)
        })
    })
}

// endregion

// region Lines

/// A heading line `^ {0,3}(#{1,6})(?:[ \t]+(.*?))?(?:[ \t]+#+)?[ \t]*$`: its level and text.
fn atx(line: &str) -> Option<(usize, &str)> {
    let spaces = line.bytes().take_while(|b| *b == b' ').count();
    if spaces > 3 {
        return None;
    }
    let r = &line[spaces..];
    let hashes = r.bytes().take_while(|b| *b == b'#').count();
    if !(1..=6).contains(&hashes) {
        return None;
    }
    let after = &r[hashes..];
    if after.is_empty() || after.bytes().all(|b| b == b' ' || b == b'\t') {
        return Some((hashes, ""));
    }
    if !after.starts_with([' ', '\t']) {
        return None;
    }
    let body = after.trim_start_matches([' ', '\t']).trim_end_matches([' ', '\t']);
    let open = body.trim_end_matches('#');
    if open.len() < body.len() && open.ends_with([' ', '\t']) {
        return Some((hashes, open.trim_end_matches([' ', '\t'])));
    }
    Some((hashes, body))
}

/// A code fence `^(\s*)(`{3,}|~{3,})(.*)$`; a backtick fence has no backtick after it (that would be code in a line).
struct Fence<'a> {
    indent: usize,
    mark: &'a str,
}

fn fence(line: &str) -> Option<Fence<'_>> {
    let indent = line.chars().take_while(|c| is_space(*c)).count();
    let r = &line[indent..];
    let ch = r.chars().next().filter(|c| *c == '`' || *c == '~')?;
    let run = r.chars().take_while(|c| *c == ch).count();
    if run < 3 || (ch == '`' && r[run..].contains('`')) {
        return None;
    }
    Some(Fence { indent, mark: &r[..run] })
}

/// A rule `^ {0,3}((\*[ \t]*){3,}|(-[ \t]*){3,}|(_[ \t]*){3,})$`.
fn is_rule(line: &str) -> bool {
    let spaces = line.bytes().take_while(|b| *b == b' ').count();
    if spaces > 3 {
        return false;
    }
    let r = &line[spaces..];
    let Some(ch) = r.chars().next().filter(|c| matches!(c, '*' | '-' | '_')) else { return false };
    r.chars().all(|c| c == ch || c == ' ' || c == '\t') && r.chars().filter(|c| *c == ch).count() >= 3
}

/// A setext underline `^ {0,3}c+[ \t]*$`.
fn is_underline(line: &str, ch: char) -> bool {
    let spaces = line.bytes().take_while(|b| *b == b' ').count();
    if spaces > 3 {
        return false;
    }
    let r = line[spaces..].trim_end_matches([' ', '\t']);
    !r.is_empty() && r.chars().all(|c| c == ch)
}

/// A quote line `^ {0,3}> ?`: the rest after it.
fn quote_rest(line: &str) -> Option<&str> {
    let spaces = line.bytes().take_while(|b| *b == b' ').count();
    if spaces > 3 {
        return None;
    }
    let rest = line[spaces..].strip_prefix('>')?;
    Some(rest.strip_prefix(' ').unwrap_or(rest))
}

/// A list line `^(\s*)([-*+]|\d{1,9}[.)])[ \t]+(.*)$`.
struct ListLine<'a> {
    indent: usize,
    marker: &'a str,
    content: &'a str,
}

fn list(line: &str) -> Option<ListLine<'_>> {
    let indent = line.chars().take_while(|c| is_space(*c)).count();
    let r = &line[indent..];
    let len = match r.bytes().next()? {
        b'-' | b'*' | b'+' => 1,
        b'0'..=b'9' => {
            let digits = r.bytes().take_while(u8::is_ascii_digit).count();
            if digits > 9 || !matches!(r.as_bytes().get(digits), Some(b'.' | b')')) {
                return None;
            }
            digits + 1
        }
        _ => return None,
    };
    if !r[len..].starts_with([' ', '\t']) {
        return None;
    }
    Some(ListLine { indent, marker: &r[..len], content: r[len..].trim_start_matches([' ', '\t']) })
}

/// A table's delimiter row `^\s*\|?\s*:?-+:?\s*(\|\s*:?-+:?\s*)*\|?\s*$`.
pub(crate) fn is_delimiter_row(line: &str) -> bool {
    let c: Vec<char> = line.chars().collect();
    let mut i = 0;
    let skip = |i: &mut usize| {
        while *i < c.len() && is_space(c[*i]) {
            *i += 1;
        }
    };
    skip(&mut i);
    if c.get(i) == Some(&'|') {
        i += 1;
    }
    loop {
        skip(&mut i);
        if c.get(i) == Some(&':') {
            i += 1;
        }
        let dashes = c[i.min(c.len())..].iter().take_while(|x| **x == '-').count();
        if dashes == 0 {
            return false;
        }
        i += dashes;
        if c.get(i) == Some(&':') {
            i += 1;
        }
        skip(&mut i);
        match c.get(i) {
            None => return true,
            Some('|') => {
                i += 1;
                skip(&mut i);
                if i == c.len() {
                    return true;
                }
            }
            Some(_) => return false,
        }
    }
}

/// A table row's cells: the outer pipes dropped, split at `|`; `\|` is a pipe in a cell.
fn cells(line: &str) -> Vec<String> {
    let mut s = line.trim();
    s = s.strip_prefix('|').unwrap_or(s);
    if s.ends_with('|') && !s.ends_with("\\|") {
        s = &s[..s.len() - 1];
    }
    let (mut out, mut cell) = (Vec::new(), String::new());
    let mut chars = s.chars().peekable();
    while let Some(c) = chars.next() {
        match c {
            '\\' if chars.peek() == Some(&'|') => {
                cell.push('|');
                chars.next();
            }
            '|' => out.push(std::mem::take(&mut cell).trim().to_owned()),
            c => cell.push(c),
        }
    }
    out.push(cell.trim().to_owned());
    out
}

// endregion

// region Blocks

/// A block before the chapters are made.
enum Md {
    Heading { level: usize, raw: String },
    Para { raw: String, prefix: String, quote: bool, level: usize },
    Code(String),
    Rule,
    Grid { header: Vec<String>, rows: Vec<Vec<String>> },
}

/// Reads a Markdown file's text (decoded, `\n` line ends).
pub(crate) fn read(text: &str) -> Book {
    let source: Vec<String> = text.split('\n').map(|l| l.replace('\t', "    ")).collect();
    let mut meta: HashMap<String, Vec<String>> = HashMap::new();
    let lines = front_matter(&source, &mut meta);
    let mut blocks = Vec::new();
    parse(lines, &mut blocks, false);
    assemble(blocks, &meta)
}

/// A front matter block between `---` lines at the very start (closed within 200 lines, `...` closes too): `key:
/// value` fields and `- value` lists under a key. Without any field it is not front matter. Gives the lines after it.
fn front_matter<'a>(source: &'a [String], meta: &mut HashMap<String, Vec<String>>) -> &'a [String] {
    if source.first().is_none_or(|l| l.trim() != "---") {
        return source;
    }
    let Some(end) = (1..source.len().min(200)).find(|&i| matches!(source[i].trim(), "---" | "...")) else { return source };
    let unquote = |s: &str| -> String {
        let mut s = s.trim();
        for q in ['"', '\''] {
            if s.len() >= 2 && s.starts_with(q) && s.ends_with(q) {
                s = &s[1..s.len() - 1];
            }
        }
        s.trim().to_owned()
    };
    let mut key: Option<String> = None;
    for line in &source[1..end] {
        let item = {
            let r = line.trim_start_matches(is_space);
            r.strip_prefix('-').filter(|rest| rest.starts_with(is_space)).map(|rest| rest.trim_start_matches(is_space))
        };
        let pair = line.split_once(':').filter(|(k, _)| {
            let k = k.trim_end_matches(is_space);
            k.starts_with(|c: char| c.is_ascii_alphabetic() || c == '_')
                && k.chars().all(|c| c.is_ascii_alphanumeric() || c == '_' || c == '-')
        });
        if let (Some(item), Some(key)) = (item, &key) {
            meta.entry(key.clone()).or_default().push(unquote(item));
        } else if let Some((k, v)) = pair {
            let k = k.trim_end_matches(is_space).to_lowercase();
            let value = unquote(v.trim_start_matches(is_space));
            if !value.is_empty() {
                meta.entry(k.clone()).or_default().push(value);
            }
            key = Some(k);
        }
    }
    if meta.is_empty() { source } else { &source[end + 1..] }
}

/// The blocks of `lines`, line by line; a quote's lines recursively, its paragraphs quoted.
fn parse(lines: &[String], out: &mut Vec<Md>, quote: bool) {
    let mut para: Vec<String> = Vec::new();
    let mut prefix = String::new();
    let mut level = 0;
    let mut in_list = false;
    // Indents of the open lists' markers: how many, the nesting.
    let mut indents: Vec<usize> = Vec::new();
    let flush = |para: &mut Vec<String>, prefix: &mut String, level: usize, out: &mut Vec<Md>| {
        if !para.is_empty() {
            out.push(Md::Para { raw: join(para), prefix: std::mem::take(prefix), quote, level });
            para.clear();
        }
        prefix.clear();
    };
    let mut i = 0;
    while i < lines.len() {
        let line = lines[i].as_str();
        let list = list(line);
        if line.trim().is_empty() {
            flush(&mut para, &mut prefix, level, out);
        } else if let Some(f) = fence(line) {
            flush(&mut para, &mut prefix, level, out);
            let mut body: Vec<&str> = Vec::new();
            i += 1;
            while i < lines.len() {
                let l = lines[i].as_str();
                let t = l.trim_start();
                if t.starts_with(f.mark) && t.trim_end().chars().all(|c| f.mark.starts_with(c)) {
                    break;
                }
                let leading = l.chars().take_while(|c| c.is_whitespace()).count();
                let drop = f.indent.min(leading);
                body.push(&l[l.char_indices().nth(drop).map_or(l.len(), |(at, _)| at)..]);
                i += 1;
            }
            out.push(Md::Code(body.join("\n").trim_end_matches('\n').to_owned()));
        } else if !para.is_empty() && prefix.is_empty() && is_underline(line, '=') {
            out.push(Md::Heading { level: 1, raw: para.iter().map(|l| l.trim()).collect::<Vec<_>>().join(" ") });
            para.clear();
        } else if !para.is_empty() && prefix.is_empty() && is_underline(line, '-') {
            out.push(Md::Heading { level: 2, raw: para.iter().map(|l| l.trim()).collect::<Vec<_>>().join(" ") });
            para.clear();
        } else if is_rule(line) {
            flush(&mut para, &mut prefix, level, out);
            out.push(Md::Rule);
        } else if let Some((hashes, text)) = atx(line) {
            flush(&mut para, &mut prefix, level, out);
            if !text.trim().is_empty() {
                out.push(Md::Heading { level: hashes, raw: text.to_owned() });
            }
            in_list = false;
        } else if quote_rest(line).is_some() {
            flush(&mut para, &mut prefix, level, out);
            let mut inner = Vec::new();
            while let Some(rest) = lines.get(i).and_then(|l| quote_rest(l)) {
                inner.push(rest.to_owned());
                i += 1;
            }
            parse(&inner, out, true);
            continue;
        } else if line.contains('|')
            && lines
                .get(i + 1)
                .is_some_and(|next| next.contains('-') && is_delimiter_row(next) && (next.contains('|') || cells(line).len() > 1))
        {
            flush(&mut para, &mut prefix, level, out);
            let header = cells(line);
            let mut rows = Vec::new();
            i += 2;
            while let Some(row) = lines.get(i).filter(|l| !l.trim().is_empty() && l.contains('|')) {
                rows.push(cells(row));
                i += 1;
            }
            out.push(Md::Grid { header, rows });
            continue;
        } else if let Some(item) = list.filter(|l| {
            !l.content.trim().is_empty()
                && (para.is_empty()
                    || !prefix.is_empty()
                    || !l.marker.starts_with(|c: char| c.is_ascii_digit())
                    || l.marker.starts_with('1'))
        }) {
            flush(&mut para, &mut prefix, level, out);
            if !in_list {
                indents.clear();
            }
            while indents.last().is_some_and(|&last| last > item.indent) {
                indents.pop();
            }
            if indents.last().is_none_or(|&last| last < item.indent) {
                indents.push(item.indent);
            }
            level = indents.len() - 1;
            let mut content = item.content;
            prefix = if let Some(rest) = content.strip_prefix("[ ] ") {
                content = rest;
                "☐ ".to_owned()
            } else if content.get(..4).is_some_and(|p| p.eq_ignore_ascii_case("[x] ")) {
                content = &content[4..];
                "☑ ".to_owned()
            } else if item.marker.starts_with(|c: char| c.is_ascii_digit()) {
                format!("{}. ", &item.marker[..item.marker.len() - 1])
            } else {
                "• ".to_owned()
            };
            para.push(content.to_owned());
            in_list = true;
        } else if para.is_empty() && !in_list && line.starts_with("    ") {
            let mut body: Vec<&str> = Vec::new();
            while let Some(l) = lines.get(i).filter(|l| l.starts_with("    ") || l.trim().is_empty()) {
                body.push(l.char_indices().nth(4).map_or("", |(at, _)| &l[at..]));
                i += 1;
            }
            out.push(Md::Code(body.join("\n").trim_end_matches(['\n', ' ']).to_owned()));
            continue;
        } else {
            if para.is_empty() && !line.starts_with(' ') {
                in_list = false;
            }
            para.push(line.to_owned());
        }
        i += 1;
    }
    flush(&mut para, &mut prefix, level, out);
}

/// A paragraph's lines as one: a line end is a space; two spaces or `\` at the end of a line, a line break.
fn join(lines: &[String]) -> String {
    let mut out = String::new();
    for (k, raw) in lines.iter().enumerate() {
        let line = raw.trim_start();
        if k + 1 == lines.len() {
            out.push_str(line.trim_end());
        } else if let Some(line) = line.strip_suffix('\\') {
            out.push_str(line);
            out.push('\n');
        } else if line.ends_with("  ") {
            out.push_str(line.trim_end());
            out.push('\n');
        } else {
            out.push_str(line.trim_end());
            out.push(' ');
        }
    }
    out
}

// endregion

// region Chapters

/// Link keys and what they lead to, gathered while the text is read; anchors are resolved once the headings are known.
#[derive(Default)]
struct Links {
    targets: HashMap<String, LinkTarget>,
    /// Key → the anchor it names (percent-decoded, lower case).
    anchors: HashMap<String, String>,
}

impl Links {
    /// The key for a link to `url`: an outside address (`http`, `https`, `mailto`) or an anchor (`#…`, unless
    /// `anchors` is off: table cells); none for anything else, which stays text.
    fn key(&mut self, url: &str, anchors: bool) -> Option<String> {
        let key = format!("l{}", self.targets.len() + self.anchors.len());
        if let Some(anchor) = url.strip_prefix('#').filter(|a| !a.is_empty()) {
            if !anchors {
                return None;
            }
            self.anchors.insert(key.clone(), percent_decode(anchor).to_lowercase());
            Some(key)
        } else if url.starts_with("http://") || url.starts_with("https://") || url.starts_with("mailto:") {
            self.targets.insert(key.clone(), LinkTarget::External(url.to_owned()));
            Some(key)
        } else {
            None
        }
    }
}

/// The book from the blocks. The title is the front matter's, else the first block when it is the only first-level
/// heading (it is then not in the text). Chapters start at the top level of the headings left; deeper headings stay
/// inside, a level below per level. Text before the first chapter is an untitled chapter.
fn assemble(mut blocks: Vec<Md>, meta: &HashMap<String, Vec<String>>) -> Book {
    let mut links = Links::default();
    let mut title = meta.get("title").and_then(|t| t.first()).cloned();
    let first_levels = blocks.iter().filter(|b| matches!(b, Md::Heading { level: 1, .. })).count();
    if title.is_none()
        && first_levels == 1
        && matches!(blocks.first(), Some(Md::Heading { level: 1, .. }))
        && let Md::Heading { raw, .. } = blocks.remove(0)
    {
        title = Some(inline(&raw, &mut |url| links.key(url, true)).text.trim().to_owned());
    }
    let top = blocks.iter().filter_map(|b| if let Md::Heading { level, .. } = b { Some(*level) } else { None }).min();

    let mut book = Book::default();
    let mut slugs: HashMap<String, Locator> = HashMap::new();
    let mut chapter_title: Option<String> = None;
    let mut current: Vec<Block> = Vec::new();
    for md in blocks {
        match md {
            Md::Heading { level, raw } => {
                let top = top.unwrap_or(level);
                let text = inline(&raw, &mut |url| links.key(url, true));
                if level == top {
                    if !current.is_empty() {
                        book.chapters.push(Chapter { title: chapter_title.clone(), blocks: std::mem::take(&mut current) });
                    }
                    chapter_title = Some(text.text.clone());
                }
                let level = 1 + level - top;
                let at = Locator { chapter: book.chapters.len(), block: current.len(), offset: 0 };
                if !text.text.trim().is_empty() {
                    book.toc.push(TocEntry { title: text.text.clone(), level: level as u32, target: at });
                    let base = slug(&text.text);
                    let name = if slugs.contains_key(&base) {
                        (1..).map(|n| format!("{base}-{n}")).find(|s| !slugs.contains_key(s)).unwrap_or(base)
                    } else {
                        base
                    };
                    slugs.insert(name, at);
                }
                current.push(Block::Heading { text, level: level as u32 });
            }
            Md::Para { raw, prefix, quote, level } => {
                let mut text = inline(&raw, &mut |url| links.key(url, true));
                if text.text.trim().is_empty() {
                    continue;
                }
                if !prefix.is_empty() {
                    for s in &mut text.styles {
                        s.start += prefix.len();
                        s.end += prefix.len();
                    }
                    text.text.insert_str(0, &prefix);
                }
                let kind = if quote {
                    Kind::Cite
                } else if !prefix.is_empty() {
                    Kind::ListItem(level as u32)
                } else {
                    Kind::Body
                };
                current.push(Block::Paragraph { text, kind });
            }
            Md::Code(code) => current.push(Block::Paragraph { text: StyledText { text: code, styles: Vec::new() }, kind: Kind::Code }),
            Md::Rule => current.push(Block::EmptyLine),
            Md::Grid { header, rows } => {
                let width = header.len();
                let mut cells = |row: &[String], header: bool| -> Vec<TableCell> {
                    (0..width)
                        .map(|c| TableCell {
                            text: inline(row.get(c).map_or("", String::as_str), &mut |url| links.key(url, false)),
                            header,
                            col_span: 1,
                        })
                        .collect()
                };
                let mut table = vec![cells(&header, true)];
                table.extend(rows.iter().map(|row| cells(row, false)));
                let table = Arc::new(Table { rows: table });
                for row in 0..table.rows.len() {
                    current.push(Block::TableRow { table: table.clone(), row });
                }
            }
        }
    }
    if !current.is_empty() {
        book.chapters.push(Chapter { title: chapter_title, blocks: current });
    }

    // Anchors to their headings; a link to an anchor nowhere stays text.
    let mut dead = Vec::new();
    for (key, anchor) in links.anchors {
        match slugs.get(&anchor).or_else(|| slugs.get(&slug(&anchor))) {
            Some(&start) => {
                links.targets.insert(key, LinkTarget::Place { start, end: None });
            }
            None => dead.push(key),
        }
    }
    if !dead.is_empty() {
        for block in book.chapters.iter_mut().flat_map(|c| &mut c.blocks) {
            if let Block::Paragraph { text, .. } | Block::Heading { text, .. } = block {
                text.styles.retain(|s| s.style != Style::Link || s.target.as_ref().is_none_or(|k| !dead.contains(k)));
            }
        }
    }
    book.links = links.targets;
    book.title = title.unwrap_or_default();
    book.authors = meta.get("authors").or_else(|| meta.get("author")).cloned().unwrap_or_default();
    book.language = meta.get("lang").or_else(|| meta.get("language")).and_then(|l| l.first()).cloned();
    book
}

/// A heading's anchor as GitHub makes it: lower case, no punctuation, spaces as hyphens.
fn slug(text: &str) -> String {
    normalized(text)
        .text
        .to_lowercase()
        .chars()
        .filter_map(|c| match c {
            ' ' | '\u{a0}' => Some('-'),
            c if c.is_alphanumeric() || c == '-' || c == '_' => Some(c),
            _ => None,
        })
        .collect()
}

fn percent_decode(s: &str) -> String {
    let hex = |b: Option<&u8>| b.and_then(|b| (*b as char).to_digit(16));
    let bytes = s.as_bytes();
    let mut out = Vec::with_capacity(bytes.len());
    let mut i = 0;
    while i < bytes.len() {
        if let (b'%', Some(h), Some(l)) = (bytes[i], hex(bytes.get(i + 1)), hex(bytes.get(i + 2))) {
            out.push((h * 16 + l) as u8);
            i += 3;
        } else {
            out.push(bytes[i]);
            i += 1;
        }
    }
    String::from_utf8_lossy(&out).into_owned()
}

// endregion

// region Inline

/// ASCII punctuation: what a backslash escapes.
const PUNCT: &str = "!\"#$%&'()*+,-./:;<=>?@[\\]^_`{|}~";

fn is_punct(c: char) -> bool {
    PUNCT.contains(c) || (!c.is_alphanumeric() && !c.is_whitespace())
}

/// A piece of a paragraph's text before emphasis is matched.
enum Tok {
    Text {
        s: String,
        code: bool,
    },
    /// A run of `*`, `_` or `~~`: characters left unmatched, whether it may open or close, the pairs (by number) it
    /// opens and closes.
    Delim {
        ch: char,
        n: usize,
        open: bool,
        close: bool,
        starts: Vec<usize>,
        ends: Vec<usize>,
    },
    Mark {
        style: Style,
        open: bool,
        key: Option<String>,
    },
}

/// The text of a paragraph, heading or cell with its marks: italics, bold, strikethrough, code, escapes, links (by the
/// key `key` gives, or text only), images (their caption in italics), HTML tags (dropped; `<br>` a line break).
/// Emphasis is CommonMark's simplified: runs of `*`, `_`, `~~` with the flanking rules and a stack of openers.
/// Typography is normalized outside code.
fn inline(raw: &str, key: &mut dyn FnMut(&str) -> Option<String>) -> StyledText {
    let chars: Vec<char> = raw.chars().collect();
    let mut toks = Vec::new();
    tokenize(&chars, &mut toks, key);
    let styles = match_emphasis(&mut toks);
    emit(toks, &styles)
}

fn run(s: &[char], i: usize, c: char) -> usize {
    s[i..].iter().take_while(|x| **x == c).count()
}

fn tokenize(s: &[char], out: &mut Vec<Tok>, key: &mut dyn FnMut(&str) -> Option<String>) {
    let mut plain = String::new();
    let flush = |plain: &mut String, out: &mut Vec<Tok>| {
        if !plain.is_empty() {
            out.push(Tok::Text { s: std::mem::take(plain), code: false });
        }
    };
    // Backtick runs with no closing run of their length after them: no need to look again.
    let mut unclosed: Vec<usize> = Vec::new();
    let mut i = 0;
    while i < s.len() {
        let c = s[i];
        if c == '\\' && s.get(i + 1).is_some_and(|n| PUNCT.contains(*n)) {
            plain.push(s[i + 1]);
            i += 2;
        } else if c == '`' {
            let r = run(s, i, '`');
            let close = if unclosed.contains(&r) { None } else { closing_backticks(s, i + r, r) };
            flush(&mut plain, out);
            match close {
                None => {
                    unclosed.push(r);
                    out.push(Tok::Text { s: "`".repeat(r), code: false });
                    i += r;
                }
                Some(close) => {
                    let mut code: Vec<char> = s[i + r..close].iter().map(|c| if *c == '\n' { ' ' } else { *c }).collect();
                    if code.len() >= 2 && code[0] == ' ' && code[code.len() - 1] == ' ' && code.iter().any(|c| !c.is_whitespace()) {
                        code = code[1..code.len() - 1].to_vec();
                    }
                    out.push(Tok::Text { s: code.into_iter().collect(), code: true });
                    i = close + r;
                }
            }
        } else if c == '*' || c == '_' || c == '~' {
            let r = run(s, i, c);
            flush(&mut plain, out);
            out.push(delimiter(s, i, r, c));
            i += r;
        } else if c == '!' && s.get(i + 1) == Some(&'[') {
            match link(s, i + 1) {
                None => {
                    plain.push(c);
                    i += 1;
                }
                Some(link) => {
                    flush(&mut plain, out);
                    let alt: String = s[link.text.0..link.text.1].iter().collect();
                    let alt = inline(&alt, &mut |_| None).text.trim().to_owned();
                    if !alt.is_empty() {
                        out.push(Tok::Mark { style: Style::Emphasis, open: true, key: None });
                        out.push(Tok::Text { s: alt, code: false });
                        out.push(Tok::Mark { style: Style::Emphasis, open: false, key: None });
                    }
                    i = link.end;
                }
            }
        } else if c == '[' {
            match link(s, i) {
                None => {
                    plain.push(c);
                    i += 1;
                }
                Some(link) => {
                    flush(&mut plain, out);
                    let k = key(&link.url);
                    if k.is_some() {
                        out.push(Tok::Mark { style: Style::Link, open: true, key: k.clone() });
                    }
                    tokenize(&s[link.text.0..link.text.1], out, key);
                    if k.is_some() {
                        out.push(Tok::Mark { style: Style::Link, open: false, key: k });
                    }
                    i = link.end;
                }
            }
        } else if c == '<' {
            if let Some((len, url)) = autolink(&s[i..]) {
                flush(&mut plain, out);
                add_link(out, &url, key);
                i += len;
            } else if let Some(len) = tag(&s[i..]) {
                if s[i..i + 3.min(len)].iter().collect::<String>().eq_ignore_ascii_case("<br") {
                    plain.push('\n');
                }
                i += len;
            } else {
                plain.push(c);
                i += 1;
            }
        } else if c == 'h' && (i == 0 || !s[i - 1].is_alphanumeric()) && (starts_with(s, i, "https://") || starts_with(s, i, "http://")) {
            let url = bare_url(s, i);
            flush(&mut plain, out);
            i += url.chars().count();
            add_link(out, &url, key);
        } else {
            plain.push(c);
            i += 1;
        }
    }
    flush(&mut plain, out);
}

fn starts_with(s: &[char], i: usize, prefix: &str) -> bool {
    let p: Vec<char> = prefix.chars().collect();
    s.len() >= i + p.len() && s[i..i + p.len()] == p[..]
}

fn add_link(out: &mut Vec<Tok>, url: &str, key: &mut dyn FnMut(&str) -> Option<String>) {
    let k = key(url);
    if k.is_some() {
        out.push(Tok::Mark { style: Style::Link, open: true, key: k.clone() });
    }
    out.push(Tok::Text { s: url.to_owned(), code: false });
    if k.is_some() {
        out.push(Tok::Mark { style: Style::Link, open: false, key: k });
    }
}

fn delimiter(s: &[char], i: usize, r: usize, c: char) -> Tok {
    if c == '~' && r != 2 {
        return Tok::Text { s: "~".repeat(r), code: false };
    }
    let before = if i > 0 { s[i - 1] } else { ' ' };
    let after = s.get(i + r).copied().unwrap_or(' ');
    let left = !after.is_whitespace() && !(is_punct(after) && !(before.is_whitespace() || is_punct(before)));
    let right = !before.is_whitespace() && !(is_punct(before) && !(after.is_whitespace() || is_punct(after)));
    let (open, close) = if c == '_' { (left && (!right || is_punct(before)), right && (!left || is_punct(after))) } else { (left, right) };
    Tok::Delim { ch: c, n: r, open, close, starts: Vec::new(), ends: Vec::new() }
}

/// Where the next run of exactly `r` backticks from `from` starts.
fn closing_backticks(s: &[char], from: usize, r: usize) -> Option<usize> {
    let mut j = from;
    while j < s.len() {
        if s[j] == '`' {
            let n = run(s, j, '`');
            if n == r {
                return Some(j);
            }
            j += n;
        } else {
            j += 1;
        }
    }
    None
}

/// Pairs opening and closing runs, nearest opener first: `~~` with `~~` strike through, two and two `*`/`_` make
/// bold, else one and one italics. Gives each pair's style by its number.
fn match_emphasis(toks: &mut [Tok]) -> Vec<Style> {
    let mut styles = Vec::new();
    let mut stack: Vec<usize> = Vec::new();
    for t in 0..toks.len() {
        let Tok::Delim { ch, close, open, .. } = toks[t] else { continue };
        if close {
            let mut k = stack.len() as isize - 1;
            while k >= 0 {
                let Tok::Delim { n: tn, .. } = toks[t] else { break };
                if tn == 0 {
                    break;
                }
                let o = stack[k as usize];
                let Tok::Delim { ch: och, n: on, .. } = toks[o] else { break };
                if och != ch || on == 0 {
                    k -= 1;
                    continue;
                }
                let used = if ch == '~' || (on >= 2 && tn >= 2) { 2 } else { 1 };
                let id = styles.len();
                styles.push(if ch == '~' {
                    Style::Strikethrough
                } else if used == 2 {
                    Style::Strong
                } else {
                    Style::Emphasis
                });
                if let Tok::Delim { n, starts, .. } = &mut toks[o] {
                    *n -= used;
                    starts.push(id);
                }
                if let Tok::Delim { n, ends, .. } = &mut toks[t] {
                    *n -= used;
                    ends.push(id);
                }
                stack.truncate(k as usize + 1);
                if on == used {
                    stack.remove(k as usize);
                }
                k = stack.len() as isize - 1;
            }
        }
        if let Tok::Delim { n, .. } = toks[t]
            && open
            && n > 0
        {
            stack.push(t);
        }
    }
    styles
}

/// The tokens as text with style ranges, typography normalized outside code (code stands in as `x` meanwhile, so it
/// does not change and the ranges stay where they are).
fn emit(toks: Vec<Tok>, styles: &[Style]) -> StyledText {
    let mut chars: Vec<char> = Vec::new();
    let mut ranges: Vec<(usize, usize, Style, Option<String>)> = Vec::new();
    let mut start_at: HashMap<usize, usize> = HashMap::new();
    let mut marks: Vec<(Style, Option<String>, usize)> = Vec::new();
    for t in toks {
        match t {
            Tok::Text { s, code } => {
                let start = chars.len();
                chars.extend(s.chars());
                if code && chars.len() > start {
                    ranges.push((start, chars.len(), Style::Code, None));
                }
            }
            Tok::Delim { ch, n, starts, ends, .. } => {
                for id in ends {
                    if let Some(&start) = start_at.get(&id)
                        && chars.len() > start
                    {
                        ranges.push((start, chars.len(), styles[id], None));
                    }
                }
                chars.extend(std::iter::repeat_n(ch, n));
                for id in starts {
                    start_at.insert(id, chars.len());
                }
            }
            Tok::Mark { style, open: true, key } => marks.push((style, key, chars.len())),
            Tok::Mark { style, open: false, .. } => {
                if let Some(at) = marks.iter().rposition(|m| m.0 == style) {
                    let (style, key, start) = marks.remove(at);
                    if chars.len() > start {
                        ranges.push((start, chars.len(), style, key));
                    }
                }
            }
        }
    }
    ranges.sort_by_key(|r| r.0);
    let mut masked = chars.clone();
    for r in ranges.iter().filter(|r| r.2 == Style::Code) {
        masked[r.0..r.1].iter_mut().for_each(|c| *c = 'x');
    }
    typography::normalize(&mut masked);
    for r in ranges.iter().filter(|r| r.2 == Style::Code) {
        masked[r.0..r.1].copy_from_slice(&chars[r.0..r.1]);
    }
    let mut byte_at = Vec::with_capacity(masked.len() + 1);
    let mut text = String::with_capacity(masked.len());
    for c in &masked {
        byte_at.push(text.len());
        text.push(*c);
    }
    byte_at.push(text.len());
    let styles = ranges.into_iter().map(|(s, e, style, target)| StyleRange { start: byte_at[s], end: byte_at[e], style, target }).collect();
    StyledText { text, styles }
}

/// `<http://…>`, `<https://…>`, `<mailto:…>`: its length and the address.
fn autolink(s: &[char]) -> Option<(usize, String)> {
    let rest: String = s[1..].iter().take_while(|c| !is_space(**c) && **c != '<' && **c != '>').collect();
    let len = rest.chars().count();
    let scheme = ["https://", "http://", "mailto:"].into_iter().find(|p| rest.starts_with(p))?;
    (len > scheme.len() && s.get(1 + len) == Some(&'>')).then_some((len + 2, rest))
}

/// An HTML comment or tag: `<!--…-->`, `</?name(\s[^<>]*)?/?>`. Its length.
fn tag(s: &[char]) -> Option<usize> {
    if starts_with(s, 0, "<!--") {
        return (4..s.len().saturating_sub(2)).find(|&j| s[j] == '-' && s[j + 1] == '-' && s[j + 2] == '>').map(|j| j + 3);
    }
    let mut i = 1;
    if s.get(i) == Some(&'/') {
        i += 1;
    }
    if !s.get(i)?.is_ascii_alphabetic() {
        return None;
    }
    while s.get(i).is_some_and(|c| c.is_ascii_alphanumeric() || *c == '-') {
        i += 1;
    }
    match *s.get(i)? {
        '>' => Some(i + 1),
        '/' if s.get(i + 1) == Some(&'>') => Some(i + 2),
        c if is_space(c) => {
            let end = i + s[i..].iter().position(|c| *c == '<' || *c == '>')?;
            (s[end] == '>').then_some(end + 1)
        }
        _ => None,
    }
}

/// A bare address: up to a space or `<`; dots, commas and the like at its end are not part of it, nor is a `)`
/// without its `(`.
fn bare_url(s: &[char], at: usize) -> String {
    let end = at + s[at..].iter().position(|c| c.is_whitespace() || *c == '<').unwrap_or(s.len() - at);
    let mut url: Vec<char> = s[at..end].to_vec();
    while url.last().is_some_and(|c| ".,;:!?'\"*_~".contains(*c)) {
        url.pop();
    }
    while url.last() == Some(&')') && url.iter().filter(|c| **c == ')').count() > url.iter().filter(|c| **c == '(').count() {
        url.pop();
    }
    url.into_iter().collect()
}

/// `[text](address "title")` at `at` (a `[`): the text's range, the address, the index after it. Brackets in the text
/// and parentheses in the address nest.
struct Link {
    text: (usize, usize),
    url: String,
    end: usize,
}

fn link(s: &[char], at: usize) -> Option<Link> {
    let mut depth = 0;
    let mut j = at;
    while j < s.len() {
        match s[j] {
            '\\' => j += 1,
            '[' => depth += 1,
            ']' => {
                depth -= 1;
                if depth == 0 {
                    break;
                }
            }
            _ => {}
        }
        j += 1;
    }
    if j + 1 >= s.len() || s[j + 1] != '(' {
        return None;
    }
    let mut k = j + 2;
    let mut parens = 1;
    while k < s.len() {
        match s[k] {
            '\\' => k += 1,
            '(' => parens += 1,
            ')' => {
                parens -= 1;
                if parens == 0 {
                    break;
                }
            }
            '\n' => return None,
            _ => {}
        }
        k += 1;
    }
    if k >= s.len() {
        return None;
    }
    let inside: String = s[j + 2..k].iter().collect::<String>().trim().to_owned();
    let url = match inside.strip_prefix('<') {
        Some(rest) => rest.split('>').next().unwrap_or(rest).to_owned(),
        None => inside.split(' ').next().unwrap_or(&inside).to_owned(),
    };
    Some(Link { text: (at + 1, j), url, end: k + 1 })
}

// endregion

#[cfg(test)]
mod tests;
