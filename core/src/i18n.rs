//! The interface in English and Russian (`localization`): one catalogue for both shells, keyed by the English text,
//! in the Android app's words (`values/strings.xml`, `values-ru/strings.xml`). The language is chosen once, at start:
//! the reader's choice in the settings, else the system's preferred languages.

use std::collections::HashMap;
use std::sync::OnceLock;
use std::sync::atomic::{AtomicU8, Ordering};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Language {
    English = 0,
    Russian = 1,
}

static LANGUAGE: AtomicU8 = AtomicU8::new(Language::English as u8);

/// The state's setting with the reader's choice of language (`settings`): `en`, `ru`, or empty or none for the system's.
pub const SETTING: &str = "language";

impl Language {
    /// The language's code as `SETTING` keeps it.
    pub fn code(self) -> &'static str {
        match self {
            Language::English => "en",
            Language::Russian => "ru",
        }
    }

    /// The reader's choice (`SETTING`'s value), else the first of the system's `locales` that Forzac has.
    pub fn choose<'a>(setting: Option<&'a str>, locales: impl IntoIterator<Item = &'a str>) -> Language {
        Language::from_locales(setting.into_iter().filter(|s| !s.is_empty()).chain(locales))
    }

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
const SUPPORT: &str = "Forzac is free. Support it: forzac.app";

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

