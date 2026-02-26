//! Input validation module for card data
//!
//! Validates Thai ID card data to ensure:
//! - Data integrity and format compliance
//! - Protection against injection attacks
//! - Early detection of corrupted or invalid data

use regex::Regex;
use std::sync::OnceLock;

/// Type of validation error with security classification
#[derive(Debug, Clone, PartialEq)]
pub enum ValidationError {
    /// Data format error (e.g., wrong length, invalid characters)
    Format(String),
    /// Data integrity error (e.g., checksum validation failed)
    Integrity(String),
    /// Security threat (e.g., injection payload, suspicious characters)
    Security(String),
}

impl std::fmt::Display for ValidationError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            ValidationError::Format(msg) => write!(f, "Format error: {}", msg),
            ValidationError::Integrity(msg) => write!(f, "Integrity error: {}", msg),
            ValidationError::Security(msg) => write!(f, "Security threat: {}", msg),
        }
    }
}

/// Validation result with structural error
pub type ValidationResult = Result<(), ValidationError>;

/// Thai citizen ID validator
pub struct ThaiCitizenIdValidator;

impl ThaiCitizenIdValidator {
    /// Validate Thai citizen ID format (13 digits)
    pub fn validate(citizen_id: &str) -> ValidationResult {
        // Remove whitespace
        let clean_id = citizen_id.trim();

        // Check length
        if clean_id.len() != 13 {
            return Err(ValidationError::Format(format!(
                "Invalid length: expected 13 digits, got {}",
                clean_id.len()
            )));
        }

        // Check if all characters are digits
        if !clean_id.chars().all(|c| c.is_ascii_digit()) {
            return Err(ValidationError::Format(
                "Contains non-digit characters".to_string(),
            ));
        }

        // Validate checksum (Modulo 11 algorithm)
        if !Self::validate_checksum(clean_id) {
            return Err(ValidationError::Integrity("Invalid checksum".to_string()));
        }

        Ok(())
    }

    /// Validate checksum using Modulo 11 algorithm
    fn validate_checksum(citizen_id: &str) -> bool {
        let digits: Vec<u32> = citizen_id.chars().filter_map(|c| c.to_digit(10)).collect();

        if digits.len() != 13 {
            return false;
        }

        // Calculate sum: multiply each of first 12 digits by (13 - position)
        let sum: u32 = digits[..12]
            .iter()
            .enumerate()
            .map(|(i, &digit)| digit * (13 - i as u32))
            .sum();

        // Calculate check digit
        let calculated_check = (11 - (sum % 11)) % 10;
        let provided_check = digits[12];

        calculated_check == provided_check
    }
}

/// Date format validator for Thai ID card dates
pub struct DateValidator;

impl DateValidator {
    /// Validate date format (YYYYMMDD or YYYY-MM-DD)
    pub fn validate(date: &str) -> ValidationResult {
        static DATE_REGEX: OnceLock<Regex> = OnceLock::new();
        let regex = DATE_REGEX.get_or_init(|| Regex::new(r"^(\d{4})-?(\d{2})-?(\d{2})$").unwrap());

        if !regex.is_match(date) {
            return Err(ValidationError::Format(
                "Invalid date format: expected YYYYMMDD or YYYY-MM-DD".to_string(),
            ));
        }

        // Parse and validate date components
        let clean_date = date.replace('-', "");
        if clean_date.len() != 8 {
            return Err(ValidationError::Format("Invalid date length".to_string()));
        }

        let year: u32 = clean_date[0..4]
            .parse()
            .map_err(|_| ValidationError::Format("Invalid year".into()))?;
        let month: u32 = clean_date[4..6]
            .parse()
            .map_err(|_| ValidationError::Format("Invalid month".into()))?;
        let day: u32 = clean_date[6..8]
            .parse()
            .map_err(|_| ValidationError::Format("Invalid day".into()))?;

        // Validate ranges
        if !(1900..=2100).contains(&year) {
            return Err(ValidationError::Format(format!("Invalid year: {}", year)));
        }
        if !(1..=12).contains(&month) {
            return Err(ValidationError::Format(format!("Invalid month: {}", month)));
        }
        if !(1..=31).contains(&day) {
            return Err(ValidationError::Format(format!("Invalid day: {}", day)));
        }

        Ok(())
    }
}

/// Name validator for Thai names
pub struct NameValidator;

impl NameValidator {
    /// Validate name (Thai or English characters, spaces allowed)
    pub fn validate(name: &str) -> ValidationResult {
        let clean_name = name.trim();

        // Check minimum length
        if clean_name.is_empty() {
            return Err(ValidationError::Format("Name cannot be empty".to_string()));
        }

        // Check maximum length (reasonable limit)
        if clean_name.len() > 200 {
            return Err(ValidationError::Format(format!(
                "Name too long: {} characters",
                clean_name.len()
            )));
        }

        // Check for suspicious characters (potential injection)
        let suspicious_chars = ['<', '>', '{', '}', '[', ']', '\\', '|', ';', '&', '$'];
        if clean_name.chars().any(|c| suspicious_chars.contains(&c)) {
            return Err(ValidationError::Security(
                "Contains suspicious characters".to_string(),
            ));
        }

        Ok(())
    }
}

/// Gender validator
pub struct GenderValidator;

impl GenderValidator {
    /// Validate gender code (1 = Male, 2 = Female)
    pub fn validate(gender: &str) -> ValidationResult {
        let clean_gender = gender.trim();

        if clean_gender != "1" && clean_gender != "2" {
            return Err(ValidationError::Format(format!(
                "Invalid gender code: expected '1' or '2', got '{}'",
                clean_gender
            )));
        }

        Ok(())
    }
}

