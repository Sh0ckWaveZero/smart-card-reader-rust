use pcsc::{Context, Card, Scope, ShareMode, Protocols};
use std::collections::HashSet;
use std::ffi::CString;
use std::time::Duration;
use tokio::time::sleep;
use log::{info, error, warn, debug};
use anyhow::{Result, anyhow};
use crate::config::CardConfig;
use crate::decoder;

pub struct CardReader {
    ctx: Option<Context>,
    config: CardConfig,
}

impl CardReader {
    pub fn new(config: CardConfig) -> Result<Self> {
        match Context::establish(Scope::User) {
            Ok(ctx) => Ok(Self { ctx: Some(ctx), config }),
            Err(e) => {
                warn!("Failed to establish PCSC context: {}. Retrying later.", e);
                Ok(Self { ctx: None, config })
            }
        }
    }

    /// Check if PCSC context is healthy by attempting to list readers
    fn is_context_healthy(&self) -> bool {
        if let Some(ctx) = &self.ctx {
            let mut readers_buf = [0; 2048];
            match ctx.list_readers(&mut readers_buf) {
                Ok(_) => true,
                Err(e) => {
                    debug!("Context health check failed: {}", e);
                    false
                }
            }
        } else {
            false
        }
    }

    pub async fn run_monitor<F>(&mut self, on_card_event: F)
    where F: Fn(decoder::CardEvent) + Send + Sync + 'static + Clone
    {
        // Track readers that already have a card processed
        let mut card_present: HashSet<String> = HashSet::new();

        loop {
            // Check context health and re-establish if needed
            if !self.is_context_healthy() {
                if self.ctx.is_some() {
                    warn!("PCSC Context unhealthy, resetting...");
                    self.ctx = None;
                    card_present.clear();
                }

                match Context::establish(Scope::User) {
                    Ok(ctx) => {
                        info!("PCSC Context established.");
                        self.ctx = Some(ctx);
                    }
                    Err(e) => {
                        debug!("Failed to establish context: {}, retrying...", e);
                        sleep(Duration::from_secs(2)).await;
                        continue;
                    }
                }
            }

            let ctx = self.ctx.as_ref().unwrap();
            let mut readers_buf = [0; 2048];

            // Collect reader names into owned CStrings so they live long enough
            let reader_names: Vec<CString> = match ctx.list_readers(&mut readers_buf) {
                Ok(readers) => readers
                    .filter_map(|r| CString::new(r.to_bytes()).ok())
                    .collect(),
                Err(e) => {
                    error!("Failed to list readers: {}", e);
                    self.ctx = None;
                    card_present.clear();
                    sleep(Duration::from_secs(2)).await;
                    continue;
                }
            };

            if reader_names.is_empty() {
                sleep(Duration::from_secs(1)).await;
                continue;
            }

            // Build reader states with UNAWARE for initial poll
            let mut reader_states: Vec<pcsc::ReaderState> = reader_names
                .iter()
                .map(|name| pcsc::ReaderState::new(name.as_c_str(), pcsc::State::UNAWARE))
                .collect();

            // Wait for status change
            if let Err(e) = ctx.get_status_change(Duration::from_secs(2), &mut reader_states) {
                if e != pcsc::Error::Timeout {
                    error!("Get status change error: {}", e);
                    self.ctx = None;
                    card_present.clear();
                }
                sleep(Duration::from_millis(500)).await;
                continue;
            }

            // Process each reader
            for rs in &reader_states {
                let name = rs.name().to_string_lossy().to_string();
                let state = rs.event_state();
                let is_present = state.contains(pcsc::State::PRESENT)
                    && !state.contains(pcsc::State::EMPTY);

                if is_present && !card_present.contains(&name) {
                    // New card detected
                    info!("Card detected in reader: {}", name);

                    let retry_attempts = self.config.retry_attempts;
                    let retry_delay = Duration::from_millis(self.config.retry_delay_ms);
                    let settle_delay = Duration::from_millis(self.config.card_settle_delay_ms);

                    let mut read_success = false;
                    for attempt in 1..=retry_attempts {
                        // Wait for card to settle after insertion
                        sleep(settle_delay).await;

                        match ctx.connect(rs.name(), ShareMode::Shared, Protocols::ANY) {
                            Ok(card) => {
                                info!("Card connected in reader: {} (attempt {})", name, attempt);

                                // Retry read operation up to 3 times
                                for read_attempt in 1..=3 {
                                    match self.read_thai_id(&card) {
                                        Ok(data) => {
                                            info!("Successfully read Thai ID: {} (read attempt {}/3)",
                                                decoder::mask_citizen_id(&data.citizen_id), read_attempt);
                                            on_card_event(decoder::CardEvent::Inserted(data));
                                            read_success = true;
                                            break;
                                        }
                                        Err(e) => {
                                            warn!("Failed to read card data (read attempt {}/3): {}", read_attempt, e);
                                            if read_attempt < 3 {
                                                sleep(Duration::from_millis(300)).await;
                                            }
                                        }
                                    }
                                }

                                if read_success {
                                    break;
                                }
                            }
                            Err(e) => {
                                warn!("Failed to connect to card (attempt {}/{}): {}", attempt, retry_attempts, e);
                                if attempt < retry_attempts {
                                    sleep(retry_delay).await;
                                }
                            }
                        }
                    }

                    // Only mark as present if read was successful
                    if read_success {
                        card_present.insert(name);
                    } else {
                        error!("Failed to read card after {} connection attempts with 3 read retries each. Will retry on next poll cycle.", retry_attempts);
                    }
                } else if !is_present && card_present.contains(&name) {
                    // Card removed — allow re-read on next insert
                    info!("Card removed from reader: {}", name);
                    card_present.remove(&name);
                    on_card_event(decoder::CardEvent::Removed);
                }
            }

            sleep(Duration::from_millis(500)).await;
        }
    }

