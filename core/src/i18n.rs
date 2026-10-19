//! The interface in English and Russian (`localization`): one catalogue for both shells, keyed by the English text,
//! in the Android app's words (`values/strings.xml`, `values-ru/strings.xml`). The language is chosen once, at start,
//! from the system's preferred languages.

use std::collections::HashMap;
use std::sync::OnceLock;
use std::sync::atomic::{AtomicU8, Ordering};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Language {
    English = 0,
    Russian = 1,
}

static LANGUAGE: AtomicU8 = AtomicU8::new(Language::English as u8);

impl Language {
    /// The first of `locales` (preferred first, such as `ru_RU.UTF-8`, `ru-RU`, `en`) that Forzac has; English when none.
    pub fn from_locales<'a>(locales: impl IntoIterator<Item = &'a str>) -> Language {
        for locale in locales {
            match locale.split(['_', '-', '.', '@']).next().unwrap_or("").to_lowercase().as_str() {
                "ru" => return Language::Russian,
                "en" => return Language::English,
                _ => {}
            }
        }
        Language::English
    }
}

/// Sets the interface language (at start).
pub fn set_language(language: Language) {
    LANGUAGE.store(language as u8, Ordering::Relaxed);
}

pub fn language() -> Language {
    if LANGUAGE.load(Ordering::Relaxed) == Language::Russian as u8 { Language::Russian } else { Language::English }
}

/// `english` in the interface language.
pub fn tr(english: &'static str) -> &'static str {
    tr_in(language(), english)
}

/// `english` in `language`.
pub fn tr_in(language: Language, english: &'static str) -> &'static str {
    match language {
        Language::English => english,
        Language::Russian => russian(english).unwrap_or(english),
    }
}

/// The Russian of `english`, if the catalogue has it.
pub fn russian(english: &str) -> Option<&'static str> {
    static MAP: OnceLock<HashMap<&'static str, &'static str>> = OnceLock::new();
    MAP.get_or_init(|| RUSSIAN.iter().copied().collect()).get(english).copied()
}

/// A count with its noun: `one` and `other` are the English forms with `{n}` ("{n} match", "{n} matches"); Russian
/// takes one, few or many (the CLDR rule for `ru`) from the plural catalogue, keyed by `other`.
pub fn tr_count(n: usize, one: &str, other: &str) -> String {
    count_in(language(), n, one, other)
}

/// The same in `language`.
pub fn count_in(language: Language, n: usize, one: &str, other: &str) -> String {
    let form = match language {
        Language::English => {
            if n == 1 {
                one
            } else {
                other
            }
        }
        Language::Russian => match PLURALS.iter().find(|p| p.0 == other) {
            Some((_, [one, few, many])) => match (n % 10, n % 100) {
                (1, h) if h != 11 => one,
                (2..=4, h) if !(12..=14).contains(&h) => few,
                _ => many,
            },
            None => other,
        },
    };
    form.replace("{n}", &n.to_string())
}

/// `text` with each `{key}` replaced by its value.
pub fn fill(text: &str, values: &[(&str, &str)]) -> String {
    values.iter().fold(text.to_owned(), |t, (k, v)| t.replace(&format!("{{{k}}}"), v))
}

/// Which shell's help.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Platform {
    Linux,
    Mac,
}

/// What `?` shows (`help`): every key by section, in the interface language; the keys stay as they are.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Help {
    pub title: &'static str,
    pub sections: Vec<Section>,
    pub support: &'static str,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Section {
    pub title: &'static str,
    pub rows: Vec<Row>,
}

/// `keys`: the key caps separated by spaces (a cap has none), empty for the mouse.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Row {
    pub keys: &'static str,
    pub meaning: &'static str,
}

/// The keys and the support line in the interface language.
pub fn help(platform: Platform) -> Help {
    Help {
        title: tr("Keys"),
        sections: HELP
            .iter()
            .map(|(title, rows)| Section {
                title: tr(title),
                rows: rows
                    .iter()
                    .filter_map(|&(linux, mac, meaning)| {
                        let keys = if platform == Platform::Mac { mac } else { linux };
                        // Empty keys leave the row out, except in the mouse's section, which has none.
                        (!keys.is_empty() || *title == MOUSE).then(|| Row { keys, meaning: tr(meaning) })
                    })
                    .collect(),
            })
            .collect(),
        support: tr(SUPPORT),
    }
}

