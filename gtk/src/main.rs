//! Forzac for Linux: a GTK 4 shell over `forzac-core`.
//!
//! `forzac book.fb2` opens a window per book. GApplication keeps one process per user: opening another book
//! (`xdg-open`, double click) hands it to the running instance over D-Bus.
//!
//! Spike flags: `--measure` prints the time to the first presented page and quits; `--png out.png` saves the first
//! page as rendered by the window's renderer and quits; `--turns N` turns N pages with the animation and prints
//! frame-interval statistics; `--scrolls N` the same in the "Scroll" mode, a screen at a time; `--chapter N` opens
//! chapter N; `--page N` goes to page N of it
//! (for `--png` and `--turns`); `--marked` puts a bookmark at every paragraph and highlights every second one, so nearly
//! every page shows the ribbon and highlights; `--toggles N` sets and removes a bookmark N times (the ribbon drops and
//! rises), `--strokes N` highlights N paragraphs one after another (the marker's stroke), each printing the same
//! statistics; `--mouse "down X Y; drag X Y; up X Y; click X Y N"` sends those mouse events to the page's handlers after
//! the first frame, 0.15 s apart, and leaves the window open; `--show-help` opens the help (with `--png`: the whole
//! window is saved once it has faded in); `--size WxH` makes the window W × H instead of 760 × 980 (`1920x1080`: a
//! two-page spread).
//! With any of them the reading state is neither read nor written.
//!
//! Without a file, the library is shown (`library.rs`); `--measure` then prints the time to its first frame and to its
//! last cover, `--png out.png` saves it, both after the covers arrive, and quit. Reading positions and the settings are
//! remembered in `$XDG_DATA_HOME/forzac/state`, cover thumbnails in `$XDG_CACHE_HOME/forzac/covers`. Books are read
//! from the Books folder (`~/Books`, `~/Книги`): a book opened from elsewhere is copied there first (`books-folder`).

mod appearance;
mod fonts;
mod help;
mod icons;
mod library;
mod menu;
mod page_view;
mod panels;
mod settings_sheet;
mod stats;
mod viewer;

use std::cell::{Cell, RefCell};
use std::path::{Path, PathBuf};
use std::rc::Rc;
use std::time::{Duration, Instant};

use forzac_core::i18n::{fill, language, tr};
use forzac_core::log;
use forzac_core::settings::{DEFAULT_SIZE, Setting, Settings, Turning};
use forzac_core::state::Store;
use forzac_core::{EndLines, Locator};
use gtk::{gio, glib, graphene, prelude::*};
use page_view::{Mode, Open, PageView};

const APP_ID: &str = "app.truenrth.forzac";

#[derive(Clone)]
struct Options {
    start: Instant,
    measure: bool,
    png: Option<PathBuf>,
    turns: usize,
    scrolls: usize,
    toggles: usize,
    strokes: usize,
    mouse: Option<String>,
    chapter: Option<usize>,
    page: usize,
    size: Option<(i32, i32)>,
}

impl Options {
    /// Measurement runs neither read nor write the reading state.
    fn stateless(&self) -> bool {
        self.measure
            || self.png.is_some()
            || self.turns > 0
            || self.scrolls > 0
            || self.toggles > 0
            || self.strokes > 0
            || self.mouse.is_some()
            || self.chapter.is_some()
            || self.page > 0
            || self.size.is_some()
    }
}

type SharedStore = Rc<RefCell<Store>>;

/// One sync at a time (`sync`): the library's and the book windows'.
static SYNCING: std::sync::Mutex<()> = std::sync::Mutex::new(());

/// Syncs the state file `state` with its Books folder's sync folder, off the main thread; the books whose position it set.
fn sync_state(state: &Path) -> Vec<String> {
    let _one = SYNCING.lock().unwrap_or_else(|e| e.into_inner());
    let mut s = Store::open(state);
    let Some(folder) = forzac_core::library::remembered_folder(&s) else { return Vec::new() };
    forzac_core::sync::sync(&folder, &mut s).unwrap_or_else(|e| {
        eprintln!("cannot sync: {e}");
        Vec::new()
    })
}

/// The reading position is saved this long after the last turn.
const SAVE_DELAY: Duration = Duration::from_secs(1);
/// The screen stays on this long after the last key, turn, scroll or click in a book window.
const KEEP_AWAKE: Duration = Duration::from_secs(10 * 60);

thread_local! {
    /// The open book windows by the canonical path of their file: opening a book again brings its window forward.
    static WINDOWS: RefCell<Vec<(PathBuf, glib::WeakRef<gtk::ApplicationWindow>)>> = const { RefCell::new(Vec::new()) };
    /// Files being copied into the Books folder: opening one again meanwhile does nothing.
    static IMPORTING: RefCell<Vec<PathBuf>> = const { RefCell::new(Vec::new()) };
    /// The library is being read for its window.
    static SHOWING: Cell<bool> = const { Cell::new(false) };
}

/// A window's default size, no larger than 85% of the first monitor in either direction: a small screen (the Pi
/// Connect desktop is 1280 × 673) would otherwise cut the window off. Tiling compositors decide the size themselves.
fn fitted(width: i32, height: i32) -> (i32, i32) {
    let monitor = gtk::gdk::Display::default().and_then(|d| d.monitors().item(0)).and_downcast::<gtk::gdk::Monitor>();
    match monitor {
        Some(m) => (width.min(m.geometry().width() * 85 / 100), height.min(m.geometry().height() * 85 / 100)),
        None => (width, height),
    }
}

/// A window's size as it was last left (`WxH` under `key` in the state), else `default`; fitted to the monitor.
fn remembered_size(store: &Store, key: &str, default: (i32, i32)) -> (i32, i32) {
    let saved = store.setting(key).and_then(|s| s.split_once('x')).and_then(|(w, h)| Some((w.parse().ok()?, h.parse().ok()?)));
    let (width, height) = saved.filter(|&(w, h): &(i32, i32)| w > 0 && h > 0).unwrap_or(default);
    fitted(width, height)
}

/// Remembers the window's size under `key` when it closes. GTK keeps the default size at the window's size as last
/// resized, also while it is maximized or full screen.
fn remember_size(window: &gtk::ApplicationWindow, store: &SharedStore, key: &'static str) {
    let store = store.clone();
    window.connect_close_request(move |w| {
        let (width, height) = w.default_size();
        if width > 0 && height > 0 {
            let _ = store.borrow_mut().set_setting(key, &format!("{width}x{height}"));
        }
        glib::Propagation::Proceed
    });
}

/// A book window still open: closed windows may outlive their closing (references in closures), but they leave the
/// application.
fn open_book_window(w: &glib::WeakRef<gtk::ApplicationWindow>) -> Option<gtk::ApplicationWindow> {
    w.upgrade().filter(|w| w.application().is_some())
}

