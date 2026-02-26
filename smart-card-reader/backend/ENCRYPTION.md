# PII Data Encryption Guide

This guide explains how to enable and use PII (Personally Identifiable Information) data encryption for the Smart Card Reader application.

## Overview

The application uses **AES-256-GCM** (Advanced Encryption Standard with Galois/Counter Mode) to encrypt sensitive data before transmission over WebSocket. This provides:

- **Confidentiality**: Data is unreadable without the encryption key
- **Authentication**: Tampering is detected through authentication tags
- **Industry Standard**: AES-256-GCM is widely used and trusted

## Quick Start

### 1. Generate Encryption Key

Generate a secure random 256-bit key:

```bash
# Option 1: Using OpenSSL (Recommended)
export ENCRYPTION_KEY=$(openssl rand -base64 32)

# Option 2: Using built-in generator (requires compiling with --features cli)
# cargo run --features cli -- generate-key
```

### 2. Enable Encryption

Update `config.toml`:

```toml
[security]
enable_encryption = true
encrypted_fields = [
    "Citizenid",
    "Th_Firstname",
    "Th_Lastname",
    "full_name_en",
    "Address",
]
```

### 3. Start Application

```bash
# Set encryption key environment variable
export ENCRYPTION_KEY="your-generated-key-here"

# Start the application
cargo run --release
```

## Configuration

### Environment Variables

| Variable | Required | Description |
|----------|----------|-------------|
| `ENCRYPTION_KEY` | Yes (if encryption enabled) | Base64-encoded 32-byte AES-256 key |

### Config File (`config.toml`)

```toml
[security]
# Enable encryption
enable_encryption = true

# Specify fields to encrypt
# Empty list = encrypt all fields (not recommended)
encrypted_fields = [
    "Citizenid",        # Thai citizen ID number
    "Th_Firstname",     # Thai first name
    "Th_Lastname",      # Thai last name
    "full_name_en",     # English full name
    "Address",          # Full address
]
```

## Encrypted Data Format

Encrypted data is transmitted as base64-encoded string containing:
- **Nonce** (12 bytes): Random value for each encryption
- **Ciphertext**: Encrypted data with authentication tag

Example JSON response with encryption enabled:

```json
{
  "mode": "readsmartcard",
  "Citizenid": "AgMEBQYHCAkKCwwNDg8QERITFBUWF...",  // Encrypted (base64)
  "Th_Firstname": "BQYHCAkKCwwNDg8QERITFBUWGB...",  // Encrypted (base64)
  "Birthday": "1990-01-15",                          // Not encrypted
  "Sex": "M",                                         // Not encrypted
  "PhotoRaw": "data:image/jpeg;base64,/9j/4AAQ..." // Not encrypted
}
```

## Client-Side Decryption

### Node.js Example

```javascript
const crypto = require('crypto');

function decryptField(encryptedBase64, keyBase64) {
  const key = Buffer.from(keyBase64, 'base64');
  const combined = Buffer.from(encryptedBase64, 'base64');

  // Split nonce and ciphertext
  const nonce = combined.slice(0, 12);
  const ciphertext = combined.slice(12);

  // Decrypt with AES-256-GCM
  const decipher = crypto.createDecipheriv('aes-256-gcm', key, nonce);
  const authTag = ciphertext.slice(-16);
  decipher.setAuthTag(authTag);

  let plaintext = decipher.update(ciphertext.slice(0, -16), null, 'utf8');
  plaintext += decipher.final('utf8');

  return plaintext;
}

// Usage
const citizenId = decryptField(data.Citizenid, process.env.ENCRYPTION_KEY);
console.log('Citizen ID:', citizenId);
```

### Browser JavaScript Example

```javascript
async function decryptField(encryptedBase64, keyBase64) {
  // Convert base64 to ArrayBuffer
  const combined = Uint8Array.from(atob(encryptedBase64), c => c.charCodeAt(0));
  const key = Uint8Array.from(atob(keyBase64), c => c.charCodeAt(0));

  // Split nonce and ciphertext
  const nonce = combined.slice(0, 12);
  const ciphertext = combined.slice(12);

  // Import key
  const cryptoKey = await crypto.subtle.importKey(
    'raw', key, { name: 'AES-GCM' }, false, ['decrypt']
  );

  // Decrypt
  const plaintext = await crypto.subtle.decrypt(
    { name: 'AES-GCM', iv: nonce },
    cryptoKey,
    ciphertext
  );

  // Convert to string
  return new TextDecoder().decode(plaintext);
}

// Usage
const citizenId = await decryptField(data.Citizenid, ENCRYPTION_KEY);
console.log('Citizen ID:', citizenId);
```

