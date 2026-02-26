# TLS/SSL Certificates for Smart Card Reader

This directory contains TLS/SSL certificates for secure WebSocket (wss://) connections.

## ⚠️ IMPORTANT SECURITY NOTES

- **NEVER commit real certificates or private keys to version control**
- Add `*.pem`, `*.key`, `*.crt` to `.gitignore`
- For production, use certificates from a trusted Certificate Authority (CA)
- Self-signed certificates are **ONLY for development and testing**

## Quick Start - Self-Signed Certificate for Development

### Option 1: Using OpenSSL (Recommended)

Generate a self-signed certificate valid for 365 days:

```bash
# Generate private key and certificate in one command
openssl req -x509 -newkey rsa:4096 -keyout key.pem -out cert.pem -days 365 -nodes \
  -subj "/CN=localhost/O=SmartCardReader/C=TH"
```

### Option 2: Using mkcert (Easy, trusted by system)

Install mkcert:
```bash
# macOS
brew install mkcert

# Install local CA
mkcert -install

# Generate certificate
mkcert -key-file key.pem -cert-file cert.pem localhost 127.0.0.1 ::1
```

## Configuration

Update `config.toml`:

```toml
[server]
enable_tls = true
tls_cert_path = "certs/cert.pem"
tls_key_path = "certs/key.pem"
```

## Production Setup

For production, obtain certificates from:
- [Let's Encrypt](https://letsencrypt.org/) (Free, automated)
- Commercial CA (DigiCert, GlobalSign, etc.)
- Your organization's internal CA

### Let's Encrypt Example

```bash
# Install certbot
sudo apt-get install certbot

# Generate certificate
sudo certbot certonly --standalone -d your-domain.com

# Certificates will be at:
# /etc/letsencrypt/live/your-domain.com/fullchain.pem
# /etc/letsencrypt/live/your-domain.com/privkey.pem
```

## Testing TLS Connection

Test with `wscat`:

```bash
# Install wscat
npm install -g wscat

# Test wss:// connection (skip certificate verification for self-signed)
wscat -c wss://localhost:8182 --no-check

# With API key authentication
wscat -c wss://localhost:8182 -H "X-API-Key: your-key-here" --no-check
```

## File Permissions (Production)

Ensure proper file permissions:

```bash
chmod 600 key.pem     # Private key - read/write owner only
chmod 644 cert.pem    # Certificate - readable by all
```

## Troubleshooting

### Certificate Not Found Error
```
❌ Failed to load TLS config: Failed to open certificate file 'certs/cert.pem'
```
**Solution**: Generate certificates using instructions above

### Invalid Certificate Error
```
❌ Failed to load TLS config: Failed to parse certificates
```
**Solution**: Ensure certificate is in PEM format, regenerate if corrupted

### Browser Security Warning
**Expected for self-signed certificates**. Click "Advanced" → "Proceed to localhost" (development only)

## Certificate Renewal

Self-signed certificates expire. Regenerate before expiration:

```bash
# Check expiration date
openssl x509 -in cert.pem -noout -enddate

# Regenerate if needed
openssl req -x509 -newkey rsa:4096 -keyout key.pem -out cert.pem -days 365 -nodes \
  -subj "/CN=localhost/O=SmartCardReader/C=TH"
```

## Security Checklist for Production

- [ ] Use certificates from trusted CA
- [ ] Enable TLS 1.2 or higher only
- [ ] Implement certificate pinning (optional, high security)
- [ ] Set up automatic certificate renewal
- [ ] Monitor certificate expiration
- [ ] Secure private key storage (HSM for critical systems)
- [ ] Enable CORS origin restriction
- [ ] Enable WebSocket authentication
- [ ] Regular security audits
