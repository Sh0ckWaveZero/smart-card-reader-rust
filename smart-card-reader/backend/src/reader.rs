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

    pub async fn run_monitor<F>(&mut self, on_card_event: F)
    where F: Fn(decoder::CardEvent) + Send + Sync + 'static + Clone
    {
        // Track readers that already have a card processed
        let mut card_present: HashSet<String> = HashSet::new();

        loop {
            // Establish context if needed
            if self.ctx.is_none() {
                match Context::establish(Scope::User) {
                    Ok(ctx) => {
                        info!("PCSC Context established.");
                        self.ctx = Some(ctx);
                    }
                    Err(_) => {
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

                    let mut connected = false;
                    for attempt in 1..=retry_attempts {
                        // Wait for card to settle after insertion
                        sleep(settle_delay).await;

                        match ctx.connect(rs.name(), ShareMode::Shared, Protocols::ANY) {
                            Ok(card) => {
                                info!("Card connected in reader: {} (attempt {})", name, attempt);
                                match self.read_thai_id(&card) {
                                    Ok(data) => {
                                        info!("Read Thai ID: {}", data.citizen_id);
                                        on_card_event(decoder::CardEvent::Inserted(data));
                                    }
                                    Err(e) => error!("Failed to read card: {}", e),
                                }
                                connected = true;
                                break;
                            }
                            Err(e) => {
                                warn!("Failed to connect to card (attempt {}/{}): {}", attempt, retry_attempts, e);
                                if attempt < retry_attempts {
                                    sleep(retry_delay).await;
                                }
                            }
                        }
                    }

                    // Mark as present regardless to avoid retry spam
                    card_present.insert(name);

                    if !connected {
                        error!("Failed to connect after 3 attempts, will retry on re-insert");
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
        self.send_apdu(card, &select_apdu)?;

        // Helper to read field by name from config
        let read_field = |name: &str| -> Result<String> {
            if let Some(field) = self.config.get_field(name) {
                let apdu = field.to_bytes();
                debug!("Reading {}: APDU {:02X?}", name, apdu);
                let data = self.send_apdu(card, &apdu)?;
                Ok(decoder::decode_tis620(&data))
            } else {
                warn!("Field '{}' not found in config, using empty string", name);
                Ok(String::new())
            }
        };

        // Read all configured fields
        let citizen_id = read_field("citizen_id")?;
        let full_name_th = read_field("full_name_th")?;
        let full_name_en = read_field("full_name_en")?;
        let date_of_birth = read_field("date_of_birth")?;
        let gender = read_field("gender")?;
        let card_issuer = read_field("card_issuer").unwrap_or_default();
        let issue_date = read_field("issue_date")?;
        let expire_date = read_field("expire_date")?;
        let address = read_field("address").unwrap_or_default();

        // Read Photo using configured chunk APDUs
        let mut photo_chunks = Vec::new();
        let photo_apdus = self.config.photo_chunk_bytes();

        for (i, apdu) in photo_apdus.iter().enumerate() {
            match self.send_apdu(card, apdu) {
                Ok(data) => {
                    debug!("Photo chunk {}: {} bytes", i + 1, data.len());
                    photo_chunks.push(data);
                }
                Err(e) => {
                    warn!("Failed to read photo chunk {}: {}", i + 1, e);
                }
            }
        }

        info!("Total photo chunks read: {}, total bytes: {}",
            photo_chunks.len(),
            photo_chunks.iter().map(|c| c.len()).sum::<usize>()
        );
        let photo = decoder::combine_photo_chunks(photo_chunks);

        Ok(decoder::ThaiIDData {
            citizen_id,
            full_name_th,
            full_name_en,
            date_of_birth,
            gender,
            card_issuer,
            issue_date,
            expire_date,
            address,
            photo,
        })
    }

    fn send_apdu(&self, card: &Card, apdu: &[u8]) -> Result<Vec<u8>> {
        let mut rapdu_buf = [0u8; 258]; // max 256 data + 2 SW bytes
        let rapdu = card.transmit(apdu, &mut rapdu_buf)?;

        if rapdu.len() < 2 {
            return Err(anyhow!("Invalid APDU response length"));
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
                    return Err(anyhow!("GET RESPONSE failed: {:02X} {:02X}", rsw1, rsw2));
                }
            }
            Ok(result)
        } else if sw1 == 0x90 && sw2 == 0x00 {
            Ok(rapdu[..rapdu.len() - 2].to_vec())
        } else {
            Err(anyhow!("APDU Failed: {:02X} {:02X}", sw1, sw2))
        }
    }
}
