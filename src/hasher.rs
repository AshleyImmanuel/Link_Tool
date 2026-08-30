use sha2::{Digest, Sha256};

/// Compute SHA-256 hash of file content, returned as hex string.
pub fn hash_bytes(data: &[u8]) -> String {
    let mut hasher = Sha256::new();
    hasher.update(data);
    format!("{:x}", hasher.finalize())
}
