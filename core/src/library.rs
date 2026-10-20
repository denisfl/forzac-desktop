//! The library's rules, shared by both shells (`library`): the "Continue reading" book, the share read under a cover,
//! search by title, author or series (the Android app's `LibrarySearch.kt`), the views and their groups and moving the
//! selection through them (`library-metadata`), a book's information, generated covers (its `Covers.kt`) and the empty
//! state's sample page (the web reader's). The Books folder is in `folder.rs`.

use crate::i18n::{Language, language, tr};
use crate::model::Book;
use crate::state::Entry;

mod folder;
pub use folder::*;

/// A book counts as finished from this share on: its last page starts just before the end (Android's `FINISHED`).
pub const FINISHED: f32 = 0.999;

/// The "Continue reading" book among the library's books (`(progress, opened)`, last opened first): the first opened
/// and not finished.
pub fn continue_book(books: impl IntoIterator<Item = (f32, bool)>) -> Option<usize> {
    books.into_iter().position(|(p, opened)| opened && p < FINISHED)
}

/// The share read under a cover: "New" for a book never opened, "Finished", else "N%" — at least 1 %, since an
/// opened book has been started.
pub fn share_label(progress: f32, opened: bool) -> String {
    if !opened {
        return tr("New").to_owned();
    }
    if progress >= FINISHED {
        return tr("Finished").to_owned();
    }
    format!("{}%", ((progress * 100.0) as u32).max(1))
}

// region search

const CYRILLIC: [(char, &str); 32] = [
    ('а', "a"),
    ('б', "b"),
    ('в', "v"),
    ('г', "g"),
    ('д', "d"),
    ('е', "e"),
    ('ж', "zh"),
    ('з', "z"),
    ('и', "i"),
    ('й', "y"),
    ('к', "k"),
    ('л', "l"),
    ('м', "m"),
    ('н', "n"),
    ('о', "o"),
    ('п', "p"),
    ('р', "r"),
    ('с', "s"),
    ('т', "t"),
    ('у', "u"),
    ('ф', "f"),
    ('х', "kh"),
    ('ц', "ts"),
    ('ч', "ch"),
    ('ш', "sh"),
    ('щ', "shch"),
    ('ъ', ""),
    ('ы', "y"),
    ('ь', ""),
    ('э', "e"),
    ('ю', "yu"),
    ('я', "ya"),
];

/// Latin spellings of Russian letters reduced to one: Dostoyevsky, Dostoevskiy and Достоевский are all "dostoevski".
const SPELLINGS: [(&str, &str); 22] = [
    ("shch", "s"),
    ("sch", "s"),
    ("sh", "s"),
    ("ch", "c"),
    ("zh", "z"),
    ("kh", "h"),
    ("ts", "c"),
    ("tz", "c"),
    ("ye", "e"),
    ("yo", "o"),
    ("yu", "u"),
    ("ya", "a"),
    ("je", "e"),
    ("jo", "o"),
    ("ju", "u"),
    ("ja", "a"),
    ("iu", "u"),
    ("ia", "a"),
    ("j", "i"),
    ("y", "i"),
    ("w", "v"),
    ("ii", "i"),
];

/// The words of `s`: runs of letters and digits, lower case, "ё" as "е".
fn words(s: &str) -> Vec<String> {
    let folded: String = s.chars().flat_map(char::to_lowercase).map(|c| if c == 'ё' { 'е' } else { c }).collect();
    folded.split(|c: char| !c.is_alphanumeric()).filter(|w| !w.is_empty()).map(str::to_owned).collect()
}

/// `word` in Latin letters with the spellings reduced.
fn skeleton(word: &str) -> String {
    let mut s = String::with_capacity(word.len());
    for c in word.chars() {
        match CYRILLIC.iter().find(|(r, _)| *r == c) {
            Some((_, latin)) => s.push_str(latin),
            None => s.push(c),
        }
    }
    for (from, to) in SPELLINGS {
        if s.contains(from) {
            s = s.replace(from, to);
        }
    }
    s
}

