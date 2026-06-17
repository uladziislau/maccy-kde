use serde::{Deserialize, Serialize};

#[derive(Debug, PartialEq, Clone, Serialize, Deserialize)]
pub enum Category {
    Url,
    Email,
    Account,
    Picture,
    Other,
}

impl Category {
    pub fn from_text(text: &str) -> Self {
        // URL detection
        if text.starts_with("http://") || text.starts_with("https://") ||
           text.starts_with("www.") || (text.contains(".") && text.contains("/")) {
            return Category::Url;
        }

        // Email detection
        let email_regex = regex::Regex::new(r"^[a-zA-Z0-9._%+-]+@[a-zA-Z0-9.-]+\.[a-zA-Z]{2,}$").unwrap();
        if email_regex.is_match(text) {
            return Category::Email;
        }

        // Account/Username detection
        let account_regex_with_at = regex::Regex::new(r"^@[a-zA-Z0-9_]+$").unwrap();
        let account_regex_plain = regex::Regex::new(r"^[a-zA-Z0-9_]{3,20}$").unwrap();

        if account_regex_with_at.is_match(text) || (account_regex_plain.is_match(text) && !text.contains(" ") && !text.contains("@")) {
            return Category::Account;
        }

        Category::Other
    }
}

impl std::fmt::Display for Category {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Category::Url => write!(f, "Url"),
            Category::Email => write!(f, "Email"),
            Category::Account => write!(f, "Account"),
            Category::Picture => write!(f, "Picture"),
            Category::Other => write!(f, "Other"),
        }
    }
}