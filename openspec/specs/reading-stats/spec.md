# reading-stats Specification

## Purpose
TBD - created by archiving change reading-stats. Update Purpose after archive.

## Requirements

### Requirement: Statistics for a period
Forzac SHALL count statistics for a calendar month or year in local time from the reading log and the finishes
(`reading-log`) of every device whose data reached this computer (`sync`), over all books and all devices:
- **time** — the seconds of the log on the period's days;
- **days with a book** — the period's days with at least a minute read (all devices together);
- **books** — the different books with a counted finish in the period, also approximate finishes, books removed from
  the library and books finished on other devices;
- **a day's level** — by its minutes: 0 (under 1), 1 (1–14), 2 (15–29), 3 (30–59), 4 (60 and more).

One reading marked by several finishes SHALL count once: a finish SHALL NOT count when a counted finish of the same
book was less than 30 days before it; an approximate finish SHALL NOT count either when the book has an earlier finish
or an exact one less than 30 days after it. The statistics SHALL be counted afresh each time they show, and SHALL NOT
leave the computer.

#### Scenario: A book read twice in a year
- **WHEN** the reader finished a book in March and again in November
- **THEN** the year counts one book, March and November one each

#### Scenario: A removed book
- **WHEN** the reader finished a book and removed it from the library
- **THEN** it stays among its month's and year's "Finished", its time in the numbers

#### Scenario: Reading through New Year's midnight
- **WHEN** the reader read from 23:30 on 31 December to 00:30 on 1 January
- **THEN** the first half hour counts in December and that year, the second in January and the next year

#### Scenario: Opened for a moment
- **WHEN** a day has 20 seconds of reading
- **THEN** the day is not a day with a book and is not shaded; the seconds count in the time

#### Scenario: A month on the phone and the Mac
- **WHEN** in October the reader read 6 h on the phone and 4 h on the Mac, finished one book on each, and they synced
- **THEN** the Mac's October shows "10 h" and 2 books, both under "Finished"

#### Scenario: Half a minute on each device
- **WHEN** the reader read 40 seconds on the phone and 40 seconds on the Mac on one day
- **THEN** the day is a day with a book

#### Scenario: Finished offline on two devices
- **WHEN** the reader finished a book on the phone on 31 October, and on 2 November, before the phone's finish
  arrived, the same book on the Mac, and then they synced
- **THEN** the book counts once, in October; November does not have it, the year has one

#### Scenario: Approximate dates from two devices
- **WHEN** a book finished before the log has an approximate finish in May 2025 from the phone and in October 2026
  from the Mac
- **THEN** the book counts once, in May 2025

### Requirement: The reading calendar
The calendar SHALL show the period's days in a grid of weeks starting on Monday in Russian and on Sunday in English. A
month is one grid with the day numbers in its cells and the weekday names above; a year is twelve named months, three a
row, without numbers. A level-0 cell SHALL be the theme's line colour, levels 1–4 the theme's link colour at about 30,
50, 75 and 100 % opacity. Today SHALL be outlined; the current period's future days SHALL not be shaded. A click on a
past day SHALL show under the calendar its date and time ("12 October · 48 min" / "12 октября · 48 мин", or "· not
read" / "· не читали").

#### Scenario: The Forest theme
- **WHEN** the reader's theme is "Forest" and the stats open
- **THEN** the window has the theme's paper and ink, and the days are shades of its link colour

#### Scenario: A click on a day
- **WHEN** the reader clicks 12 October, a day with 48 minutes
- **THEN** "12 October · 48 min" shows under the calendar

### Requirement: The picture
The share button (and `s`) SHALL make a 1080 × 1920 PNG of the shown period: the period large (a month's name with the
year under it, or the year), the calendar as in the window (a year four months a row), the three figures with
captions, and a small "Forzac" ("Форзац") at the bottom, in the reader's theme's colours. It SHALL hold no links, QR
codes, store names or covers. On macOS it SHALL open the system's share menu with the picture; on Linux it SHALL save
it where the file dialog says, "forzac-2026-10.png" (or "forzac-2026.png") in the Pictures folder by default.

#### Scenario: Sharing a month on the Mac
- **WHEN** the reader on "October 2026" presses `s` and chooses Messages
- **THEN** a 1080 × 1920 picture goes: "October", "2026", October's calendar, the figures, "Forzac"

#### Scenario: Saving a year on Linux in the dark theme
- **WHEN** the theme is Dark, the window shows "2026" and the reader presses `s` and "Save"
- **THEN** a dark `forzac-2026.png` with twelve months is in the Pictures folder

### Requirement: Speed
The stats window SHALL show within 150 ms of the menu item or the end page's month (a year of the log on three
devices, 50 books) on the Pi 5, a period change within 50 ms, the picture and the share menu (or the file dialog)
within 1 s. Nothing SHALL be drawn while the window is idle.

#### Scenario: A year with 50 books
- **WHEN** the log holds a year on three devices and 50 finished books, and the reader opens the stats and presses `y`
- **THEN** the window shows within 150 ms and the year within 50 ms

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