fn main() -> glib::ExitCode {
    let start = Instant::now();
    // The Wayland app_id comes from the program name; it must match the .desktop file name for the compositor,
    // docks and `StartupWMClass`-less matching to tie windows to Forzac.
    glib::set_prgname(Some(APP_ID));
    forzac_core::fonts::set_assets(fonts::assets());
    fonts::install_fallback_provider();
    let args: Vec<String> = std::env::args().collect();
    let png = args.iter().position(|a| a == "--png").and_then(|i| args.get(i + 1)).map(PathBuf::from);
    let turns = args.iter().position(|a| a == "--turns").and_then(|i| args.get(i + 1)).and_then(|n| n.parse().ok()).unwrap_or(0);
    let chapter = args.iter().position(|a| a == "--chapter").and_then(|i| args.get(i + 1)).and_then(|n| n.parse().ok());
    let scrolls = args.iter().position(|a| a == "--scrolls").and_then(|i| args.get(i + 1)).and_then(|n| n.parse().ok()).unwrap_or(0);
    let page = args.iter().position(|a| a == "--page").and_then(|i| args.get(i + 1)).and_then(|n| n.parse().ok()).unwrap_or(0);
    let toggles = args.iter().position(|a| a == "--toggles").and_then(|i| args.get(i + 1)).and_then(|n| n.parse().ok()).unwrap_or(0);
    let strokes = args.iter().position(|a| a == "--strokes").and_then(|i| args.get(i + 1)).and_then(|n| n.parse().ok()).unwrap_or(0);
    let mouse = args.iter().position(|a| a == "--mouse").and_then(|i| args.get(i + 1)).cloned();
    let size_arg = args.iter().position(|a| a == "--size").and_then(|i| args.get(i + 1));
    let stats_dir = args.iter().position(|a| a == "--stats").and_then(|i| args.get(i + 1)).map(PathBuf::from);
    let size = size_arg.and_then(|s| s.split_once('x')).and_then(|(w, h)| Some((w.parse().ok()?, h.parse().ok()?)));
    let options = Options {
        start,
        measure: args.iter().any(|a| a == "--measure"),
        png,
        turns,
        scrolls,
        toggles,
        strokes,
        mouse,
        chapter,
        page,
        size,
    };
    let files: Vec<&String> = args
        .iter()
        .skip(1)
        .filter(|a| {
            !a.starts_with("--")
                && a.parse::<usize>().is_err()
                && Some(PathBuf::from(a.as_str())) != options.png
                && Some(*a) != options.mouse.as_ref()
                && Some(*a) != size_arg
                && Some(PathBuf::from(a.as_str())) != stats_dir
        })
        .collect();

    let mut flags = gio::ApplicationFlags::HANDLES_OPEN;
    if options.measure
        || options.png.is_some()
        || options.turns > 0
        || options.scrolls > 0
        || options.toggles > 0
        || options.strokes > 0
        || stats_dir.is_some()
    {
        flags |= gio::ApplicationFlags::NON_UNIQUE;
    }
    let app = gtk::Application::builder().application_id(APP_ID).flags(flags).build();
    // Under X11 the window icon comes from the theme by this name; on Wayland the compositor finds it by app id.
    app.connect_startup(|_| gtk::Window::set_default_icon_name(APP_ID));
    let store: SharedStore = Rc::new(RefCell::new(Store::open(glib::user_data_dir().join("forzac").join("state"))));
    // The interface language: the reader's choice in the settings, else the environment as GLib reads it (`LANGUAGE`,
    // `LC_ALL`, `LC_MESSAGES`, `LANG`).
    let languages = glib::language_names();
    let setting = store.borrow().setting(forzac_core::i18n::SETTING).map(str::to_owned);
    forzac_core::i18n::set_language(forzac_core::i18n::Language::choose(setting.as_deref(), languages.iter().map(|l| l.as_str())));
    forzac_core::sync::set_device_name(&glib::host_name());
    let (opts, st) = (options.clone(), store.clone());
    app.connect_open(move |app, files, _| {
        for file in files {
            if let Some(path) = file.path() {
                open_book(app, &path, &opts, &st, |_| {});
            }
        }
    });
    // Quitting gives this computer's reading away (`sync`).
    if !options.stateless() {
        let state = store.borrow().path().to_owned();
        app.connect_shutdown(move |_| drop(sync_state(&state)));
    }
    // After the last book window, the library (as on macOS); closing it quits. Shown while the window is being removed,
    // so the application never runs out of windows.
    if !options.stateless() {
        let (opts, st) = (options.clone(), store.clone());
        app.connect_window_removed(move |app, window| {
            if app.windows().is_empty() && !window.has_css_class("forzac-library") {
                library_window(app, &opts, &st, |_| {});
            }
        });
    }
    let opts = options.clone();
    app.connect_activate(move |app| {
        if let Some(dir) = &stats_dir {
            stats::save_pngs(&store, dir);
            return;
        }
        let o = opts.clone();
        library_window(app, &opts, &store, move |library| {
            if !o.measure && o.png.is_none() {
                return;
            }
            // Measurements: the library's first frame and its last cover, then quit.
            let (opts, weak) = (o.clone(), Rc::downgrade(&library));
            library.connect_settled(move || {
                let Some(library) = weak.upgrade() else { return };
                if opts.measure {
                    let first = library.first_frame().map_or(Duration::ZERO, |t| t.duration_since(opts.start));
                    println!(
                        "library: {} books, first frame presented {first:?}, covers loaded {:?}",
                        library.book_count(),
                        opts.start.elapsed()
                    );
                }
                if let Some(png) = &opts.png {
                    library.save_png(png);
                }
                library.window.close();
            });
        });
    });
    let mut run_args = vec![args[0].clone()];
    run_args.extend(files.into_iter().cloned());
    app.run_with_args(&run_args)
}

/// Why a file cannot be read, in the Android app's words (without PDF).
fn reason(e: forzac_core::Error) -> &'static str {
    match e {
        forzac_core::Error::NotABook => tr("unsupported format — FB2, FB3, EPUB, TXT or Markdown needed"),
        forzac_core::Error::Damaged => tr("the file is damaged"),
        forzac_core::Error::Protected => tr("the book is DRM-protected and can't be opened"),
        forzac_core::Error::NoText => tr("the book has no text"),
        forzac_core::Error::Unavailable => tr("the file is unavailable"),
    }
}