/// The common FB2 genre codes (FictionBook 2.1's list) with their English and Russian names.
const FB2_GENRES: &[(&str, &str, &str)] = &[
    ("sf_history", "Alternative history", "Альтернативная история"),
    ("sf_action", "Action science fiction", "Боевая фантастика"),
    ("sf_epic", "Epic science fiction", "Эпическая фантастика"),
    ("sf_heroic", "Heroic fantasy", "Героическая фантастика"),
    ("sf_detective", "Science fiction detective", "Детективная фантастика"),
    ("sf_cyberpunk", "Cyberpunk", "Киберпанк"),
    ("sf_space", "Space fiction", "Космическая фантастика"),
    ("sf_social", "Social science fiction", "Социальная фантастика"),
    ("sf_horror", "Horror and mystery", "Ужасы и мистика"),
    ("sf_humor", "Humorous science fiction", "Юмористическая фантастика"),
    ("sf_fantasy", "Fantasy", "Фэнтези"),
    ("sf_postapocalyptic", "Post-apocalyptic fiction", "Постапокалипсис"),
    ("sf", "Science fiction", "Научная фантастика"),
    ("det_classic", "Classic detective", "Классический детектив"),
    ("det_police", "Police procedural", "Полицейский детектив"),
    ("det_action", "Action", "Боевик"),
    ("det_irony", "Ironic detective", "Иронический детектив"),
    ("det_history", "Historical detective", "Исторический детектив"),
    ("det_espionage", "Espionage", "Шпионский детектив"),
    ("det_crime", "Crime", "Криминальный детектив"),
    ("det_political", "Political detective", "Политический детектив"),
    ("det_maniac", "Maniacs", "Маньяки"),
    ("det_hard", "Hardboiled detective", "Крутой детектив"),
    ("thriller", "Thriller", "Триллер"),
    ("detective", "Detective", "Детектив"),
    ("prose_classic", "Classic prose", "Классическая проза"),
    ("prose_history", "Historical prose", "Историческая проза"),
    ("prose_contemporary", "Contemporary prose", "Современная проза"),
    ("prose_counter", "Counterculture", "Контркультура"),
    ("prose_rus_classic", "Russian classics", "Русская классика"),
    ("prose_su_classics", "Soviet classics", "Советская классика"),
    ("prose_military", "Military prose", "Военная проза"),
    ("love_contemporary", "Contemporary romance", "Современные любовные романы"),
    ("love_history", "Historical romance", "Исторические любовные романы"),
    ("love_detective", "Romantic suspense", "Остросюжетные любовные романы"),
    ("love_short", "Short romance", "Короткие любовные романы"),
    ("love_erotica", "Erotica", "Эротика"),
    ("adv_western", "Western", "Вестерн"),
    ("adv_history", "Historical adventure", "Исторические приключения"),
    ("adv_indian", "Adventures among Native Americans", "Приключения про индейцев"),
    ("adv_maritime", "Sea adventure", "Морские приключения"),
    ("adv_geo", "Travel and geography", "Путешествия и география"),
    ("adv_animal", "Nature and animals", "Природа и животные"),
    ("adventure", "Adventure", "Приключения"),
    ("child_tale", "Fairy tales", "Сказки"),
    ("child_verse", "Children's verse", "Детские стихи"),
    ("child_prose", "Children's prose", "Детская проза"),
    ("child_sf", "Children's science fiction", "Детская фантастика"),
    ("child_det", "Children's thrillers", "Детские остросюжетные"),
    ("child_adv", "Children's adventure", "Детские приключения"),
    ("child_education", "Children's educational books", "Детская образовательная литература"),
    ("children", "Children's books", "Детская литература"),
    ("poetry", "Poetry", "Поэзия"),
    ("dramaturgy", "Drama", "Драматургия"),
    ("antique_ant", "Ancient literature", "Античная литература"),
    ("antique_european", "Old European literature", "Европейская старинная литература"),
    ("antique_russian", "Old Russian literature", "Древнерусская литература"),
    ("antique_east", "Old Eastern literature", "Древневосточная литература"),
    ("antique_myths", "Myths and legends", "Мифы, легенды, эпос"),
    ("antique", "Old literature", "Старинная литература"),
    ("sci_history", "History", "История"),
    ("sci_psychology", "Psychology", "Психология"),
    ("sci_culture", "Cultural studies", "Культурология"),
    ("sci_religion", "Religious studies", "Религиоведение"),
    ("sci_philosophy", "Philosophy", "Философия"),
    ("sci_politics", "Politics", "Политика"),
    ("sci_business", "Business", "Деловая литература"),
    ("sci_juris", "Law", "Юриспруденция"),
    ("sci_linguistic", "Linguistics", "Языкознание"),
    ("sci_medicine", "Medicine", "Медицина"),
    ("sci_phys", "Physics", "Физика"),
    ("sci_math", "Mathematics", "Математика"),
    ("sci_chem", "Chemistry", "Химия"),
    ("sci_biology", "Biology", "Биология"),
    ("sci_tech", "Engineering", "Технические науки"),
    ("science", "Science", "Научная литература"),
    ("comp_www", "Internet", "Интернет"),
    ("comp_programming", "Programming", "Программирование"),
    ("comp_hard", "Computer hardware", "Компьютерное железо"),
    ("comp_soft", "Software", "Программы"),
    ("comp_db", "Databases", "Базы данных"),
    ("comp_osnet", "Operating systems and networks", "ОС и сети"),
    ("computers", "Computers", "Компьютеры"),
    ("ref_encyc", "Encyclopedias", "Энциклопедии"),
    ("ref_dict", "Dictionaries", "Словари"),
    ("ref_ref", "Reference books", "Справочники"),
    ("ref_guide", "Guides", "Руководства"),
    ("reference", "Reference", "Справочная литература"),
    ("nonf_biography", "Biographies and memoirs", "Биографии и мемуары"),
    ("nonf_publicism", "Essays and journalism", "Публицистика"),
    ("nonf_criticism", "Criticism", "Критика"),
    ("design", "Art and design", "Искусство и дизайн"),
    ("nonfiction", "Nonfiction", "Документальная литература"),
    ("religion_rel", "Religion", "Религия"),
    ("religion_esoterics", "Esoterics", "Эзотерика"),
    ("religion_self", "Self-improvement", "Самосовершенствование"),
    ("religion", "Religion and spirituality", "Религия и духовность"),
    ("humor_anecdote", "Jokes", "Анекдоты"),
    ("humor_prose", "Humorous prose", "Юмористическая проза"),
    ("humor_verse", "Humorous verse", "Юмористические стихи"),
    ("humor", "Humour", "Юмор"),
    ("home_cooking", "Cooking", "Кулинария"),
    ("home_pets", "Pets", "Домашние животные"),
    ("home_crafts", "Crafts and hobbies", "Хобби и ремёсла"),
    ("home_entertain", "Entertainment", "Развлечения"),
    ("home_health", "Health", "Здоровье"),
    ("home_garden", "Gardening", "Сад и огород"),
    ("home_diy", "Do it yourself", "Сделай сам"),
    ("home_sport", "Sport", "Спорт"),
    ("home", "Home and family", "Дом и семья"),
    ("economics", "Economics", "Экономика"),
];

