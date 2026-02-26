//! Audit logging module for security events
//!
//! Provides structured logging for security-relevant events including:
//! - Authentication attempts (success/failure)
//! - Rate limiting violations
//! - WebSocket connections (open/close)
//! - Card read operations
//! - Configuration changes
//! - Security errors

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use std::net::IpAddr;

/// Audit event type classification
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "lowercase")]
pub enum AuditEventType {
    /// Authentication attempt
    Authentication,
    /// Rate limit violation
    RateLimit,
    /// WebSocket connection event
    Connection,
    /// Card read operation
    CardRead,
    /// Configuration change
    Configuration,
    /// Security-related error
    SecurityError,
    /// Data validation error
    Validation,
}

/// Audit event severity level
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, PartialOrd, Ord)]
#[serde(rename_all = "lowercase")]
pub enum AuditSeverity {
    /// Informational event
    Info,
    /// Warning event
    Warning,
    /// Error event
    Error,
    /// Critical event requiring immediate attention
    Critical,
}

/// Structured audit log entry
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AuditLogEntry {
    /// ISO 8601 timestamp
    pub timestamp: DateTime<Utc>,
    /// Event type classification
    pub event_type: AuditEventType,
    /// Severity level
    pub severity: AuditSeverity,
    /// Client IP address
    pub client_ip: IpAddr,
    /// Event action (e.g., "login_success", "rate_limit_exceeded")
    pub action: String,
    /// Detailed event message
    pub message: String,
    /// Additional structured metadata
    #[serde(skip_serializing_if = "Option::is_none")]
    pub metadata: Option<serde_json::Value>,
}

impl AuditLogEntry {
    /// Create a new audit log entry
    #[must_use]
    pub fn new(
        event_type: AuditEventType,
        severity: AuditSeverity,
        client_ip: IpAddr,
        action: impl Into<String>,
        message: impl Into<String>,
    ) -> Self {
        Self {
            timestamp: Utc::now(),
            event_type,
            severity,
            client_ip,
            action: action.into(),
            message: message.into(),
            metadata: None,
        }
    }

    /// Log the audit entry to the logger
    pub fn log(&self) {
        let json = serde_json::to_string(self)
            .unwrap_or_else(|_| format!("Failed to serialize audit log: {:?}", self));

        match self.severity {
            AuditSeverity::Info => log::info!("AUDIT: {}", json),
            AuditSeverity::Warning => log::warn!("AUDIT: {}", json),
            AuditSeverity::Error => log::error!("AUDIT: {}", json),
            AuditSeverity::Critical => log::error!("AUDIT[CRITICAL]: {}", json),
        }
    }
}

/// Audit logger for security events
pub struct AuditLogger {
    enabled: bool,
}

impl AuditLogger {
    /// Create a new audit logger
    #[must_use]
    pub fn new(enabled: bool) -> Self {
        if enabled {
            log::info!("📝 Audit logging ENABLED");
        } else {
            log::warn!("⚠️ Audit logging DISABLED - Security events will not be recorded!");
        }
        Self { enabled }
    }

    /// Log authentication success
    pub fn log_auth_success(&self, client_ip: IpAddr, api_key_hint: Option<&str>) {
        if !self.enabled {
            return;
        }

        let message = if let Some(hint) = api_key_hint {
            format!("Authentication successful (key: {}...)", hint)
        } else {
            "Authentication successful".to_string()
        };

        AuditLogEntry::new(
            AuditEventType::Authentication,
            AuditSeverity::Info,
            client_ip,
            "auth_success",
            message,
        )
        .log();
    }

    /// Log authentication failure
    pub fn log_auth_failure(&self, client_ip: IpAddr, reason: &str) {
        if !self.enabled {
            return;
        }

        AuditLogEntry::new(
            AuditEventType::Authentication,
            AuditSeverity::Warning,
            client_ip,
            "auth_failure",
            format!("Authentication failed: {}", reason),
        )
        .log();
    }