/// Says why `path` cannot be opened; then, when no book is open, the library follows.
fn cannot_open(app: &gtk::Application, path: &Path, e: forzac_core::Error, options: &Options, store: &SharedStore) {
    let name = path.file_name().map_or_else(|| path.display().to_string(), |n| n.to_string_lossy().into_owned());
    let message = fill(tr("Couldn't open “{name}”: {reason}"), &[("name", &name), ("reason", reason(e))]);
    eprintln!("{message}");
    if options.stateless() {
        return;
    }
    // The dialog is not a window of the application: hold it, or it would quit before the message shows.
    let hold = app.hold();
    let parent = app.active_window();
    let (app, options, store) = (app.clone(), options.clone(), store.clone());
    gtk::AlertDialog::builder().message(message).buttons([tr("Close")]).cancel_button(0).default_button(0).modal(true).build().choose(
        parent.as_ref(),
        gio::Cancellable::NONE,
        move |_| {
            if !has_book_window() {
                library_window(&app, &options, &store, |_| {});
            }
            drop(hold);
        },
    );
}

/// Whether a book window is open.
fn has_book_window() -> bool {
    WINDOWS.with(|w| w.borrow().iter().any(|(_, w)| open_book_window(w).is_some()))
}

/// Opens the book at `path`, wherever it is: copied into the Books folder first, off the main thread (`books-folder`),
/// then in a window of its own; `done` learns whether it opened. Measurement runs open the file where it is.
fn open_book(app: &gtk::Application, path: &Path, options: &Options, store: &SharedStore, done: impl FnOnce(bool) + 'static) {
    if options.stateless() {
        done(open_window(app, path, options, store));
        return;
    }
    if IMPORTING.with(|i| i.borrow().iter().any(|p| p == path)) {
        return;
    }
    IMPORTING.with(|i| i.borrow_mut().push(path.to_owned()));
    // No window may be open meanwhile: hold the application.
    let hold = app.hold();
    let (app, options, store, path) = (app.clone(), options.clone(), store.clone(), path.to_owned());
    let (state, home, src) = (store.borrow().path().to_owned(), glib::home_dir(), path.clone());
    glib::spawn_future_local(async move {
        let copied = gio::spawn_blocking(move || {
            let mut s = Store::open(state);
            let folder = forzac_core::library::books_folder(&home, language(), &mut s).map_err(|_| forzac_core::Error::Unavailable)?;
            forzac_core::library::import(&src, &folder, &mut s)
        })
        .await
        .unwrap_or(Err(forzac_core::Error::Unavailable));
        IMPORTING.with(|i| i.borrow_mut().retain(|p| *p != path));
        store.borrow_mut().reload();
        let opened = match copied {
            Ok(copy) => open_window(&app, &copy, &options, &store),
            Err(e) => {
                cannot_open(&app, &path, e, &options, &store);
                false
            }
        };
        done(opened);
        drop(hold);
    });
}