/// A match in transliteration. A word still being typed may stop in the middle of a letter combination (chek —
/// chekhov), so the word without its last letter matches too.
fn translit_match(field: &[String], word: &str) -> bool {
    let sk = skeleton(word);
    let loose = if word.chars().count() >= 4 { skeleton(&word[..word.char_indices().last().map_or(0, |(i, _)| i)]) } else { sk.clone() };
    field.iter().any(|f| f.starts_with(&sk) || f.starts_with(&loose))
}

/// How well a book matches the library's search `query`: `None` no match, lower ranks first. Every word of the query
/// starts a word of the title, the authors or the series, in any order; case, "ё"/"е" and punctuation do not matter;
/// transliteration both ways. The title before the authors before the series, the own alphabet before
/// transliteration. An empty query matches every book.
pub fn rank(query: &str, title: &str, authors: &str, series: &str) -> Option<u32> {
    let fields: Vec<Vec<String>> = [title, authors, series].iter().map(|f| words(f)).collect();
    let skeletons: Vec<Vec<String>> = fields.iter().map(|f| f.iter().map(|w| skeleton(w)).collect()).collect();
    let mut rank = 0;
    for w in words(query) {
        let found = (0..fields.len()).find_map(|i| {
            if fields[i].iter().any(|t| t.starts_with(&w)) {
                Some(2 * i as u32)
            } else {
                translit_match(&skeletons[i], &w).then_some(2 * i as u32 + 1)
            }
        });
        rank += found?;
    }
    Some(rank)
}

// endregion

// region views

/// The setting that remembers the view.
pub const VIEW: &str = "library.view";

/// The library's views, in the tabs' order.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum View {
    #[default]
    Recent,
    Title,
    Author,
    Series,
}

impl View {
    pub const ALL: [View; 4] = [View::Recent, View::Title, View::Author, View::Series];

    /// The tab's name in the interface language.
    pub fn label(self) -> &'static str {
        match self {
            View::Recent => tr("Recent"),
            View::Title => tr("Title"),
            View::Author => tr("Author"),
            View::Series => tr("Series"),
        }
    }

    /// The view remembered as `key` (the setting's value); Recent for anything else.
    pub fn from_key(key: Option<&str>) -> View {
        View::ALL.into_iter().find(|v| Some(v.key()) == key).unwrap_or_default()
    }

    pub fn key(self) -> &'static str {
        ["recent", "title", "author", "series"][self as usize]
    }

    /// The next view (`Tab`), or the previous one (`Shift+Tab`), round.
    pub fn next(self, back: bool) -> View {
        let n = View::ALL.len();
        View::ALL[(self as usize + if back { n - 1 } else { 1 }) % n]
    }
}

/// Books of the library under one heading; `None` for the one group of Recent and Title.
#[derive(Debug, Clone, PartialEq)]
pub struct Group {
    pub heading: Option<String>,
    /// Indices into the books given to [`arrange`].
    pub books: Vec<usize>,
}

/// The key books are compared by: lower case, "ё" as "е", without leading punctuation.
fn sort_key(s: &str) -> String {
    s.trim_start_matches(|c: char| !c.is_alphanumeric())
        .chars()
        .flat_map(char::to_lowercase)
        .map(|c| if c == 'ё' { 'е' } else { c })
        .collect()
}

/// The surname in an author's name: its last word that is not an initial ("Лев Толстой", "Фило В." → Фило), else the
/// name.
fn surname(name: &str) -> &str {
    let initial = |w: &str| w.ends_with('.') || w.chars().filter(|c| c.is_alphabetic()).count() < 2;
    name.split_whitespace().rev().find(|w| !initial(w)).unwrap_or(name)
}

/// A book's title as the library shows it: its own, else its file's name.
fn shown_title(e: &Entry) -> String {
    if e.title.is_empty() { e.path.file_name().unwrap_or_default().to_string_lossy().into_owned() } else { e.title.clone() }
}

