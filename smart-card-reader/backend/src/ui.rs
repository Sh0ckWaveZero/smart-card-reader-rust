use crate::config::FontConfig;
use crate::decoder::{format_thai_date, CardEvent, ThaiIDData};
use chrono::Local;
use eframe::egui;
use std::sync::mpsc::Receiver;

const MAX_LOGS: usize = 100;

// ---------------------------------------------------------------------------
// Language
// ---------------------------------------------------------------------------

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum Language {
    En,
    Th,
}

/// All UI strings in both languages.
struct T {
    app_title: &'static str,
    websocket: &'static str,
    last_read: &'static str,
    waiting: &'static str,
    btn_show: &'static str,
    btn_hide: &'static str,
    logs: &'static str,
    photo: &'static str,
    no_photo: &'static str,
    card_info: &'static str,
    citizen_id: &'static str,
    th_prefix: &'static str,
    th_firstname: &'static str,
    th_middlename: &'static str,
    th_lastname: &'static str,
    en_prefix: &'static str,
    en_firstname: &'static str,
    en_middlename: &'static str,
    en_lastname: &'static str,
    name_en: &'static str,
    birthday: &'static str,
    sex: &'static str,
    issuer: &'static str,
    issue: &'static str,
    expire: &'static str,
    address: &'static str,
    insert_card: &'static str,
    insert_card_hint: &'static str,
}

const EN: T = T {
    app_title: "Smart Card Reader",
    websocket: "WebSocket:",
    last_read: "Last read:",
    waiting: "Waiting for card...",
    btn_show: "👁  Show Data",
    btn_hide: "🚫 Hide Data",
    logs: "Logs",
    photo: "Photo",
    no_photo: "No photo",
    card_info: "Card Information",
    citizen_id: "Citizen ID:",
    th_prefix: "Prefix (TH):",
    th_firstname: "First Name (TH):",
    th_middlename: "Middle Name (TH):",
    th_lastname: "Last Name (TH):",
    en_prefix: "Prefix (EN):",
    en_firstname: "First Name (EN):",
    en_middlename: "Middle Name (EN):",
    en_lastname: "Last Name (EN):",
    name_en: "Name (EN):",
    birthday: "Date of Birth:",
    sex: "Sex:",
    issuer: "Card Issuer:",
    issue: "Issue Date:",
    expire: "Expire Date:",
    address: "Address:",
    insert_card: "Please insert a Thai ID card",
    insert_card_hint: "Card data will appear here automatically.",
};

const TH: T = T {
    app_title: "เครื่องอ่านบัตรประชาชน",
    websocket: "WebSocket:",
    last_read: "อ่านล่าสุด:",
    waiting: "รอการ์ด...",
    btn_show: "👁  แสดงข้อมูล",
    btn_hide: "🚫 ซ่อนข้อมูล",
    logs: "บันทึก",
    photo: "รูปภาพ",
    no_photo: "ไม่มีรูป",
    card_info: "ข้อมูลบัตร",
    citizen_id: "เลขบัตรประชาชน:",
    th_prefix: "คำนำหน้า:",
    th_firstname: "ชื่อ:",
    th_middlename: "ชื่อกลาง:",
    th_lastname: "นามสกุล:",
    en_prefix: "Prefix (EN):",
    en_firstname: "First Name (EN):",
    en_middlename: "Middle Name (EN):",
    en_lastname: "Last Name (EN):",
    name_en: "ชื่อ-นามสกุล (อังกฤษ):",
    birthday: "วันเกิด:",
    sex: "เพศ:",
    issuer: "หน่วยงานออกบัตร:",
    issue: "วันออกบัตร:",
    expire: "วันหมดอายุ:",
    address: "ที่อยู่:",
    insert_card: "กรุณาใส่บัตรประชาชน",
    insert_card_hint: "ข้อมูลจะแสดงที่นี่โดยอัตโนมัติ",
};

fn t(lang: Language) -> &'static T {
    match lang {
        Language::En => &EN,
        Language::Th => &TH,
    }
}