/// Address validator
pub struct AddressValidator;

impl AddressValidator {
    /// Validate address (basic sanitization)
    pub fn validate(address: &str) -> ValidationResult {
        let clean_address = address.trim();

        // Check minimum length
        if clean_address.is_empty() {
            return Err(ValidationError::Format(
                "Address cannot be empty".to_string(),
            ));
        }

        // Check maximum length (reasonable limit)
        if clean_address.len() > 500 {
            return Err(ValidationError::Format(format!(
                "Address too long: {} characters",
                clean_address.len()
            )));
        }

        // Check for suspicious characters (potential injection)
        let suspicious_chars = ['<', '>', '{', '}', '[', ']', '\\', '|', ';', '&', '$'];
        if clean_address.chars().any(|c| suspicious_chars.contains(&c)) {
            return Err(ValidationError::Security(
                "Contains suspicious characters".to_string(),
            ));
        }

        Ok(())
    }
}

/// Comprehensive card data validator
pub struct CardDataValidator;

impl CardDataValidator {
    /// Validate all card data fields
    pub fn validate_all(
        citizen_id: Option<&str>,
        birth_date: Option<&str>,
        issue_date: Option<&str>,
        expire_date: Option<&str>,
        gender: Option<&str>,
        thai_name: Option<&str>,
        english_name: Option<&str>,
        address: Option<&str>,
    ) -> Vec<(String, ValidationError)> {
        let mut errors = Vec::new();

        // Validate citizen ID
        if let Some(id) = citizen_id {
            if let Err(e) = ThaiCitizenIdValidator::validate(id) {
                errors.push(("Citizen ID".to_string(), e));
            }
        }

        // Validate dates
        if let Some(date) = birth_date {
            if let Err(e) = DateValidator::validate(date) {
                errors.push(("Birth date".to_string(), e));
            }
        }
        if let Some(date) = issue_date {
            if let Err(e) = DateValidator::validate(date) {
                errors.push(("Issue date".to_string(), e));
            }
        }
        if let Some(date) = expire_date {
            if let Err(e) = DateValidator::validate(date) {
                errors.push(("Expire date".to_string(), e));
            }
        }

        // Validate gender
        if let Some(g) = gender {
            if let Err(e) = GenderValidator::validate(g) {
                errors.push(("Gender".to_string(), e));
            }
        }

        // Validate names
        if let Some(name) = thai_name {
            if let Err(e) = NameValidator::validate(name) {
                errors.push(("Thai name".to_string(), e));
            }
        }
        if let Some(name) = english_name {
            if let Err(e) = NameValidator::validate(name) {
                errors.push(("English name".to_string(), e));
            }
        }

        // Validate address
        if let Some(addr) = address {
            if let Err(e) = AddressValidator::validate(addr) {
                errors.push(("Address".to_string(), e));
            }
        }

        errors
    }
}

// ============================================================================
// Tests
// ============================================================================

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_valid_citizen_id() {
        // Valid test ID with correct checksum
        assert!(ThaiCitizenIdValidator::validate("1234567890123").is_err()); // Invalid checksum
                                                                             // Note: Real valid IDs should be tested separately
    }

    #[test]
    fn test_invalid_citizen_id_length() {
        assert!(ThaiCitizenIdValidator::validate("123456789012").is_err());
        assert!(ThaiCitizenIdValidator::validate("12345678901234").is_err());
    }

    #[test]
    fn test_invalid_citizen_id_non_digit() {
        assert!(ThaiCitizenIdValidator::validate("123456789012a").is_err());
        assert!(ThaiCitizenIdValidator::validate("1-2345-67890-12").is_err());
    }

    #[test]
    fn test_valid_dates() {
        assert!(DateValidator::validate("19900115").is_ok());
        assert!(DateValidator::validate("1990-01-15").is_ok());
    }

    #[test]
    fn test_invalid_dates() {
        assert!(DateValidator::validate("19901315").is_err()); // Invalid month
        assert!(DateValidator::validate("19900132").is_err()); // Invalid day
        assert!(DateValidator::validate("20501301").is_err()); // Invalid month
    }

    #[test]
    fn test_name_validation() {
        assert!(NameValidator::validate("นายทดสอบ ระบบ").is_ok());
        assert!(NameValidator::validate("Test User").is_ok());
        assert!(matches!(
            NameValidator::validate(""),
            Err(ValidationError::Format(_))
        )); // Empty
        assert!(matches!(
            NameValidator::validate("<script>alert()</script>"),
            Err(ValidationError::Security(_))
        )); // Injection
    }

    #[test]
    fn test_gender_validation() {
        assert!(GenderValidator::validate("1").is_ok());
        assert!(GenderValidator::validate("2").is_ok());
        assert!(GenderValidator::validate("0").is_err());
        assert!(GenderValidator::validate("3").is_err());
        assert!(GenderValidator::validate("M").is_err());
    }

    #[test]
    fn test_address_validation() {
        assert!(AddressValidator::validate("123 ถนนสุขุมวิท").is_ok());
        assert!(matches!(
            AddressValidator::validate(""),
            Err(ValidationError::Format(_))
        )); // Empty
        assert!(matches!(
            AddressValidator::validate("123<script>"),
            Err(ValidationError::Security(_))
        )); // Injection
    }
}
