//! Forzac for Linux: a GTK 4 shell over `forzac-core`.
//!
//! `forzac book.fb2` opens a window per book. GApplication keeps one process per user: opening another book
//! (`xdg-open`, double click) hands it to the running instance over D-Bus.
//!
//! Spike flags: `--measure` prints the time to the first presented page and quits; `--png out.png` saves the first
//! page as rendered by the window's renderer and quits; `--turns N` turns N pages with the animation and prints
//! frame-interval statistics; `--chapter N` opens chapter N.

mod fonts;
mod page_view;

use std::path::PathBuf;
use std::time::Instant;

use gtk::{gio, glib, graphene, prelude::*};
use page_view::PageView;

const APP_ID: &str = "app.truenrth.forzac";

#[derive(Clone)]
struct Options {
    start: Instant,
    measure: bool,
    png: Option<PathBuf>,
    turns: usize,
    chapter: usize,
}

fn main() -> glib::ExitCode {
    let start = Instant::now();
    // The Wayland app_id comes from the program name; it must match the .desktop file name for the compositor,
    // docks and `StartupWMClass`-less matching to tie windows to Forzac.
    glib::set_prgname(Some(APP_ID));
    let args: Vec<String> = std::env::args().collect();
    let png = args.iter().position(|a| a == "--png").and_then(|i| args.get(i + 1)).map(PathBuf::from);
    let turns = args.iter().position(|a| a == "--turns").and_then(|i| args.get(i + 1)).and_then(|n| n.parse().ok()).unwrap_or(0);
    let chapter = args.iter().position(|a| a == "--chapter").and_then(|i| args.get(i + 1)).and_then(|n| n.parse().ok()).unwrap_or(0);
    let options = Options { start, measure: args.iter().any(|a| a == "--measure"), png, turns, chapter };
    let files: Vec<&String> = args
        .iter()
        .skip(1)
        .filter(|a| !a.starts_with("--") && a.parse::<usize>().is_err() && Some(PathBuf::from(a.as_str())) != options.png)
        .collect();

    let mut flags = gio::ApplicationFlags::HANDLES_OPEN;
    if options.measure || options.png.is_some() || options.turns > 0 {
        flags |= gio::ApplicationFlags::NON_UNIQUE;
    }
    let app = gtk::Application::builder().application_id(APP_ID).flags(flags).build();
    let opts = options.clone();
    app.connect_open(move |app, files, _| {
        for file in files {
            if let Some(path) = file.path() {
                open_window(app, &path, &opts);
            }
        }
    });
    app.connect_activate(|_| eprintln!("usage: forzac <book.fb2|book.fb2.zip>"));
    let mut run_args = vec![args[0].clone()];
    run_args.extend(files.into_iter().cloned());
    app.run_with_args(&run_args)
}

fn open_window(app: &gtk::Application, path: &std::path::Path, options: &Options) {
    let book = match forzac_core::open(path) {
        Ok(book) => book,
        Err(e) => {
            eprintln!("{}: not a readable book ({e:?})", path.display());
            return;
        }
    };
    let parsed = options.start.elapsed();
    let title = if book.title.is_empty() { "Forzac".to_owned() } else { format!("{} — Forzac", book.title) };
    let view = PageView::new(book, options.chapter);

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
    window.add_controller(shortcuts(&view, &help, &window));

    if options.measure || options.png.is_some() || options.turns > 0 {
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
            if let Some(png) = &options.png {
                save_png(view, png);
            }
            if options.turns == 0 {
                window.close();
                return;
            }
            // One turn every 400 ms (the animation takes 200), then the statistics.
            let left = std::cell::Cell::new(options.turns);
            let (view, window) = (view.clone(), window.clone());
            glib::timeout_add_local(std::time::Duration::from_millis(400), move || {
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
j  l  →  ↓  Space  PgDn    next page
k  h  ←  ↑  Shift+Space  PgUp    previous page
]  [    next / previous chapter
g  G  Home  End    chapter start / end
+  −    text size
t  T    theme
f  F11    full screen
?    this help
q  Ctrl+W    close

Forzac is free. Support it: forzac.truenrth.app";

/// Keys by keyval; GTK also matches them in the first (Latin) layout group, so `j`/`k` work in a Russian layout.
fn shortcuts(view: &PageView, help: &gtk::Label, window: &gtk::ApplicationWindow) -> gtk::ShortcutController {
    let controller = gtk::ShortcutController::new();
    controller.set_scope(gtk::ShortcutScope::Global);
    let add = |trigger: &str, f: Box<dyn Fn()>| {
        let action = gtk::CallbackAction::new(move |_, _| {
            f();
            glib::Propagation::Stop
        });
        controller.add_shortcut(gtk::Shortcut::new(gtk::ShortcutTrigger::parse_string(trigger), Some(action)));
    };
    let v = view.clone();
    add("j|l|Right|Down|space|Page_Down|Return", Box::new(move || v.turn(1)));
    let v = view.clone();
    add("k|h|Left|Up|<Shift>space|Page_Up|BackSpace", Box::new(move || v.turn(-1)));
    let v = view.clone();
    add("bracketright", Box::new(move || v.chapter(1)));
    let v = view.clone();
    add("bracketleft", Box::new(move || v.chapter(-1)));
    let v = view.clone();
    add("g|Home", Box::new(move || v.chapter_edge(false)));
    let v = view.clone();
    add("<Shift>g|End", Box::new(move || v.chapter_edge(true)));
    let v = view.clone();
    add("plus|equal|KP_Add", Box::new(move || v.text_size(1)));
    let v = view.clone();
    add("minus|KP_Subtract", Box::new(move || v.text_size(-1)));
    let v = view.clone();
    add("t", Box::new(move || v.next_theme(1)));
    let v = view.clone();
    add("<Shift>t", Box::new(move || v.next_theme(-1)));
    let h = help.clone();
    add("question", Box::new(move || h.set_visible(!h.is_visible())));
    let h = help.clone();
    add("Escape", Box::new(move || h.set_visible(false)));
    let w = window.clone();
    add("f|F11", Box::new(move || if w.is_fullscreen() { w.unfullscreen() } else { w.fullscreen() }));
    let w = window.clone();
    add("q|<Control>w", Box::new(move || w.close()));
    controller
}
