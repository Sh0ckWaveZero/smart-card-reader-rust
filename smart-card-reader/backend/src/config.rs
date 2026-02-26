//! Configuration module for Thai Smart Card Reader
//!
//! Provides strongly-typed configuration with sensible defaults,
//! loaded from TOML files with fallback to environment variables.

use serde::Deserialize;
use std::collections::HashMap;
use std::net::{IpAddr, Ipv4Addr};
use std::path::{Path, PathBuf};
use std::{fmt, io};

// ============================================================================
// Constants
// ============================================================================

/// Default WebSocket server host
pub const DEFAULT_HOST: IpAddr = IpAddr::V4(Ipv4Addr::new(127, 0, 0, 1));
/// Default WebSocket server port
pub const DEFAULT_PORT: u16 = 8182;
/// Default window title
pub const DEFAULT_WINDOW_TITLE: &str = "Thai Smart Card Reader";
/// Default window dimensions (fixed size - cannot be resized)
pub const DEFAULT_WINDOW_WIDTH: f32 = 1100.0;
pub const DEFAULT_WINDOW_HEIGHT: f32 = 750.0;
pub const DEFAULT_MIN_WIDTH: f32 = 1100.0;
pub const DEFAULT_MIN_HEIGHT: f32 = 750.0;
pub const DEFAULT_MAX_WIDTH: f32 = 1100.0;
pub const DEFAULT_MAX_HEIGHT: f32 = 750.0;
/// Default log level
pub const DEFAULT_LOG_LEVEL: &str = "info";
/// Environment variable for config path
pub const CONFIG_ENV_VAR: &str = "SMART_CARD_CONFIG";
/// Default config filename
pub const CONFIG_FILENAME: &str = "config.toml";

// ============================================================================
// Error Types
// ============================================================================

/// Configuration loading errors
#[derive(Debug)]
pub enum ConfigError {
    /// Failed to read config file
    Io(io::Error),
    /// Failed to parse TOML
    Parse(toml::de::Error),
}

impl fmt::Display for ConfigError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Io(e) => write!(f, "Failed to read config: {e}"),
            Self::Parse(e) => write!(f, "Failed to parse config: {e}"),
        }
    }
}

impl std::error::Error for ConfigError {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        match self {
            Self::Io(e) => Some(e),
            Self::Parse(e) => Some(e),
        }
    }
}

impl From<io::Error> for ConfigError {
    fn from(err: io::Error) -> Self {
        Self::Io(err)
    }
}

impl From<toml::de::Error> for ConfigError {
    fn from(err: toml::de::Error) -> Self {
        Self::Parse(err)
    }
}

// ============================================================================
// Output Format Enum
// ============================================================================

/// Output format for card data
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum OutputFormat {
    /// Standard format with all fields
    #[default]
    Standard,
    /// Minimal format with essential fields only
    Minimal,
    /// Full format with metadata
    Full,
}

impl fmt::Display for OutputFormat {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Standard => write!(f, "standard"),
            Self::Minimal => write!(f, "minimal"),
            Self::Full => write!(f, "full"),
        }
    }
}

// ============================================================================
// Configuration Structs
// ============================================================================

/// Application configuration loaded from config.toml
///
/// # Example
/// ```toml
/// [server]
/// host = "127.0.0.1"
/// port = 8182
///
/// [output]
/// format = "standard"
/// include_photo = true
///
/// [security]
/// enable_authentication = true
/// api_keys = ["your-secret-key-here"]
/// ```
#[derive(Debug, Clone, Deserialize)]
#[serde(default)]
#[non_exhaustive]
pub struct AppConfig {
    /// WebSocket server configuration
    pub server: ServerConfig,
    /// Output format and field mapping
    pub output: OutputConfig,
    /// UI window settings
    pub ui: UiConfig,
    /// Font configuration
    pub fonts: FontConfig,
    /// Logging configuration
    pub logging: LoggingConfig,
    /// Card reading configuration
    pub card: CardConfig,
    /// Security configuration
    pub security: SecurityConfig,
}