/// `books` (last opened first, as `scan` gives them) in `view`: Recent as they are; Title A to Z; Author under each first
/// author, authors by surname ([`surname`]), books by series and number, then title, no author last; Series
/// under each series, books by number (unnumbered last), books outside a series last.
pub fn arrange(books: &[Entry], view: View) -> Vec<Group> {
    let title = |i: usize| sort_key(&shown_title(&books[i]));
    let in_series = |i: usize| books[i].series.as_ref().map(|s| (sort_key(&s.name), s.number.unwrap_or(f32::MAX)));
    let by = |mut idx: Vec<usize>, key: &dyn Fn(usize) -> (String, f32, String)| {
        idx.sort_by(|&a, &b| key(a).partial_cmp(&key(b)).unwrap_or(std::cmp::Ordering::Equal));
        idx
    };
    let all: Vec<usize> = (0..books.len()).collect();
    match view {
        View::Recent => vec![Group { heading: None, books: all }],
        View::Title => vec![Group { heading: None, books: by(all, &|i| (title(i), 0.0, String::new())) }],
        View::Author | View::Series => {
            let head = |i: usize| -> Option<String> {
                match view {
                    View::Author => books[i].authors.split(", ").next().map(str::trim).filter(|a| !a.is_empty()).map(str::to_owned),
                    _ => books[i].series.as_ref().map(|s| s.name.clone()),
                }
            };
            let group_key = |h: &str| match view {
                View::Author => (sort_key(surname(h)), sort_key(h)),
                _ => (sort_key(h), String::new()),
            };
            let mut heads: Vec<String> = all.iter().filter_map(|&i| head(i)).collect();
            heads.sort_by_key(|h| group_key(h));
            heads.dedup();
            let order = |i: usize| match (view, in_series(i)) {
                (View::Author, Some((s, n))) => (s, n, title(i)),
                (View::Author, None) => ("\u{10FFFF}".to_owned(), 0.0, title(i)),
                (_, s) => (String::new(), s.map_or(f32::MAX, |s| s.1), title(i)),
            };
            let mut groups: Vec<Group> = heads
                .into_iter()
                .map(|h| {
                    let members = all.iter().copied().filter(|&i| head(i).as_deref() == Some(h.as_str())).collect();
                    Group { heading: Some(h), books: by(members, &order) }
                })
                .collect();
            let rest: Vec<usize> = all.iter().copied().filter(|&i| head(i).is_none()).collect();
            if !rest.is_empty() {
                let heading = if view == View::Author { tr("Unknown author") } else { tr("Not in a series") };
                groups.push(Group { heading: Some(heading.to_owned()), books: by(rest, &order) });
            }
            groups
        }
    }
}

/// The selection moved by `dx` books along the reading order or `dy` rows, over groups laid out as `(books, columns)`
/// (the "Continue reading" card is a group of one); `at` and the result count books through all groups. Down from a
/// group's last row goes to the same column of the next group's first row, or its last book when that row is shorter;
/// up the same way back. At the edges the selection stays.
pub fn step(groups: &[(usize, usize)], at: usize, dx: i32, dy: i32) -> usize {
    let total: usize = groups.iter().map(|g| g.0).sum();
    if total == 0 {
        return 0;
    }
    let at = at.min(total - 1);
    if dx != 0 {
        return (at as i64 + dx as i64).clamp(0, total as i64 - 1) as usize;
    }
    let starts: Vec<usize> = groups
        .iter()
        .scan(0, |s, g| {
            let start = *s;
            *s += g.0;
            Some(start)
        })
        .collect();
    let g = (0..groups.len()).rfind(|&g| starts[g] <= at && groups[g].0 > 0).unwrap_or(0);
    let (n, cols) = (groups[g].0, groups[g].1.max(1));
    let (row, col) = ((at - starts[g]) / cols, (at - starts[g]) % cols);
    let last_row = |n: usize, cols: usize| (n - 1) / cols;
    if dy > 0 {
        if row < last_row(n, cols) {
            return starts[g] + ((row + 1) * cols + col).min(n - 1);
        }
        match (g + 1..groups.len()).find(|&h| groups[h].0 > 0) {
            Some(h) => starts[h] + col.min(groups[h].0.min(groups[h].1.max(1)) - 1),
            None => at,
        }
    } else if dy < 0 {
        if row > 0 {
            return starts[g] + (row - 1) * cols + col;
        }
        match (0..g).rfind(|&h| groups[h].0 > 0) {
            Some(h) => {
                let (n, cols) = (groups[h].0, groups[h].1.max(1));
                starts[h] + (last_row(n, cols) * cols + col).min(n - 1)
            }
            None => at,
        }
    } else {
        at
    }
}

