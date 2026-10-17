//! Highlights (the Android app's quotes): passages kept as Locator ranges, their list entries, and the texts for
//! sharing one and exporting all (Android's `shareQuote` and `QuoteExport`).

use crate::model::{Book, Locator};

/// A highlight as the list shows it: its range, its whole text, the section it starts in and its share of the book.
#[derive(Debug, Clone, PartialEq)]
pub struct Entry {
    pub start: Locator,
    pub end: Locator,
    pub text: String,
    pub section: Option<String>,
    pub position: f32,
}

/// The list entries of `quotes`.
pub fn entries(book: &Book, quotes: &[(Locator, Locator)]) -> Vec<Entry> {
    let contents = book.contents();
    let starts: Vec<Locator> = quotes.iter().map(|q| q.0).collect();
    let positions = crate::state::progress_of(book, &starts);
    quotes
        .iter()
        .zip(positions)
        .map(|(&(start, end), position)| Entry {
            start,
            end,
            text: book.text(start, end, usize::MAX),
            section: contents.iter().rev().find(|e| e.target <= start).map(|e| e.title.clone()).filter(|t| !t.trim().is_empty()),
            position,
        })
        .collect()
}

/// One passage to share: «text», then a blank line and "— Author, «Title»" (the first author), as Android sends it.
pub fn share_text(book: &Book, text: &str) -> String {
    let title = (!book.title.trim().is_empty()).then(|| format!("«{}»", book.title));
    let signature: Vec<String> = book.authors.first().cloned().into_iter().chain(title).collect();
    if signature.is_empty() { format!("«{text}»") } else { format!("«{text}»\n\n— {}", signature.join(", ")) }
}

/// All highlights as one text, readable as plain text and as Markdown: the title and authors joined by " — ", then
/// each highlight with its lines prefixed "> " and its section on a line "— Section", a blank line between them.
pub fn export(book: &Book, quotes: &[Entry]) -> String {
    let header: Vec<String> = [book.title.clone(), book.authors.join(", ")].into_iter().filter(|s| !s.trim().is_empty()).collect();
    let header = header.join(" — ");
    let body = quotes.iter().map(|q| {
        let lines = q.text.lines().map(|l| if l.trim().is_empty() { ">".to_owned() } else { format!("> {l}") });
        let section = q.section.as_ref().filter(|s| !s.trim().is_empty()).map(|s| format!("— {s}"));
        lines.chain(section).collect::<Vec<_>>().join("\n")
    });
    (!header.is_empty()).then_some(header).into_iter().chain(body).collect::<Vec<_>>().join("\n\n")
}

#[cfg(test)]
mod tests;
