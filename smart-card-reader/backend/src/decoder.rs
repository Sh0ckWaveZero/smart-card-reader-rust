use encoding_rs::WINDOWS_874;
use serde::{Deserialize, Serialize}; // TIS-620 is effectively Windows-874

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
    s.split_whitespace().collect::<Vec<&str>>().join(" ")
}

pub fn combine_photo_chunks(chunks: Vec<Vec<u8>>) -> String {
    let mut full_data = Vec::new();
    for chunk in chunks {
        full_data.extend_from_slice(&chunk);
    }
    use base64::Engine;
    base64::engine::general_purpose::STANDARD.encode(&full_data)
}
