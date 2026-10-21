## MODIFIED Requirements

### Requirement: The language
The interface SHALL be in the language the reader chose in the settings (`settings`), kept in the state; when they
chose "As the system" (the default), in Russian when the first of the system's preferred languages that Forzac has
(English or Russian) is Russian, else in English: on Linux the `LANGUAGE`, `LC_ALL`, `LC_MESSAGES` and `LANG`
environment as GLib reads it, on macOS the user's preferred languages. The language is read when Forzac starts. The
book's language does not change it, and the interface language does not change how a book is hyphenated.

#### Scenario: A Russian desktop
- **WHEN** Forzac starts with `LANG=ru_RU.UTF-8` on the Pi 5
- **THEN** the contents panel's tabs read "Содержание", "Закладки", "Цитаты"

#### Scenario: Another language
- **WHEN** the system prefers German, then English
- **THEN** the interface is in English

#### Scenario: The reader's choice
- **WHEN** the reader chose "Русский" in the settings and Forzac starts with `LANG=en_US.UTF-8`
- **THEN** the contents panel's tabs read "Содержание", "Закладки", "Цитаты"