    /// Log rate limit violation
    pub fn log_rate_limit(&self, client_ip: IpAddr, limit_type: &str) {
        if !self.enabled {
            return;
        }

        AuditLogEntry::new(
            AuditEventType::RateLimit,
            AuditSeverity::Warning,
            client_ip,
            "rate_limit_exceeded",
            format!("{} rate limit exceeded", limit_type),
        )
        .log();
    }

    /// Log WebSocket connection opened
    pub fn log_connection_open(&self, client_ip: IpAddr) {
        if !self.enabled {
            return;
        }

        AuditLogEntry::new(
            AuditEventType::Connection,
            AuditSeverity::Info,
            client_ip,
            "connection_open",
            "WebSocket connection established",
        )
        .log();
    }

    /// Log WebSocket connection closed
    pub fn log_connection_close(&self, client_ip: IpAddr, duration_ms: Option<u64>) {
        if !self.enabled {
            return;
        }

        let message = if let Some(ms) = duration_ms {
            format!("WebSocket connection closed (duration: {}ms)", ms)
        } else {
            "WebSocket connection closed".to_string()
        };

        AuditLogEntry::new(
            AuditEventType::Connection,
            AuditSeverity::Info,
            client_ip,
            "connection_close",
            message,
        )
        .log();
    }

    /// Log validation failure
    pub fn log_validation_failure(
        &self,
        client_ip: Option<IpAddr>,
        field: &str,
        error_type: &str,
        details: &str,
        is_security_threat: bool,
    ) {
        if !self.enabled {
            return;
        }

        let ip = client_ip
            .unwrap_or_else(|| std::net::IpAddr::V4(std::net::Ipv4Addr::new(127, 0, 0, 1)));

        let severity = if is_security_threat {
            AuditSeverity::Error
        } else {
            AuditSeverity::Warning
        };

        let event_type = if is_security_threat {
            AuditEventType::SecurityError
        } else {
            AuditEventType::Validation
        };

        let message = if is_security_threat {
            format!("Security threat detected in field '{}': {}", field, details)
        } else {
            format!(
                "Validation failed for field '{}': {} - {}",
                field, error_type, details
            )
        };

        AuditLogEntry::new(event_type, severity, ip, "validation_failure", message).log();
    }
}

// ============================================================================
// Tests
// ============================================================================

#[cfg(test)]
mod tests {
    use super::*;
    use std::net::Ipv4Addr;

    #[test]
    fn test_audit_log_entry_creation() {
        let ip = IpAddr::V4(Ipv4Addr::new(127, 0, 0, 1));
        let entry = AuditLogEntry::new(
            AuditEventType::Authentication,
            AuditSeverity::Info,
            ip,
            "test_action",
            "test message",
        );

        assert_eq!(entry.event_type, AuditEventType::Authentication);
        assert_eq!(entry.severity, AuditSeverity::Info);
        assert_eq!(entry.client_ip, ip);
        assert_eq!(entry.action, "test_action");
        assert_eq!(entry.message, "test message");
        assert!(entry.metadata.is_none());
    }

    #[test]
    fn test_audit_logger_disabled() {
        let logger = AuditLogger::new(false);
        let ip = IpAddr::V4(Ipv4Addr::new(10, 0, 0, 1));

        // These should not panic even when disabled
        logger.log_auth_success(ip, Some("test"));
        logger.log_auth_failure(ip, "test");
        logger.log_rate_limit(ip, "request");
        logger.log_connection_open(ip);
        logger.log_connection_close(ip, Some(1000));
    }

    #[test]
    fn test_severity_ordering() {
        assert!(AuditSeverity::Info < AuditSeverity::Warning);
        assert!(AuditSeverity::Warning < AuditSeverity::Error);
        assert!(AuditSeverity::Error < AuditSeverity::Critical);
    }
}
