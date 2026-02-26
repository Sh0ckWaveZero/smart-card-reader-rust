//! Cryptography module for PII data encryption
//!
//! Provides AES-256-GCM authenticated encryption for sensitive personally
//! identifiable information (PII) before transmission over WebSocket.

#[cfg(test)]
use aes_gcm::Nonce;
use aes_gcm::{
    aead::{Aead, AeadCore, KeyInit, OsRng},
    Aes256Gcm, Key,
};
use base64::{engine::general_purpose::STANDARD as BASE64, Engine};

/// Size of AES-256 key in bytes
const KEY_SIZE: usize = 32;

/// Size of GCM nonce in bytes
#[cfg(test)]
const NONCE_SIZE: usize = 12;

/// Encrypted data wrapper containing nonce and ciphertext
#[derive(Debug, Clone)]
pub struct EncryptedData {
    /// Random nonce used for this encryption (12 bytes)
    pub nonce: Vec<u8>,
    /// Encrypted data with authentication tag
    pub ciphertext: Vec<u8>,
}

impl EncryptedData {
    /// Encode to base64 format: nonce||ciphertext
    #[must_use]
    pub fn to_base64(&self) -> String {
        let mut combined = self.nonce.clone();
        combined.extend_from_slice(&self.ciphertext);
        BASE64.encode(combined)
    }

    /// Decode from base64 format: nonce||ciphertext
    ///
    /// # Errors
    /// Returns error if base64 decoding fails or data is too short
    #[cfg(test)]
    pub fn from_base64(encoded: &str) -> anyhow::Result<Self> {
        let combined = BASE64
            .decode(encoded)
            .map_err(|e| anyhow::anyhow!("Invalid base64: {}", e))?;

        if combined.len() < NONCE_SIZE {
            anyhow::bail!("Encrypted data too short");
        }

        let (nonce, ciphertext) = combined.split_at(NONCE_SIZE);
        Ok(Self {
            nonce: nonce.to_vec(),
            ciphertext: ciphertext.to_vec(),
        })
    }
}

/// PII encryption service using AES-256-GCM
pub struct CryptoService {
    cipher: Aes256Gcm,
}

impl CryptoService {
    /// Create new crypto service with encryption key
    ///
    /// # Arguments
    /// * `key_bytes` - 32-byte encryption key (AES-256)
    ///
    /// # Errors
    /// Returns error if key length is not 32 bytes
    pub fn new(key_bytes: &[u8]) -> anyhow::Result<Self> {
        if key_bytes.len() != KEY_SIZE {
            anyhow::bail!(
                "Invalid key size: expected {} bytes, got {}",
                KEY_SIZE,
                key_bytes.len()
            );
        }

        let key = Key::<Aes256Gcm>::from_slice(key_bytes);
        let cipher = Aes256Gcm::new(key);

        Ok(Self { cipher })
    }

    /// Create crypto service from base64-encoded key
    ///
    /// # Errors
    /// Returns error if base64 decoding fails or key size is invalid
    pub fn from_base64_key(key_b64: &str) -> anyhow::Result<Self> {
        let key_bytes = BASE64
            .decode(key_b64)
            .map_err(|e| anyhow::anyhow!("Invalid base64 key: {}", e))?;
        Self::new(&key_bytes)
    }

    /// Create crypto service from environment variable
    ///
    /// Reads encryption key from `ENCRYPTION_KEY` environment variable.
    ///
    /// # Errors
    /// Returns error if env var not found or key is invalid
    pub fn from_env() -> anyhow::Result<Self> {
        let key_b64 = std::env::var("ENCRYPTION_KEY")
            .map_err(|_| anyhow::anyhow!("ENCRYPTION_KEY environment variable not set"))?;
        Self::from_base64_key(&key_b64)
    }

    /// Encrypt plaintext data
    ///
    /// # Arguments
    /// * `plaintext` - Data to encrypt
    ///
    /// # Returns
    /// Encrypted data with random nonce
    ///
    /// # Errors
    /// Returns error if encryption fails
    pub fn encrypt(&self, plaintext: &str) -> anyhow::Result<EncryptedData> {
        // Generate random nonce (12 bytes for GCM)
        let nonce = Aes256Gcm::generate_nonce(&mut OsRng);

        // Encrypt with authentication
        let ciphertext = self
            .cipher
            .encrypt(&nonce, plaintext.as_bytes())
            .map_err(|e| anyhow::anyhow!("Encryption failed: {}", e))?;

        Ok(EncryptedData {
            nonce: nonce.to_vec(),
            ciphertext,
        })
    }