// endregion

// region the book's information

/// What "About the book" shows, in the interface language; empty values left out by the shells.
#[derive(Debug, Clone, PartialEq, Default)]
pub struct BookInfo {
    pub title: String,
    pub authors: String,
    /// "Series · 2".
    pub series: String,
    /// Genres, the year and the language on one line: "Fantasy, Humour · 1987 · English".
    pub details: String,
    pub annotation: String,
}

/// The name of the language `tag` (BCP 47, "ru", "en-US") in the interface language, for the common ones.
pub fn language_name(tag: &str) -> Option<&'static str> {
    let primary = tag.split(['-', '_']).next().unwrap_or_default().trim().to_lowercase();
    Some(match primary.as_str() {
        "ru" => tr("Russian"),
        "en" => tr("English"),
        "uk" => tr("Ukrainian"),
        "be" => tr("Belarusian"),
        "de" => tr("German"),
        "fr" => tr("French"),
        "es" => tr("Spanish"),
        "it" => tr("Italian"),
        "pl" => tr("Polish"),
        "pt" => tr("Portuguese"),
        "ja" => tr("Japanese"),
        "zh" => tr("Chinese"),
        _ => return None,
    })
}

pub fn book_info(book: &Book) -> BookInfo {
    let series = book.series.as_ref().map_or(String::new(), |s| match s.number_label() {
        Some(n) => format!("{} · {n}", s.name),
        None => s.name.clone(),
    });
    let genres: Vec<&str> = book.genres.iter().map(|g| crate::i18n::genre_name(g)).collect();
    let details: Vec<String> = [
        (!genres.is_empty()).then(|| genres.join(", ")),
        book.year.map(|y| y.to_string()),
        book.language.as_deref().and_then(language_name).map(str::to_owned),
    ]
    .into_iter()
    .flatten()
    .collect();
    BookInfo {
        title: book.title.clone(),
        authors: book.authors.join(", "),
        series,
        details: details.join(" · "),
        annotation: book.annotation.clone().unwrap_or_default(),
    }
}

// endregion

// region generated covers

/// The cover's paper label, its frame and its ink; the spine of a marbled cover (the Android app's brand colours).
pub const LABEL_PAPER: u32 = 0xF1EADB;
pub const LABEL_FRAME: u32 = 0xC49A52;
pub const LABEL_INK: u32 = 0x231F1C;
pub const LABEL_MUTED: u32 = 0x6E655B;
pub const INDIGO: u32 = 0x2B3A5A;

/// Bookcloth colours: muted but distinct (indigo, brick, teal, plum, mustard, graphite).
const CLOTHS: [u32; 6] = [0x2B3A5A, 0x7A3B2E, 0x3E5F5A, 0x5A4060, 0x8A6A2E, 0x3A3D42];

/// How a book without its own cover looks: cloth or marbled paper, chosen by the book's id, so it looks the same every
/// time.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct CoverStyle {
    /// Marbled paper (`marble(seed, …)`) over `cloth`; else plain cloth.
    pub marble: bool,
    /// The material's colour (indigo under marbled paper, shown until the paper is drawn).
    pub cloth: u32,
    /// The spine, 9 % of the width: the cloth darkened, or indigo.
    pub spine: u32,
    /// The marbled paper's seed (from the title, as on Android).
    pub seed: i32,
    /// The title's size as a share of the base size (11.5 % of the cover's width), by its length.
    pub title_scale: f32,
    /// At most this many lines of title.
    pub title_lines: u32,
}

/// Java's `String.hashCode` (over UTF-16), which the Android app picks covers by.
fn java_hash(s: &str) -> i32 {
    s.encode_utf16().fold(0i32, |h, c| h.wrapping_mul(31).wrapping_add(c as i32))
}

