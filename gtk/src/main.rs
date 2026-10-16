//! Forzac for Linux: a GTK 4 shell over `forzac-core`.
//!
//! `forzac book.fb2` opens a window per book. GApplication keeps one process per user: opening another book
//! (`xdg-open`, double click) hands it to the running instance over D-Bus.
//!
//! Spike flags: `--measure` prints the time to the first presented page and quits; `--png out.png` saves the first
//! page as rendered by the window's renderer and quits; `--turns N` turns N pages with the animation and prints
//! frame-interval statistics; `--scrolls N` the same in the "Scroll" mode, a screen at a time; `--chapter N` opens
//! chapter N; `--page N` goes to page N of it
//! (for `--png` and `--turns`). With any of them the reading state is neither read nor written.
//!
//! Without a file, the recent books are listed. Reading positions and the settings are remembered in
//! `$XDG_DATA_HOME/forzac/state`.

mod appearance;
mod fonts;
mod icons;
mod page_view;
mod panels;
mod settings_sheet;
mod viewer;

use std::cell::{Cell, RefCell};
use std::path::{Path, PathBuf};
use std::rc::Rc;
use std::time::{Duration, Instant};

use forzac_core::Locator;
use forzac_core::settings::{DEFAULT_SIZE, Setting, Settings, Turning};
use forzac_core::state::Store;
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
    chapter: Option<usize>,
    page: usize,
}

impl Options {
    /// Measurement runs neither read nor write the reading state.
    fn stateless(&self) -> bool {
        self.measure || self.png.is_some() || self.turns > 0 || self.scrolls > 0 || self.chapter.is_some() || self.page > 0
    }
}

type SharedStore = Rc<RefCell<Store>>;

/// Remembered books shown when Forzac starts without a file.
const RECENT: usize = 20;
/// The reading position is saved this long after the last turn.
const SAVE_DELAY: Duration = Duration::from_secs(1);
/// The screen stays on this long after the last key, turn, scroll or click in a book window.
const KEEP_AWAKE: Duration = Duration::from_secs(10 * 60);

thread_local! {
    /// The open book windows by the canonical path of their file: opening a book again brings its window forward.
    static WINDOWS: RefCell<Vec<(PathBuf, glib::WeakRef<gtk::ApplicationWindow>)>> = const { RefCell::new(Vec::new()) };
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
    let options = Options { start, measure: args.iter().any(|a| a == "--measure"), png, turns, scrolls, chapter, page };
    let files: Vec<&String> = args
        .iter()
        .skip(1)
        .filter(|a| !a.starts_with("--") && a.parse::<usize>().is_err() && Some(PathBuf::from(a.as_str())) != options.png)
        .collect();

    let mut flags = gio::ApplicationFlags::HANDLES_OPEN;
    if options.measure || options.png.is_some() || options.turns > 0 || options.scrolls > 0 {
        flags |= gio::ApplicationFlags::NON_UNIQUE;
    }
    let app = gtk::Application::builder().application_id(APP_ID).flags(flags).build();
    let store: SharedStore = Rc::new(RefCell::new(Store::open(glib::user_data_dir().join("forzac").join("state"))));
    let (opts, st) = (options.clone(), store.clone());
    app.connect_open(move |app, files, _| {
        for file in files {
            if let Some(path) = file.path() {
                open_window(app, &path, &opts, &st);
            }
        }
    });
    // After the last book window, the recent books (as on macOS); closing them quits. Shown while the window is being
    // removed, so the application never runs out of windows.
    if !options.stateless() {
        let (opts, st) = (options.clone(), store.clone());
        app.connect_window_removed(move |app, window| {
            if app.windows().is_empty() && !window.has_css_class("forzac-recent") {
                recent_window(app, &opts, &st);
            }
        });
    }
    let opts = options.clone();
    app.connect_activate(move |app| recent_window(app, &opts, &store));
    let mut run_args = vec![args[0].clone()];
    run_args.extend(files.into_iter().cloned());
    app.run_with_args(&run_args)
}