/// Opens `path` in a window of its own (or brings its open window forward); false when it cannot be read.
fn open_window(app: &gtk::Application, path: &Path, options: &Options, store: &SharedStore) -> bool {
    let canonical = path.canonicalize().unwrap_or_else(|_| path.to_owned());
    let open = WINDOWS.with(|w| {
        let mut w = w.borrow_mut();
        w.retain(|(_, win)| open_book_window(win).is_some());
        w.iter().find(|(p, _)| *p == canonical).and_then(|(_, win)| open_book_window(win))
    });
    if let Some(window) = open {
        window.present();
        return true;
    }
    let book = match forzac_core::open(path) {
        Ok(book) => book,
        Err(e) => {
            cannot_open(app, path, e, options, store);
            return false;
        }
    };
    let parsed = options.start.elapsed();
    let title = if book.title.is_empty() { tr("Forzac").to_owned() } else { fill(tr("{title} — Forzac"), &[("title", &book.title)]) };
    let stateless = options.stateless();
    let (at, settings) = if stateless {
        // Measurements: the defaults in Light, whatever the desktop.
        let turning = if options.scrolls > 0 { Turning::Scroll } else { Turning::Slide };
        (
            Locator { chapter: options.chapter.unwrap_or(0), ..Locator::default() },
            Settings { theme: Some(0), turning, ..Settings::default() },
        )
    } else {
        let s = store.borrow();
        (s.position(&book, path).unwrap_or_default(), Settings::load(&s))
    };
    let view = PageView::new(book, at, settings, appearance::dark());
    view.set_mode(if settings.turning == Turning::Scroll { Mode::Scroll } else { Mode::Slide });

    let help = help::Help::new();
    let overlay = gtk::Overlay::new();
    overlay.set_child(Some(&view));
    // The help over everything, the panels (added to `overlay` later) and the menu included.
    let outer = gtk::Overlay::new();
    outer.set_child(Some(&overlay));
    outer.add_overlay(&help.root);

    let (width, height) = match options.size {
        Some((w, h)) => (w, h),
        None if stateless => fitted(760, 980),
        None => remembered_size(&store.borrow(), "window-book", (760, 980)),
    };
    let window =
        gtk::ApplicationWindow::builder().application(app).title(&title).default_width(width).default_height(height).child(&outer).build();
    // While the help is open it takes every key, before the panels, the menu and the page.
    let keys = gtk::EventControllerKey::new();
    keys.set_propagation_phase(gtk::PropagationPhase::Capture);
    let h = help.clone();
    keys.connect_key_pressed(move |_, key, code, state| {
        if h.key(key, page_view::latin(key, code), state) { glib::Propagation::Stop } else { glib::Propagation::Proceed }
    });
    window.add_controller(keys);
    // Tiling compositors draw no title bars; a client-side one would only take space.
    if std::env::var_os("HYPRLAND_INSTANCE_SIGNATURE").is_some() {
        window.set_decorated(false);
    }
    panels::set_theme(view.theme());
    let save_style: Rc<dyn Fn()> = if stateless {
        let view = view.clone();
        Rc::new(move || panels::set_theme(view.theme()))
    } else {
        let (view, store) = (view.clone(), store.clone());
        Rc::new(move || {
            panels::set_theme(view.theme());
            if let Err(e) = view.settings().save(&mut store.borrow_mut()) {
                eprintln!("cannot save the reading state: {e}");
            }
        })
    };
    // The image or table view open over the page, if any.
    let viewer: Rc<RefCell<Option<glib::WeakRef<viewer::Canvas>>>> = Rc::default();
    if !stateless {
        let (v, open_viewer) = (view.downgrade(), viewer.clone());
        let watch = std::cell::Cell::new(appearance::watch(move |dark| {
            if let Some(v) = v.upgrade() {
                v.set_system_dark(dark);
                panels::set_theme(v.theme());
            }
            if let Some(open) = open_viewer.borrow().as_ref().and_then(|w| w.upgrade()) {
                open.queue_draw();
            }
        }));
        window.connect_destroy(move |_| drop(watch.take()));
    }
    let settings_panel: Rc<RefCell<Option<Rc<settings_sheet::SettingsSheet>>>> = Rc::default();
    // The contents panel is built in the first pause in reading (a second without turning), so `o` only shows it and
    // the building never delays a page turn.
    let contents: Rc<RefCell<Option<Rc<panels::Contents>>>> = Rc::default();
    let search: Rc<RefCell<Option<Rc<panels::SearchPanel>>>> = Rc::default();
    let note: Rc<RefCell<Option<gtk::Box>>> = Rc::default();
    let menu = menu::Menu::new(&overlay, &view);
    // Closes whatever panel (or the menu) is over the page; false when none was.
    let close_panels: Rc<dyn Fn() -> bool> = {
        let (settings_panel, contents, search, note, overlay, view, menu) =
            (settings_panel.clone(), contents.clone(), search.clone(), note.clone(), overlay.clone(), view.clone(), menu.clone());
        Rc::new(move || {
            let mut closed = menu.close();
            if let Some(p) = settings_panel.borrow().as_ref().filter(|p| p.is_open()) {
                p.close(&view);
                closed = true;
            }
            if let Some(p) = contents.borrow().as_ref().filter(|p| p.is_open()) {
                p.close(&view);
                closed = true;
            }
            if let Some(p) = search.borrow().as_ref().filter(|p| p.is_open()) {
                p.close(&view);
                closed = true;
            }
            if let Some(n) = note.borrow_mut().take().filter(|n| n.parent().is_some()) {
                overlay.remove_overlay(&n);
                view.grab_focus();
                closed = true;
            }
            closed
        })
    };
    // `s`, `o` and `/` open their panel (closing the others) or close it when it is open.
    let open_settings: Rc<dyn Fn()> = {
        let (panel, overlay, view, save, close) =
            (settings_panel.clone(), overlay.clone(), view.clone(), save_style.clone(), close_panels.clone());
        Rc::new(move || {
            let panel = panel.borrow_mut().get_or_insert_with(|| settings_sheet::SettingsSheet::new(&overlay, &view, save.clone())).clone();
            view.dismiss();
            let was_open = panel.is_open();
            close();
            if !was_open {
                panel.open(&view);
            }
        })
    };
    let open_search: Rc<dyn Fn()> = {
        let (search, overlay, view, close) = (search.clone(), overlay.clone(), view.clone(), close_panels.clone());
        Rc::new(move || {
            let s = search.borrow_mut().get_or_insert_with(|| panels::SearchPanel::new(&overlay, &view)).clone();
            view.dismiss();
            let was_open = s.is_open();
            close();
            if !was_open {
                s.open(&view);
            }
        })
    };
    // `o` and `B` open the contents panel on their tab, switch to it, or close the panel when it shows it.
    let open_contents: Rc<dyn Fn(panels::Tab)> = {
        let (contents, overlay, view, close) = (contents.clone(), overlay.clone(), view.clone(), close_panels.clone());
        Rc::new(move |tab| {
            let c = contents.borrow_mut().get_or_insert_with(|| panels::Contents::new(&overlay, &view)).clone();
            view.dismiss();
            let was_open = c.is_open();
            if was_open && c.tab() != tab {
                c.show(&view, tab);
                return;
            }
            close();
            if !was_open {
                c.open(&view, tab);
            }
        })
    };
    // A click on the page while a panel or the menu is over it closes it, and only that; the second click of a double
    // click that opened the menu still selects a word.
    let click = gtk::GestureClick::new();
    click.set_propagation_phase(gtk::PropagationPhase::Capture);
    let (c, m) = (close_panels.clone(), menu.clone());
    click.connect_pressed(move |g, n, _, _| {
        let menu_closed = m.close();
        if c() || (menu_closed && n == 1) {
            g.set_state(gtk::EventSequenceState::Claimed);
        }
    });
    view.add_controller(click);
    // The menu's keys come before the page's while it is open.
    let keys = gtk::EventControllerKey::new();
    keys.set_propagation_phase(gtk::PropagationPhase::Capture);
    let m = menu.clone();
    keys.connect_key_pressed(
        move |_, key, code, _| if m.key(page_view::latin(key, code)) { glib::Propagation::Stop } else { glib::Propagation::Proceed },
    );
    view.add_controller(keys);
    let m = menu.clone();
    view.connect_middle(move || m.open());
    {
        let (window, app, options, store, contents, settings) =
            (window.downgrade(), app.downgrade(), options.clone(), store.clone(), open_contents.clone(), open_settings.clone());
        menu.set_actions(menu::Actions {
            // The window closes (saving the position), and the library shows or comes forward.
            back: Box::new(move || {
                if let Some(w) = window.upgrade() {
                    w.close();
                }
                if let Some(app) = app.upgrade().filter(|_| !stateless) {
                    library_window(&app, &options, &store, |_| {});
                }
            }),
            contents: Box::new(move || contents(panels::Tab::Contents)),
            settings: Box::new(move || settings()),
        });
    }
    if stateless {
        // Measurements: when the book's pages were counted.
        let v = view.downgrade();
        let start = options.start;
        view.connect_pages(move || {
            let pages = v.upgrade().and_then(|v| v.book_pages()).map_or(0, |p| p.total());
            println!("pages: {pages} counted {:?} after start", start.elapsed());
        });
    }
    glib::timeout_add_local(
        Duration::from_millis(500),
        glib::clone!(
            #[weak]
            overlay,
            #[weak]
            view,
            #[upgrade_or]
            glib::ControlFlow::Break,
            move || {
                if contents.borrow().is_some() {
                    return glib::ControlFlow::Break;
                }
                if view.quiet_for() < Duration::from_secs(1) {
                    return glib::ControlFlow::Continue;
                }
                contents.borrow_mut().replace(panels::Contents::new(&overlay, &view));
                glib::ControlFlow::Break
            }
        ),
    );
    view.add_controller(shortcuts(
        &view,
        &help,
        PanelKeys { contents: open_contents, settings: open_settings, search: open_search, close: close_panels, menu: menu.clone() },
        &window,
        save_style,
    ));
    let stats_store = store.clone();
    view.connect_open(glib::clone!(
        #[weak]
        overlay,
        #[weak]
        window,
        #[weak]
        view,
        move |open| match open {
            Open::Note(id) => *note.borrow_mut() = panels::note(&overlay, &view, &id),
            Open::Preview(key) => *note.borrow_mut() = panels::preview(&overlay, &view, &key),
            Open::External(url) => gtk::UriLauncher::new(&url).launch(Some(&window), gio::Cancellable::NONE, |_| {}),
            Open::Image(id) => *viewer.borrow_mut() = viewer::open(&overlay, &view, viewer::Subject::Image(id)).map(|v| v.downgrade()),
            Open::Table(at) => *viewer.borrow_mut() = viewer::open(&overlay, &view, viewer::Subject::Table(at)).map(|v| v.downgrade()),
            Open::Stats => stats::open(&window, &stats_store),
        }
    ));
    // For screenshots: `wtype` cannot type `?`.
    if stateless && std::env::args().any(|a| a == "--show-help") {
        help.show();
    }
    if stateless && std::env::args().any(|a| a == "--marked") {
        let book = view.book();
        let starts = book
            .chapters
            .iter()
            .enumerate()
            .flat_map(|(c, ch)| (0..ch.blocks.len()).map(move |b| Locator { chapter: c, block: b, offset: 0 }))
            .collect::<Vec<_>>();
        view.set_highlights(starts.iter().step_by(2).map(|&at| book.block_range(at)).filter(|(a, b)| a < b).collect());
        view.set_bookmarks(starts);
    }
    if !stateless {
        // The marks as this window last loaded or saved them: a save changes only what differs (a sync or another window
        // may have changed the state meanwhile), and the view then shows them all.
        let saved_quotes = Rc::new(RefCell::new(store.borrow().quotes(&view.book())));
        let saved_marks = Rc::new(RefCell::new(store.borrow().bookmarks(&view.book())));
        view.set_highlights(saved_quotes.borrow().clone());
        view.set_bookmarks(saved_marks.borrow().clone());
        let (st, book, saved, v) = (store.clone(), view.book(), saved_quotes.clone(), view.downgrade());
        view.connect_highlights(move |quotes| {
            let base = saved.borrow().clone();
            match st.borrow_mut().change_quotes_of(&book.id, &base, quotes) {
                Ok(held) => {
                    if held != quotes
                        && let Some(v) = v.upgrade()
                    {
                        v.set_highlights(held.clone());
                    }
                    *saved.borrow_mut() = held;
                }
                Err(e) => eprintln!("cannot save the reading state: {e}"),
            }
        });
        let (st, book, saved, v) = (store.clone(), view.book(), saved_marks.clone(), view.downgrade());
        view.connect_bookmarks(move |marks| {
            let base = saved.borrow().clone();
            match st.borrow_mut().change_bookmarks_of(&book.id, &base, marks) {
                Ok(held) => {
                    if held != marks
                        && let Some(v) = v.upgrade()
                    {
                        v.set_bookmarks(held.clone());
                    }
                    *saved.borrow_mut() = held;
                }
                Err(e) => eprintln!("cannot save the reading state: {e}"),
            }
        });
        // After a sync: the marks from the state again.
        let refresh: Rc<dyn Fn()> = {
            let (st, v) = (store.clone(), view.downgrade());
            Rc::new(move || {
                let Some(v) = v.upgrade() else { return };
                let book = v.book();
                let (marks, quotes) = (st.borrow().bookmarks(&book), st.borrow().quotes(&book));
                *saved_marks.borrow_mut() = marks.clone();
                *saved_quotes.borrow_mut() = quotes.clone();
                v.set_bookmarks(marks);
                v.set_highlights(quotes);
            })
        };
        remember_position(&view, &window, path, store, refresh);
        remember_size(&window, store, "window-book");
        library::cache_cover(&view.book());
        keep_awake(app, &window);
        WINDOWS.with(|w| w.borrow_mut().push((canonical, window.downgrade())));
    }

    if let Some(events) = options.mouse.clone() {
        let menu = menu.clone();
        view.connect_first_frame(move |view| {
            for (i, e) in events.split(';').map(|e| e.split_whitespace().collect::<Vec<_>>()).filter(|e| e.len() >= 3).enumerate() {
                let (view, kind, n) = (view.clone(), e[0].to_owned(), e.get(3).and_then(|n| n.parse().ok()).unwrap_or(1));
                let (x, y) = (e[1].parse().unwrap_or(0.0), e[2].parse().unwrap_or(0.0));
                let menu = menu.clone();
                glib::timeout_add_local_once(Duration::from_millis(500 + 150 * i as u64), move || {
                    // A press with the menu open closes it, as the window's click does.
                    if menu.is_open() && kind != "drag" && kind != "up" && n == 1 {
                        menu.close();
                    } else {
                        view.test_mouse(&kind, x, y, n)
                    }
                });
            }
        });
    }
    if options.measure || options.png.is_some() || options.turns > 0 || options.scrolls > 0 || options.toggles > 0 || options.strokes > 0 {
        let options = options.clone();
        let window = window.clone();
        view.connect_first_frame(move |view| {
            if options.measure {
                println!(
                    "startup: parsed {parsed:?}, first page presented {:?}, pages in chapter {}",
                    options.start.elapsed(),
                    view.page_count()
                );
            }
            if options.page > 0 {
                view.go_to_page(options.page);
            }
            if let Some(png) = options.png.clone().filter(|_| std::env::args().any(|a| a == "--show-help")) {
                // The whole window once the help has faded in.
                let window = window.clone();
                glib::timeout_add_local_once(Duration::from_millis(400), move || {
                    save_window_png(&window, &png);
                    window.close();
                });
                return;
            }
            if let Some(png) = &options.png {
                save_png(view, png);
            }
            if options.turns == 0 && options.scrolls == 0 && options.toggles == 0 && options.strokes == 0 {
                window.close();
                return;
            }
            // One turn (or scroll by a screen) every 400 ms (the animation takes 200 or 150), then the statistics; the
            // first 1.2 s after the first frame, as a reader would, when the neighbouring chapters are laid out.
            let left = std::cell::Cell::new(options.turns.max(options.scrolls).max(options.toggles).max(options.strokes));
            let (toggles, strokes) = (options.toggles > 0, options.strokes > 0);
            let (view, window) = (view.clone(), window.clone());
            let wait = std::cell::Cell::new(2); // ticks before the first turn
            // A drop takes 0.6 s, a stroke at most 0.6 s: every 0.8 s.
            glib::timeout_add_local(std::time::Duration::from_millis(if toggles || strokes { 800 } else { 400 }), move || {
                if wait.get() > 0 {
                    wait.set(wait.get() - 1);
                    return glib::ControlFlow::Continue;
                }
                if left.get() == 0 {
                    report_turns(&view);
                    window.close();
                    return glib::ControlFlow::Break;
                }
                left.set(left.get() - 1);
                if toggles {
                    view.toggle_bookmark();
                } else if strokes {
                    view.highlight_next();
                } else {
                    view.turn(1);
                }
                glib::ControlFlow::Continue
            });
        });
    }
    window.present();
    view.grab_focus();
    true
}