    /// Decrypt encrypted data
    ///
    /// # Arguments
    /// * `encrypted` - Encrypted data with nonce
    ///
    /// # Returns
    /// Original plaintext
    ///
    /// # Errors
    /// Returns error if decryption or authentication fails
    #[cfg(test)]
    pub fn decrypt(&self, encrypted: &EncryptedData) -> anyhow::Result<String> {
        let nonce = Nonce::from_slice(&encrypted.nonce);

        let plaintext = self
            .cipher
            .decrypt(nonce, encrypted.ciphertext.as_ref())
            .map_err(|e| anyhow::anyhow!("Decryption failed: {}", e))?;

        String::from_utf8(plaintext).map_err(|e| anyhow::anyhow!("Invalid UTF-8: {}", e))
    }

    /// Encrypt and encode to base64 in one step
    ///
    /// # Arguments
    /// * `plaintext` - Data to encrypt
    ///
    /// # Returns
    /// Base64-encoded encrypted data
    pub fn encrypt_to_base64(&self, plaintext: &str) -> anyhow::Result<String> {
        let encrypted = self.encrypt(plaintext)?;
        Ok(encrypted.to_base64())
    }

    /// Decrypt from base64-encoded data in one step
    ///
    /// # Arguments
    /// * `encoded` - Base64-encoded encrypted data
    ///
    /// # Returns
    /// Original plaintext
    #[cfg(test)]
    pub fn decrypt_from_base64(&self, encoded: &str) -> anyhow::Result<String> {
        let encrypted = EncryptedData::from_base64(encoded)?;
        self.decrypt(&encrypted)
    }
}

/// Generate a new random 256-bit encryption key
///
/// # Returns
/// Base64-encoded encryption key suitable for use with `CryptoService`
#[must_use]
#[cfg(test)]
pub fn generate_key() -> String {
    let key = Aes256Gcm::generate_key(&mut OsRng);
    BASE64.encode(key)
}

// ============================================================================
// Tests
// ============================================================================

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_encrypt_decrypt() {
        let key = Aes256Gcm::generate_key(&mut OsRng);
        let crypto = CryptoService::new(&key).unwrap();

        let plaintext = "1234567890123";
        let encrypted = crypto.encrypt(plaintext).unwrap();
        let decrypted = crypto.decrypt(&encrypted).unwrap();

        assert_eq!(plaintext, decrypted);
    }

    #[test]
    fn test_encrypt_decrypt_base64() {
        let key = Aes256Gcm::generate_key(&mut OsRng);
        let crypto = CryptoService::new(&key).unwrap();

        let plaintext = "นายทดสอบ ระบบ";
        let encoded = crypto.encrypt_to_base64(plaintext).unwrap();
        let decrypted = crypto.decrypt_from_base64(&encoded).unwrap();

        assert_eq!(plaintext, decrypted);
    }

    #[test]
    fn test_from_base64_key() {
        let key_b64 = generate_key();
        let crypto = CryptoService::from_base64_key(&key_b64).unwrap();

        let plaintext = "test data";
        let encoded = crypto.encrypt_to_base64(plaintext).unwrap();
        let decrypted = crypto.decrypt_from_base64(&encoded).unwrap();

        assert_eq!(plaintext, decrypted);
    }

    #[test]
    fn test_invalid_key_size() {
        let short_key = vec![0u8; 16]; // Only 16 bytes
        assert!(CryptoService::new(&short_key).is_err());
    }

    #[test]
    fn test_encrypted_data_encoding() {
        let encrypted = EncryptedData {
            nonce: vec![1, 2, 3, 4, 5, 6, 7, 8, 9, 10, 11, 12],
            ciphertext: vec![13, 14, 15, 16],
        };

        let encoded = encrypted.to_base64();
        let decoded = EncryptedData::from_base64(&encoded).unwrap();

        assert_eq!(encrypted.nonce, decoded.nonce);
        assert_eq!(encrypted.ciphertext, decoded.ciphertext);
    }
}
