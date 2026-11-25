use std::collections::HashMap;

use crate::string_utils::normalize;
/// Simple registry that maps window_id -> normalized character name and provides
/// matching (exact, substring, fuzzy) against OCR text.
pub struct NameRegistry {
    by_window: HashMap<u32, String>,
}

impl NameRegistry {
    pub fn new() -> Self {
        Self {
            by_window: HashMap::new(),
        }
    }

    pub fn set_name(&mut self, window: u32, name: &str) {
        let norm = normalize(name);
        if norm.is_empty() {
            self.by_window.remove(&window);
        } else {
            self.by_window.insert(window, norm);
        }
    }

    pub fn remove(&mut self, window: u32) {
        self.by_window.remove(&window);
    }

    /// Find a unique character match. Short OCR fragments are ignored so a
    /// partial word cannot accidentally select a similarly named character.
    pub fn find_best_match(&self, ocr_text: &str) -> Option<u32> {
        let norm = normalize(ocr_text);
        if norm.is_empty() {
            return None;
        }

        let mut candidates: Vec<(u32, usize)> = self
            .by_window
            .iter()
            .filter_map(|(window, name)| {
                let ocr_len = norm.chars().count();
                let name_len = name.chars().count();
                let distance = if norm == *name || norm.contains(name) {
                    0
                } else if ocr_len >= 4 && name.contains(&norm) {
                    // OCR may return a truncated suffix, but never trust a tiny fragment.
                    name_len.saturating_sub(ocr_len)
                } else if ocr_len >= 4 && name_len >= 4 {
                    levenshtein(&norm, name)
                } else {
                    return None;
                };
                let max_distance = if name_len >= 8 { 2 } else { 1 };
                (distance <= max_distance).then_some((*window, distance))
            })
            .collect();
        candidates.sort_by_key(|(_, distance)| *distance);
        match candidates.as_slice() {
            [(window, _)] => Some(*window),
            [(window, best), (_, next), ..] if best < next => Some(*window),
            _ => None,
        }
    }
}

fn levenshtein(a: &str, b: &str) -> usize {
    let a_chars: Vec<char> = a.chars().collect();
    let b_chars: Vec<char> = b.chars().collect();
    let al = a_chars.len();
    let bl = b_chars.len();
    if al == 0 {
        return bl;
    }
    if bl == 0 {
        return al;
    }

    let mut d = vec![vec![0usize; bl + 1]; al + 1];
    for i in 0..=al {
        d[i][0] = i;
    }
    for j in 0..=bl {
        d[0][j] = j;
    }

    for i in 1..=al {
        for j in 1..=bl {
            let cost = if a_chars[i - 1] == b_chars[j - 1] {
                0
            } else {
                1
            };
            d[i][j] = *[
                d[i - 1][j] + 1,        // deletion
                d[i][j - 1] + 1,        // insertion
                d[i - 1][j - 1] + cost, // substitution
            ]
            .iter()
            .min()
            .unwrap();
        }
    }

    d[al][bl]
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn simple_match_exact() {
        let mut reg = NameRegistry::new();
        reg.set_name(0x10, "toto");
        assert_eq!(reg.find_best_match("toto"), Some(0x10));
    }

    #[test]
    fn ignores_short_fragments_and_accepts_clear_matches() {
        let mut reg = NameRegistry::new();
        reg.set_name(1, "toto");
        reg.set_name(2, "titi");

        assert_eq!(reg.find_best_match("to"), None);
        assert_eq!(reg.find_best_match("totoa"), Some(1));
        assert_eq!(reg.find_best_match("tit"), None);
        assert_eq!(reg.find_best_match("titi"), Some(2));
    }
}