pub fn cover_style(id: &str, title: &str) -> CoverStyle {
    let hash = java_hash(id).wrapping_abs();
    let marble = hash % 5 < 2;
    let cloth = CLOTHS[hash.rem_euclid(CLOTHS.len() as i32) as usize];
    let darken = |c: u32| {
        let ch = |shift: u32| ((((c >> shift) & 0xFF) as f32 * 0.78) as u32) << shift;
        ch(16) | ch(8) | ch(0)
    };
    let length = title.encode_utf16().count();
    let title_scale = match length {
        0..=12 => 1.0,
        13..=22 => 0.86,
        23..=36 => 0.74,
        37..=50 => 0.64,
        _ => 0.56,
    };
    CoverStyle {
        marble,
        cloth: if marble { INDIGO } else { cloth },
        spine: if marble { INDIGO } else { darken(cloth) },
        seed: java_hash(title),
        title_scale,
        title_lines: if length > 50 { 5 } else { 4 },
    }
}

const MARBLE_PALETTES: [[u32; 6]; 3] = [
    [0x1E2940, 0xC49A52, 0xEFE6D2, 0x2B3A5A, 0x7A3B2E, 0x3E5F5A],
    [0x3E5F5A, 0xEFE6D2, 0x1E2940, 0xB08446, 0x5E342C, 0x8BA096],
    [0x7A3B2E, 0xEFE6D2, 0xC49A52, 0x1E2940, 0xA86048, 0x2B3A5A],
];

/// The generator the Android app and its design share, so the patterns match.
struct Mulberry32(i32);

impl Mulberry32 {
    fn next(&mut self) -> f64 {
        self.0 = self.0.wrapping_add(0x6D2B_79F5);
        let a = self.0;
        let ushr = |x: i32, n: u32| ((x as u32) >> n) as i32;
        let mut t = (a ^ ushr(a, 15)).wrapping_mul(1 | a);
        t = t.wrapping_add((t ^ ushr(t, 7)).wrapping_mul(61 | t)) ^ t;
        ((t ^ ushr(t, 14)) as u32) as f64 / 4_294_967_296.0
    }
}

/// Marbled endpaper `width` × `height` pixels as RGBA, opaque: combed wavy bands with a dark edge (the Android app's
/// `Marble.render`; drawn at half the cover's size, the pattern is soft). The same `seed` gives the same paper.
pub fn marble(seed: i32, width: usize, height: usize) -> Vec<u8> {
    use std::f64::consts::PI;
    let mut random = Mulberry32(seed);
    let colors = MARBLE_PALETTES[seed.rem_euclid(MARBLE_PALETTES.len() as i32) as usize];
    let (a1, a2, a3) = (random.next() * 2.0 * PI, random.next() * 2.0 * PI, random.next() * 2.0 * PI);
    let f1 = 4.0 + random.next() * 5.0;
    let f2 = 11.0 + random.next() * 9.0;
    let f3 = 26.0 + random.next() * 20.0;
    let amp = 0.07 + random.next() * 0.07;
    let bands = 6.0 + (random.next() * 6.0).floor();
    let tilt = (random.next() - 0.5) * 0.6;
    let mut pixels = Vec::with_capacity(width * height * 4);
    for y in 0..height {
        let v = y as f64 / height as f64;
        let wave = amp * (v * f1 + a1).sin() + amp * 0.45 * (v * f2 + a2).sin();
        for x in 0..width {
            let u = x as f64 / width as f64;
            let mut t = u + tilt * v + wave + 0.018 * (u * f3 + v * 9.0 + a3).sin();
            t += 0.025 * (v * 70.0 + u * 6.0).sin();
            let s = t * bands;
            let k = s.floor();
            let f = s - k;
            let c = colors[(k as i64).rem_euclid(colors.len() as i64) as usize];
            // The band's dark edge, a "vein".
            let shade = if f < 0.05 {
                0.62 + f * 7.0
            } else if f > 0.93 {
                1.08
            } else {
                1.0
            };
            for shift in [16, 8, 0] {
                pixels.push(((((c >> shift) & 0xFF) as f64 * shade) as u32).min(255) as u8);
            }
            pixels.push(255);
        }
    }
    pixels
}