/// Keeps the screen from blanking while the reader reads in `window`: for `KEEP_AWAKE` after the last key, turn,
/// scroll or click, and only while the window is active.
fn keep_awake(app: &gtk::Application, window: &gtk::ApplicationWindow) {
    let cookie: Rc<Cell<Option<u32>>> = Rc::default();
    let timer: Rc<RefCell<Option<glib::SourceId>>> = Rc::default();
    let release: Rc<dyn Fn()> = {
        let (app, cookie) = (app.downgrade(), cookie.clone());
        Rc::new(move || {
            if let (Some(app), Some(c)) = (app.upgrade(), cookie.take()) {
                app.uninhibit(c);
            }
        })
    };
    let touch: Rc<dyn Fn()> = {
        let (app, window, timer, release) = (app.downgrade(), window.downgrade(), timer.clone(), release.clone());
        Rc::new(move || {
            let (Some(app), Some(window)) = (app.upgrade(), window.upgrade()) else { return };
            if !window.is_active() {
                return;
            }
            if cookie.get().is_none() {
                cookie.set(Some(app.inhibit(Some(&window), gtk::ApplicationInhibitFlags::IDLE, Some(tr("Reading a book")))));
            }
            if let Some(id) = timer.borrow_mut().take() {
                id.remove();
            }
            let (t, release) = (timer.clone(), release.clone());
            *timer.borrow_mut() = Some(glib::timeout_add_local_once(KEEP_AWAKE, move || {
                t.borrow_mut().take(); // fired: the id is no longer valid
                release();
            }));
        })
    };
    let input = gtk::EventControllerLegacy::new();
    input.set_propagation_phase(gtk::PropagationPhase::Capture);
    let t = touch.clone();
    input.connect_event(move |_, event| {
        use gtk::gdk::EventType;
        if matches!(
            event.event_type(),
            EventType::KeyPress | EventType::ButtonPress | EventType::Scroll | EventType::TouchpadSwipe | EventType::TouchBegin
        ) {
            t();
        }
        glib::Propagation::Proceed
    });
    window.add_controller(input);
    let (t, r) = (touch.clone(), release.clone());
    window.connect_is_active_notify(move |w| if w.is_active() { t() } else { r() });
    window.connect_close_request(move |_| {
        if let Some(id) = timer.borrow_mut().take() {
            id.remove();
        }
        release();
        glib::Propagation::Proceed
    });
    touch(); // opening a book is reading
}