/// The help as text for the C ABI: the title, a section as `# title`, a row as `keys\tmeaning`, the support line
/// last; one per line.
pub fn help_text(platform: Platform) -> String {
    let help = help(platform);
    let mut lines = vec![help.title.to_owned()];
    for section in &help.sections {
        lines.push(format!("# {}", section.title));
        lines.extend(section.rows.iter().map(|r| format!("{}\t{}", r.keys, r.meaning)));
    }
    lines.push(help.support.to_owned());
    lines.join("\n")
}

const MOUSE: &str = "Mouse";
const SUPPORT: &str = "Forzac is free. Support it: forzac.truenrth.app";

/// A help row: Linux keys, macOS keys, meaning; empty keys: not on that platform (but see `MOUSE`).
type HelpRow = (&'static str, &'static str, &'static str);

/// Sections of rows.
const HELP: &[(&str, &[HelpRow])] = &[
    (
        "Reading",
        &[
            ("j l → ↓ Space PgDn", "j l → ↓ Space PgDn", "next page (in Scroll: a line)"),
            ("k h ← ↑ Shift+Space PgUp", "k h ← ↑ Shift+Space PgUp", "previous page (in Scroll: a line)"),
            ("Ctrl+D Ctrl+U", "⌃D ⌃U", "half a screen on or back (Scroll)"),
            ("m", "m", "pages or scroll"),
            ("Enter", "Return", "menu (← → h l through the book)"),
        ],
    ),
    (
        "Moving",
        &[
            ("] [", "] [", "next / previous chapter"),
            ("g G Home End", "g G Home End", "chapter start / end"),
            ("Tab Shift+Tab", "Tab Shift+Tab", "notes, links, pictures and tables on the page"),
            ("Ctrl+O", "⌃O ⌘[", "back after a jump"),
        ],
    ),
    (
        "Book",
        &[
            ("o", "o", "contents (/ filters, ← → tabs)"),
            ("b", "b ⌘D", "bookmark the page, again to remove it"),
            ("B", "B", "bookmarks (d removes)"),
            ("/", "/ ⌘F", "search"),
            ("n N", "n N ⌘G ⇧⌘G", "next / previous match"),
        ],
    ),
    (
        "Selection",
        &[
            ("v", "v", "select; again to anchor"),
            ("h l", "h l", "by words"),
            ("j k", "j k", "by lines"),
            ("o", "o", "the other end"),
            ("Enter", "Return", "highlight"),
            ("c", "c ⌘C", "copy"),
            ("", "s", "share"),
            ("Esc", "Esc", "cancel"),
        ],
    ),
    (
        "View",
        &[
            ("+ − 0", "+ − 0", "text size, default size"),
            ("t T", "t T", "theme"),
            ("s", "s ⌘,", "settings"),
            ("f F11", "f ⌃⌘F", "full screen"),
            ("?", "?", "this help"),
            ("q Ctrl+W", "q ⌘W", "close"),
        ],
    ),
    (
        MOUSE,
        &[
            ("", "", "a click in the left or right third turns the page, in the middle opens the menu"),
            ("", "", "a click on a picture or a table opens it"),
            ("", "", "a drag selects; a double click a word, a triple the paragraph"),
        ],
    ),
    (
        "Picture or table",
        &[("+ − 0", "+ − 0", "zoom, fit"), ("← → ↑ ↓ h j k l Space", "← → ↑ ↓ h j k l Space", "move"), ("Esc q", "Esc q", "close")],
    ),
];

/// The English the core asks for itself: the help's.
#[cfg(test)]
fn asked_by_help() -> Vec<&'static str> {
    let rows = HELP.iter().flat_map(|(title, rows)| std::iter::once(*title).chain(rows.iter().map(|r| r.2)));
    ["Keys", SUPPORT].into_iter().chain(rows).collect()
}