impl Default for AppConfig {
    fn default() -> Self {
        Self {
            server: ServerConfig::default(),
            output: OutputConfig::default(),
            ui: UiConfig::default(),
            fonts: FontConfig::default(),
            logging: LoggingConfig::default(),
            card: CardConfig::default(),
            security: SecurityConfig::default(),
        }
    }
}

/// WebSocket server configuration
#[derive(Debug, Clone, Deserialize)]
#[serde(default)]
pub struct ServerConfig {
    /// Server bind address (e.g., "127.0.0.1" or "0.0.0.0")
    #[serde(deserialize_with = "deserialize_ip_addr")]
    pub host: IpAddr,
    /// Server port number
    pub port: u16,
    /// Allow all CORS origins (⚠️ INSECURE: Set to false in production)
    pub cors_allow_all: bool,
    /// Allowed CORS origins (used when cors_allow_all = false)
    /// Example: ["https://your-app.com", "https://localhost:3000"]
    pub allowed_origins: Vec<String>,
    /// Enable TLS/SSL for secure WebSocket (wss://)
    pub enable_tls: bool,
    /// Path to TLS certificate file (.pem or .crt)
    pub tls_cert_path: String,
    /// Path to TLS private key file (.pem or .key)
    pub tls_key_path: String,
}

impl Default for ServerConfig {
    fn default() -> Self {
        Self {
            host: DEFAULT_HOST,
            port: DEFAULT_PORT,
            cors_allow_all: true,
            allowed_origins: vec![
                "http://localhost:3000".to_string(),
                "https://localhost:3000".to_string(),
            ],
            enable_tls: false,
            tls_cert_path: "certs/cert.pem".to_string(),
            tls_key_path: "certs/key.pem".to_string(),
        }
    }
}

impl ServerConfig {
    /// Returns the WebSocket URL for client connections
    #[must_use]
    pub fn websocket_url(&self) -> String {
        let protocol = if self.enable_tls { "wss" } else { "ws" };
        format!("{}://{}:{}", protocol, self.host, self.port)
    }

    /// Returns the socket address for binding
    #[must_use]
    pub fn socket_addr(&self) -> std::net::SocketAddr {
        std::net::SocketAddr::new(self.host, self.port)
    }

    /// Get allowed CORS origins from config or environment variable
    ///
    /// Priority: config.toml > ALLOWED_ORIGINS env var > empty
    ///
    /// # Environment Variable Format
    /// Comma-separated list: `ALLOWED_ORIGINS=http://localhost:3000,https://app.com`
    #[must_use]
    pub fn get_allowed_origins(&self) -> Vec<String> {
        // Use config value if present
        if !self.allowed_origins.is_empty() {
            return self.allowed_origins.clone();
        }

        // Try to read from environment variable
        if let Ok(origins_str) = std::env::var("ALLOWED_ORIGINS") {
            return origins_str
                .split(',')
                .map(|s| s.trim().to_string())
                .filter(|s| !s.is_empty())
                .collect();
        }

        // No origins configured
        Vec::new()
    }
}

impl fmt::Display for ServerConfig {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}:{}", self.host, self.port)
    }
}

/// Output format and field mapping configuration
#[derive(Debug, Clone, Deserialize)]
#[serde(default)]
pub struct OutputConfig {
    /// Output format type
    pub format: OutputFormat,
    /// Include base64-encoded photo in output
    pub include_photo: bool,
    /// Field name mappings (original -> custom)
    pub field_mapping: HashMap<String, String>,
    /// Fields to include (empty = all fields)
    pub enabled_fields: Vec<String>,
}

impl Default for OutputConfig {
    fn default() -> Self {
        Self {
            format: OutputFormat::default(),
            include_photo: true,
            field_mapping: HashMap::new(),
            enabled_fields: Vec::new(),
        }
    }
}

