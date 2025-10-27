//! File system utilities
//!
//! Helper functions for file system operations

use std::path::Path;

/// Generate a URL-safe slug from a string
pub fn slugify(text: &str) -> String {
    text.to_lowercase()
        .chars()
        .map(|c| {
            if c.is_alphanumeric() || c == '-' {
                c
            } else {
                '-'
            }
        })
        .collect::<String>()
        .split('-')
        .filter(|s| !s.is_empty())
        .collect::<Vec<&str>>()
        .join("-")
}

/// Check if a directory is a valid .chikn project
pub fn is_chikn_project(path: &Path) -> bool {
    path.is_dir() && path.join("project.yaml").exists()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_slugify() {
        assert_eq!(slugify("Hello World"), "hello-world");
        assert_eq!(slugify("Chapter 01 - Opening"), "chapter-01-opening");
        assert_eq!(slugify("Test  Multiple   Spaces"), "test-multiple-spaces");
    }
}
