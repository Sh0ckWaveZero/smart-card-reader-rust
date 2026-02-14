use crate::decoder::{format_thai_date, CardEvent, ThaiIDData};
use chrono::Local;
use eframe::egui;
use std::sync::mpsc::Receiver;

const MAX_LOGS: usize = 100;

fn get_font_paths() -> Vec<std::path::PathBuf> {
    let mut paths = Vec::new();

    // Try relative to executable first
    if let Ok(exe_path) = std::env::current_exe() {
        if let Some(exe_dir) = exe_path.parent() {
            paths.push(exe_dir.join("fonts/NotoSansThai-Regular.ttf"));
            // Also check parent directory (for when running from target/debug or target/release)
            if let Some(parent) = exe_dir.parent() {
                paths.push(parent.join("fonts/NotoSansThai-Regular.ttf"));
                if let Some(grandparent) = parent.parent() {
                    paths.push(grandparent.join("fonts/NotoSansThai-Regular.ttf"));
                }
            }
        }
    }

    // Current working directory
    paths.push(std::path::PathBuf::from("fonts/NotoSansThai-Regular.ttf"));

    // System fonts - Windows (Thai-supporting fonts)
    paths.push(std::path::PathBuf::from(
        "C:\\Windows\\Fonts\\LeelawUI.ttf",
    )); // Leelawadee UI - best Thai font on Windows
    paths.push(std::path::PathBuf::from(
        "C:\\Windows\\Fonts\\LeelUIsl.ttf",
    )); // Leelawadee UI Semilight
    paths.push(std::path::PathBuf::from(
        "C:\\Windows\\Fonts\\tahoma.ttf",
    )); // Tahoma - fallback
    paths.push(std::path::PathBuf::from(
        "C:\\Windows\\Fonts\\cordia.ttf",
    )); // Cordia New
    paths.push(std::path::PathBuf::from(
        "C:\\Windows\\Fonts\\angsau.ttf",
    )); // AngsanaUPC

    // System fonts - Linux
    paths.push(std::path::PathBuf::from(
        "/usr/share/fonts/opentype/noto/NotoSansThai-Regular.ttf",
    ));
    paths.push(std::path::PathBuf::from(
        "/usr/share/fonts/truetype/noto/NotoSansThai-Regular.ttf",
    ));

    // System fonts - macOS (only .ttf files, not .ttc)
    paths.push(std::path::PathBuf::from(
        "/System/Library/Fonts/Supplemental/Silom.ttf",
    ));
    paths.push(std::path::PathBuf::from(
        "/System/Library/Fonts/Supplemental/Ayuthaya.ttf",
    ));
    paths.push(std::path::PathBuf::from(
        "/System/Library/Fonts/Supplemental/Krungthep.ttf",
    ));
    paths.push(std::path::PathBuf::from(
        "/System/Library/Fonts/Supplemental/Sathu.ttf",
    ));

    // User fonts
    paths.push(std::path::PathBuf::from(
        "/Library/Fonts/NotoSansThai-Regular.ttf",
    ));

    paths
}

fn setup_fonts(ctx: &egui::Context) {
    let mut fonts = egui::FontDefinitions::default();

    log::info!("Searching for Thai fonts...");
    for path in get_font_paths() {
        log::debug!("Checking font path: {:?}", path);
        if let Ok(font_data) = std::fs::read(&path) {
            let font_data = egui::FontData::from_owned(font_data);
            fonts.font_data.insert(
                "noto_sans_thai".to_owned(),
                std::sync::Arc::new(font_data),
            );

            fonts
                .families
                .entry(egui::FontFamily::Proportional)
                .or_insert_with(Vec::new)
                .insert(0, "noto_sans_thai".to_owned());

            fonts
                .families
                .entry(egui::FontFamily::Monospace)
                .or_insert_with(Vec::new)
                .insert(0, "noto_sans_thai".to_owned());

            log::info!("Loaded Thai font from: {:?}", path);
            ctx.set_fonts(fonts);

            // Set larger font size
            let mut style = (*ctx.style()).clone();
            style.text_styles.insert(
                egui::TextStyle::Body,
                egui::FontId::new(16.0, egui::FontFamily::Proportional),
            );
            style.text_styles.insert(
                egui::TextStyle::Heading,
                egui::FontId::new(22.0, egui::FontFamily::Proportional),
            );
            style.text_styles.insert(
                egui::TextStyle::Monospace,
                egui::FontId::new(14.0, egui::FontFamily::Monospace),
            );
            ctx.set_style(style);
            return;
        }
    }

    log::warn!("Thai font not found! Tried the following paths:");
    for path in get_font_paths() {
        log::warn!("  - {:?} (exists: {})", path, path.exists());
    }
    log::warn!("Thai text will display as boxes. Please ensure a Thai font is available.");
    ctx.set_fonts(fonts);
}

pub struct SmartCardApp {
    rx: Receiver<CardEvent>,
    card_data: Option<ThaiIDData>,
    logs: Vec<String>,
    photo_texture: Option<egui::TextureHandle>,
    last_read_time: Option<String>,
    fonts_configured: bool,
}

impl SmartCardApp {
    pub fn new(rx: Receiver<CardEvent>) -> Self {
        Self {
            rx,
            card_data: None,
            logs: vec![format!("[{}] Application started", Local::now().format("%H:%M:%S"))],
            photo_texture: None,
            last_read_time: None,
            fonts_configured: false,
        }
    }

    fn clear_card_data(&mut self) {
        self.card_data = None;
        self.photo_texture = None;
        self.add_log("Card removed - data cleared");
    }

    fn add_log(&mut self, message: &str) {
        let timestamp = Local::now().format("%H:%M:%S");
        self.logs.push(format!("[{}] {}", timestamp, message));
        if self.logs.len() > MAX_LOGS {
            self.logs.remove(0);
        }
    }

