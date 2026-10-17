use super::*;

#[test]
fn the_language_is_the_first_preferred_one_forzac_has() {
    assert_eq!(Language::from_locales(["ru_RU.UTF-8", "en_US"]), Language::Russian);
    assert_eq!(Language::from_locales(["de-DE", "ru-RU", "en"]), Language::Russian);
    assert_eq!(Language::from_locales(["de", "en-GB", "ru"]), Language::English);
    assert_eq!(Language::from_locales(["C", "POSIX"]), Language::English);
    assert_eq!(Language::from_locales([]), Language::English);
}

#[test]
fn russian_words_and_plurals() {
    assert_eq!(tr_in(Language::Russian, "Highlights"), "Цитаты");
    assert_eq!(tr_in(Language::English, "Highlights"), "Highlights");
    assert_eq!(tr_in(Language::Russian, "Not in the catalogue"), "Not in the catalogue");
    let count = |n| count_in(Language::Russian, n, "{n} match", "{n} matches");
    assert_eq!(
        [count(1), count(3), count(5), count(11), count(21), count(22), count(112)],
        ["1 совпадение", "3 совпадения", "5 совпадений", "11 совпадений", "21 совпадение", "22 совпадения", "112 совпадений"]
    );
    assert_eq!(count_in(Language::English, 1, "{n} match", "{n} matches"), "1 match");
    assert_eq!(count_in(Language::English, 0, "{n} match", "{n} matches"), "0 matches");
    assert_eq!(
        fill(tr_in(Language::Russian, "Couldn't open “{name}”: {reason}"), &[("name", "a.zip"), ("reason", "x")]),
        "Не удалось открыть «a.zip»: x"
    );
}

/// The string literals `call(…)` is given in `source`: `tr("…")`, or the quoted arguments of `tr_count(n, "…", "…")`.
fn asked(source: &str, call: &str) -> Vec<String> {
    let mut out = Vec::new();
    let pattern = format!("{call}(");
    let mut from = 0;
    while let Some(i) = source[from..].find(&pattern).map(|i| from + i) {
        from = i + pattern.len();
        // Not the end of a longer name (`str(`, `test_tr(`).
        if source[..i].chars().last().is_some_and(|c| c.is_alphanumeric() || c == '_' || c == '.') {
            continue;
        }
        // The literals among the arguments, up to the `)` that closes the call.
        let (mut literal, mut depth, mut chars) = (None::<String>, 0, source[from..].chars());
        while let Some(c) = chars.next() {
            match (&mut literal, c) {
                (Some(l), '\\') => match chars.next() {
                    Some('n') => l.push('\n'),
                    Some(other) => l.push(other),
                    None => {}
                },
                (Some(_), '"') => out.extend(literal.take()),
                (Some(l), c) => l.push(c),
                (None, '"') => literal = Some(String::new()),
                (None, '(') => depth += 1,
                (None, ')') if depth == 0 => break,
                (None, ')') => depth -= 1,
                _ => {}
            }
        }
    }
    out
}

#[test]
fn every_string_the_shells_ask_for_has_its_russian_and_none_is_left_over() {
    let root = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("..");
    let mut asked_for: Vec<String> = ASKED_BY_HELP.iter().map(|s| s.to_string()).collect();
    for (dir, ext, calls) in
        [("gtk/src", "rs", ["tr", "tr_count"]), ("core/src", "rs", ["tr", "tr_count"]), ("macos", "swift", ["tr", "trCount"])]
    {
        for path in files(&root.join(dir)) {
            if path.extension().is_some_and(|e| e == ext) && !path.to_string_lossy().contains("i18n") {
                let source = std::fs::read_to_string(&path).unwrap();
                for call in calls {
                    asked_for.extend(asked(&source, call));
                }
            }
        }
    }
    let plural_ones: Vec<&str> = PLURALS.iter().map(|p| p.0).collect();
    let missing: Vec<&String> =
        asked_for.iter().filter(|s| russian(s).is_none() && !plural_ones.contains(&s.as_str()) && !is_plural_one(s)).collect();
    assert!(missing.is_empty(), "no Russian for {missing:?}");
    let unused: Vec<&str> =
        RUSSIAN.iter().map(|e| e.0).chain(plural_ones.iter().copied()).filter(|e| !asked_for.iter().any(|a| a == e)).collect();
    assert!(unused.is_empty(), "nobody asks for {unused:?}");
}

/// The files under `dir`, in its subdirectories too.
fn files(dir: &std::path::Path) -> Vec<std::path::PathBuf> {
    let mut out = Vec::new();
    for entry in std::fs::read_dir(dir).unwrap() {
        let path = entry.unwrap().path();
        if path.is_dir() { out.extend(files(&path)) } else { out.push(path) }
    }
    out
}

/// The English singular of a plural entry ("{n} match"): it has no Russian of its own.
fn is_plural_one(s: &str) -> bool {
    PLURALS.iter().any(|p| p.0.starts_with("{n} ") && s.starts_with("{n} ") && !s.ends_with('s'))
}
