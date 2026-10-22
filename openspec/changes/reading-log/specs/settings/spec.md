## MODIFIED Requirements

### Requirement: Sync
The "Sync" section SHALL show, in this order:
- the explanation: "Your place, bookmarks, highlights, shelves, book details and reading stats are the same in Forzac
  on every computer and phone that shares the Books folder. Keep the folder in sync with Dropbox, iCloud Drive or
  Syncthing; on Android only Syncthing keeps a folder on the phone." ("Место чтения, закладки, цитаты, полки, правки
  книг и статистика — одни и те же в Форзаце на всех компьютерах и телефонах с общей папкой «Книги». Синхронизируйте
  папку через Dropbox, iCloud Drive или Syncthing; на Android папку на телефоне держит только Syncthing.");
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