fn report_turns(view: &PageView) {
    let (refresh, mut intervals) = view.turn_stats();
    if intervals.is_empty() {
        println!("turns: no animation frames recorded");
        return;
    }
    intervals.sort_unstable();
    // GTK may report a default refresh interval (16.67 ms) on outputs that run faster; the median frame interval of a
    // smooth animation is the real vsync period, so take whichever is shorter.
    let median = intervals[intervals.len() / 2];
    let budget = if refresh > 0 { refresh.min(median) } else { median };
    // A frame later than 1.5 refresh intervals missed at least one vsync.
    let missed = intervals.iter().filter(|&&i| i as f64 > budget as f64 * 1.5).count();
    let pct = |p: f64| intervals[((intervals.len() - 1) as f64 * p) as usize] as f64 / 1000.0;
    // Where in a turn the misses are: (frame index within the turn, interval ms).
    let late: Vec<String> = view
        .turn_frames()
        .iter()
        .flat_map(|t| t.windows(2).enumerate().map(|(i, w)| (i + 1, w[1] - w[0])).collect::<Vec<_>>())
        .filter(|&(_, d)| d as f64 > budget as f64 * 1.5)
        .map(|(i, d)| format!("#{i}:{:.1}ms", d as f64 / 1000.0))
        .collect();
    if !late.is_empty() {
        println!("late frames (index in turn): {}", late.join(" "));
    }
    println!(
        "turns: refresh {:.2} ms ({:.0} Hz), {} frames, missed {}, interval p50 {:.2} ms, p95 {:.2} ms, max {:.2} ms",
        budget as f64 / 1000.0,
        1e6 / budget as f64,
        intervals.len(),
        missed,
        pct(0.5),
        pct(0.95),
        pct(1.0)
    );
}

fn save_png(view: &PageView, path: &std::path::Path) {
    let (Some(node), Some(renderer)) = (view.current_page_node(), view.native().and_then(|n| n.renderer())) else { return };
    let scale = view.scale_factor() as f32;
    let rect = graphene::Rect::new(0.0, 0.0, view.width() as f32 * scale, view.height() as f32 * scale);
    // Render at device scale, like the window does.
    let transform = gtk::gsk::Transform::new().scale(scale, scale);
    let scaled = gtk::gsk::TransformNode::new(&node, Some(&transform));
    let texture = renderer.render_texture(&scaled, Some(&rect));
    match texture.save_to_png(path) {
        Ok(()) => println!("saved {} ({}×{}, {})", path.display(), texture.width(), texture.height(), renderer.type_().name()),
        Err(e) => eprintln!("{}: {e}", path.display()),
    }
}

/// The window's content as it is drawn (`--show-help --png`).
fn save_window_png(window: &gtk::ApplicationWindow, path: &std::path::Path) {
    let Some(child) = window.child() else { return };
    let snapshot = gtk::Snapshot::new();
    window.snapshot_child(&child, &snapshot);
    let (Some(node), Some(renderer)) = (snapshot.to_node(), window.renderer()) else { return };
    let scale = window.scale_factor() as f32;
    let rect = graphene::Rect::new(0.0, 0.0, window.width() as f32 * scale, window.height() as f32 * scale);
    let scaled = gtk::gsk::TransformNode::new(&node, Some(&gtk::gsk::Transform::new().scale(scale, scale)));
    match renderer.render_texture(&scaled, Some(&rect)).save_to_png(path) {
        Ok(()) => println!("saved {}", path.display()),
        Err(e) => eprintln!("{}: {e}", path.display()),
    }
}

