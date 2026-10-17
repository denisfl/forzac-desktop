//! Selecting text by words over the words a page (or a column stretch) shows (`ChapterLayout::words`): hit-testing,
//! moving a word cursor by words and lines, and the range between two words. Both shells use these.

use crate::layout::Word;
use crate::model::Locator;

/// The word under `(x, y)`: on the line the point is on (else the nearest line), the piece containing `x` or the nearest.
pub fn word_at(words: &[Word], x: f32, y: f32) -> Option<usize> {
    let line_distance = |w: &Word| if y < w.rect.y { w.rect.y - y } else { (y - (w.rect.y + w.rect.h)).max(0.0) };
    let nearest_line = words.iter().map(line_distance).fold(f32::INFINITY, f32::min);
    let x_distance = |w: &Word| if x < w.rect.x { w.rect.x - x } else { (x - (w.rect.x + w.rect.w)).max(0.0) };
    (0..words.len())
        .filter(|&i| line_distance(&words[i]) <= nearest_line + 0.5)
        .min_by(|&a, &b| x_distance(&words[a]).total_cmp(&x_distance(&words[b])))
}

/// The next (`delta` 1) or previous (−1) word after piece `i`, skipping the other pieces of the same word; `None` past
/// the ends.
pub fn step(words: &[Word], i: usize, delta: i32) -> Option<usize> {
    let start = words.get(i)?.start;
    if delta > 0 {
        (i + 1..words.len()).find(|&j| words[j].start != start)
    } else {
        // The first piece of the previous word.
        let j = (0..i).rev().find(|&j| words[j].start != start)?;
        (0..=j).find(|&k| words[k].start == words[j].start)
    }
}

/// The piece on the next (`delta` 1) or previous (−1) line whose middle is nearest piece `i`'s; `None` past the ends.
pub fn line_step(words: &[Word], i: usize, delta: i32) -> Option<usize> {
    let w = words.get(i)?;
    let middle = w.rect.x + w.rect.w / 2.0;
    // Lines in reading order: the next line is the nearest one below (above) the piece's.
    let line = if delta > 0 {
        words.iter().map(|o| o.rect.y).filter(|&y| y > w.rect.y + 0.5).fold(f32::INFINITY, f32::min)
    } else {
        words.iter().map(|o| o.rect.y).filter(|&y| y < w.rect.y - 0.5).fold(f32::NEG_INFINITY, f32::max)
    };
    let distance = |o: &Word| (o.rect.x + o.rect.w / 2.0 - middle).abs();
    (0..words.len()).filter(|&j| (words[j].rect.y - line).abs() < 0.5).min_by(|&a, &b| distance(&words[a]).total_cmp(&distance(&words[b])))
}

/// The first piece of the word that starts at `at`, else of the first word after it.
pub fn find(words: &[Word], at: Locator) -> Option<usize> {
    words.iter().position(|w| w.start == at).or_else(|| words.iter().position(|w| w.start > at))
}

/// The text from the earlier of two words' starts to the later one's end.
pub fn span(a: (Locator, Locator), b: (Locator, Locator)) -> (Locator, Locator) {
    (a.0.min(b.0), a.1.max(b.1))
}

#[cfg(test)]
mod tests;