// endregion

/// The illustrated book page of the empty library (the web reader's `.empty-book`): a first page in the interface
/// language. Content, not a translation: Pushkin in Russian, Austen in English.
#[derive(Debug)]
pub struct SamplePage {
    /// The page's language, for hyphenation.
    pub language: &'static str,
    /// The running head: the book's title.
    pub head: &'static str,
    pub title: &'static str,
    /// Under the chapter title, in italics; may be empty.
    pub subtitle: &'static str,
    pub paragraphs: &'static [&'static str],
    pub folio: &'static str,
}

const SAMPLE_EN: SamplePage = SamplePage {
    language: "en",
    head: "Pride and Prejudice",
    title: "Chapter I",
    subtitle: "",
    paragraphs: &[
        "It is a\u{a0}truth universally acknowledged, that a\u{a0}single man in possession of a\u{a0}good fortune, must be in want of \
         a\u{a0}wife.",
        "However little known the feelings or views of such a\u{a0}man may be on his first entering a\u{a0}neighbourhood, this truth \
         is so well fixed in the minds of the surrounding families, that he is considered as the rightful property of some one or \
         other of their daughters.",
        "“My dear Mr.\u{a0}Bennet,” said his lady to him one day, “have you heard that Netherfield Park is let at last?”",
        "Mr.\u{a0}Bennet replied that he had not.",
    ],
    folio: "1 of 348",
};

const SAMPLE_RU: SamplePage = SamplePage {
    language: "ru",
    head: "Капитанская дочка",
    title: "Глава I",
    subtitle: "Сержант гвардии",
    paragraphs: &[
        "Отец мой Андрей Петрович Гринёв в\u{a0}молодости своей служил при графе Минихе и\u{a0}вышел в\u{a0}отставку \
         премьер-майором в\u{a0}17.. году. С\u{a0}тех пор жил он\u{a0}в\u{a0}своей Симбирской деревне, где и\u{a0}женился \
         на\u{a0}девице Авдотье Васильевне Ю., дочери бедного тамошнего дворянина.",
        "Нас было девять человек детей. Все мои братья и\u{a0}сёстры умерли во\u{a0}младенчестве. Матушка была ещё мною \
         брюхата, как уже я\u{a0}был записан в\u{a0}Семёновский полк сержантом.",
        "Если\u{a0}бы паче всякого чаяния матушка родила дочь, то\u{a0}батюшка объявил\u{a0}бы куда следовало о\u{a0}смерти \
         неявившегося сержанта, и\u{a0}дело тем\u{a0}бы и\u{a0}кончилось. Я\u{a0}считался в\u{a0}отпуску до\u{a0}окончания наук.",
    ],
    folio: "1 из 212",
};

/// The sample page in the interface language.
pub fn sample_page() -> &'static SamplePage {
    match language() {
        Language::Russian => &SAMPLE_RU,
        Language::English => &SAMPLE_EN,
    }
}

/// `text` with soft hyphens (U+00AD) wherever `language`'s patterns allow a break, for text that a shell's own line
/// breaker sets (the sample page); unchanged in a language without patterns.
pub fn soft_hyphens(text: &str, language: &str) -> String {
    let Some(hyphenator) = crate::hyphen::Hyphenator::for_language(Some(language)) else { return text.to_owned() };
    let mut out = String::with_capacity(text.len() * 5 / 4);
    let mut word: Vec<char> = Vec::new();
    let flush = |word: &mut Vec<char>, out: &mut String| {
        let breaks = hyphenator.hyphenate(word);
        for (i, c) in word.iter().enumerate() {
            if breaks.contains(&i) {
                out.push('\u{ad}');
            }
            out.push(*c);
        }
        word.clear();
    };
    for c in text.chars() {
        if c.is_alphabetic() {
            word.push(c);
        } else {
            flush(&mut word, &mut out);
            out.push(c);
        }
    }
    flush(&mut word, &mut out);
    out
}

#[cfg(test)]
mod tests;