/// Saves the reading position when the book opens, `SAVE_DELAY` after the last move and when the window closes; with
/// it, the reading log (`reading-log`): the reader's keys, clicks, scrolls, moves and jumps are counted, and appended
/// at most once a minute with the delayed save and always when the window is left or closed.
fn remember_position(view: &PageView, window: &gtk::ApplicationWindow, path: &Path, store: &SharedStore, refresh: Rc<dyn Fn()>) {
    let reading = Rc::new(RefCell::new(log::Reading::new(now_ms())));
    let event: Rc<dyn Fn(log::Event)> = {
        let (view, reading) = (view.downgrade(), reading.clone());
        Rc::new(move |e| {
            if let Some(view) = view.upgrade() {
                reading.borrow_mut().event(&view.book(), e, view.locator(), now_ms(), utc_offset());
            }
        })
    };
    // The reading appended: everything when the window is `leaving`, else once a minute.
    let append: Rc<dyn Fn(bool)> = {
        let (view, store, reading) = (view.downgrade(), store.clone(), reading.clone());
        Rc::new(move |leaving| {
            let Some(view) = view.upgrade() else { return };
            if leaving {
                reading.borrow_mut().event(&view.book(), log::Event::Paused, view.locator(), now_ms(), utc_offset());
            }
            let sums = reading.borrow_mut().due(now_ms(), leaving);
            if let Err(e) = log::append(&mut store.borrow_mut(), &view.book().id, &sums, now_ms()) {
                eprintln!("cannot write the reading log: {e}");
            }
        })
    };
    let save = {
        let (view, store, path, append) = (view.downgrade(), store.clone(), path.to_owned(), append.clone());
        Rc::new(move || {
            let Some(view) = view.upgrade() else { return };
            if let Err(e) = store.borrow_mut().save_position(&view.book(), &path, view.locator(), view.finished()) {
                eprintln!("cannot save the reading state: {e}");
            }
            if view.finished()
                && let Err(e) = log::finish(&store.borrow(), &view.book(), now_ms())
            {
                eprintln!("cannot write the reading log: {e}");
            }
            append(false);
        })
    };
    let pending: Rc<RefCell<Option<glib::SourceId>>> = Rc::default();
    let (p, s, ev, v) = (pending.clone(), save.clone(), event.clone(), view.downgrade());
    view.connect_moved(move || {
        if let Some(view) = v.upgrade() {
            ev(if view.jumped() { log::Event::Jumped } else { log::Event::Moved });
        }
        if let Some(id) = p.borrow_mut().take() {
            id.remove();
        }
        let (p2, s2) = (p.clone(), s.clone());
        *p.borrow_mut() = Some(glib::timeout_add_local_once(SAVE_DELAY, move || {
            p2.borrow_mut().take(); // fired: the id is no longer valid
            s2();
        }));
    });
    // Every key, click and scroll in the window is the reader's action.
    let keys = gtk::EventControllerKey::new();
    keys.set_propagation_phase(gtk::PropagationPhase::Capture);
    let ev = event.clone();
    keys.connect_key_pressed(move |_, _, _, _| {
        ev(log::Event::Active);
        glib::Propagation::Proceed
    });
    window.add_controller(keys);
    let click = gtk::GestureClick::new();
    click.set_propagation_phase(gtk::PropagationPhase::Capture);
    let ev = event.clone();
    click.connect_pressed(move |_, _, _, _| ev(log::Event::Active));
    window.add_controller(click);
    let wheel = gtk::EventControllerScroll::new(gtk::EventControllerScrollFlags::BOTH_AXES);
    wheel.set_propagation_phase(gtk::PropagationPhase::Capture);
    let ev = event.clone();
    wheel.connect_scroll(move |_, _, _| {
        ev(log::Event::Active);
        glib::Propagation::Proceed
    });
    window.add_controller(wheel);
    let waiting = pending.clone();
    let (s2, a) = (save.clone(), append.clone());
    window.connect_close_request(move |_| {
        if let Some(id) = pending.borrow_mut().take() {
            id.remove();
        }
        s2();
        a(true);
        glib::Propagation::Proceed
    });
    // The end page's totals from the log (`reading-stats`).
    let (st, v) = (store.clone(), view.downgrade());
    view.connect_end_lines(move || {
        let Some(view) = v.upgrade() else { return EndLines::default() };
        let (book, store, now) = (view.book(), st.borrow(), now_ms());
        let log = log::load(&store).unwrap_or_default().as_if_finished(&book, now);
        EndLines {
            book: forzac_core::stats::book_line(
                &log,
                &book.id,
                store.quotes_of(&book.id).len(),
                store.bookmarks_of(&book.id).len(),
                now,
                utc_offset(),
            ),
            month: forzac_core::stats::month_line(&log, now, utc_offset()),
        }
    });
    // Leaving the book window gives the reading away, coming back takes the other devices' (`sync`); then the marks come
    // from the state again, and a place that came from another device moves the book there (keeping the place left for
    // going back) unless the book was moved since the sync began. Only an unsaved move is saved first: an unmoved book
    // keeps the time it was last read.
    let (v, st, p, s) = (view.downgrade(), store.clone(), path.to_owned(), save.clone());
    let (ev, a) = (event.clone(), append.clone());
    window.connect_is_active_notify(move |w| {
        if let Some(id) = waiting.borrow_mut().take() {
            id.remove();
            s();
        }
        if w.is_active() {
            ev(log::Event::Active);
        } else {
            a(true);
        }
        let Some(view) = v.upgrade() else { return };
        let (v, st, p, refresh, start) = (v.clone(), st.clone(), p.clone(), refresh.clone(), view.locator());
        let state = st.borrow().path().to_owned();
        glib::spawn_future_local(async move {
            let moved = gio::spawn_blocking(move || sync_state(&state)).await.unwrap_or_default();
            st.borrow_mut().reload();
            refresh();
            let Some(view) = v.upgrade() else { return };
            let book = view.book();
            let synced = st.borrow().position(&book, &p);
            if moved.contains(&book.id)
                && view.locator() == start
                && let Some(at) = synced
                && at != start
            {
                view.go_to(at);
            }
        });
    });
    save(); // an opened book is in the library, even if it is closed without turning a page
    if let Err(e) = log::opened(&store.borrow(), &view.book().id, now_ms()) {
        eprintln!("cannot write the reading log: {e}");
    }
}

/// Now in milliseconds since the Unix epoch, and the local time zone's offset east of UTC in seconds (`reading-log`).
fn now_ms() -> u64 {
    std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).map_or(0, |d| d.as_millis() as u64)
}

fn utc_offset() -> i64 {
    glib::DateTime::now_local().map_or(0, |d| d.utc_offset().as_seconds())
}