    pub fn read_thai_id(&self, card: &Card) -> Result<decoder::ThaiIDData> {
        // SELECT Thai ID Applet from config
        let select_apdu = self.config.select_apdu_bytes();
        debug!("SELECT APDU: {:02X?}", select_apdu);
        self.send_apdu(card, &select_apdu)
            .map_err(|e| anyhow!("Failed to SELECT Thai ID applet: {}", e))?;

        // Helper to read field by name from config
        let read_field = |name: &str| -> Result<String> {
            if let Some(field) = self.config.get_field(name) {
                let apdu = field.to_bytes();
                debug!("Reading {}: APDU {:02X?}", name, apdu);
                let data = self.send_apdu(card, &apdu)
                    .map_err(|e| anyhow!("Failed to read field '{}': {}", name, e))?;
                Ok(decoder::decode_tis620(&data))
            } else {
                warn!("Field '{}' not found in config, using empty string", name);
                Ok(String::new())
            }
        };

        // Helper: read raw bytes without stripping '#' delimiters
        let read_field_raw = |name: &str| -> Result<Vec<u8>> {
            if let Some(field) = self.config.get_field(name) {
                let apdu = field.to_bytes();
                let data = self.send_apdu(card, &apdu)
                    .map_err(|e| anyhow!("Failed to read raw field '{}': {}", name, e))?;
                Ok(data)
            } else {
                Ok(Vec::new())
            }
        };

        // Helper: split TIS-620 bytes by '#' into up to `n` parts
        let split_tis620 = |bytes: Vec<u8>, n: usize| -> Vec<String> {
            use encoding_rs::WINDOWS_874;
            use unicode_normalization::UnicodeNormalization;
            let (cow, _, _) = WINDOWS_874.decode(&bytes);
            let raw = cow.into_owned();
            let mut parts: Vec<String> = raw
                .splitn(n, '#')
                .map(|s| s.split_whitespace().collect::<Vec<&str>>().join(" ").nfc().collect())
                .collect();
            while parts.len() < n {
                parts.push(String::new());
            }
            parts
        };

        // Read all configured fields
        let citizen_id   = read_field("citizen_id")?;
        let date_of_birth = read_field("date_of_birth")?;
        let sex           = read_field("gender")?;
        let issuer        = read_field("issuer").unwrap_or_default();
        let issue    = read_field("issue")?;
        let mut expire   = read_field("expire")?;
        let full_name_en  = read_field("full_name_en")?;

        // Thai name: "คำนำหน้า#ชื่อ#ชื่อกลาง#นามสกุล"
        let name_th_raw = read_field_raw("full_name_th")?;
        let name_parts = split_tis620(name_th_raw, 4);
        let name_en_raw = read_field_raw("full_name_en")?;
        let en_name_parts = split_tis620(name_en_raw, 4);
        let th_prefix     = name_parts[0].clone();
        let th_firstname  = name_parts[1].clone();
        let th_middlename = name_parts[2].clone();
        let th_lastname   = name_parts[3].clone();
        let en_prefix     = en_name_parts[0].clone();
        let en_firstname  = en_name_parts[1].clone();
        let en_middlename = en_name_parts[2].clone();
        let en_lastname   = en_name_parts[3].clone();

        // Address on Thai ID card
        // Thai ID card address format: [#]เลขที่#หมู่ที่#ตำบล#อำเภอ#จังหวัด#...
        // We take the raw bytes, decode TIS-620, split by '#', take first 6 parts max,
        // and keep only parts that contain at least one Thai or ASCII printable character
        // (filtering out garbage binary padding that may appear after the real data).
        // Address on Thai ID card: เลขที่#หมู่ที่###ตำบล#อำเภอ#จังหวัด[garbage]
        // Split by '#', strip garbage from each part (keep only Thai + basic ASCII),
        // then filter out empty parts → gives clean ordered list.
        let addr_raw = read_field_raw("address")?;

        // Thai ID card stores address as TIS-620 bytes separated by '#' (0x23).
        // Valid TIS-620 address bytes: 0x20-0x7E (ASCII printable) and 0xA1-0xFB (Thai).
        // Garbage padding at end of field uses bytes outside these ranges (e.g. 0x00, 0x80-0x9F, 0xFC+).
        // Truncate at the first invalid byte to strip garbage BEFORE decoding.
        let addr_raw_clean: Vec<u8> = addr_raw.iter()
            .copied()
            .take_while(|&b| {
                b == 0x23           // '#' delimiter
                || (b >= 0x20 && b <= 0x7E)   // ASCII printable
                || (b >= 0xA1 && b <= 0xFB)   // TIS-620 Thai range
            })
            .collect();

        // Split by '#', filter empty parts, NFC-normalize
        let addr_meaningful_parts: Vec<String> = {
            use encoding_rs::WINDOWS_874;
            use unicode_normalization::UnicodeNormalization;
            let (cow, _, _) = WINDOWS_874.decode(&addr_raw_clean);
            cow.split('#')
                .map(|s| s.split_whitespace().collect::<Vec<_>>().join(" ").nfc().collect::<String>())
                // .filter(|s| !s.is_empty())
                .collect()
        };
        debug!("Address meaningful parts ({}): {:?}", addr_meaningful_parts.len(), addr_meaningful_parts);
        info!("Address meaningful parts ({}): {:?}", addr_meaningful_parts.len(), addr_meaningful_parts);

        // Strip any trailing non-Thai-letter content from a part
        // (Thai letters: U+0E01-U+0E2E, U+0E30-U+0E3A, U+0E40-U+0E45, U+0E47-U+0E4E)
        // Thai digits U+0E50-U+0E59 and punctuation are excluded — they indicate garbage
        let strip_garbage = |s: &str| -> String {
            // Keep only Thai consonants/vowels/tone-marks and space
            let clean: String = s.chars()
                .filter(|&c| {
                    (c >= '\u{0E01}' && c <= '\u{0E2E}')   // Thai consonants
                    || (c >= '\u{0E30}' && c <= '\u{0E3A}')// Thai vowels/sara
                    || (c >= '\u{0E40}' && c <= '\u{0E4E}')// Thai vowels/tone marks
                    || c == ' '
                })
                .collect();
            // Thai place names never have single-character words; filter them out
            // to eliminate stray garbage bytes that happen to decode as valid Thai chars
            clean.split_whitespace()
                .filter(|w| w.chars().count() >= 2)
                .collect::<Vec<_>>()
                .join(" ")
        };

        let addr_house_no   = addr_meaningful_parts.get(0).cloned().unwrap_or_default();
        let addr_village_no = addr_meaningful_parts.get(1).cloned().unwrap_or_default();
        let addr_lane = addr_meaningful_parts.get(2).cloned().unwrap_or_default();
        let addr_road = addr_meaningful_parts.get(3).cloned().unwrap_or_default();

        // Thai ID card address can be 7 or 8 fields depending on card variant:
        //   7-field: house#village#lane#road#tambol#amphur#province         (indices 4,5,6)
        //   8-field: house#village#lane#road#(empty)#tambol#amphur#province (indices 5,6,7)
        // Detect by checking if index 4 is non-empty after strip_garbage.
        let part4_clean = addr_meaningful_parts.get(4).map(|s| strip_garbage(s)).unwrap_or_default();

        info!("Determined address format: part4='{}' → {}", part4_clean, if part4_clean.is_empty() { "8-field" } else { "7-field" });

        let (tambol_idx, amphur_idx, province_idx) = if part4_clean.is_empty() {
            (5, 6, 7) // 8-field format: index 4 is empty filler
        } else {
            (4, 5, 6) // 7-field format: tambol starts at index 4
        };
        let addr_tambol   = addr_meaningful_parts.get(tambol_idx).map(|s| strip_garbage(s)).unwrap_or_default();
        let addr_amphur   = addr_meaningful_parts.get(amphur_idx).map(|s| strip_garbage(s)).unwrap_or_default();
        let addr_province = addr_meaningful_parts.get(province_idx).map(|s| strip_garbage(s)).unwrap_or_default();

        info!("Cleaned address components: house_no='{}', village_no='{}', road='{}', lane='{}', tambol='{}', amphur='{}', province='{}'",
            addr_house_no, addr_village_no, addr_road, addr_lane, addr_tambol, addr_amphur, addr_province
        );

        // Full address: house + village + road + lane + tambol + amphur + province
        let address = [&addr_house_no, &addr_village_no, &addr_road, &addr_lane, &addr_tambol, &addr_amphur, &addr_province]
            .iter()
            .map(|s| s.as_str())
            .collect::<Vec<_>>()
            .join(" "); 

        // Read Photo using configured chunk APDUs
        let mut photo_chunks = Vec::new();
        let photo_apdus = self.config.photo_chunk_bytes();
        let total_chunks = photo_apdus.len();

        for (i, apdu) in photo_apdus.iter().enumerate() {
            match self.send_apdu(card, apdu) {
                Ok(data) => {
                    debug!("Photo chunk {}/{}: {} bytes", i + 1, total_chunks, data.len());
                    photo_chunks.push(data);
                }
                Err(e) => {
                    warn!("Failed to read photo chunk {}/{}: {}", i + 1, total_chunks, e);
                }
            }
        }

        let total_bytes: usize = photo_chunks.iter().map(|c| c.len()).sum();
        if photo_chunks.len() < total_chunks {
            warn!("Photo incomplete: read {}/{} chunks ({} bytes)",
                photo_chunks.len(), total_chunks, total_bytes);
        } else {
            info!("Photo complete: {}/{} chunks ({} bytes)",
                photo_chunks.len(), total_chunks, total_bytes);
        }
        let photo = decoder::combine_photo_chunks(photo_chunks);

        // Convert date from YYYYMMDD → YYYY/MM/DD (required by HIS moment() parsing)
        let format_date_slash = |d: &str| -> String {
            if d.len() == 8 {
                format!("{}/{}/{}", &d[0..4], &d[4..6], &d[6..8])
            } else {
                d.to_string()
            }
        };

        let nationality: String = "THA".to_string();
        if expire == "99999999" {
            expire = "29991231".to_string(); // Treat "99999999" as "31 Dec 2599 for practical purposes
        }

        Ok(decoder::ThaiIDData {
            citizen_id,
            th_prefix,
            th_firstname,
            th_middlename,
            th_lastname,
            en_prefix,
            en_firstname,
            en_middlename,
            en_lastname,
            full_name_en,
            birthday: format_date_slash(&date_of_birth),
            sex,
            issuer,
            issue: format_date_slash(&issue),
            expire: format_date_slash(&expire),
            address,
            addr_house_no,
            addr_village_no,
            addr_road,
            addr_lane,
            addr_tambol,
            addr_amphur,
            addr_province,
            photo,
            nationality
        })
    }

