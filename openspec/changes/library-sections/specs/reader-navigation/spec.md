## MODIFIED Requirements

### Requirement: The end of the book
In "Slide", turning forward from the last page of the book SHALL show an end page: in the middle of the window, the
book's title, its authors and "Finished", in the reader's theme, without running head or foot. In "Scroll", the same
end screen, one window high, SHALL follow the last paragraph. Showing it SHALL remember the book as finished: its
position stays at the start of the last page of text and its share read becomes 100 % (the contents and the recent
books show 100 %). Turning back or scrolling up returns to the text; forward keys on the end page do nothing.

Under "Finished" the end page SHALL show the book's totals, the parts that are not zero: its time by the log of every
device (`reading-log`, in the stats' form: "35 min", "6 h"), the days since it was first opened on this computer up to
today inclusive, its highlights and its bookmarks, with plural forms ("6 h · 12 days · 7 highlights"). The time SHALL
show only when the log's first day (any device) is not later than the day the book was first opened here; without a
known first opening neither time nor days show. Under the totals, in the link colour, the current month SHALL show its
finished books and time by the stats (`reading-stats`): "October: 3 books · 14 h" ("Октябрь: 3 книги · 14 ч"). A
click on it, or `Tab` to it and `Enter`, SHALL bring the library window forward (opening it if needed) on "Reading
stats" (`library`, `Sections`) at the current month; the book window stays on its end page behind it.

#### Scenario: Finishing a book
- **WHEN** the reader presses `Space` on the last page of the book
- **THEN** the end page with the title, the author and "Finished" slides in, and the recent books show the book at
  100 %

#### Scenario: Back to the text
- **WHEN** the reader presses `k` on the end page
- **THEN** the last page of the text is shown

#### Scenario: The totals
- **WHEN** the reader, who first opened the book here 12 days ago, logged 6 h 10 min on it and made 7 highlights,
  turns past its last page
- **THEN** under "Finished" the end page shows "6 h · 12 days · 7 highlights" and "October: 3 books · 14 h"

#### Scenario: A book begun before the log
- **WHEN** the reader finishes a book opened before the version with the log
- **THEN** the totals show no time and no days, only the highlights and bookmarks; the month shows

#### Scenario: Read on the phone too
- **WHEN** the reader read the book 4 h on the phone and 2 h on the Mac, the devices synced, and the Mac shows its end
  page
- **THEN** the totals start with "6 h"

#### Scenario: The stats from the end page
- **WHEN** the reader presses `Tab` on the end page, then `Enter`
- **THEN** the stats window opens on the current month with the book first under "Finished"; `Esc` returns to the end
  page

#### Scenario: The month opens the stats
- **WHEN** the reader clicks "October: 3 books · 14 h" on the end page while the library window is closed
- **THEN** the library window opens on "Reading stats" at October, and the book window still shows its end page
