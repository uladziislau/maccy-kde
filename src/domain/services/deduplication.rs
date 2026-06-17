use std::collections::hash_map::DefaultHasher;
use std::hash::{Hash, Hasher};

#[allow(dead_code)] // Reserved for future deduplication enhancement
pub struct DeduplicationService;

#[allow(dead_code)]
impl DeduplicationService {
    pub fn compute_text_hash(text: &str) -> u64 {
        let mut hasher = DefaultHasher::new();
        text.hash(&mut hasher);
        hasher.finish()
    }
    
    pub fn compute_bytes_hash(bytes: &[u8]) -> u64 {
        let mut hasher = DefaultHasher::new();
        bytes.hash(&mut hasher);
        hasher.finish()
    }
    
    pub fn is_duplicate_text(current: &str, last: &str) -> bool {
        Self::compute_text_hash(current) == Self::compute_text_hash(last)
    }
    
    pub fn is_duplicate_bytes(current: &[u8], last: &[u8]) -> bool {
        Self::compute_bytes_hash(current) == Self::compute_bytes_hash(last)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_compute_text_hash() {
        let hash1 = DeduplicationService::compute_text_hash("hello");
        let hash2 = DeduplicationService::compute_text_hash("hello");
        let hash3 = DeduplicationService::compute_text_hash("world");
        
        assert_eq!(hash1, hash2);
        assert_ne!(hash1, hash3);
    }

    #[test]
    fn test_is_duplicate_text() {
        assert!(DeduplicationService::is_duplicate_text("same", "same"));
        assert!(!DeduplicationService::is_duplicate_text("same", "different"));
    }

    #[test]
    fn test_is_duplicate_bytes() {
        let bytes1 = b"test data";
        let bytes2 = b"test data";
        let bytes3 = b"different data";
        
        assert!(DeduplicationService::is_duplicate_bytes(bytes1, bytes2));
        assert!(!DeduplicationService::is_duplicate_bytes(bytes1, bytes3));
    }
}