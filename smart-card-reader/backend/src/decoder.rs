use crate::config::OutputConfig;
use encoding_rs::WINDOWS_874;
use serde::{Deserialize, Serialize};
use serde_json::{json, Value};
use unicode_normalization::UnicodeNormalization;

/// Events from the card reader
#[derive(Debug, Clone)]
pub enum CardEvent {
    /// Card was inserted and data was read
    Inserted(ThaiIDData),
    /// Card was removed from the reader
    Removed,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct ThaiIDData {
    // --- Identity ---
    pub citizen_id: String,
    // --- Thai name components ---
    pub th_prefix: String,
    pub th_firstname: String,
    pub th_middlename: String,
    pub th_lastname: String,
    // --- English name (full, from card) ---
    pub full_name_en: String,
    // --- Date / Sex ---
    pub birthday: String, // YYYYMMDD (Buddhist Era from card)
    pub sex: String,      // "1" = male, other = female
    // --- Card meta ---
    pub card_issuer: String,
    pub issue_date: String,
    pub expire_date: String,
    // --- Address components ---
    pub address: String, // full combined address (raw from card)
    pub addr_house_no: String,
    pub addr_village_no: String,
    pub addr_tambol: String,
    pub addr_amphur: String,
    // --- Photo ---
    pub photo: String, // Base64 encoded
}

pub fn decode_tis620(bytes: &[u8]) -> String {
    let (cow, _encoding_used, _had_errors) = WINDOWS_874.decode(bytes);
    // '#' is used as a field delimiter on Thai ID cards — replace with space
    // then collapse multiple spaces and trim
    let s = cow.into_owned().replace('#', " ");
    let s = s.split_whitespace().collect::<Vec<&str>>().join(" ");
    // Normalize to NFC for proper Thai character composition
    s.nfc().collect()
}

pub fn combine_photo_chunks(chunks: Vec<Vec<u8>>) -> String {
    let mut full_data = Vec::new();
    for chunk in chunks {
        full_data.extend_from_slice(&chunk);
    }
    use base64::Engine;
    base64::engine::general_purpose::STANDARD.encode(&full_data)
}

/// Mask citizen ID for logging - shows only last 4 digits with asterisks
/// Example: "3100600123456" → "****0123456"
pub fn mask_citizen_id(citizen_id: &str) -> String {
    if citizen_id.len() <= 4 {
        "*".repeat(citizen_id.len())
    } else {
        let last_4 = &citizen_id[citizen_id.len() - 4..];
        format!("{}*{}", "*".repeat(citizen_id.len() - 4), last_4)
    }
}

/// Mask address for logging - only show province to prevent location identification
/// Example: "99 หมู่ที่ 4 ตำบลบางรัก อำเภอเมือง จังหวัดกรุงเทพมหานคร" → "[hidden] จังหวัดกรุงเทพมหานคร"
pub fn mask_address(province: &str) -> String {
    if province.is_empty() {
        "[masked address]".to_string()
    } else {
        format!("[hidden] {}", province)
    }
}

/// Format date from YYYYMMDD to "DD MMM YYYY" in Buddhist Era (พ.ศ.)
/// Input is already in Buddhist Era from the card
pub fn format_thai_date(date_str: &str) -> String {
    if date_str.len() != 8 {
        return date_str.to_string();
    }

    let year = &date_str[0..4];
    let month: u32 = date_str[4..6].parse().unwrap_or(0);
    let day = &date_str[6..8];

    let thai_months = [
        "ม.ค.",
        "ก.พ.",
        "มี.ค.",
        "เม.ย.",
        "พ.ค.",
        "มิ.ย.",
        "ก.ค.",
        "ส.ค.",
        "ก.ย.",
        "ต.ค.",
        "พ.ย.",
        "ธ.ค.",
    ];

    let month_name = if month >= 1 && month <= 12 {
        thai_months[(month - 1) as usize]
    } else {
        return date_str.to_string();
    };

    // Remove leading zero from day
    let day_num: u32 = day.parse().unwrap_or(0);

    format!("{} {} {}", day_num, month_name, year)
}

/// Apply output configuration to card data
/// - Filter enabled fields
/// - Apply field mapping
/// - Optionally exclude photo
pub fn apply_output_config(data: &ThaiIDData, config: &OutputConfig) -> Value {
    let mut result = serde_json::Map::new();

    // Define all available fields (internal_name, value)
    let fields: &[(&str, &str)] = &[
        ("Citizenid", &data.citizen_id),
        ("Th_Prefix", &data.th_prefix),
        ("Th_Firstname", &data.th_firstname),
        ("Th_Middlename", &data.th_middlename),
        ("Th_Lastname", &data.th_lastname),
        ("full_name_en", &data.full_name_en),
        ("Birthday", &data.birthday),
        ("Sex", &data.sex),
        ("card_issuer", &data.card_issuer),
        ("issue_date", &data.issue_date),
        ("expire_date", &data.expire_date),
        ("Address", &data.address),
        ("addrHouseNo", &data.addr_house_no),
        ("addrVillageNo", &data.addr_village_no),
        ("addrTambol", &data.addr_tambol),
        ("addrAmphur", &data.addr_amphur),
    ];

    // Process each field
    for &(field_name, field_value) in fields {
        if config.is_field_enabled(field_name) {
            let output_name = config.get_field_name(field_name).to_owned();
            result.insert(output_name, json!(field_value));
        }
    }

    // Handle photo separately (can be large)
    if config.include_photo && config.is_field_enabled("PhotoRaw") {
        let output_name = config.get_field_name("PhotoRaw").to_owned();
        result.insert(output_name, json!(&data.photo));
    }

    Value::Object(result)
}
