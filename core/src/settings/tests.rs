use super::*;

fn store() -> Store {
    static N: std::sync::atomic::AtomicUsize = std::sync::atomic::AtomicUsize::new(0);
    let dir = std::env::temp_dir().join(format!(
        "forzac-settings-{}-{}",
        std::process::id(),
        N.fetch_add(1, std::sync::atomic::Ordering::SeqCst)
    ));
    let _ = std::fs::remove_dir_all(&dir);
    Store::open(dir.join("state"))
}

#[test]
fn defaults_follow_the_system_theme() {
    let s = Settings::default();
    assert_eq!((s.theme_index(false), s.theme_index(true)), (0, 2), "Light, or Dark on a dark desktop");
    let style = s.page_style(1200.0, 900.0);
    assert_eq!((style.text_size, style.line_height, style.justify, style.family), (20.0, 1.62, true, Family::PtSerif));
    assert!((style.text_width() - 680.0).abs() < 0.01, "34 em");
}

#[test]
fn settings_round_trip_and_unknown_values_fall_back() {
    let mut store = store();
    let mut s = Settings::default();
    s.step(Setting::Font, 1, false);
    s.step(Setting::Spacing, 1, false);
    s.step(Setting::Size, 1, false);
    s.step(Setting::Align, 1, false);
    s.step(Setting::Margins, -1, false);
    s.step(Setting::Turning, 1, false);
    assert_eq!(s.theme, None, "the theme still follows the system");
    s.save(&mut store).unwrap();
    let loaded = Settings::load(&store);
    assert_eq!(loaded, s);
    assert_eq!(
        (loaded.family, loaded.spacing, loaded.size, loaded.align, loaded.margins, loaded.turning),
        (Family::Literata, Spacing::Loose, 3, Align::Left, Margins::Narrow, Turning::Scroll)
    );

    store.set_settings(&[("font", "comic-sans".into()), ("spacing", "huge".into()), ("text-size", "13".into())]).unwrap();
    let loaded = Settings::load(&store);
    assert_eq!((loaded.family, loaded.spacing, loaded.size), (Family::PtSerif, Spacing::Normal, DEFAULT_SIZE));
}

#[test]
fn choosing_a_theme_starts_from_the_one_shown() {
    let mut s = Settings::default();
    s.step(Setting::Theme, 1, true); // Dark shown → the next is Dawn
    assert_eq!(s.theme, Some(3));
    assert_eq!(s.theme_index(false), 3, "chosen: no longer follows the system");
    let mut size = Settings::default();
    for _ in 0..20 {
        size.step(Setting::Size, 1, false);
    }
    assert_eq!(size.text_size(), 30.0, "sizes stop at the ends");
}

#[test]
fn margins_change_the_column() {
    let mut s = Settings::default();
    let medium = s.page_style(1600.0, 900.0).text_width();
    s.margins = Margins::Narrow;
    let narrow = s.page_style(1600.0, 900.0).text_width();
    s.margins = Margins::Wide;
    let wide = s.page_style(1600.0, 900.0).text_width();
    assert!(narrow > medium && medium > wide, "{narrow} {medium} {wide}");
    // A narrow window: the least side margin decides.
    s.margins = Margins::Wide;
    assert!((s.page_style(500.0, 900.0).margin_h - 72.0).abs() < 0.01);
}