/// The English name of the FB2 genre `code`; none for a code not in the list.
pub fn fb2_genre(code: &str) -> Option<&'static str> {
    let code = code.trim().to_lowercase();
    FB2_GENRES.iter().find(|g| g.0 == code).map(|g| g.1)
}

/// A genre as shown: an FB2 genre's name in the interface language, any other (an EPUB's subject) as written.
pub fn genre_name(genre: &str) -> &str {
    match FB2_GENRES.iter().find(|g| g.1 == genre) {
        Some(g) if language() == Language::Russian => g.2,
        _ => genre,
    }
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
    ("Forzac is free. Support it: forzac.app", "Форзац бесплатный. Поддержать: forzac.app"),
    ("Made by Denis Fedosov-Ledovskikh", "Сделал Денис Федосов-Ледовских"),
    // The app and its windows.
    ("Forzac", "Форзац"),
    ("{title} — Forzac", "{title} — Форзац"),
    ("Reading a book", "Чтение книги"),
    // Opening books.
    ("Couldn't open “{name}”: {reason}", "Не удалось открыть «{name}»: {reason}"),
    ("unsupported format — FB2, FB3, EPUB, TXT or Markdown needed", "формат не поддерживается — нужен FB2, FB3, EPUB, TXT или Markdown"),
    ("the file is damaged", "файл повреждён"),
    ("the book is DRM-protected and can't be opened", "книга защищена (DRM) — такие файлы открыть нельзя"),
    ("the book has no text", "в книге нет текста"),
    ("the file is unavailable", "файл недоступен"),
    // The library.
    ("Open a book", "Открыть книгу"),
    ("Version {version} is out", "Вышла версия {version}"),
    ("download", "скачать"),
    ("install", "установить"),
    ("Installing version {version}…", "Устанавливается версия {version}…"),
    ("Read books right on your computer", "Читайте книги прямо на\u{a0}компьютере"),
    (
        "Open a book file — a copy goes into your Books folder, the original stays where it is.",
        "Откройте файл книги — копия ляжет в\u{a0}папку «Книги», оригинал останется на\u{a0}месте.",
    ),
    ("or drop a file into this window", "или перетащите файл в\u{a0}это окно"),
    ("Show the Books folder", "Показать папку «Книги»"),
    ("FB2, FB2.ZIP, FB3, EPUB, TXT and Markdown are supported.", "Поддерживаются FB2, FB2.ZIP, FB3, EPUB, TXT и\u{a0}Markdown."),
    ("Release the file to open it", "Отпустите файл, чтобы открыть"),
    ("Add a Folder…", "Добавить папку…"),
    ("Adding books… {n} of {m}", "Добавляю книги… {n} из\u{a0}{m}"),
    ("No books in “{folder}”: FB2, FB3 or EPUB needed.", "В\u{a0}«{folder}» нет книг: нужны FB2, FB3 или EPUB."),
    ("No books were added.", "Книги не добавлены."),
    ("Continue reading", "Продолжить"),
    ("Search", "Поиск"),
    ("Title, author or series", "Название, автор или серия"),
    ("Recent", "Недавние"),
    ("Title", "Название"),
    ("Author", "Автор"),
    ("Series", "Серия"),
    ("Unknown author", "Автор не указан"),
    ("Not in a series", "Вне серий"),
    ("Shelves", "Полки"),
    ("Reading", "Читаю"),
    ("Not started", "Не начато"),
    ("Add to shelf", "На полку"),
    ("New shelf…", "Новая полка…"),
    ("New shelf", "Новая полка"),
    ("Shelf name", "Название полки"),
    ("Add", "Добавить"),
    ("Edit…", "Изменить…"),
    ("Edit the book", "Данные книги"),
    ("Authors", "Авторы"),
    ("Number", "Номер"),
    ("Genres", "Жанры"),
    ("From the book", "Как в книге"),
    ("Save", "Сохранить"),
    ("Change the Books folder…", "Сменить папку «Книги»…"),
    ("Move the books to “{folder}”?", "Перенести книги в\u{a0}«{folder}»?"),
    (
        "The books in “{folder}” go to the new folder; where you stopped in each stays.",
        "Книги из\u{a0}«{folder}» переедут в\u{a0}новую папку, места чтения сохранятся.",
    ),
    ("Move", "Перенести"),
    ("Leave them", "Оставить"),
    ("Send to Kindle", "Отправить на Kindle"),
    ("Connect your Kindle with a cable and try again.", "Подключите Kindle кабелем и\u{a0}попробуйте ещё раз."),
    ("“{title}” is on your Kindle. Eject it before reading.", "«{title}» уже на\u{a0}Kindle. Извлеките его, прежде чем читать."),
    ("Couldn't write to the Kindle.", "Не\u{a0}удалось записать на\u{a0}Kindle."),
    ("About the book", "О книге"),
    ("Read", "Читать"),
    ("More", "Ещё"),
    ("Russian", "Русский"),
    ("English", "Английский"),
    ("Ukrainian", "Украинский"),
    ("Belarusian", "Белорусский"),
    ("German", "Немецкий"),
    ("French", "Французский"),
    ("Spanish", "Испанский"),
    ("Italian", "Итальянский"),
    ("Polish", "Польский"),
    ("Portuguese", "Португальский"),
    ("Japanese", "Японский"),
    ("Chinese", "Китайский"),
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
    // The settings (`settings`).
    ("Settings", "Настройки"),
    ("Language", "Язык"),
    ("As the system", "Как в\u{a0}системе"),
    ("Forzac will speak the chosen language after a restart.", "Форзац заговорит на\u{a0}выбранном языке после перезапуска."),
    ("Sync", "Синхронизация"),
    (
        "Your place, bookmarks, highlights, shelves and book details are the same in Forzac on every computer and phone that shares the Books folder. Keep the folder in sync with Dropbox, iCloud Drive or Syncthing; on Android only Syncthing keeps a folder on the phone.",
        "Место чтения, закладки, цитаты, полки и\u{a0}правки книг\u{a0}— одни и\u{a0}те\u{a0}же в\u{a0}Форзаце на\u{a0}всех компьютерах и\u{a0}телефонах с\u{a0}общей папкой «Книги». Синхронизируйте папку через Dropbox, iCloud Drive или Syncthing; на\u{a0}Android папку на\u{a0}телефоне держит только Syncthing.",
    ),
    ("Books folder", "Папка «Книги»"),
    ("Change…", "Сменить…"),
    ("Syncing…", "Синхронизация…"),
    ("Synced {when}", "Синхронизировано {when}"),
    ("Couldn't sync: {reason}", "Не\u{a0}удалось синхронизировать: {reason}"),
    ("Not synced yet", "Ещё не\u{a0}синхронизировано"),
    ("Other devices", "Другие устройства"),
    ("Another device", "Другое устройство"),
    ("None yet", "Пока нет"),
    ("Sync Now", "Синхронизировать сейчас"),
    ("today", "сегодня"),
    ("yesterday", "вчера"),
    ("{date} at {time}", "{date} в\u{a0}{time}"),
    ("January", "января"),
    ("February", "февраля"),
    ("March", "марта"),
    ("April", "апреля"),
    ("May", "мая"),
    ("June", "июня"),
    ("July", "июля"),
    ("August", "августа"),
    ("September", "сентября"),
    ("October", "октября"),
    ("November", "ноября"),
    ("December", "декабря"),
];

/// English "other" form → Russian one, few, many.
const PLURALS: &[(&str, [&str; 3])] = &[
    ("{n} matches", ["{n} совпадение", "{n} совпадения", "{n} совпадений"]),
    ("Added {n} books.", ["Добавлена {n} книга.", "Добавлены {n} книги.", "Добавлено {n} книг."]),
    (
        "Couldn't move {n} books.",
        ["Не\u{a0}удалось перенести {n} книгу.", "Не\u{a0}удалось перенести {n} книги.", "Не\u{a0}удалось перенести {n} книг."],
    ),
    (
        "{n} files couldn't be read.",
        ["{n} файл не\u{a0}удалось прочитать.", "{n} файла не\u{a0}удалось прочитать.", "{n} файлов не\u{a0}удалось прочитать."],
    ),
    (
        "No new books in “{folder}”: all {n} are already in the library.",
        [
            "В\u{a0}«{folder}» нет новых книг: {n} книга уже в\u{a0}библиотеке.",
            "В\u{a0}«{folder}» нет новых книг: все {n} уже в\u{a0}библиотеке.",
            "В\u{a0}«{folder}» нет новых книг: все {n} уже в\u{a0}библиотеке.",
        ],
    ),
];

#[cfg(test)]
mod tests;