fn get_font_paths(font_config: &FontConfig) -> Vec<std::path::PathBuf> {
    let mut paths = Vec::new();

    // Custom paths from config (highest priority)
    for custom_path in &font_config.custom_paths {
        paths.push(std::path::PathBuf::from(custom_path));
    }

    // Skip system fonts if disabled
    if !font_config.use_system_fonts {
        return paths;
    }

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
    paths.push(std::path::PathBuf::from("C:\\Windows\\Fonts\\LeelawUI.ttf")); // Leelawadee UI - best Thai font on Windows
    paths.push(std::path::PathBuf::from("C:\\Windows\\Fonts\\LeelUIsl.ttf")); // Leelawadee UI Semilight
    paths.push(std::path::PathBuf::from("C:\\Windows\\Fonts\\tahoma.ttf")); // Tahoma - fallback
    paths.push(std::path::PathBuf::from("C:\\Windows\\Fonts\\cordia.ttf")); // Cordia New
    paths.push(std::path::PathBuf::from("C:\\Windows\\Fonts\\angsau.ttf")); // AngsanaUPC

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

fn setup_fonts(ctx: &egui::Context, font_config: &FontConfig) {
    let mut fonts = egui::FontDefinitions::default();

    log::info!("Searching for Thai fonts...");
    for path in get_font_paths(font_config) {
        log::debug!("Checking font path: {:?}", path);
        if let Ok(font_data) = std::fs::read(&path) {
            let font_data = egui::FontData::from_owned(font_data);
            fonts
                .font_data
                .insert("noto_sans_thai".to_owned(), std::sync::Arc::new(font_data));

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
    for path in get_font_paths(font_config) {
        log::warn!("  - {:?} (exists: {})", path, path.exists());
    }
    log::warn!("Thai text will display as boxes. Please ensure a Thai font is available.");
    ctx.set_fonts(fonts);
}

// Embedded flag images (PNG bytes baked into binary)
const FLAG_TH_PNG: &[u8] = include_bytes!("../assets/flag_th.png");
const FLAG_GB_PNG: &[u8] = include_bytes!("../assets/flag_gb.png");

pub struct SmartCardApp {
    rx: Receiver<CardEvent>,
    card_data: Option<ThaiIDData>,
    logs: Vec<String>,
    photo_texture: Option<egui::TextureHandle>,
    flag_th: Option<egui::TextureHandle>,
    flag_gb: Option<egui::TextureHandle>,
    last_read_time: Option<String>,
    fonts_configured: bool,
    ws_url: String,
    font_config: FontConfig,
    data_hidden: bool,
    lang: Language,
}

impl SmartCardApp {
    pub fn new(rx: Receiver<CardEvent>, ws_url: String, font_config: FontConfig) -> Self {
        Self {
            rx,
            card_data: None,
            logs: vec![format!(
                "[{}] Application started",
                Local::now().format("%H:%M:%S")
            )],
            photo_texture: None,
            flag_th: None,
            flag_gb: None,
            last_read_time: None,
            fonts_configured: false,
            ws_url,
            font_config,
            data_hidden: true,
            lang: Language::Th,
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

    fn load_flag_textures(&mut self, ctx: &egui::Context) {
        if self.flag_th.is_none() {
            if let Ok(img) = image::load_from_memory(FLAG_TH_PNG) {
                let rgba = img.to_rgba8();
                let size = [rgba.width() as usize, rgba.height() as usize];
                let color_image = egui::ColorImage::from_rgba_unmultiplied(size, &rgba.into_raw());
                self.flag_th =
                    Some(ctx.load_texture("flag_th", color_image, egui::TextureOptions::LINEAR));
            }
        }
        if self.flag_gb.is_none() {
            if let Ok(img) = image::load_from_memory(FLAG_GB_PNG) {
                let rgba = img.to_rgba8();
                let size = [rgba.width() as usize, rgba.height() as usize];
                let color_image = egui::ColorImage::from_rgba_unmultiplied(size, &rgba.into_raw());
                self.flag_gb =
                    Some(ctx.load_texture("flag_gb", color_image, egui::TextureOptions::LINEAR));
            }
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
        let img = match image::load_from_memory_with_format(&photo_bytes, image::ImageFormat::Jpeg)
        {
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
        self.photo_texture =
            Some(ctx.load_texture("id_photo", color_image, egui::TextureOptions::LINEAR));
    }
}

impl eframe::App for SmartCardApp {
    fn update(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        // Setup fonts only once
        if !self.fonts_configured {
            setup_fonts(ctx, &self.font_config);
            self.fonts_configured = true;
        }

        // Load flag textures once
        self.load_flag_textures(ctx);

        // Check for card events
        while let Ok(event) = self.rx.try_recv() {
            match event {
                CardEvent::Inserted(data) => {
                    let id = &data.citizen_id;
                    let masked = if id.len() > 4 {
                        format!("{}{}", "*".repeat(id.len() - 4), &id[id.len() - 4..])
                    } else {
                        "*".repeat(id.len())
                    };
                    self.add_log(&format!("Card read: {}", masked));
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
        let tr = t(self.lang);
        egui::TopBottomPanel::top("status_bar").show(ctx, |ui| {
            ui.horizontal(|ui| {
                ui.label(egui::RichText::new(tr.app_title).strong());
                ui.separator();
                ui.label(format!("{} {}", tr.websocket, self.ws_url));
                ui.separator();
                if let Some(time) = &self.last_read_time {
                    ui.label(format!("{} {}", tr.last_read, time));
                } else {
                    ui.label(tr.waiting);
                }

                ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                    // Language toggle — flag image + label
                    let (flag_tex, lang_text, next_lang) = match self.lang {
                        Language::En => (self.flag_th.as_ref(), "TH", Language::Th),
                        Language::Th => (self.flag_gb.as_ref(), "EN", Language::En),
                    };

                    let clicked = ui
                        .horizontal(|ui| {
                            let resp = ui.add(
                                egui::Button::new(
                                    egui::RichText::new(lang_text)
                                        .color(egui::Color32::from_rgb(251, 191, 36)),
                                )
                                .min_size(egui::vec2(30.0, 0.0)),
                            );
                            if let Some(tex) = flag_tex {
                                let size = tex.size_vec2();
                                let scale = 20.0 / size.y;
                                ui.add(egui::Image::new((tex.id(), size * scale)));
                            }
                            resp.clicked()
                        })
                        .inner;

                    if clicked {
                        self.lang = next_lang;
                    }

                    // Show/hide toggle - only when card data is present
                    if self.card_data.is_some() {
                        ui.separator();
                        let (label, color) = if self.data_hidden {
                            (tr.btn_show, egui::Color32::from_rgb(129, 140, 248))
                        } else {
                            (tr.btn_hide, egui::Color32::from_rgb(148, 163, 184))
                        };
                        if ui
                            .add(
                                egui::Button::new(egui::RichText::new(label).color(color))
                                    .min_size(egui::vec2(130.0, 0.0)),
                            )
                            .clicked()
                        {
                            self.data_hidden = !self.data_hidden;
                        }
                    }
                });
            });
        });

        // Bottom panel - Logs (full width)
        let tr = t(self.lang);
        egui::TopBottomPanel::bottom("logs_panel")
            .resizable(true)
            .min_height(120.0)
            .default_height(160.0)
            .show(ctx, |ui| {
                ui.label(egui::RichText::new(tr.logs).size(13.0).strong());
                egui::ScrollArea::both()
                    .stick_to_bottom(true)
                    .auto_shrink([false, false])
                    .show(ui, |ui| {
                        ui.set_width(ui.available_width());
                        for log in &self.logs {
                            ui.add(
                                egui::Label::new(egui::RichText::new(log).size(14.0))
                                    .wrap_mode(egui::TextWrapMode::Extend),
                            );
                        }
                    });
            });

        // Central panel - Card data
        let data_hidden = self.data_hidden;
        let tr = t(self.lang);
        egui::CentralPanel::default().show(ctx, |ui| {
            if let Some(data) = &self.card_data {
                // Helper: masked value when hidden
                let mask = |_s: &str| "••••••••••••".to_string();

                const PHOTO_W: f32 = 180.0;
                const PHOTO_H: f32 = 240.0;

                egui::ScrollArea::vertical()
                    .auto_shrink([false, false])
                    .show(ui, |ui| {
                        ui.horizontal_top(|ui| {
                            // Left side - Photo
                            ui.vertical(|ui| {
                                ui.heading(tr.photo);
                                if data_hidden {
                                    let (rect, _) = ui.allocate_exact_size(
                                        egui::vec2(PHOTO_W, PHOTO_H),
                                        egui::Sense::hover(),
                                    );
                                    ui.painter().rect_filled(
                                        rect,
                                        8.0,
                                        egui::Color32::from_rgb(40, 45, 60),
                                    );
                                    ui.painter().text(
                                        rect.center(),
                                        egui::Align2::CENTER_CENTER,
                                        "🔒",
                                        egui::FontId::proportional(36.0),
                                        egui::Color32::from_rgb(100, 116, 139),
                                    );
                                } else if let Some(texture) = &self.photo_texture {
                                    ui.add(
                                        egui::Image::new((
                                            texture.id(),
                                            egui::vec2(PHOTO_W, PHOTO_H),
                                        ))
                                        .fit_to_exact_size(egui::vec2(PHOTO_W, PHOTO_H)),
                                    );
                                } else {
                                    let (rect, _) = ui.allocate_exact_size(
                                        egui::vec2(PHOTO_W, PHOTO_H),
                                        egui::Sense::hover(),
                                    );
                                    ui.painter().rect_filled(
                                        rect,
                                        8.0,
                                        egui::Color32::from_rgb(40, 45, 60),
                                    );
                                    ui.painter().text(
                                        rect.center(),
                                        egui::Align2::CENTER_CENTER,
                                        tr.no_photo,
                                        egui::FontId::proportional(14.0),
                                        egui::Color32::from_rgb(100, 116, 139),
                                    );
                                }
                            });

                            ui.separator();

                            // Right side - Card details
                            ui.vertical(|ui| {
                                ui.heading(tr.card_info);
                                ui.add_space(10.0);

                                egui::Grid::new("card_info_grid")
                                    .num_columns(2)
                                    .spacing([20.0, 8.0])
                                    .show(ui, |ui| {
                                        // --- Identity ---
                                        ui.label(egui::RichText::new(tr.citizen_id).strong());
                                        ui.label(if data_hidden {
                                            mask(&data.citizen_id)
                                        } else {
                                            data.citizen_id.clone()
                                        });
                                        ui.end_row();

                                        // --- Thai name components ---
                                        ui.label(egui::RichText::new(tr.th_prefix).strong());
                                        ui.label(if data_hidden {
                                            mask(&data.th_prefix)
                                        } else {
                                            data.th_prefix.clone()
                                        });
                                        ui.end_row();

                                        ui.label(egui::RichText::new(tr.th_firstname).strong());
                                        ui.label(if data_hidden {
                                            mask(&data.th_firstname)
                                        } else {
                                            data.th_firstname.clone()
                                        });
                                        ui.end_row();

                                        ui.label(egui::RichText::new(tr.th_middlename).strong());
                                        ui.label(if data_hidden {
                                            mask(&data.th_middlename)
                                        } else {
                                            data.th_middlename.clone()
                                        });
                                        ui.end_row();

                                        ui.label(egui::RichText::new(tr.th_lastname).strong());
                                        ui.label(if data_hidden {
                                            mask(&data.th_lastname)
                                        } else {
                                            data.th_lastname.clone()
                                        });
                                        ui.end_row();

                                        // --- English name ---
                                        ui.label(egui::RichText::new(tr.en_prefix).strong());
                                        ui.label(if data_hidden {
                                            mask(&data.en_prefix)
                                        } else {
                                            data.en_prefix.clone()
                                        });
                                        ui.end_row();

                                        ui.label(egui::RichText::new(tr.en_firstname).strong());
                                        ui.label(if data_hidden {
                                            mask(&data.en_firstname)
                                        } else {
                                            data.en_firstname.clone()
                                        });
                                        ui.end_row();

                                        ui.label(egui::RichText::new(tr.en_middlename).strong());
                                        ui.label(if data_hidden {
                                            mask(&data.en_middlename)
                                        } else {
                                            data.en_middlename.clone()
                                        });
                                        ui.end_row();

                                        ui.label(egui::RichText::new(tr.en_lastname).strong());
                                        ui.label(if data_hidden {
                                            mask(&data.en_lastname)
                                        } else {
                                            data.en_lastname.clone()
                                        });
                                        ui.end_row();

                                        // --- Date / Sex ---
                                        ui.label(egui::RichText::new(tr.birthday).strong());
                                        ui.label(if data_hidden {
                                            mask("")
                                        } else {
                                            format_thai_date(&data.birthday)
                                        });
                                        ui.end_row();

                                        ui.label(egui::RichText::new(tr.sex).strong());
                                        ui.label(if data_hidden {
                                            mask(&data.sex)
                                        } else {
                                            data.sex.clone()
                                        });
                                        ui.end_row();

                                        // --- Card meta ---
                                        ui.label(egui::RichText::new(tr.issuer).strong());
                                        ui.label(if data_hidden {
                                            mask(&data.issuer)
                                        } else {
                                            data.issuer.clone()
                                        });
                                        ui.end_row();

                                        ui.label(egui::RichText::new(tr.issue).strong());
                                        ui.label(if data_hidden {
                                            mask("")
                                        } else {
                                            format_thai_date(&data.issue)
                                        });
                                        ui.end_row();

                                        ui.label(egui::RichText::new(tr.expire).strong());
                                        ui.label(if data_hidden {
                                            mask("")
                                        } else {
                                            format_thai_date(&data.expire)
                                        });
                                        ui.end_row();

                                        // --- Address (UI only shows combined address) ---
                                        ui.label(egui::RichText::new(tr.address).strong());
                                        ui.label(if data_hidden {
                                            mask(&data.address)
                                        } else {
                                            data.address.clone()
                                        });
                                        ui.end_row();
                                    });
                            });
                        }); // horizontal_top
                    }); // ScrollArea
            } else {
                ui.centered_and_justified(|ui| {
                    ui.vertical_centered(|ui| {
                        ui.add_space(50.0);
                        ui.heading(tr.insert_card);
                        ui.add_space(20.0);
                        ui.label(tr.insert_card_hint);
                    });
                });
            }
        });
    }
}