    fn send_apdu(&self, card: &Card, apdu: &[u8]) -> Result<Vec<u8>> {
        let mut rapdu_buf = [0u8; 514]; // 512 data + 2 SW bytes
        let rapdu = card.transmit(apdu, &mut rapdu_buf)
            .map_err(|e| anyhow!("Card transmit failed: {}", e))?;

        if rapdu.len() < 2 {
            return Err(anyhow!("Invalid APDU response length: {} bytes (expected >= 2)", rapdu.len()));
        }

        let sw1 = rapdu[rapdu.len() - 2];
        let sw2 = rapdu[rapdu.len() - 1];

        // Handle chained T=0 GET RESPONSE (61 XX)
        if sw1 == 0x61 {
            let mut result = Vec::new();
            // Collect data before SW if any
            if rapdu.len() > 2 {
                result.extend_from_slice(&rapdu[..rapdu.len() - 2]);
            }
            let mut remaining = sw2;
            loop {
                let get_response_cmd = [0x00, 0xC0, 0x00, 0x00, remaining];
                let resp = card.transmit(&get_response_cmd, &mut rapdu_buf)?;
                if resp.len() < 2 {
                    return Err(anyhow!("Invalid GET RESPONSE length"));
                }
                let rsw1 = resp[resp.len() - 2];
                let rsw2 = resp[resp.len() - 1];
                result.extend_from_slice(&resp[..resp.len() - 2]);

                if rsw1 == 0x61 {
                    // More data available
                    remaining = rsw2;
                } else if rsw1 == 0x90 && rsw2 == 0x00 {
                    break;
                } else {
                    return Err(anyhow!("GET RESPONSE failed with status: SW1={:02X} SW2={:02X} ({})",
                        rsw1, rsw2, Self::interpret_sw(rsw1, rsw2)));
                }
            }
            Ok(result)
        } else if sw1 == 0x90 && sw2 == 0x00 {
            Ok(rapdu[..rapdu.len() - 2].to_vec())
        } else {
            Err(anyhow!("APDU failed with status: SW1={:02X} SW2={:02X} ({})",
                sw1, sw2, Self::interpret_sw(sw1, sw2)))
        }
    }

