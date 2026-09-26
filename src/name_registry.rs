use std::collections::HashMap;

use crate::string_utils::normalize;
/// Simple registry that maps window_id -> normalized character name and provides
/// matching (exact, substring, fuzzy) against OCR text.
pub struct NameRegistry {
    by_window: HashMap<u32, String>,
}

impl NameRegistry {
    pub fn new() -> Self {
        Self { by_window: HashMap::new() }
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

    /// Find best matching window for the given OCR text.
    /// Strategy: exact match -> substring -> fuzzy (levenshtein <= 2)
    pub fn find_best_match(&self, ocr_text: &str) -> Option<u32> {
        let norm = normalize(ocr_text);
        if norm.is_empty() {
            return None;
        }

        // exact
        for (w, name) in &self.by_window {
            if &norm == name {
                return Some(*w);
            }
        }

        // substring matches (prefer the one with longest name)
        let mut substring_candidates: Vec<(&u32, &String)> = self
            .by_window
            .iter()
            .filter(|(_, name)| norm.contains(name.as_str()) || name.contains(&norm))
            .collect();
        if substring_candidates.len() == 1 {
            return Some(*substring_candidates[0].0);
        } else if substring_candidates.len() > 1 {
            // pick longest name to reduce ambiguity
            substring_candidates.sort_by_key(|(_, name)| -(name.len() as isize));
            if substring_candidates[0].1.len() != substring_candidates[1].1.len() {
                return Some(*substring_candidates[0].0);
            }
            return None;
        }

        // fuzzy
        let mut best: Option<(u32, usize)> = None; // (window, dist)
        for (w, name) in &self.by_window {
            let d = levenshtein(&norm, name);
            if d <= 2 {
                match best {
                    None => best = Some((*w, d)),
                    Some((_, prev_d)) => {
                        if d < prev_d {
                            best = Some((*w, d));
                        } else if d == prev_d {
                            // tie -> ambiguous
                            best = None;
                        }
                    }
                }
            }
        }

        best.map(|(w, _)| w)
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
            let cost = if a_chars[i - 1] == b_chars[j - 1] { 0 } else { 1 };
            d[i][j] = *[
                d[i - 1][j] + 1,     // deletion
                d[i][j - 1] + 1,     // insertion
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
    fn substring_and_fuzzy() {
        let mut reg = NameRegistry::new();
        reg.set_name(1, "toto");
        reg.set_name(2, "titi");

        assert_eq!(reg.find_best_match("to"), Some(1)); // substring
        assert_eq!(reg.find_best_match("totoa"), Some(1)); // fuzzy (distance 1)
        assert_eq!(reg.find_best_match("tit"), Some(2));
    }
}