    fn load_photo_texture(&mut self, ctx: &egui::Context, base64_photo: &str) {
        // Decode base64 to bytes
        use base64::Engine;
        let photo_bytes = match base64::engine::general_purpose::STANDARD.decode(base64_photo) {
            Ok(bytes) => bytes,
            Err(e) => {
                self.add_log(&format!("Failed to decode photo base64: {}", e));
                return;
            }
        };

        // Try to load as JPEG
        let img = match image::load_from_memory_with_format(&photo_bytes, image::ImageFormat::Jpeg) {
            Ok(img) => img,
            Err(_) => {
                // Try without format hint
                match image::load_from_memory(&photo_bytes) {
                    Ok(img) => img,
                    Err(e) => {
                        self.add_log(&format!("Failed to decode photo image: {}", e));
                        return;
                    }
                }
            }
        };

        let rgba = img.to_rgba8();
        let size = [rgba.width() as usize, rgba.height() as usize];
        let pixels = rgba.into_raw();

        let color_image = egui::ColorImage::from_rgba_unmultiplied(size, &pixels);
        self.photo_texture = Some(ctx.load_texture(
            "id_photo",
            color_image,
            egui::TextureOptions::LINEAR,
        ));
    }
}

impl eframe::App for SmartCardApp {
    fn update(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        // Setup fonts only once
        if !self.fonts_configured {
            setup_fonts(ctx);
            self.fonts_configured = true;
        }

        // Check for card events
        while let Ok(event) = self.rx.try_recv() {
            match event {
                CardEvent::Inserted(data) => {
                    self.add_log(&format!("Card read: {}", data.citizen_id));
                    self.last_read_time = Some(Local::now().format("%H:%M:%S").to_string());

                    // Load photo texture
                    if !data.photo.is_empty() {
                        self.load_photo_texture(ctx, &data.photo);
                    }

                    self.card_data = Some(data);
                }
                CardEvent::Removed => {
                    self.clear_card_data();
                }
            }
        }

        // Request continuous repaints to check for new data
        ctx.request_repaint_after(std::time::Duration::from_millis(100));

        // Top panel - Status bar
        egui::TopBottomPanel::top("status_bar").show(ctx, |ui| {
            ui.horizontal(|ui| {
                ui.label(egui::RichText::new("Smart Card Reader").strong());
                ui.separator();
                ui.label("WebSocket: ws://127.0.0.1:8182/ws");
                ui.separator();
                if let Some(time) = &self.last_read_time {
                    ui.label(format!("Last read: {}", time));
                } else {
                    ui.label("Waiting for card...");
                }
            });
        });

        // Bottom panel - Logs (full width)
        egui::TopBottomPanel::bottom("logs_panel")
            .resizable(true)
            .min_height(100.0)
            .show(ctx, |ui| {
                ui.heading("Logs");
                egui::ScrollArea::both()
                    .stick_to_bottom(true)
                    .auto_shrink([false, false])
                    .show(ui, |ui| {
                        ui.set_width(ui.available_width());
                        for log in &self.logs {
                            ui.add(egui::Label::new(log).wrap_mode(egui::TextWrapMode::Extend));
                        }
                    });
            });

        // Central panel - Card data
        egui::CentralPanel::default().show(ctx, |ui| {
            if let Some(data) = &self.card_data {
                ui.horizontal(|ui| {
                    // Left side - Photo
                    ui.vertical(|ui| {
                        ui.heading("Photo");
                        if let Some(texture) = &self.photo_texture {
                            let size = texture.size_vec2();
                            let scale = 150.0 / size.x.max(size.y);
                            ui.image((texture.id(), size * scale));
                        } else {
                            ui.label("No photo available");
                        }
                    });

                    ui.separator();

                    // Right side - Card details
                    ui.vertical(|ui| {
                        ui.heading("Card Information");
                        ui.add_space(10.0);

                        egui::Grid::new("card_info_grid")
                            .num_columns(2)
                            .spacing([20.0, 8.0])
                            .show(ui, |ui| {
                                ui.label(egui::RichText::new("Citizen ID:").strong());
                                ui.label(&data.citizen_id);
                                ui.end_row();

                                ui.label(egui::RichText::new("Name (TH):").strong());
                                ui.label(&data.full_name_th);
                                ui.end_row();

                                ui.label(egui::RichText::new("Name (EN):").strong());
                                ui.label(&data.full_name_en);
                                ui.end_row();

                                ui.label(egui::RichText::new("Date of Birth:").strong());
                                ui.label(format_thai_date(&data.date_of_birth));
                                ui.end_row();

                                ui.label(egui::RichText::new("Gender:").strong());
                                ui.label(&data.gender);
                                ui.end_row();

                                ui.label(egui::RichText::new("Card Issuer:").strong());
                                ui.label(&data.card_issuer);
                                ui.end_row();

                                ui.label(egui::RichText::new("Issue Date:").strong());
                                ui.label(format_thai_date(&data.issue_date));
                                ui.end_row();

                                ui.label(egui::RichText::new("Expire Date:").strong());
                                ui.label(format_thai_date(&data.expire_date));
                                ui.end_row();
                            });

                        ui.add_space(10.0);
                        ui.label(egui::RichText::new("Address:").strong());
                        ui.label(&data.address);
                    });
                });
            } else {
                ui.centered_and_justified(|ui| {
                    ui.vertical_centered(|ui| {
                        ui.add_space(50.0);
                        ui.heading("Please insert a Thai ID card");
                        ui.add_space(20.0);
                        ui.label("The card data will appear here automatically.");
                    });
                });
            }
        });
    }
}
