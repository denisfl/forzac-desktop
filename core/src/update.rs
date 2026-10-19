//! Whether a newer Forzac is out (`update-check`): when the library asks GitHub for the latest release, reading the
//! answer, and the version the library offers. The shells make the request (GTK through `curl`, macOS with
//! `URLSession`); the core decides and remembers, in the state file's settings.

use std::io;
use std::time::{SystemTime, UNIX_EPOCH};

use crate::state::Store;

/// The latest release, as GitHub's API gives it (pre-releases and drafts are never "latest").
pub const RELEASES: &str = "https://api.github.com/repos/denisfl/forzac-desktop/releases/latest";
/// Where a newer version is downloaded.
pub const PAGE: &str = "https://forzac.truenrth.app/desktop/";
/// This build's version.
pub const VERSION: &str = env!("CARGO_PKG_VERSION");

const CHECKED: &str = "update-checked";
const LATEST: &str = "update-latest";
const DISMISSED: &str = "update-dismissed";
const DAY: u64 = 24 * 60 * 60;

/// Whether the library should ask now: no successful check in the last 24 hours.
pub fn due(store: &Store) -> bool {
    due_at(store.setting(CHECKED).and_then(|t| t.parse().ok()), now())
}

/// Remembers GitHub's `answer` (the latest release as JSON) and the time; false when it names no release version or
/// the state cannot be written.
pub fn record(store: &mut Store, answer: &str) -> bool {
    let Some(latest) = tag(answer).filter(|t| version(t).is_some()) else { return false };
    let latest = latest.trim_start_matches('v').to_owned();
    store.set_settings(&[(CHECKED, now().to_string()), (LATEST, latest)]).is_ok()
}

/// The version the library offers: the latest release when it is newer than this build and not dismissed.
pub fn notice(store: &Store) -> Option<&str> {
    let latest = store.setting(LATEST)?;
    (newer(latest, VERSION) && store.setting(DISMISSED) != Some(latest)).then_some(latest)
}

/// Hides the offered version until a newer one is released.
pub fn dismiss(store: &mut Store) -> io::Result<()> {
    match notice(store).map(str::to_owned) {
        Some(latest) => store.set_setting(DISMISSED, &latest),
        None => Ok(()),
    }
}

fn due_at(checked: Option<u64>, now: u64) -> bool {
    checked.is_none_or(|t| t > now || now - t >= DAY)
}

/// The `tag_name` string of a JSON object, found by hand: one field needs no JSON parser.
fn tag(json: &str) -> Option<&str> {
    let rest = json[json.find("\"tag_name\"")? + "\"tag_name\"".len()..].trim_start().strip_prefix(':')?;
    let rest = rest.trim_start().strip_prefix('"')?;
    Some(&rest[..rest.find('"')?])
}

/// `v0.9.1` or `0.9.1` as its numbers; `None` for anything else (`v1.0.0-rc1`).
fn version(tag: &str) -> Option<Vec<u32>> {
    let numbers = tag.strip_prefix('v').unwrap_or(tag);
    numbers.split('.').map(|n| if n.bytes().all(|b| b.is_ascii_digit()) { n.parse().ok() } else { None }).collect()
}

/// Whether version `a` is later than `b`, a missing number counting as 0; false when either is not a version.
fn newer(a: &str, b: &str) -> bool {
    let (Some(a), Some(b)) = (version(a), version(b)) else { return false };
    let at = |v: &[u32], i: usize| v.get(i).copied().unwrap_or(0);
    (0..a.len().max(b.len())).map(|i| at(&a, i).cmp(&at(&b, i))).find(|o| o.is_ne()) == Some(std::cmp::Ordering::Greater)
}

fn now() -> u64 {
    SystemTime::now().duration_since(UNIX_EPOCH).map_or(0, |d| d.as_secs())
}

#[cfg(test)]
mod tests;
