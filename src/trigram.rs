use std::collections::HashSet;

/// Generate trigrams from a string
///
/// Trigrams are 3-character sequences used for fuzzy matching.
/// For example, the word "apple" generates trigrams: "$$a", "$ap", "app", "ppl", "ple", "le$"
/// where $ represents the start/end of the word.
pub fn generate_trigrams(text: &str) -> HashSet<String> {
    let normalized = normalize_text(text);
    
    if normalized.is_empty() {
        return HashSet::new();
    }
    
    let padded = format!("$${normalized}$");
    let chars: Vec<char> = padded.chars().collect();
    
    let mut trigrams = HashSet::new();
    
    for i in 0..chars.len().saturating_sub(2) {
        let trigram = format!("{}{}{}", chars[i], chars[i + 1], chars[i + 2]);
        trigrams.insert(trigram);
    }
    
    trigrams
}

/// Normalize text for indexing and searching
///
/// This function:
/// 1. Converts text to lowercase
/// 2. Removes punctuation
/// 3. Trims whitespace
pub fn normalize_text(text: &str) -> String {
    text.chars()
        .filter(|c| c.is_alphanumeric() || c.is_whitespace())
        .collect::<String>()
        .to_lowercase()
        .trim()
        .to_string()
}

/// Extract tokens (words) from text
///
/// This function:
/// 1. Normalizes the text
/// 2. Splits it into words
/// 3. Filters out empty tokens
pub fn extract_tokens(text: &str) -> Vec<String> {
    normalize_text(text)
        .split_whitespace()
        .map(|s| s.to_string())
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_normalize_text() {
        assert_eq!(normalize_text("Hello, World!"), "hello world");
        assert_eq!(normalize_text("  Test-123  "), "test123");
        assert_eq!(normalize_text(""), "");
    }

    #[test]
    fn test_extract_tokens() {
        assert_eq!(
            extract_tokens("Hello, World!"),
            vec!["hello", "world"]
        );
        assert_eq!(
            extract_tokens("  Multiple   Spaces  "),
            vec!["multiple", "spaces"]
        );
        assert_eq!(extract_tokens(""), Vec::<String>::new());
    }

    #[test]
    fn test_generate_trigrams() {
        let trigrams = generate_trigrams("apple");
        assert!(trigrams.contains("$$a"));
        assert!(trigrams.contains("$ap"));
        assert!(trigrams.contains("app"));
        assert!(trigrams.contains("ppl"));
        assert!(trigrams.contains("ple"));
        assert!(trigrams.contains("le$"));
        assert_eq!(trigrams.len(), 6);

        // Empty string should return empty set
        assert_eq!(generate_trigrams("").len(), 0);
        
        // Short strings
        assert_eq!(generate_trigrams("a").len(), 2); // $$a, $a$
        assert_eq!(generate_trigrams("ab").len(), 3); // $$a, $ab, ab$
    }
}
