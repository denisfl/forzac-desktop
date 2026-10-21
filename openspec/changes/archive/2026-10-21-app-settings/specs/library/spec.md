## MODIFIED Requirements

### Requirement: The library's menu
While no search is open — also when the library is empty — its head SHALL show a "⋯" button before the search button; a click
SHALL open a menu with "Show the Books folder", "Change the Books folder…" (`books-folder`), "Add a Folder…"
(`Opening books`) and, after them, "Settings…" (`settings`). On macOS the File menu SHALL also hold "Change the Books
folder…".

#### Scenario: Adding a folder with the mouse on Linux
- **WHEN** the reader clicks "⋯" in the library's head and chooses "Add a Folder…"
- **THEN** the folder dialog opens, and the chosen folder's books are added as `Ctrl+Shift+O` adds them

#### Scenario: Back from an empty folder
- **WHEN** the reader changed the Books folder to an empty one and chose "Leave them"
- **THEN** the empty library still shows "⋯", and "Change the Books folder…" there leads back

#### Scenario: The settings from an empty library
- **WHEN** the library is empty and the reader chooses "Settings…" in its "⋯" menu
- **THEN** the settings open
