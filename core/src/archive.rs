//! Zip packages (EPUB, FB3): unpacked into memory, as both formats need random access to their parts.

use std::collections::HashMap;
use std::io::{Cursor, Read};

use crate::Error;

/// Unpacked content larger than this is treated as a zip bomb.
pub(crate) const MAX_UNPACKED: u64 = 256 * 1024 * 1024;

/// The files of a zip archive by name (directories left out), at most `max` bytes in all.
pub(crate) fn unzip(bytes: &[u8], max: u64) -> Result<HashMap<String, Vec<u8>>, Error> {
    let mut zip = zip::ZipArchive::new(Cursor::new(bytes)).map_err(|_| Error::NotABook)?;
    let mut files = HashMap::new();
    let mut total = 0u64;
    for i in 0..zip.len() {
        // A damaged entry is left out; the book may still be readable without it.
        let Ok(entry) = zip.by_index(i) else { continue };
        if entry.is_dir() {
            continue;
        }
        let Ok(name) = entry.name().map(|n| n.into_owned()) else { continue };
        let mut data = Vec::new();
        let read = entry.take(max - total + 1).read_to_end(&mut data);
        total += data.len() as u64;
        if total > max {
            return Err(Error::NotABook);
        }
        if read.is_ok() {
            files.insert(name, data);
        }
    }
    Ok(files)
}

/// `%XX` sequences decoded (paths in hrefs and relationships); invalid ones are kept as they are.
pub(crate) fn percent_decode(s: &str) -> String {
    let bytes = s.as_bytes();
    let mut out = Vec::with_capacity(bytes.len());
    let mut i = 0;
    while i < bytes.len() {
        let hex = |b: u8| (b as char).to_digit(16);
        if bytes[i] == b'%'
            && let (Some(h), Some(l)) = (bytes.get(i + 1).copied().and_then(hex), bytes.get(i + 2).copied().and_then(hex))
        {
            out.push((h * 16 + l) as u8);
            i += 3;
        } else {
            out.push(bytes[i]);
            i += 1;
        }
    }
    String::from_utf8(out).unwrap_or_else(|_| s.to_owned())
}

/// `.` and `..` segments resolved, empty segments dropped: `a/./b/../c` → `a/c`.
pub(crate) fn normalize_path(path: &str) -> String {
    let mut parts: Vec<&str> = Vec::new();
    for part in path.split('/') {
        match part {
            "" | "." => {}
            ".." => {
                parts.pop();
            }
            _ => parts.push(part),
        }
    }
    parts.join("/")
}

/// A zip of `entries` (name, content), for tests of the package formats.
#[cfg(test)]
pub(crate) fn zip(entries: &[(&str, &[u8])]) -> Vec<u8> {
    use std::io::Write;
    let mut zip = zip::ZipWriter::new(Cursor::new(Vec::new()));
    for (name, data) in entries {
        zip.start_file(*name, zip::write::SimpleFileOptions::default().compression_method(zip::CompressionMethod::Deflated)).unwrap();
        zip.write_all(data).unwrap();
    }
    zip.finish().unwrap().into_inner()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn paths() {
        assert_eq!(percent_decode("pic%201.png"), "pic 1.png");
        assert_eq!(percent_decode("100%"), "100%");
        assert_eq!(percent_decode("%D0%B0"), "а");
        assert_eq!(normalize_path("OPS/text/../img/./a.png"), "OPS/img/a.png");
    }
}
