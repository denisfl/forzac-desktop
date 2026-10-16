//! Typographic clean-up of paragraph text, as in the Android app: a non-breaking space before an em dash and after
//! short Russian prepositions and conjunctions, an em dash for a spaced hyphen and for a dialogue hyphen.
//!
//! Works on chars and replaces one char with one char, so char-indexed style ranges stay valid.

const NBSP: char = '\u{a0}';
const EM_DASH: char = '—';
const SHORT_WORDS: &str = "вксуоаияВКСУОАИЯ";

pub fn normalize(c: &mut [char]) {
    let is_dash = |ch: char| ch == '-' || ch == '–';
    if c.len() >= 2 && is_dash(c[0]) && c[1] == ' ' {
        c[0] = EM_DASH;
        c[1] = NBSP;
    }
    for i in 1..c.len() {
        if is_dash(c[i]) && c[i - 1] == ' ' && i + 1 < c.len() && c[i + 1] == ' ' {
            c[i] = EM_DASH;
        }
        if c[i] == EM_DASH && c[i - 1] == ' ' {
            c[i - 1] = NBSP;
        }
        if c[i] == ' ' && SHORT_WORDS.contains(c[i - 1]) && (i == 1 || !c[i - 2].is_alphabetic()) {
            c[i] = NBSP;
        }
    }
}

#[cfg(test)]
mod tests {
    fn run(s: &str) -> String {
        let mut c: Vec<char> = s.chars().collect();
        super::normalize(&mut c);
        c.into_iter().collect()
    }

    #[test]
    fn dashes_and_short_words() {
        assert_eq!(run("- Да, - сказал он и ушёл в лес"), "—\u{a0}Да,\u{a0}— сказал он и\u{a0}ушёл в\u{a0}лес");
    }
}
