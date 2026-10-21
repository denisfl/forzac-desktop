## ADDED Requirements

### Requirement: The settings window
"Settings…" ("Настройки…") in the library's menu SHALL open the settings window: on macOS a window of its own titled
"Settings" ("Настройки"), not resizable, brought forward when it is already open; on Linux a sheet over the library in
the library's theme. It SHALL hold only two sections, "Language" ("Язык") and "Sync" ("Синхронизация"), in that order.
On macOS `⌘W` and `Esc` SHALL close the window; on Linux `Esc`, "Close" ("Закрыть") and a click beside the sheet SHALL
close it. Opening it SHALL NOT sync or read any book; it SHALL show within 100 ms with ten device files in `.forzac/`,
and SHALL draw nothing while idle.

#### Scenario: Opening the settings on Linux
- **WHEN** the reader clicks "⋯" in the library's head and chooses "Settings…"
- **THEN** a sheet with "Language" and "Sync" covers the dimmed library, and `Esc` closes it

#### Scenario: Opening the settings on the Mac
- **WHEN** the reader chooses "Settings…" in the library's "⋯" menu, then presses `⌘W`
- **THEN** the "Settings" window opens and then closes; the library stays as it was

### Requirement: Language
The "Language" section SHALL offer "As the system" ("Как в системе"), "English" and "Русский" — the language names in
their own language, whatever the interface language — with the reader's choice selected, "As the system" when none.
Choosing one SHALL keep it in the state at once (`localization`). The section SHALL say "Forzac will speak the chosen
language after a restart." ("Форзац заговорит на выбранном языке после перезапуска.").

#### Scenario: Russian on an English Mac
- **WHEN** the Mac's preferred languages are English only, the reader chooses "Русский" and restarts Forzac
- **THEN** the menus and the library are in Russian, and the settings window shows "Русский"

#### Scenario: Back to the system's
- **WHEN** the reader chooses "As the system" and restarts Forzac on a system whose first language is English
- **THEN** the interface is in English

### Requirement: Sync
The "Sync" section SHALL show, in this order:
- the explanation: "Your place, bookmarks, highlights, shelves and book details are the same in Forzac on every
  computer and phone that shares the Books folder. Keep the folder in sync with Dropbox, iCloud Drive or Syncthing; on
  Android only Syncthing keeps a folder on the phone." ("Место чтения, закладки, цитаты, полки и правки книг — одни и
  те же в Форзаце на всех компьютерах и телефонах с общей папкой «Книги». Синхронизируйте папку через Dropbox, iCloud
  Drive или Syncthing; на Android папку на телефоне держит только Syncthing.");
- the Books folder's path, the home folder written `~`, with "Change…" ("Сменить…"), which runs "Change the Books
  folder…" (`books-folder`) and shows the new path when it is done;
- the state: "Syncing…" ("Синхронизация…") while a sync runs; else "Synced {when}" ("Синхронизировано {when}") after
  this run of Forzac has synced, or "Couldn't sync: {reason}" ("Не удалось синхронизировать: {reason}") when the last
  sync failed, or "Not synced yet" ("Ещё не синхронизировано");
- "Other devices" ("Другие устройства"): one line per `.sync` file in `.forzac/` other than this device's, the name
  from its `device` line ("Another device" / "Другое устройство" when empty) and "{when}" its file last changed, the
  latest first; "None yet" ("Пока нет") when there is none;
- "Sync Now" ("Синхронизировать сейчас"): the sync that runs when the app is left and come back to (`sync`), with
  open books moving to newer places the same way; the button is disabled while it runs, and the state and the devices
  are read again when it ends.

`{when}` SHALL be in the interface language and the local time zone with a 24-hour clock: "today at 10:14" ("сегодня в
10:14"), "yesterday at 22:10" ("вчера в 22:10"), "21 October at 10:14" ("21 октября в 10:14") in this year,
"21 October 2025 at 10:14" ("21 октября 2025 в 10:14") before it.

#### Scenario: The phone is seen
- **WHEN** the Books folder is kept by Syncthing and the phone synced yesterday at 22:10
- **THEN** "Other devices" lists the phone's name from its file with "yesterday at 22:10"

#### Scenario: Sync Now
- **WHEN** the reader presses "Sync Now"
- **THEN** the state reads "Syncing…" and the button is disabled, then the state reads "Synced today at" the time it
  ended

#### Scenario: A folder that cannot be written
- **WHEN** the Books folder's `.forzac/` cannot be written and the reader presses "Sync Now"
- **THEN** the state reads "Couldn't sync:" and the system's reason, and reading goes on as before

#### Scenario: Changing the folder from the settings
- **WHEN** the reader presses "Change…", chooses `~/Dropbox/Книги` and "Move"
- **THEN** the books move as with "Change the Books folder…", and the settings show `~/Dropbox/Книги`