/// English → Russian. The Android app's words where it has the string.
const RUSSIAN: &[(&str, &str)] = &[
    // The help.
    ("Reading", "Чтение"),
    ("next page (in Scroll: a line)", "следующая страница (в ленте — строка)"),
    ("previous page (in Scroll: a line)", "предыдущая страница (в ленте — строка)"),
    ("half a screen on or back (Scroll)", "полэкрана вперёд или назад (лента)"),
    ("pages or scroll", "страницы или лента"),
    ("menu (← → h l through the book)", "меню (← → h l — по книге)"),
    ("Moving", "Переходы"),
    ("next / previous chapter", "следующая / предыдущая глава"),
    ("chapter start / end", "начало / конец главы"),
    ("notes, links, pictures and tables on the page", "примечания, ссылки, картинки и таблицы на странице"),
    ("back after a jump", "вернуться после перехода"),
    ("Book", "Книга"),
    ("contents (/ filters, ← → tabs)", "содержание (/ — фильтр, ← → — вкладки)"),
    ("bookmark the page, again to remove it", "закладка на странице, ещё раз — убрать"),
    ("bookmarks (d removes)", "закладки (d — удалить)"),
    ("search", "поиск"),
    ("next / previous match", "следующее / предыдущее совпадение"),
    ("Selection", "Выделение"),
    ("select; again to anchor", "выделить; ещё раз — закрепить"),
    ("by words", "по словам"),
    ("by lines", "по строкам"),
    ("the other end", "другой конец"),
    ("highlight", "цитата"),
    ("copy", "копировать"),
    ("share", "поделиться"),
    ("cancel", "отменить"),
    ("text size, default size", "размер текста, обычный размер"),
    ("theme", "тема"),
    ("settings", "настройки"),
    ("full screen", "во весь экран"),
    ("this help", "эта справка"),
    ("close", "закрыть"),
    ("Mouse", "Мышь"),
    (
        "a click in the left or right third turns the page, in the middle opens the menu",
        "щелчок в левой или правой трети листает, в середине открывает меню",
    ),
    ("a click on a picture or a table opens it", "щелчок по картинке или таблице открывает её"),
    ("a drag selects; a double click a word, a triple the paragraph", "протягивание выделяет; двойной щелчок — слово, тройной — абзац"),
    ("Picture or table", "Картинка или таблица"),
    ("zoom, fit", "масштаб, целиком"),
    ("move", "сдвиг"),
    ("Forzac is free. Support it: forzac.truenrth.app", "Форзац бесплатный. Поддержать: forzac.truenrth.app"),
    // The app and its windows.
    ("Forzac", "Форзац"),
    ("{title} — Forzac", "{title} — Форзац"),
    ("Reading a book", "Чтение книги"),
    // Opening books.
    ("Couldn't open “{name}”: {reason}", "Не удалось открыть «{name}»: {reason}"),
    ("unsupported format — FB2, FB3 or EPUB needed", "формат не поддерживается — нужен FB2, FB3 или EPUB"),
    ("the file is damaged", "файл повреждён"),
    ("the book is DRM-protected and can't be opened", "книга защищена (DRM) — такие файлы открыть нельзя"),
    ("the book has no text", "в книге нет текста"),
    ("the file is unavailable", "файл недоступен"),
    // The library.
    ("Open a book", "Открыть книгу"),
    ("Version {version} is out", "Вышла версия {version}"),
    ("download", "скачать"),
    ("Read books right on your computer", "Читайте книги прямо на\u{a0}компьютере"),
    (
        "Open a book file — a copy goes into your Books folder, the original stays where it is.",
        "Откройте файл книги — копия ляжет в\u{a0}папку «Книги», оригинал останется на\u{a0}месте.",
    ),
    ("or drop a file into this window", "или перетащите файл в\u{a0}это окно"),
    ("Show the Books folder", "Показать папку «Книги»"),
    ("FB2, FB2.ZIP, FB3 and EPUB are supported.", "Поддерживаются FB2, FB2.ZIP, FB3 и\u{a0}EPUB."),
    ("Release the file to open it", "Отпустите файл, чтобы открыть"),
    ("Continue reading", "Продолжить"),
    ("Search", "Поиск"),
    ("Title or author", "Название или автор"),
    ("No results", "Ничего не найдено"),
    ("Remove from library", "Убрать из библиотеки"),
    ("Remove “{title}” from the library?", "Убрать «{title}» из библиотеки?"),
    ("The book file goes to the Trash.", "Файл книги отправится в\u{a0}корзину."),
    ("Couldn't move “{title}” to the Trash", "Не удалось переместить «{title}» в\u{a0}корзину"),
    ("Cancel", "Отмена"),
    ("Remove", "Убрать"),
    ("Books", "Книги"),
    ("{n}% read", "прочитано {n}%"),
    // The page.
    ("Finished", "Прочитано"),
    ("New", "Новая"),
    ("Chapter {n}", "Глава {n}"),
    ("← Back · Ctrl+O", "← Вернуться · Ctrl+O"),
    ("← Back · ⌃O", "← Вернуться · ⌃O"),
    ("{n} of {m}", "{n} из {m}"),
    ("Highlight", "Цитата"),
    ("Copy", "Копировать"),
    ("Share", "Поделиться"),
    ("Delete", "Удалить"),
    // Panels.
    ("Contents", "Содержание"),
    ("Bookmarks", "Закладки"),
    ("Highlights", "Цитаты"),
    ("Filter contents", "Найти в оглавлении"),
    ("No bookmarks yet", "Закладок пока нет"),
    ("Press b on a page — a ribbon marks the place.", "Нажмите b на странице — ленточка отметит место."),
    ("No highlights yet", "Цитат пока нет"),
    (
        "Select text with the mouse, or press v on a page, then choose Highlight.",
        "Выделите текст мышью или нажмите v на странице, затем выберите «Цитата».",
    ),
    ("Search in book", "Поиск по книге"),
    ("No matches", "Ничего не найдено"),
    ("Note", "Примечание"),
    // The menu.
    ("Back to library", "К библиотеке"),
    ("Page {n} of {m}", "Страница {n} из {m}"),
    ("{n} min left in chapter", "{n} мин до конца главы"),
    ("Less than a minute left in chapter", "Меньше минуты до конца главы"),
    ("Text settings", "Настройки текста"),
    ("p. {n}", "стр. {n}"),
    // Settings.
    ("Line spacing", "Интервал"),
    ("Margins", "Поля"),
    ("Pages", "Листание"),
    ("Slide", "Сдвиг"),
    ("Scroll", "Лента"),
    ("Light", "Светлая"),
    ("Sepia", "Сепия"),
    ("Dark", "Тёмная"),
    ("Dawn", "Рассвет"),
    ("Wave", "Волна"),
    ("Forest", "Лес"),
    // macOS menus.
    ("About Forzac", "О программе «Форзац»"),
    ("Settings…", "Настройки…"),
    ("Hide Forzac", "Скрыть «Форзац»"),
    ("Quit Forzac", "Завершить «Форзац»"),
    ("File", "Файл"),
    ("Open…", "Открыть…"),
    ("Open Recent", "Открыть недавние"),
    ("Close", "Закрыть"),
    ("Edit", "Правка"),
    ("Find…", "Найти…"),
    ("Find Next", "Найти далее"),
    ("Find Previous", "Найти ранее"),
    ("View", "Вид"),
    ("Pages or Scroll", "Страницы или лента"),
    ("Next Theme", "Следующая тема"),
    ("Bigger Text", "Крупнее"),
    ("Smaller Text", "Мельче"),
    ("Default Size", "Обычный размер"),
    ("Enter Full Screen", "Во весь экран"),
    ("Go", "Переход"),
    ("Next Chapter", "Следующая глава"),
    ("Previous Chapter", "Предыдущая глава"),
    ("Back", "Назад"),
    ("Bookmark", "Закладка"),
    ("Window", "Окно"),
    ("Minimize", "Свернуть"),
    ("Help", "Справка"),
    ("Keys", "Клавиши"),
];

/// English "other" form → Russian one, few, many.
const PLURALS: &[(&str, [&str; 3])] = &[("{n} matches", ["{n} совпадение", "{n} совпадения", "{n} совпадений"])];

#[cfg(test)]
mod tests;
