use crate::domain::entities::Category;

pub struct CategoryDetector;

impl CategoryDetector {
    pub fn detect(text: &str) -> Category {
        Category::from_text(text)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_detect_url() {
        assert!(matches!(CategoryDetector::detect("https://example.com"), Category::Url));
        assert!(matches!(CategoryDetector::detect("http://test.org"), Category::Url));
        assert!(matches!(CategoryDetector::detect("www.example.com"), Category::Url));
    }

    #[test]
    fn test_detect_email() {
        assert!(matches!(CategoryDetector::detect("user@example.com"), Category::Email));
        assert!(matches!(CategoryDetector::detect("test.name@domain.org"), Category::Email));
    }

    #[test]
    fn test_detect_account() {
        assert!(matches!(CategoryDetector::detect("@username"), Category::Account));
        assert!(matches!(CategoryDetector::detect("user"), Category::Account));
    }

    #[test]
    fn test_detect_other() {
        assert!(matches!(CategoryDetector::detect("plain text"), Category::Other));
        assert!(matches!(CategoryDetector::detect("12"), Category::Other)); // too short for account
        assert!(matches!(CategoryDetector::detect("text with spaces"), Category::Other));
    }
}