//! Liang's hyphenation (the TeX algorithm) on hyph-utf8 patterns: `hyph-<lang>.pat.txt` holds the patterns,
//! `.hyp.txt` the exceptions. Licences sit next to the files in `assets/hyph`.

use std::collections::HashMap;

pub struct Hyphenator {
    /// Pattern letters → weights between them (`letters + 1` long).
    patterns: HashMap<Vec<char>, Vec<u8>>,
    exceptions: HashMap<String, Vec<u8>>,
    max_len: usize,
    /// Minimum letters before and after a break.
    left: usize,
    right: usize,
}

impl Hyphenator {
    pub fn new(patterns: &str, exceptions: &str, left: usize, right: usize) -> Self {
        let mut map = HashMap::new();
        let mut max_len = 0;
        for p in patterns.split_whitespace() {
            let mut letters = Vec::new();
            let mut weights = vec![0u8];
            for c in p.chars() {
                if let Some(d) = c.to_digit(10) {
                    *weights.last_mut().unwrap() = d as u8;
                } else {
                    letters.push(c);
                    weights.push(0);
                }
            }
            max_len = max_len.max(letters.len());
            map.insert(letters, weights);
        }
        let mut exc = HashMap::new();
        for e in exceptions.split_whitespace() {
            let e = e.to_lowercase();
            let mut points = vec![0u8];
            for c in e.chars() {
                if c == '-' {
                    *points.last_mut().unwrap() = 1;
                } else {
                    points.push(0);
                }
            }
            exc.insert(e.replace('-', ""), points);
        }
        Self { patterns: map, exceptions: exc, max_len, left, right }
    }

    /// Patterns bundled for `language` (`ru`, `en`); `None` for other languages and for none (`parse` gives every book
    /// a language, guessed from its text when it names none).
    pub fn for_language(language: Option<&str>) -> Option<Self> {
        let lang = language?.split(['-', '_']).next().unwrap_or("").to_lowercase();
        match lang.as_str() {
            "ru" => {
                Some(Self::new(include_str!("../../assets/hyph/hyph-ru.pat.txt"), include_str!("../../assets/hyph/hyph-ru.hyp.txt"), 2, 2))
            }
            "en" => Some(Self::new(
                include_str!("../../assets/hyph/hyph-en-us.pat.txt"),
                include_str!("../../assets/hyph/hyph-en-us.hyp.txt"),
                2,
                3,
            )),
            _ => None,
        }
    }

    /// Char positions inside `word` before which it may break, ascending.
    pub fn hyphenate(&self, word: &[char]) -> Vec<usize> {
        let n = word.len();
        if n < self.left + self.right {
            return Vec::new();
        }
        let lower: Vec<char> = word.iter().map(|c| c.to_lowercase().next().unwrap_or(*c)).collect();
        let key: String = lower.iter().collect();
        let points: Vec<u8> = match self.exceptions.get(&key) {
            Some(p) => p.clone(),
            None => {
                let mut dotted = Vec::with_capacity(n + 2);
                dotted.push('.');
                dotted.extend_from_slice(&lower);
                dotted.push('.');
                let mut weights = vec![0u8; dotted.len() + 1];
                for start in 0..dotted.len() {
                    for end in start + 1..=dotted.len().min(start + self.max_len) {
                        if let Some(w) = self.patterns.get(&dotted[start..end]) {
                            for (k, &v) in w.iter().enumerate() {
                                weights[start + k] = weights[start + k].max(v);
                            }
                        }
                    }
                }
                // The weight between dotted[i] and dotted[i + 1] sits before word letter i.
                (0..=n).map(|i| weights[i + 1]).collect()
            }
        };
        (self.left..=n - self.right).filter(|&i| points[i] % 2 == 1).collect()
    }
}

#[cfg(test)]
mod tests {
    use super::Hyphenator;

    fn split(h: &Hyphenator, word: &str) -> String {
        let chars: Vec<char> = word.chars().collect();
        let points = h.hyphenate(&chars);
        chars.iter().enumerate().map(|(i, c)| if points.contains(&i) { format!("-{c}") } else { c.to_string() }).collect()
    }

    #[test]
    fn russian_and_english() {
        let ru = Hyphenator::for_language(Some("ru")).unwrap();
        assert_eq!(split(&ru, "распущенность"), "рас-пу-щен-ность");
        assert_eq!(split(&ru, "Переносы"), "Пе-ре-но-сы");
        assert_eq!(split(&ru, "асбест"), "ас-бест"); // exception list
        assert_eq!(split(&ru, "сон"), "сон");
        let en = Hyphenator::for_language(Some("en-GB")).unwrap();
        assert_eq!(split(&en, "hyphenation"), "hy-phen-ation");
        assert!(Hyphenator::for_language(Some("de")).is_none());
        assert!(Hyphenator::for_language(None).is_none());
    }
}
