# Tasks

## 1. Core

- [x] 1.1 Model: series, number, genres, year; FB2 (`sequence`, `genre`, `date`, `publish-info/year`), FB3 (`sequence`,
      `fb3-classification/subject`, `written/date`), EPUB (`calibre:series`, `belongs-to-collection`, `dc:subject`,
      `dc:date`); tests per format
- [x] 1.2 FB2 genre names in English and Russian
- [x] 1.3 State: `file` lines with series and number, old lines dropped; `Entry` series from the cache; search over series
- [x] 1.4 `library::View`, `arrange`, the comparison key and the surname; tests
- [x] 1.5 `library::step` across groups; tests
- [x] 1.6 `library::BookInfo`; the strings; C ABI (`forzac_library_books` with series, `forzac_library_arrange`,
      `forzac_library_step`, `forzac_book_info`), `macos/forzac.h`; tests

## 2. Shells

- [x] 2.1 GTK: tabs, group headings, series under covers, selection through `step`, `Tab`, the "⋯" menu and right
      click, the sheet and `i`
- [x] 2.2 macOS: the same
- [x] 2.3 Both builds and clippy green; by hand on the Mac in both languages over the maintainer's ~/Books (a state of
      its own): the tabs, Author and Series groups, the series under a cover, the "⋯" menu and right click, About the
      book, Close; `j`/`l` across groups through `--keys`. GTK seen as a PNG under headless Weston in the Debian
      container (tabs, headings, "⋯"); not on a Linux desktop
- [x] 2.4 `docs/roadmap.md` status
