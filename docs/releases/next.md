Draft notes for the release after 0.9.0 (the version is decided when it is cut).

## What is new

- **The Books folder.** Forzac keeps its books in `~/Books` (`~/Книги` with a Russian interface). Opening a book from
  anywhere — Finder or the file manager, the file dialog, a drop on the library — copies it into the folder as
  `Author — Title.ext` and opens the copy; the original file stays where it was, and the same book is never copied
  twice. Books copied into the folder by hand, or by Dropbox and the like, show up in the library. If you already
  have a `~/Books` folder, Forzac uses it as it is: it renames and moves nothing there, ignores subfolders and other
  formats, and only adds its copies.
- **Your library moves in once.** The first start of this version copies every book you have read into the folder,
  with its place, bookmarks and highlights. The originals stay. On macOS this may ask once for access to the folder
  the books were read from (Downloads, Documents, Desktop); after that, books are read from the folder in your home
  folder and macOS does not ask again.
- **"Show the Books folder"** in the library (and `F`; on the Mac also in the File menu).
- **Update check.** The release archives and the Mac app look for a newer release on GitHub at most once a day, when
  the library opens, and offer it in one line there. The library's line "We don't store or send any data to a server"
  is gone, since this check asks GitHub.

## Changed

- **BREAKING: removing a book from the library moves its file to the Trash** (the question says so). In 0.9.0 the file
  stayed. Bookmarks and highlights are kept: put the file back from the Trash into the Books folder and they come
  back.