impl OutputConfig {
    /// Checks if a field should be included in output
    ///
    /// Returns `true` if `enabled_fields` is empty (all fields enabled)
    /// or if the field is in the enabled list.
    #[must_use]
    pub fn is_field_enabled(&self, field: &str) -> bool {
        self.enabled_fields.is_empty() || self.enabled_fields.iter().any(|f| f == field)
    }

    /// Returns the output field name (mapped or original)
    #[must_use]
    pub fn get_field_name<'a>(&'a self, original: &'a str) -> &'a str {
        self.field_mapping
            .get(original)
            .map(String::as_str)
            .unwrap_or(original)
    }
}

/// UI window configuration
#[derive(Debug, Clone, Deserialize)]
#[serde(default)]
pub struct UiConfig {
    /// Window title
    pub window_title: String,
    /// Initial window width
    pub window_width: f32,
    /// Initial window height
    pub window_height: f32,
    /// Minimum window width
    pub min_width: f32,
    /// Minimum window height
    pub min_height: f32,
    /// Maximum window width (set equal to min to lock size)
    pub max_width: f32,
    /// Maximum window height (set equal to min to lock size)
    pub max_height: f32,
}

impl Default for UiConfig {
    fn default() -> Self {
        Self {
            window_title: DEFAULT_WINDOW_TITLE.to_owned(),
            window_width: DEFAULT_WINDOW_WIDTH,
            window_height: DEFAULT_WINDOW_HEIGHT,
            min_width: DEFAULT_MIN_WIDTH,
            min_height: DEFAULT_MIN_HEIGHT,
            max_width: DEFAULT_MAX_WIDTH,
            max_height: DEFAULT_MAX_HEIGHT,
        }
    }
}

/// Font configuration
#[derive(Debug, Clone, Deserialize)]
#[serde(default)]
pub struct FontConfig {
    /// Custom font paths (checked first)
    pub custom_paths: Vec<String>,
    /// Search system fonts if custom not found
    pub use_system_fonts: bool,
}

impl Default for FontConfig {
    fn default() -> Self {
        Self {
            custom_paths: Vec::new(),
            use_system_fonts: true,
        }
    }
}

/// Logging configuration
#[derive(Debug, Clone, Deserialize)]
#[serde(default)]
pub struct LoggingConfig {
    /// Log level: trace, debug, info, warn, error
    pub level: String,
}

impl Default for LoggingConfig {
    fn default() -> Self {
        Self {
            level: DEFAULT_LOG_LEVEL.to_owned(),
        }
    }
}

/// Security configuration
#[derive(Debug, Clone, Deserialize)]
#[serde(default)]
pub struct SecurityConfig {
    /// Enable API key authentication for WebSocket connections
    pub enable_authentication: bool,
    /// List of valid API keys (read from environment variable API_KEYS if empty)
    pub api_keys: Vec<String>,
    /// API key header name
    pub api_key_header: String,
    /// Enable PII data encryption before transmission
    pub enable_encryption: bool,
    /// List of field names to encrypt (empty = encrypt all sensitive fields)
    /// Common sensitive fields: Citizenid, Th_Firstname, Th_Lastname, full_name_en, Address
    pub encrypted_fields: Vec<String>,
    /// Enable rate limiting for WebSocket connections
    pub enable_rate_limiting: bool,
    /// Maximum requests per time window (per IP)
    pub rate_limit_requests: u32,
    /// Time window in seconds for rate limiting
    pub rate_limit_window_secs: u64,
    /// Maximum concurrent connections per IP
    pub rate_limit_max_connections: u32,
    /// Enable audit logging for security events
    pub enable_audit_logging: bool,
}

