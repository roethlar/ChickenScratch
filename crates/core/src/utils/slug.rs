//! # Slug Utilities
//!
//! Shared slug generation and uniqueness checking.

use crate::models::Document;
use std::collections::HashMap;

/// Slugifies a string for use as a filename.
///
/// Converts "My Chapter Name" to "my-chapter-name"
pub fn slugify(s: &str) -> String {
    s.to_lowercase()
        .chars()
        .map(|c| if c.is_alphanumeric() { c } else { '-' })
        .collect::<String>()
        .split('-')
        .filter(|s| !s.is_empty())
        .collect::<Vec<&str>>()
        .join("-")
}

/// Generates a unique slug by checking existing document paths.
///
/// # Arguments
/// * `name` - Display name to slugify
/// * `base_path` - Base path (e.g., "manuscript/")
/// * `documents` - Existing documents to check against
///
/// # Returns
/// Unique slug with counter suffix if needed
///
/// # Example
/// ```rust
/// let slug = unique_slug("Chapter 1", "manuscript/", &documents);
/// // Returns "chapter-1" or "chapter-1-1" if collision
/// ```
pub fn unique_slug(name: &str, base_path: &str, documents: &HashMap<String, Document>) -> String {
    let mut slug = slugify(name);
    let mut counter = 1;
    let original_slug = slug.clone();

    // Check if path exists
    while documents
        .values()
        .any(|d| d.path == format!("{}{}.html", base_path, slug))
    {
        slug = format!("{}-{}", original_slug, counter);
        counter += 1;
    }

    slug
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_slugify() {
        assert_eq!(slugify("My Chapter Name"), "my-chapter-name");
        assert_eq!(
            slugify("Chapter 1: The Beginning"),
            "chapter-1-the-beginning"
        );
        assert_eq!(slugify("Hello World!!!"), "hello-world");
        assert_eq!(slugify("a--b--c"), "a-b-c");
    }

    #[test]
    fn test_unique_slug_no_collision() {
        let documents = HashMap::new();
        let slug = unique_slug("Chapter 1", "manuscript/", &documents);
        assert_eq!(slug, "chapter-1");
    }

    #[test]
    fn test_unique_slug_with_collision() {
        let mut documents = HashMap::new();

        // Add existing document
        let doc = Document {
            id: "1".to_string(),
            name: "Chapter 1".to_string(),
            path: "manuscript/chapter-1.html".to_string(),
            content: String::new(),
            parent_id: None,
            created: "2025-01-01T00:00:00Z".to_string(),
            modified: "2025-01-01T00:00:00Z".to_string(),
        };
        documents.insert("1".to_string(), doc);

        // Generate unique slug
        let slug = unique_slug("Chapter 1!", "manuscript/", &documents);
        assert_eq!(slug, "chapter-1-1");
    }
}