/// Why a file cannot be read, in the Android app's words (without PDF).
fn reason(e: forzac_core::Error) -> &'static str {
    match e {
        forzac_core::Error::NotABook => "unsupported format — FB2, FB3 or EPUB needed",
        forzac_core::Error::Damaged => "the file is damaged",
        forzac_core::Error::Protected => "the book is DRM-protected and can't be opened",
        forzac_core::Error::NoText => "the book has no text",
        forzac_core::Error::Unavailable => "the file is unavailable",
    }
}

/// Says why `path` cannot be opened; then, when no book is open, the recent books follow.
fn cannot_open(app: &gtk::Application, path: &Path, e: forzac_core::Error, options: &Options, store: &SharedStore) {
    let name = path.file_name().map_or_else(|| path.display().to_string(), |n| n.to_string_lossy().into_owned());
    let message = format!("Couldn't open “{name}”: {}", reason(e));
    eprintln!("{message}");
    if options.stateless() {
        return;
    }
    // The dialog is not a window of the application: hold it, or it would quit before the message shows.
    let hold = app.hold();
    let parent = app.active_window();
    let (app, options, store) = (app.clone(), options.clone(), store.clone());
    gtk::AlertDialog::builder().message(message).modal(true).build().choose(parent.as_ref(), gio::Cancellable::NONE, move |_| {
        if !has_book_window() {
            recent_window(&app, &options, &store);
        }
        drop(hold);
    });
}