impl Default for SecurityConfig {
    fn default() -> Self {
        Self {
            enable_authentication: false,
            api_keys: Vec::new(),
            api_key_header: "X-API-Key".to_string(),
            enable_encryption: false,
            encrypted_fields: vec![
                "Citizenid".to_string(),
                "Th_Firstname".to_string(),
                "Th_Lastname".to_string(),
                "full_name_en".to_string(),
                "Address".to_string(),
            ],
            enable_rate_limiting: false,
            rate_limit_requests: 60,
            rate_limit_window_secs: 60,
            rate_limit_max_connections: 5,
            enable_audit_logging: false,
        }
    }
}

impl SecurityConfig {
    /// Get API keys from config or environment variable
    #[must_use]
    pub fn get_api_keys(&self) -> Vec<String> {
        if !self.api_keys.is_empty() {
            return self.api_keys.clone();
        }

        // Try to read from environment variable
        if let Ok(keys_str) = std::env::var("API_KEYS") {
            return keys_str
                .split(',')
                .map(|s| s.trim().to_string())
                .filter(|s| !s.is_empty())
                .collect();
        }

        Vec::new()
    }

    /// Validate an API key
    #[must_use]
    pub fn is_valid_key(&self, key: &str) -> bool {
        if !self.enable_authentication {
            return true; // Authentication disabled
        }

        let valid_keys = self.get_api_keys();
        if valid_keys.is_empty() {
            log::warn!("⚠️ Authentication enabled but no API keys configured!");
            return false;
        }

        valid_keys.iter().any(|k| k == key)
    }

    /// Check if a field should be encrypted
    ///
    /// Returns `true` if encryption is enabled and the field is in the encrypted list.
    /// If `encrypted_fields` is empty, all fields are considered for encryption.
    #[must_use]
    pub fn should_encrypt_field(&self, field_name: &str) -> bool {
        if !self.enable_encryption {
            return false;
        }

        // If encrypted_fields is empty, encrypt all fields (not recommended)
        if self.encrypted_fields.is_empty() {
            return true;
        }

        // Check if field is in the encrypted list
        self.encrypted_fields.iter().any(|f| f == field_name)
    }
}

// ============================================================================
// Card Reading Configuration
// ============================================================================

/// APDU command definition for reading card data
#[derive(Debug, Clone, Deserialize)]
pub struct ApduCommand {
    /// Field name for this APDU
    pub name: String,
    /// APDU bytes as hex string (e.g., "80B0000402000D")
    pub apdu: String,
    /// Whether this field is required (reserved for validation logic)
    #[serde(default = "default_true")]
    #[allow(dead_code)]
    pub required: bool,
}

impl ApduCommand {
    /// Parse APDU hex string to bytes
    #[must_use]
    pub fn to_bytes(&self) -> Vec<u8> {
        hex_to_bytes(&self.apdu)
    }
}

/// Card reading configuration with APDU commands
#[derive(Debug, Clone, Deserialize)]
#[serde(default)]
pub struct CardConfig {
    /// SELECT APDU for Thai ID applet (hex string)
    pub select_apdu: String,
    /// Field APDU commands
    pub fields: Vec<ApduCommand>,
    /// Photo chunk APDU commands
    pub photo_chunks: Vec<String>,
    /// Number of connection retry attempts
    pub retry_attempts: u8,
    /// Delay between retries in milliseconds
    pub retry_delay_ms: u64,
    /// Delay after card insertion before reading (ms)
    pub card_settle_delay_ms: u64,
}

fn default_true() -> bool {
    true
}

