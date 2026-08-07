use hmac::{Hmac, Mac};
use sha2::Sha256;

type HmacSha256 = Hmac<Sha256>;

pub fn compute_device_key_hash(device_key: &str, secret_salt: &str) -> Vec<u8> {
    let mut mac = HmacSha256::new_from_slice(secret_salt.as_bytes())
        .expect("HMAC can take key of any size");
    mac.update(device_key.as_bytes());
    mac.finalize().into_bytes().to_vec()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_compute_device_key_hash() {
        let hash1 = compute_device_key_hash("device-123", "secret-salt");
        let hash2 = compute_device_key_hash("device-123", "secret-salt");
        let hash3 = compute_device_key_hash("device-456", "secret-salt");

        assert_eq!(hash1, hash2);
        assert_ne!(hash1, hash3);
        assert_eq!(hash1.len(), 32); // HMAC-SHA256 generates 32 bytes
    }
}
