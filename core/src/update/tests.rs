use super::*;

/// A fresh state file for one test.
fn store() -> Store {
    static N: std::sync::atomic::AtomicUsize = std::sync::atomic::AtomicUsize::new(0);
    let n = N.fetch_add(1, std::sync::atomic::Ordering::SeqCst);
    let dir = std::env::temp_dir().join(format!("forzac-update-{}-{n}", std::process::id()));
    let _ = std::fs::remove_dir_all(&dir);
    Store::open(dir.join("state"))
}

/// GitHub's answer, shortened.
fn answer(tag: &str) -> String {
    format!(
        r#"{{"url":"https://api.github.com/repos/denisfl/forzac-desktop/releases/1","tag_name": "{tag}","name":"Forzac {tag}","draft":false}}"#
    )
}

#[test]
fn versions() {
    assert_eq!(version("v0.9.1"), Some(vec![0, 9, 1]));
    assert_eq!(version("0.9"), Some(vec![0, 9]));
    assert_eq!(version("v1.0.0-rc1"), None);
    assert_eq!(version("v1..0"), None);
    assert_eq!(version("latest"), None);
    assert_eq!(version(""), None);
    assert!(newer("0.9.1", "0.9.0"));
    assert!(newer("0.10", "0.9.9"));
    assert!(newer("1.0", "0.9.12"));
    assert!(!newer("0.9", "0.9.0"));
    assert!(!newer("0.9.0", "0.9.1"));
    assert!(!newer("1.0-rc1", "0.9.0"));
}

#[test]
fn a_check_is_due_once_a_day() {
    let now = 1_800_000_000;
    assert!(due_at(None, now));
    assert!(!due_at(Some(now - 3 * 3600), now));
    assert!(due_at(Some(now - 25 * 3600), now));
    assert!(due_at(Some(now + 3600), now), "a time in the future (a clock set back) does not stop the checks");
}

#[test]
fn the_answer_is_remembered_and_offered() {
    let mut store = store();
    assert!(due(&store));
    assert_eq!(notice(&store), None);
    assert!(!record(&mut store, r#"{"message":"API rate limit exceeded","documentation_url":"https://docs.github.com"}"#));
    assert!(!record(&mut store, &answer("v99.0.0-rc1")));
    assert!(!record(&mut store, "<html>offline</html>"));
    assert!(due(&store), "a failed check is tried again");

    assert!(record(&mut store, &answer(&format!("v{VERSION}"))));
    assert!(!due(&store));
    assert_eq!(notice(&store), None, "the same version is not offered");

    assert!(record(&mut store, &answer("v99.0.1")));
    assert_eq!(notice(&store), Some("99.0.1"));
    dismiss(&mut store).unwrap();
    assert_eq!(notice(&store), None);
    assert!(record(&mut store, &answer("v99.0.1")));
    assert_eq!(notice(&store), None, "a dismissed version stays hidden");
    assert!(record(&mut store, &answer("99.1")));
    assert_eq!(notice(&store), Some("99.1"), "a newer one is offered again");
}