impl Default for CardConfig {
    fn default() -> Self {
        Self {
            // Thai ID Applet SELECT
            select_apdu: "00A4040008A000000054480001".to_owned(),
            fields: vec![
                ApduCommand {
                    name: "citizen_id".to_owned(),
                    apdu: "80B0000402000D".to_owned(),
                    required: true,
                },
                ApduCommand {
                    name: "full_name_th".to_owned(),
                    apdu: "80B00011020064".to_owned(),
                    required: true,
                },
                ApduCommand {
                    name: "full_name_en".to_owned(),
                    apdu: "80B00075020064".to_owned(),
                    required: true,
                },
                ApduCommand {
                    name: "date_of_birth".to_owned(),
                    apdu: "80B000D9020008".to_owned(),
                    required: true,
                },
                ApduCommand {
                    name: "gender".to_owned(),
                    apdu: "80B000E1020001".to_owned(),
                    required: true,
                },
                ApduCommand {
                    name: "card_issuer".to_owned(),
                    apdu: "80B000F6020064".to_owned(),
                    required: false,
                },
                ApduCommand {
                    name: "issue_date".to_owned(),
                    apdu: "80B00167020008".to_owned(),
                    required: true,
                },
                ApduCommand {
                    name: "expire_date".to_owned(),
                    apdu: "80B0016F020008".to_owned(),
                    required: true,
                },
                ApduCommand {
                    name: "address".to_owned(),
                    apdu: "80B015790200FF".to_owned(),
                    required: false,
                },
            ],
            photo_chunks: vec![
                "80B0017B0200FF".to_owned(),
                "80B0027A0200FF".to_owned(),
                "80B003790200FF".to_owned(),
                "80B004780200FF".to_owned(),
                "80B005770200FF".to_owned(),
                "80B006760200FF".to_owned(),
                "80B007750200FF".to_owned(),
                "80B008740200FF".to_owned(),
                "80B009730200FF".to_owned(),
                "80B00A720200FF".to_owned(),
                "80B00B710200FF".to_owned(),
                "80B00C700200FF".to_owned(),
                "80B00D6F0200FF".to_owned(),
                "80B00E6E0200FF".to_owned(),
                "80B00F6D0200FF".to_owned(),
                "80B0106C0200FF".to_owned(),
                "80B0116B0200FF".to_owned(),
                "80B0126A0200FF".to_owned(),
                "80B013690200FF".to_owned(),
                "80B014680200FF".to_owned(),
            ],
            retry_attempts: 3,
            retry_delay_ms: 500,
            card_settle_delay_ms: 500,
        }
    }
}

impl CardConfig {
    /// Get SELECT APDU as bytes
    #[must_use]
    pub fn select_apdu_bytes(&self) -> Vec<u8> {
        hex_to_bytes(&self.select_apdu)
    }

    /// Get photo chunk APDUs as bytes
    #[must_use]
    pub fn photo_chunk_bytes(&self) -> Vec<Vec<u8>> {
        self.photo_chunks.iter().map(|s| hex_to_bytes(s)).collect()
    }

    /// Get field APDU by name
    #[must_use]
    pub fn get_field(&self, name: &str) -> Option<&ApduCommand> {
        self.fields.iter().find(|f| f.name == name)
    }
}

/// Convert hex string to bytes
fn hex_to_bytes(hex: &str) -> Vec<u8> {
    let hex = hex.replace(' ', "");
    (0..hex.len())
        .step_by(2)
        .filter_map(|i| u8::from_str_radix(&hex[i..i + 2], 16).ok())
        .collect()
}

// ============================================================================
// Custom Deserializers
// ============================================================================

/// Deserialize IP address from string
fn deserialize_ip_addr<'de, D>(deserializer: D) -> Result<IpAddr, D::Error>
where
    D: serde::Deserializer<'de>,
{
    let s = String::deserialize(deserializer)?;
    s.parse().map_err(serde::de::Error::custom)
}

// ============================================================================
// Config Loading Functions
// ============================================================================

/// Loads configuration from default locations
///
/// Search order:
/// 1. Environment variable `SMART_CARD_CONFIG`
/// 2. Working directory `./config.toml`
/// 3. Executable directory `<exe>/config.toml`
/// 4. Default values
///
/// # Returns
/// Configuration with values from file or defaults
#[must_use]
pub fn load() -> AppConfig {
    load_from_path(None).unwrap_or_else(|e| {
        log::warn!("Config error: {e}, using defaults");
        AppConfig::default()
    })
}

