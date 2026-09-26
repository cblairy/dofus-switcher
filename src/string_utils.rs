pub fn normalize(input: &str) -> String {
    // Lowercase, trim, remove punctuation (keep alphanumeric and spaces), collapse multiple spaces
    let lower = input.to_lowercase();
    let mut out = String::with_capacity(lower.len());

    let mut prev_space = false;
    for ch in lower.chars() {
        if ch.is_alphanumeric() {
            out.push(ch);
            prev_space = false;
        } else if ch.is_whitespace() {
            if !prev_space {
                out.push(' ');
                prev_space = true;
            }
        } else {
            // skip punctuation
        }
    }

    out.trim().to_string()
}

#[cfg(test)]
mod tests {
    use super::normalize;

    #[test]
    fn normalize_basic() {
        assert_eq!(normalize(" Toto "), "toto");
        assert_eq!(normalize("Abc-123!"), "abc123");
        assert_eq!(normalize("A   B"), "a b");
    }
}