### Python Example

```python
import base64
from cryptography.hazmat.primitives.ciphers.aead import AESGCM

def decrypt_field(encrypted_base64: str, key_base64: str) -> str:
    key = base64.b64decode(key_base64)
    combined = base64.b64decode(encrypted_base64)

    # Split nonce and ciphertext
    nonce = combined[:12]
    ciphertext = combined[12:]

    # Decrypt
    aesgcm = AESGCM(key)
    plaintext = aesgcm.decrypt(nonce, ciphertext, None)

    return plaintext.decode('utf-8')

# Usage
citizen_id = decrypt_field(data['Citizenid'], os.environ['ENCRYPTION_KEY'])
print(f'Citizen ID: {citizen_id}')
```

## Security Best Practices

### Key Management

1. **Generate Strong Keys**
   ```bash
   openssl rand -base64 32  # Generate cryptographically secure random key
   ```

2. **Secure Storage**
   - Store keys in secure secret management systems (AWS Secrets Manager, HashiCorp Vault, etc.)
   - Never commit keys to version control
   - Use environment variables for runtime configuration

3. **Key Rotation**
   - Rotate encryption keys periodically (e.g., every 90 days)
   - Maintain old keys temporarily for decrypting existing data
   - Update all clients with new keys during rotation

### Production Deployment

```bash
# Using AWS Secrets Manager
export ENCRYPTION_KEY=$(aws secretsmanager get-secret-value \
  --secret-id smart-card-encryption-key \
  --query SecretString --output text)

# Using Docker secrets
docker run -e ENCRYPTION_KEY_FILE=/run/secrets/encryption_key \
  smart-card-reader:latest

# Using Kubernetes secrets
kubectl create secret generic encryption-key \
  --from-literal=key=$(openssl rand -base64 32)
```

### Access Control

- Limit access to encryption keys to authorized personnel only
- Use separate keys for development, staging, and production environments
- Implement audit logging for key access
- Enable TLS/SSL for WebSocket connections (wss://)
- Enable WebSocket authentication with API keys

## Troubleshooting

### Error: ENCRYPTION_KEY environment variable not set

```
❌ Encryption enabled but failed to initialize: ENCRYPTION_KEY environment variable not set
   Set ENCRYPTION_KEY environment variable:
   export ENCRYPTION_KEY=$(openssl rand -base64 32)
```

**Solution**: Generate and set the encryption key:

```bash
export ENCRYPTION_KEY=$(openssl rand -base64 32)
```

### Error: Invalid key size

```
❌ Encryption enabled but failed to initialize: Invalid key size: expected 32 bytes, got 16
```

**Solution**: Ensure key is 32 bytes (256 bits) when base64-decoded:

```bash
# Generate correct size key
openssl rand -base64 32
```

### Error: Decryption failed

```
❌ Failed to encrypt field 'Citizenid': Decryption failed
```

**Possible causes**:
- Wrong encryption key on client side
- Corrupted encrypted data during transmission
- Key mismatch between server and client

**Solution**:
- Verify encryption key matches on both server and client
- Check WebSocket connection integrity
- Enable TLS/SSL to prevent tampering

## Performance Considerations

- **Encryption overhead**: ~0.1-0.5ms per field (negligible for typical use cases)
- **Base64 encoding**: Increases data size by ~33%
- **Recommended**: Encrypt only sensitive fields (not all fields)

## Compliance

This encryption implementation helps meet:
- **GDPR**: Encryption of personal data in transit
- **HIPAA**: Protected Health Information (PHI) encryption requirements
- **PDPA (Thailand)**: Personal data protection compliance

## Security Checklist

Production deployment checklist:

- [ ] Generate strong encryption key (32 bytes, cryptographically random)
- [ ] Store key in secure secret management system
- [ ] Enable encryption in config.toml
- [ ] Specify only necessary fields for encryption
- [ ] Enable TLS/SSL (wss://) for WebSocket connections
- [ ] Enable WebSocket authentication
- [ ] Restrict CORS origins
- [ ] Implement key rotation policy
- [ ] Set up audit logging
- [ ] Test decryption on client side
- [ ] Document key management procedures
- [ ] Train operations team on key handling

## Additional Resources

- [AES-GCM Specification (NIST SP 800-38D)](https://nvlpubs.nist.gov/nistpubs/Legacy/SP/nistspecialpublication800-38d.pdf)
- [OWASP Cryptographic Storage Cheat Sheet](https://cheatsheetseries.owasp.org/cheatsheets/Cryptographic_Storage_Cheat_Sheet.html)
- [Rust aes-gcm crate documentation](https://docs.rs/aes-gcm/)
