## REMOVED Requirements

### Requirement: The stats window
**Reason**: The stats become a section of the library window (`library`, `Sections`), as on Android, so that a second
window does not have to be found, sized and closed.
**Migration**: The same content and keys in the section "Reading stats" (`The Reading stats section`); `Esc` returns to
"Books" instead of closing a window.

## ADDED Requirements

### Requirement: The Reading stats section
"Reading stats" ("Статистика") SHALL be the library window's second section (`library`, `Sections`), showing one
period: "Month · Year" ("Месяц · Год"), the period ("October 2026" / "Октябрь 2026", or "2026") between arrows to the
neighbour periods, three figures with captions (time read — "reading" / "за чтением"; books — "books finished" / "книги
прочитано"; days — "days with a book" / "дня с книгой", with the plural forms of the interface language), the reading
calendar, and "Finished" ("Прочитано"). Its share action SHALL be in the window's head (`library`): Share on macOS,
"Save Image…" on Linux. The arrows SHALL go from the first month (year) with a log day or a finish to the current one;
beyond them an arrow is inactive. It SHALL show the current month when the library window opens on it or when the
reader comes to it the first time since the window opened, and keep the chosen period while the reader switches between
the sections. Time SHALL show in minutes under an hour and in whole hours, rounded, above ("35 min", "8 h", "41 h").
"Finished" SHALL list the period's books, the latest finished first: title, authors and date ("Alexander Pushkin · 26
September"); an empty period shows zeros, an empty calendar and "What you finish will appear here" ("Здесь появится то,
что вы прочитаете"). The section SHALL follow the reader's theme, SHALL add no setting, and SHALL lay out without
clipping in the window's narrowest size (`library`), the calendar shrinking with the width.

Keys: `←` or `h` and `→` or `l` SHALL go to the previous and next period, `m` to Month and `y` to Year, `s` SHALL share
(macOS) or save the picture (Linux), `↑` `↓` `j` `k` `Space` `Page Up` `Page Down` `Home` `End` SHALL scroll, and `Esc`
SHALL return to "Books"; also in a Russian keyboard layout.

#### Scenario: Opened mid-month
- **WHEN** on 18 October the reader switches to "Reading stats"
- **THEN** "Month" is selected, "October 2026" shows its days up to the 18th, the forward arrow is inactive

#### Scenario: The year
- **WHEN** the reader presses `y`
- **THEN** "2026" shows twelve months, the year's figures and its finished books

#### Scenario: An empty month
- **WHEN** the reader presses `←` to a month without reading
- **THEN** zeros, an empty calendar and "What you finish will appear here"

#### Scenario: The period is kept
- **WHEN** the reader goes back to September, switches to "Books" and back to "Reading stats"
- **THEN** September is shown