    /// Interpret ISO 7816-4 status words for better error messages
    fn interpret_sw(sw1: u8, sw2: u8) -> &'static str {
        match (sw1, sw2) {
            (0x90, 0x00) => "Success",
            (0x61, _) => "More data available",
            (0x62, 0x00) => "No information given",
            (0x62, 0x81) => "Part of returned data may be corrupted",
            (0x62, 0x82) => "End of file reached before reading",
            (0x63, 0x00) => "Verification failed",
            (0x63, 0xC0..=0xCF) => "Counter verification",
            (0x64, 0x00) => "State of non-volatile memory unchanged",
            (0x65, 0x00) => "State of non-volatile memory changed",
            (0x65, 0x81) => "Memory failure",
            (0x66, 0x00) => "Security-related issue",
            (0x67, 0x00) => "Wrong length",
            (0x68, 0x00) => "Functions in CLA not supported",
            (0x68, 0x81) => "Logical channel not supported",
            (0x68, 0x82) => "Secure messaging not supported",
            (0x69, 0x82) => "Security status not satisfied",
            (0x69, 0x83) => "Authentication method blocked",
            (0x69, 0x84) => "Referenced data invalidated",
            (0x69, 0x85) => "Conditions of use not satisfied",
            (0x69, 0x86) => "Command not allowed (no EF selected)",
            (0x6A, 0x80) => "Incorrect parameters in command data field",
            (0x6A, 0x81) => "Function not supported",
            (0x6A, 0x82) => "File not found",
            (0x6A, 0x83) => "Record not found",
            (0x6A, 0x84) => "Not enough memory space",
            (0x6A, 0x86) => "Incorrect parameters P1-P2",
            (0x6A, 0x88) => "Referenced data not found",
            (0x6B, 0x00) => "Wrong parameters P1-P2",
            (0x6C, _) => "Wrong Le field",
            (0x6D, 0x00) => "Instruction code not supported",
            (0x6E, 0x00) => "Class not supported",
            (0x6F, 0x00) => "No precise diagnosis",
            _ => "Unknown error"
        }
    }
}