/// Loads configuration from a specific path or searches default locations
///
/// # Arguments
/// * `config_path` - Optional explicit config file path
///
/// # Errors
/// Returns `ConfigError` if the specified file cannot be read or parsed
pub fn load_from_path(config_path: Option<&str>) -> Result<AppConfig, ConfigError> {
    let search_paths = build_search_paths(config_path);

    for path in &search_paths {
        if !path.exists() {
            continue;
        }

        log::info!("Loading config from: {}", path.display());

        match load_from_file(path) {
            Ok(config) => {
                log::info!("Configuration loaded successfully");
                return Ok(config);
            }
            Err(e) => {
                log::warn!("Failed to load config from {}: {e}", path.display());
            }
        }
    }

    log::info!("Using default configuration");
    Ok(AppConfig::default())
}

/// Loads and parses configuration from a specific file
///
/// # Errors
/// Returns `ConfigError` if the file cannot be read or parsed
pub fn load_from_file(path: &Path) -> Result<AppConfig, ConfigError> {
    let content = std::fs::read_to_string(path)?;
    let config: AppConfig = toml::from_str(&content)?;
    Ok(config)
}

/// Builds the list of paths to search for config files
fn build_search_paths(explicit_path: Option<&str>) -> Vec<PathBuf> {
    if let Some(path) = explicit_path {
        return vec![PathBuf::from(path)];
    }

    let mut paths = Vec::with_capacity(3);

    // Environment variable
    if let Ok(env_path) = std::env::var(CONFIG_ENV_VAR) {
        paths.push(PathBuf::from(env_path));
    }

    // Working directory
    paths.push(PathBuf::from(CONFIG_FILENAME));

    // Executable directory
    if let Ok(exe_path) = std::env::current_exe() {
        if let Some(exe_dir) = exe_path.parent() {
            paths.push(exe_dir.join(CONFIG_FILENAME));
        }
    }

    paths
}

// ============================================================================
// Tests
// ============================================================================

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_default_config() {
        let config = AppConfig::default();
        assert_eq!(config.server.port, DEFAULT_PORT);
        assert_eq!(config.server.host, DEFAULT_HOST);
        assert!(config.output.include_photo);
    }

    #[test]
    fn test_server_websocket_url() {
        let config = ServerConfig::default();
        assert_eq!(config.websocket_url(), "ws://127.0.0.1:8182");
    }

    #[test]
    fn test_output_field_enabled() {
        let mut config = OutputConfig::default();
        assert!(config.is_field_enabled("citizen_id"));

        config.enabled_fields = vec!["citizen_id".to_string()];
        assert!(config.is_field_enabled("citizen_id"));
        assert!(!config.is_field_enabled("photo"));
    }

    #[test]
    fn test_output_field_mapping() {
        let mut config = OutputConfig::default();
        assert_eq!(config.get_field_name("citizen_id"), "citizen_id");

        config
            .field_mapping
            .insert("citizen_id".to_string(), "nationalId".to_string());
        assert_eq!(config.get_field_name("citizen_id"), "nationalId");
    }

    #[test]
    fn test_output_format_display() {
        assert_eq!(OutputFormat::Standard.to_string(), "standard");
        assert_eq!(OutputFormat::Minimal.to_string(), "minimal");
        assert_eq!(OutputFormat::Full.to_string(), "full");
    }

    #[test]
    fn test_parse_config_toml() {
        let toml = r#"
            [server]
            host = "0.0.0.0"
            port = 9000

            [output]
            format = "minimal"
            include_photo = false
        "#;

        let config: AppConfig = toml::from_str(toml).unwrap();
        assert_eq!(config.server.port, 9000);
        assert_eq!(config.output.format, OutputFormat::Minimal);
        assert!(!config.output.include_photo);
    }
}
