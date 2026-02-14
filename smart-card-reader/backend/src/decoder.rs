use encoding_rs::WINDOWS_874;
use serde::{Deserialize, Serialize}; // TIS-620 is effectively Windows-874
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
    pub citizen_id: String,
    pub full_name_th: String,
    pub full_name_en: String,
    pub date_of_birth: String,
    pub gender: String,
    pub card_issuer: String,
    pub issue_date: String,
    pub expire_date: String,
    pub address: String,
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
        "ม.ค.", "ก.พ.", "มี.ค.", "เม.ย.", "พ.ค.", "มิ.ย.",
        "ก.ค.", "ส.ค.", "ก.ย.", "ต.ค.", "พ.ย.", "ธ.ค.",
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