/// Whether a book window is open.
fn has_book_window() -> bool {
    WINDOWS.with(|w| w.borrow().iter().any(|(_, w)| open_book_window(w).is_some()))
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
    let title = if book.title.is_empty() { "Forzac".to_owned() } else { format!("{} — Forzac", book.title) };
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

    let help = gtk::Label::new(Some(HELP));
    help.add_css_class("osd");
    help.set_halign(gtk::Align::Center);
    help.set_valign(gtk::Align::Center);
    help.set_visible(false);
    let overlay = gtk::Overlay::new();
    overlay.set_child(Some(&view));
    overlay.add_overlay(&help);

    let window =
        gtk::ApplicationWindow::builder().application(app).title(&title).default_width(760).default_height(980).child(&overlay).build();
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
    // Closes whatever panel is over the page; false when none was.
    let close_panels: Rc<dyn Fn() -> bool> = {
        let (settings_panel, contents, search, note, overlay, view) =
            (settings_panel.clone(), contents.clone(), search.clone(), note.clone(), overlay.clone(), view.clone());
        Rc::new(move || {
            let mut closed = false;
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
            let was_open = s.is_open();
            close();
            if !was_open {
                s.open(&view);
            }
        })
    };
    let open_contents: Rc<dyn Fn()> = {
        let (contents, overlay, view, close) = (contents.clone(), overlay.clone(), view.clone(), close_panels.clone());
        Rc::new(move || {
            let c = contents.borrow_mut().get_or_insert_with(|| panels::Contents::new(&overlay, &view)).clone();
            let was_open = c.is_open();
            close();
            if !was_open {
                c.open(&view);
            }
        })
    };
    // A click on the page while a panel is over it closes the panel, and only that.
    let click = gtk::GestureClick::new();
    click.set_propagation_phase(gtk::PropagationPhase::Capture);
    let c = close_panels.clone();
    click.connect_pressed(move |g, _, _, _| {
        if c() {
            g.set_state(gtk::EventSequenceState::Claimed);
        }
    });
    view.add_controller(click);
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
        PanelKeys { contents: open_contents, settings: open_settings, search: open_search, close: close_panels },
        &window,
        save_style,
    ));
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
        }
    ));
    if !stateless {
        remember_position(&view, &window, path, store);
        keep_awake(app, &window);
        WINDOWS.with(|w| w.borrow_mut().push((canonical, window.downgrade())));
    }

    if options.measure || options.png.is_some() || options.turns > 0 || options.scrolls > 0 {
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
            if let Some(png) = &options.png {
                save_png(view, png);
            }
            if options.turns == 0 && options.scrolls == 0 {
                window.close();
                return;
            }
            // One turn (or scroll by a screen) every 400 ms (the animation takes 200 or 150), then the statistics; the
            // first 1.2 s after the first frame, as a reader would, when the neighbouring chapters are laid out.
            let left = std::cell::Cell::new(options.turns.max(options.scrolls));
            let (view, window) = (view.clone(), window.clone());
            let wait = std::cell::Cell::new(2); // ticks before the first turn
            glib::timeout_add_local(std::time::Duration::from_millis(400), move || {
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
                view.turn(1);
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
                cookie.set(Some(app.inhibit(Some(&window), gtk::ApplicationInhibitFlags::IDLE, Some("Reading a book"))));
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

const HELP: &str = "\
j  l  →  ↓  Space  PgDn  Enter    next page (Scroll: j ↓ a line, Ctrl+D half a screen)
k  h  ←  ↑  Shift+Space  PgUp    previous page (Scroll: k ↑ a line, Ctrl+U half a screen)
m    page or scroll
]  [    next / previous chapter
g  G  Home  End    chapter start / end
o    contents  (/ filters)
Tab  Shift+Tab    notes, links, pictures and tables on the page, Enter opens
Ctrl+O    back after a jump
s    settings
/  n  N    search, next / previous match
+  −  0    text size, default size
t  T    theme
f  F11    full screen
?    this help
q  Ctrl+W    close
click    the left or right third of the page turns it; a picture or a table opens

Picture or table:  + − 0  zoom, fit    arrows  h j k l  Space  move    Esc  q  close

Forzac is free. Support it: forzac.truenrth.app";

/// Saves the reading position when the book opens, `SAVE_DELAY` after the last move and when the window closes.
fn remember_position(view: &PageView, window: &gtk::ApplicationWindow, path: &Path, store: &SharedStore) {
    let save = {
        let (view, store, path) = (view.downgrade(), store.clone(), path.to_owned());
        Rc::new(move || {
            let Some(view) = view.upgrade() else { return };
            if let Err(e) = store.borrow_mut().save_position(&view.book(), &path, view.locator(), view.finished()) {
                eprintln!("cannot save the reading state: {e}");
            }
        })
    };
    let pending: Rc<RefCell<Option<glib::SourceId>>> = Rc::default();
    let (p, s) = (pending.clone(), save.clone());
    view.connect_moved(move || {
        if let Some(id) = p.borrow_mut().take() {
            id.remove();
        }
        let (p2, s2) = (p.clone(), s.clone());
        *p.borrow_mut() = Some(glib::timeout_add_local_once(SAVE_DELAY, move || {
            p2.borrow_mut().take(); // fired: the id is no longer valid
            s2();
        }));
    });
    let s = save.clone();
    window.connect_close_request(move |_| {
        if let Some(id) = pending.borrow_mut().take() {
            id.remove();
        }
        s();
        glib::Propagation::Proceed
    });
    save(); // an opened book is a recent book, even if it is closed without turning a page
}

/// The recent books, the last opened first, in the reader's theme; `Enter` opens one where it was left.
fn recent_window(app: &gtk::Application, options: &Options, store: &SharedStore) {
    fonts::add_interface_fonts();
    panels::set_theme(Settings::load(&store.borrow()).theme_index(appearance::dark()));
    let books: Vec<(PathBuf, String, String)> = store
        .borrow()
        .recent(RECENT)
        .iter()
        .map(|e| {
            let title =
                if e.title.is_empty() { e.path.file_name().unwrap_or_default().to_string_lossy().into_owned() } else { e.title.clone() };
            let percent = (e.progress * 100.0).round() as u32;
            let detail = if e.authors.is_empty() { format!("{percent}%") } else { format!("{} · {percent}%", e.authors) };
            (e.path.clone(), title, detail)
        })
        .collect();
    let window = gtk::ApplicationWindow::builder().application(app).title("Forzac").default_width(560).default_height(640).build();
    if std::env::var_os("HYPRLAND_INSTANCE_SIGNATURE").is_some() {
        window.set_decorated(false);
    }
    window.add_css_class("forzac-panel");
    window.add_css_class("forzac-recent");
    let list = gtk::ListBox::new();
    list.set_selection_mode(gtk::SelectionMode::Browse);
    for (_, title, detail) in &books {
        let row = gtk::Box::new(gtk::Orientation::Vertical, 2);
        row.set_margin_top(10);
        row.set_margin_bottom(10);
        row.set_margin_start(16);
        row.set_margin_end(16);
        let t = gtk::Label::new(Some(title));
        t.add_css_class("book-title");
        let d = gtk::Label::new(Some(detail));
        d.add_css_class("book-meta");
        d.add_css_class("muted");
        for label in [&t, &d] {
            label.set_xalign(0.0);
            label.set_ellipsize(gtk::pango::EllipsizeMode::End);
            row.append(label);
        }
        list.append(&row);
    }
    if books.is_empty() {
        let empty = gtk::Label::new(Some("No books yet.\n\nOpen one from your file manager, or run\nforzac book.epub"));
        empty.set_justify(gtk::Justification::Center);
        empty.add_css_class("muted");
        window.set_child(Some(&empty));
    } else {
        let scrolled = gtk::ScrolledWindow::builder().child(&list).hscrollbar_policy(gtk::PolicyType::Never).build();
        window.set_child(Some(&scrolled));
        let (app, options, store, w) = (app.clone(), options.clone(), store.clone(), window.clone());
        list.connect_row_activated(move |_, row| {
            if let Some((path, ..)) = books.get(row.index() as usize)
                && open_window(&app, path, &options, &store)
            {
                w.close();
            }
        });
        if let Some(first) = list.row_at_index(0) {
            list.select_row(Some(&first));
            first.grab_focus();
        }
    }
    let controller = gtk::ShortcutController::new();
    controller.set_scope(gtk::ShortcutScope::Global);
    let step = |delta: i32| {
        let list = list.clone();
        gtk::CallbackAction::new(move |_, _| {
            let current = list.selected_row().map_or(-1, |r| r.index());
            if let Some(row) = list.row_at_index((current + delta).max(0)) {
                list.select_row(Some(&row));
                row.grab_focus();
            }
            glib::Propagation::Stop
        })
    };
    controller.add_shortcut(gtk::Shortcut::new(gtk::ShortcutTrigger::parse_string("j"), Some(step(1))));
    controller.add_shortcut(gtk::Shortcut::new(gtk::ShortcutTrigger::parse_string("k"), Some(step(-1))));
    let w = window.clone();
    let close = gtk::CallbackAction::new(move |_, _| {
        w.close();
        glib::Propagation::Stop
    });
    controller.add_shortcut(gtk::Shortcut::new(gtk::ShortcutTrigger::parse_string("q|Escape|<Control>w"), Some(close)));
    window.add_controller(controller);
    window.present();
}

/// Keys by keyval; GTK also matches them in the first (Latin) layout group, so `j`/`k` work in a Russian layout.
/// They act while the page has the focus: an open panel has keys of its own.
/// What the reader's keys open and close: `o`, `s`, `/` and closing whatever panel is open.
struct PanelKeys {
    contents: Rc<dyn Fn()>,
    settings: Rc<dyn Fn()>,
    search: Rc<dyn Fn()>,
    close: Rc<dyn Fn() -> bool>,
}

fn shortcuts(
    view: &PageView,
    help: &gtk::Label,
    panels: PanelKeys,
    window: &gtk::ApplicationWindow,
    save_style: Rc<dyn Fn()>,
) -> gtk::ShortcutController {
    let PanelKeys { contents: open_contents, settings: open_settings, search: open_search, close: close_panels } = panels;
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
                v.turn(1);
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
    add("o", Box::new(move || open_contents()));
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
    add("question", Box::new(move || h.set_visible(!h.is_visible())));
    let (h, v) = (help.clone(), view.clone());
    add(
        "Escape",
        Box::new(move || {
            // The help, else a panel, else the focused marker or link, else the match highlight.
            if h.is_visible() {
                h.set_visible(false);
            } else if !close_panels() && !v.unfocus() {
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
