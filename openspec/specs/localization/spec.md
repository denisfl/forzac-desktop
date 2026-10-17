# localization Specification

## Purpose
The interface in English and Russian: which language is shown, how it is chosen, what is translated and in whose
words.

## Requirements

### Requirement: The language
The interface SHALL be in Russian when the first of the system's preferred languages that Forzac has (English or
Russian) is Russian, else in English: on Linux the `LANGUAGE`, `LC_ALL`, `LC_MESSAGES` and `LANG` environment as GLib
reads it, on macOS the user's preferred languages. The language is read when Forzac starts. The book's language does
not change it, and the interface language does not change how a book is hyphenated.

#### Scenario: A Russian desktop
- **WHEN** Forzac starts with `LANG=ru_RU.UTF-8` on the Pi 5
- **THEN** the contents panel's tabs read "Содержание", "Закладки", "Цитаты"

#### Scenario: Another language
- **WHEN** the system prefers German, then English
- **THEN** the interface is in English

### Requirement: What is translated
Every string Forzac shows SHALL be translated: the panels (contents, bookmarks, highlights, search, settings, notes),
their placeholders and empty hints, the selection and highlight bars, the "← Back" pill, the end page's "Finished",
the contents' "Chapter N" for untitled chapters, the library (its buttons, note line, card, search, question, empty
state and the empty state's sample page), the messages for files that cannot be opened, the
help with its keys left as they are, the macOS menus and the reason the screen is kept on. Book texts, titles, authors,
font names and the "Aa" sample are not. Counts SHALL take Russian plural forms ("1 совпадение", "3 совпадения",
"5 совпадений").

#### Scenario: Searching in Russian
- **WHEN** the interface is Russian and a search finds 21 matches
- **THEN** the panel says "21 совпадение"

#### Scenario: The library in Russian
- **WHEN** the interface is Russian and Forzac starts without a book
- **THEN** the head says "Форзац" and "Открыть книгу", and the card "ПРОДОЛЖИТЬ"

### Requirement: The Android app's words
Strings the Android app has SHALL use its words in each language (its `strings.xml`): "Содержание", "Закладки",
"Цитаты", "Цитата", "Копировать", "Поделиться", "Удалить", "Примечание", "Глава N", "Прочитано", "Поиск по книге",
"Ничего не найдено", "прочитано N%", "Интервал", "Поля", "Листание", "Сдвиг", "Лента", the theme names "Светлая",
"Сепия", "Тёмная", "Рассвет", "Волна", "Лес", "Закладок пока нет", "Цитат пока нет", "← Вернуться", and its messages for
files that cannot be opened (without PDF). Strings only the desktop has are written in the same voice.

#### Scenario: The settings sheet in Russian
- **WHEN** the interface is Russian and the reader presses `s`
- **THEN** the themes are "Светлая", "Сепия", "Тёмная", "Рассвет", "Волна", "Лес" and the page modes "Сдвиг" and
  "Лента" under "Листание"

### Requirement: One catalogue
The translations SHALL be in one catalogue shared by both shells, keyed by the English text; a test SHALL fail when a
string either shell asks for has no Russian or when the catalogue holds a string no shell or the core asks for.

#### Scenario: A new string
- **WHEN** a developer adds `tr("Undo")` to a shell without a Russian entry
- **THEN** `cargo test -p forzac-core` fails and names the string

### Requirement: Cost
Looking up a string SHALL not show in the frame budget: page turns and scrolling at 120 Hz on a Raspberry Pi 5 miss no
more frames in Russian than in English, and the time to the first page grows by at most 5 ms.

#### Scenario: A Russian start
- **WHEN** a book is opened with the interface in Russian on a Raspberry Pi 5
- **THEN** the first page is presented within 5 ms of the English start's time
