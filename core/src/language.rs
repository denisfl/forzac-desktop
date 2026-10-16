//! The language of a book that names none, from its text, as in the Android app (`BookLanguage.kt`): an English
//! book without a language must be hyphenated in English, whatever the interface language.

use crate::model::{Block, Book};

/// Letters looked at, from the start of the book.
const SAMPLE: usize = 2000;

/// `ru` when more than half of the first 2000 letters of paragraphs and headings are Cyrillic, else `en`.
pub(crate) fn guess(book: &Book) -> &'static str {
    let (mut letters, mut cyrillic) = (0usize, 0usize);
    'book: for chapter in &book.chapters {
        for block in &chapter.blocks {
            let (Block::Paragraph { text, .. } | Block::Heading { text, .. }) = block else { continue };
            for ch in text.text.chars().filter(|c| c.is_alphabetic()) {
                letters += 1;
                if ('\u{400}'..='\u{4ff}').contains(&ch) {
                    cyrillic += 1;
                }
                if letters >= SAMPLE {
                    break 'book;
                }
            }
        }
    }
    if letters > 0 && cyrillic * 2 > letters { "ru" } else { "en" }
}

#[cfg(test)]
mod tests {
    use crate::model::{Block, Book, Chapter, Kind, StyledText};

    fn book(text: &str) -> Book {
        let paragraph = Block::Paragraph { text: StyledText { text: text.to_owned(), styles: Vec::new() }, kind: Kind::Body };
        Book { chapters: vec![Chapter { title: None, blocks: vec![paragraph] }], ..Book::default() }
    }

    #[test]
    fn cyrillic_or_not() {
        assert_eq!(super::guess(&book("Мне снился долгий сон про Pushkin")), "ru");
        assert_eq!(super::guess(&book("It was a dark and stormy night; «ночь»")), "en");
        assert_eq!(super::guess(&book("")), "en");
    }

    #[test]
    fn parse_fills_a_missing_language() {
        let fb2 = |lang: &str| {
            format!(
                "<?xml version=\"1.0\" encoding=\"utf-8\"?><FictionBook><description><title-info><book-title>T</book-title>{lang}\
                 </title-info></description><body><section><p>It was the best of times, it was the worst of times.</p></section></body></FictionBook>"
            )
        };
        assert_eq!(crate::parse(fb2("").as_bytes()).unwrap().language.as_deref(), Some("en"));
        assert_eq!(crate::parse(fb2("<lang>de</lang>").as_bytes()).unwrap().language.as_deref(), Some("de"));
    }
}