/// The library in the reader's theme, or the open one brought forward (then `shown` is not called); opening a book
/// from it closes it. The Books folder is read first, off the main thread; the first time, it is made and the
/// remembered books are copied in. Measurement runs read the folder only if it was made.
fn library_window(app: &gtk::Application, options: &Options, store: &SharedStore, shown: impl FnOnce(Rc<library::Library>) + 'static) {
    if SHOWING.get() {
        return;
    }
    SHOWING.set(true);
    // No window may be open meanwhile: hold the application.
    let hold = app.hold();
    let (app, options, store) = (app.clone(), options.clone(), store.clone());
    let (state, home, stateless) = (store.borrow().path().to_owned(), glib::home_dir(), options.stateless());
    glib::spawn_future_local(async move {
        let read = gio::spawn_blocking(move || {
            let mut s = Store::open(state);
            let folder = if stateless {
                forzac_core::library::remembered_folder(&s)
            } else {
                forzac_core::library::books_folder(&home, language(), &mut s).ok()
            };
            // The other devices' reading first (`sync`); measurements leave it alone.
            if let (Some(folder), false) = (&folder, stateless) {
                let _one = SYNCING.lock().unwrap_or_else(|e| e.into_inner());
                if let Err(e) = forzac_core::sync::sync(folder, &mut s) {
                    eprintln!("cannot sync: {e}");
                }
            }
            let books = match &folder {
                Some(folder) => forzac_core::library::scan(folder, &mut s),
                None => s.books().into_iter().cloned().collect(),
            };
            (folder, books)
        })
        .await;
        SHOWING.set(false);
        let (folder, books) = read.unwrap_or_default();
        store.borrow_mut().reload();
        let theme = Settings::load(&store.borrow()).theme_index(appearance::dark());
        panels::set_theme(theme);
        let window: Rc<RefCell<Option<glib::WeakRef<gtk::ApplicationWindow>>>> = Rc::default();
        let open: Rc<dyn Fn(&Path)> = {
            let (app, options, store, window) = (app.clone(), options.clone(), store.clone(), window.clone());
            Rc::new(move |path| {
                let window = window.clone();
                open_book(&app, path, &options, &store, move |opened| {
                    if opened && let Some(w) = window.borrow().as_ref().and_then(|w| w.upgrade()) {
                        w.close();
                    }
                });
            })
        };
        let size = if stateless { fitted(1040, 760) } else { remembered_size(&store.borrow(), "window-library", (1040, 760)) };
        if let Some(library) = library::Library::show(&app, &store, theme, size, open, books, folder) {
            *window.borrow_mut() = Some(library.window.downgrade());
            if !stateless {
                remember_size(&library.window, &store, "window-library");
                library.check_update();
            }
            shown(library);
        }
        drop(hold);
    });
}

/// Keys by keyval; GTK also matches them in the first (Latin) layout group, so `j`/`k` work in a Russian layout.
/// They act while the page has the focus: an open panel has keys of its own.
/// What the reader's keys open and close: `o`, `s`, `/` and closing whatever panel is open.
struct PanelKeys {
    contents: Rc<dyn Fn(panels::Tab)>,
    settings: Rc<dyn Fn()>,
    search: Rc<dyn Fn()>,
    close: Rc<dyn Fn() -> bool>,
    menu: Rc<menu::Menu>,
}

fn shortcuts(
    view: &PageView,
    help: &Rc<help::Help>,
    panels: PanelKeys,
    window: &gtk::ApplicationWindow,
    save_style: Rc<dyn Fn()>,
) -> gtk::ShortcutController {
    let PanelKeys { contents: open_contents, settings: open_settings, search: open_search, close: close_panels, menu } = panels;
    let controller = gtk::ShortcutController::new();
    let add = |trigger: &str, f: Box<dyn Fn()>| {
        let action = gtk::CallbackAction::new(move |_, _| {
            f();
            glib::Propagation::Stop
        });
        controller.add_shortcut(gtk::Shortcut::new(gtk::ShortcutTrigger::parse_string(trigger), Some(action)));
    };
    let v = view.clone();
    add("l|Right|space|Page_Down", Box::new(move || v.turn(1)));
    let v = view.clone();
    add("j|Down", Box::new(move || v.line(1)));
    let v = view.clone();
    add("<Control>d", Box::new(move || v.half(1)));
    let v = view.clone();
    add("<Control>u", Box::new(move || v.half(-1)));
    let (v, save) = (view.clone(), save_style.clone());
    add(
        "m",
        Box::new(move || {
            v.toggle_mode();
            save();
        }),
    );
    let v = view.clone();
    add(
        "Return|KP_Enter",
        Box::new(move || {
            if !v.open_focused() {
                menu.open();
            }
        }),
    );
    let v = view.clone();
    add("Tab", Box::new(move || v.focus_next(1)));
    let v = view.clone();
    add("<Shift>Tab|<Shift>ISO_Left_Tab", Box::new(move || v.focus_next(-1)));
    let v = view.clone();
    add(
        "<Control>o",
        Box::new(move || {
            v.back();
        }),
    );
    let v = view.clone();
    add("b", Box::new(move || v.toggle_bookmark()));
    let v = view.clone();
    add("v", Box::new(move || v.start_selection()));
    let c = open_contents.clone();
    add("<Shift>b", Box::new(move || c(panels::Tab::Bookmarks)));
    add("o", Box::new(move || open_contents(panels::Tab::Contents)));
    add("s", Box::new(move || open_settings()));
    add("slash", Box::new(move || open_search()));
    let v = view.clone();
    add("n", Box::new(move || v.next_match(1)));
    let v = view.clone();
    add("<Shift>n", Box::new(move || v.next_match(-1)));
    let v = view.clone();
    add("h|Left|<Shift>space|Page_Up|BackSpace", Box::new(move || v.turn(-1)));
    let v = view.clone();
    add("k|Up", Box::new(move || v.line(-1)));
    let v = view.clone();
    add("bracketright", Box::new(move || v.chapter(1)));
    let v = view.clone();
    add("bracketleft", Box::new(move || v.chapter(-1)));
    let v = view.clone();
    add("g|Home", Box::new(move || v.chapter_edge(false)));
    let v = view.clone();
    add("<Shift>g|End", Box::new(move || v.chapter_edge(true)));
    let (v, save) = (view.clone(), save_style.clone());
    add(
        "plus|equal|KP_Add",
        Box::new(move || {
            v.step(Setting::Size, 1);
            save();
        }),
    );
    let (v, save) = (view.clone(), save_style.clone());
    add(
        "minus|KP_Subtract",
        Box::new(move || {
            v.step(Setting::Size, -1);
            save();
        }),
    );
    let (v, save) = (view.clone(), save_style.clone());
    add(
        "0|KP_0",
        Box::new(move || {
            v.set_settings(Settings { size: DEFAULT_SIZE, ..v.settings() });
            save();
        }),
    );
    let (v, save) = (view.clone(), save_style.clone());
    add(
        "t",
        Box::new(move || {
            v.step(Setting::Theme, 1);
            save();
        }),
    );
    let (v, save) = (view.clone(), save_style);
    add(
        "<Shift>t",
        Box::new(move || {
            v.step(Setting::Theme, -1);
            save();
        }),
    );
    let h = help.clone();
    add("question", Box::new(move || h.show()));
    let v = view.clone();
    add(
        "Escape",
        Box::new(move || {
            // A panel, else the focused marker or link, else the match highlight (the help takes its own keys).
            if !close_panels() && !v.unfocus() {
                v.clear_match();
            }
        }),
    );
    let w = window.clone();
    add("f|F11", Box::new(move || if w.is_fullscreen() { w.unfullscreen() } else { w.fullscreen() }));
    let w = window.clone();
    add("q|<Control>w", Box::new(move || w.close()));
    controller
}
