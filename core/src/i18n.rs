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

/// The keys and the support line, in the interface language.
pub fn help(platform: Platform) -> &'static str {
    tr(match platform {
        Platform::Linux => HELP_LINUX,
        Platform::Mac => HELP_MAC,
    })
}

/// The English the core asks for itself (the help is asked for by its platform).
#[cfg(test)]
const ASKED_BY_HELP: [&str; 2] = [HELP_LINUX, HELP_MAC];

const HELP_LINUX: &str = "\
j  l  →  ↓  Space  PgDn  Enter    next page (Scroll: j ↓ a line, Ctrl+D half a screen)
k  h  ←  ↑  Shift+Space  PgUp    previous page (Scroll: k ↑ a line, Ctrl+U half a screen)
m    page or scroll
]  [    next / previous chapter
g  G  Home  End    chapter start / end
o    contents  (/ filters, ← → tabs)
b    bookmark the page, again to remove it
B    bookmarks  (Enter goes, d removes)
v    select: h l words, j k lines, v anchors, o other end,
     Enter highlight, c copy, Esc back
Tab  Shift+Tab    notes, links, pictures and tables on the page, Enter opens
Ctrl+O    back after a jump
s    settings
/  n  N    search, next / previous match
+  −  0    text size, default size
t  T    theme
f  F11    full screen
?    this help
q  Ctrl+W    close
click    the left or right third of the page turns it; a picture or a table opens;
         a drag selects (a double click in the middle: a word, a triple: the paragraph)

Picture or table:  + − 0  zoom, fit    arrows  h j k l  Space  move    Esc  q  close

Forzac is free. Support it: forzac.truenrth.app";

const HELP_LINUX_RU: &str = "\
j  l  →  ↓  Space  PgDn  Enter    следующая страница (Лента: j ↓ на строку, Ctrl+D на полэкрана)
k  h  ←  ↑  Shift+Space  PgUp    предыдущая страница (Лента: k ↑ на строку, Ctrl+U на полэкрана)
m    страницы или лента
]  [    следующая / предыдущая глава
g  G  Home  End    начало / конец главы
o    содержание  (/ фильтр, ← → вкладки)
b    закладка на странице, ещё раз — убрать
B    закладки  (Enter — перейти, d — удалить)
v    выделение: h l слова, j k строки, v закрепить, o другой конец,
     Enter цитата, c копировать, Esc назад
Tab  Shift+Tab    примечания, ссылки, картинки и таблицы на странице, Enter открывает
Ctrl+O    вернуться после перехода
s    настройки
/  n  N    поиск, следующее / предыдущее совпадение
+  −  0    размер текста, обычный размер
t  T    тема
f  F11    во весь экран
?    эта справка
q  Ctrl+W    закрыть
щелчок    в левой или правой трети страницы листает; картинка или таблица открывается;
          протягивание выделяет (двойной щелчок в середине — слово, тройной — абзац)

Картинка или таблица:  + − 0  масштаб, целиком    стрелки  h j k l  Space  сдвиг    Esc  q  закрыть

Форзац бесплатный. Поддержать: forzac.truenrth.app";

const HELP_MAC: &str = "\
j  l  →  ↓  Space  PgDn  Return    next page (Scroll: j ↓ a line, ⌃D half a screen)
k  h  ←  ↑  Shift+Space  PgUp    previous page (Scroll: k ↑ a line, ⌃U half a screen)
m    pages or scroll
]  [    next / previous chapter
g  G  Home  End    chapter start / end
o    contents  (/ filters, ← → tabs)
b  ⌘D    bookmark the page, again to remove it
B    bookmarks  (Return goes, d removes)
v    select: h l words, j k lines, v anchors, o other end,
     Return highlight, c ⌘C copy, s share, Esc back
Tab  Shift+Tab    notes, links, pictures and tables on the page, Return opens
⌃O  ⌘[    back after a jump
s  ⌘,    settings
/  ⌘F    search;  n  N  ⌘G  ⇧⌘G    next / previous match
+  −  0    text size, default size
t  T    theme
f  ⌃⌘F    full screen
?    this help
q  ⌘W    close
click    the left or right third of the page turns it; a picture or a table opens;
         a drag selects (a double click in the middle: a word, a triple: the paragraph)

Picture or table:  + − 0  zoom, fit    arrows  h j k l  Space  move    Esc  q  close

Forzac is free. Support it: forzac.truenrth.app";

const HELP_MAC_RU: &str = "\
j  l  →  ↓  Space  PgDn  Return    следующая страница (Лента: j ↓ на строку, ⌃D на полэкрана)
k  h  ←  ↑  Shift+Space  PgUp    предыдущая страница (Лента: k ↑ на строку, ⌃U на полэкрана)
m    страницы или лента
]  [    следующая / предыдущая глава
g  G  Home  End    начало / конец главы
o    содержание  (/ фильтр, ← → вкладки)
b  ⌘D    закладка на странице, ещё раз — убрать
B    закладки  (Return — перейти, d — удалить)
v    выделение: h l слова, j k строки, v закрепить, o другой конец,
     Return цитата, c ⌘C копировать, s поделиться, Esc назад
Tab  Shift+Tab    примечания, ссылки, картинки и таблицы на странице, Return открывает
⌃O  ⌘[    вернуться после перехода
s  ⌘,    настройки
/  ⌘F    поиск;  n  N  ⌘G  ⇧⌘G    следующее / предыдущее совпадение
+  −  0    размер текста, обычный размер
t  T    тема
f  ⌃⌘F    во весь экран
?    эта справка
q  ⌘W    закрыть
щелчок    в левой или правой трети страницы листает; картинка или таблица открывается;
          протягивание выделяет (двойной щелчок в середине — слово, тройной — абзац)

Картинка или таблица:  + − 0  масштаб, целиком    стрелки  h j k l  Space  сдвиг    Esc  q  закрыть

Форзац бесплатный. Поддержать: forzac.truenrth.app";

/// English → Russian. The Android app's words where it has the string.
const RUSSIAN: &[(&str, &str)] = &[
    (HELP_LINUX, HELP_LINUX_RU),
    (HELP_MAC, HELP_MAC_RU),
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
    // Recent books.
    (
        "No books yet.\n\nOpen one from your file manager, or run\nforzac book.epub",
        "Книг пока нет.\n\nОткройте книгу из файлового менеджера или выполните\nforzac book.epub",
    ),
    ("Open a book: ⌘O, or drop it on Forzac in the Dock.", "Откройте книгу: ⌘O — или перетащите её на значок Форзаца в Dock."),
    ("Recent books — Return opens, ⌘O opens another.", "Недавние книги — Return открывает, ⌘O открывает другую."),
    ("{n}% read", "прочитано {n}%"),
    // The page.
    ("Finished", "Прочитано"),
    ("Chapter {n}", "Глава {n}"),
    ("← Back · Ctrl+O", "← Вернуться · Ctrl+O"),
    ("← Back · ⌃O", "← Вернуться · ⌃O"),
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
