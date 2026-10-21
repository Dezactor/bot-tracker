#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

use std::time::{Duration, Instant};

use eframe::egui;

unsafe extern "system" {
    fn Beep(dw_freq: u32, dw_duration: u32) -> i32;
}

fn play_beep(freq: u32) {
    std::thread::spawn(move || unsafe {
        Beep(freq, 400);
    });
}

fn set_window_opacity(title: &str, alpha: f32) {
    use std::ffi::OsStr;
    use std::os::windows::ffi::OsStrExt;
    use windows_sys::Win32::UI::WindowsAndMessaging::{
        FindWindowW, GetWindowLongW, SetLayeredWindowAttributes, SetWindowLongW, GWL_EXSTYLE,
        LWA_ALPHA, WS_EX_LAYERED,
    };

    let wide: Vec<u16> = OsStr::new(title)
        .encode_wide()
        .chain(std::iter::once(0))
        .collect();
    unsafe {
        let hwnd = FindWindowW(std::ptr::null(), wide.as_ptr());
        if !hwnd.is_null() {
            let ex = GetWindowLongW(hwnd, GWL_EXSTYLE);
            SetWindowLongW(hwnd, GWL_EXSTYLE, ex | WS_EX_LAYERED as i32);
            SetLayeredWindowAttributes(
                hwnd,
                0,
                (alpha.clamp(0.0, 1.0) * 255.0) as u8,
                LWA_ALPHA,
            );
        }
    }
}

const COUNTDOWN_COLORS: [(&str, egui::Color32); 5] = [
    ("#ffc107", egui::Color32::from_rgb(0xff, 0xc1, 0x07)),
    ("#ff5722", egui::Color32::from_rgb(0xff, 0x57, 0x22)),
    ("#28a745", egui::Color32::from_rgb(0x28, 0xa7, 0x45)),
    ("#007bff", egui::Color32::from_rgb(0x00, 0x7b, 0xff)),
    ("#e83e8c", egui::Color32::from_rgb(0xe8, 0x3e, 0x8c)),
];

const PROGRESS_COLORS: [(&str, egui::Color32); 4] = [
    ("blue", egui::Color32::from_rgb(0x00, 0x7b, 0xff)),
    ("green", egui::Color32::from_rgb(0x28, 0xa7, 0x45)),
    ("red", egui::Color32::from_rgb(0xdc, 0x35, 0x45)),
    ("orange", egui::Color32::from_rgb(0xfd, 0x7e, 0x14)),
];

const REACTION_COLORS: [(&str, egui::Color32); 4] = [
    ("#007bff", egui::Color32::from_rgb(0x00, 0x7b, 0xff)),
    ("#dc3545", egui::Color32::from_rgb(0xdc, 0x35, 0x45)),
    ("#28a745", egui::Color32::from_rgb(0x28, 0xa7, 0x45)),
    ("#ffc107", egui::Color32::from_rgb(0xff, 0xc1, 0x07)),
];

const SOUNDS: [(&str, u32); 3] = [("Beep 880", 880), ("Beep 600", 600), ("Beep 1000", 1000)];

fn format_time(t: f64, hide_ms: bool) -> String {
    let ms = ((t * 1000.0) as u64) % 1000;
    let secs = (t as u64) % 60;
    let mins = ((t as u64) / 60) % 60;
    let hours = (t as u64) / 3600;
    if hide_ms {
        format!("{hours:02}:{mins:02}:{secs:02}")
    } else {
        format!("{hours:02}:{mins:02}:{secs:02}.{ms:03}")
    }
}

fn color_combo(
    ui: &mut egui::Ui,
    label: &str,
    selected: &mut egui::Color32,
    colors: &[(&str, egui::Color32)],
    id: &str,
) {
    ui.horizontal(|ui| {
        ui.label(label);
        let name = colors
            .iter()
            .find(|(_, c)| c == selected)
            .map(|(n, _)| *n)
            .unwrap_or("");
        egui::ComboBox::from_id_salt(id)
            .selected_text(name)
            .show_ui(ui, |ui| {
                for (cname, color) in colors {
                    if ui.selectable_label(*selected == *color, *cname).clicked() {
                        *selected = *color;
                    }
                }
            });
        let (rect, _) = ui.allocate_exact_size(egui::vec2(18.0, 18.0), egui::Sense::hover());
        ui.painter().rect_filled(rect, 3.0, *selected);
    });
}

struct StopwatchApp {
    start_time: Option<Instant>,
    elapsed: f64,
    is_running: bool,
    is_looping: bool,
    interval: f64,
    loop_period: f64,
    next_deadline: Option<Instant>,
    show_interval_until: Option<Instant>,

    show_settings: bool,
    pending_resize: bool,

    sound_enabled: bool,
    sound_idx: usize,
    hide_ms: bool,
    offset_text: String,
    opacity: f32,
    topmost: bool,

    countdown_color: egui::Color32,
    progress_color: egui::Color32,
    reaction_enabled: bool,
    reaction_color: egui::Color32,

    status_text: String,
    status_color: egui::Color32,
    flash_until: Option<Instant>,
    flash_color: egui::Color32,
}

impl StopwatchApp {
    fn new(_cc: &eframe::CreationContext<'_>) -> Self {
        Self {
            start_time: None,
            elapsed: 0.0,
            is_running: false,
            is_looping: false,
            interval: 0.0,
            loop_period: 0.1,
            next_deadline: None,
            show_interval_until: None,
            show_settings: true,
            pending_resize: false,
            sound_enabled: true,
            sound_idx: 0,
            hide_ms: true,
            offset_text: "0.0".to_string(),
            opacity: 1.0,
            topmost: false,
            countdown_color: COUNTDOWN_COLORS[0].1,
            progress_color: PROGRESS_COLORS[0].1,
            reaction_enabled: false,
            reaction_color: REACTION_COLORS[0].1,
            status_text: String::new(),
            status_color: COUNTDOWN_COLORS[0].1,
            flash_until: None,
            flash_color: REACTION_COLORS[0].1,
        }
    }

    fn offset(&self) -> f64 {
        self.offset_text.trim().parse::<f64>().unwrap_or(0.0)
    }

    fn toggle_settings(&mut self) {
        self.show_settings = !self.show_settings;
        self.pending_resize = true;
    }

    fn start(&mut self) {
        if !self.is_running && !self.is_looping {
            self.is_running = true;
            self.start_time = Some(Instant::now() - Duration::from_secs_f64(self.elapsed));
            eprintln!("--- СТАРТ ---");
        }
    }

    fn stop(&mut self) {
        if !self.is_running {
            return;
        }
        self.is_running = false;
        self.interval = self.elapsed;
        self.is_looping = true;
        self.loop_period = (self.interval - self.offset()).max(0.1);
        self.next_deadline = Some(Instant::now() + Duration::from_secs_f64(self.loop_period));
        eprintln!("--- СТОП: интервал = {:.3} сек ---", self.interval);
        self.status_text = format!("🔁 Повтор каждые {:.3} сек", self.interval);
        self.status_color = self.countdown_color;
    }

    fn reset(&mut self) {
        self.is_running = false;
        self.is_looping = false;
        self.start_time = None;
        self.elapsed = 0.0;
        self.interval = 0.0;
        self.next_deadline = None;
        self.show_interval_until = None;
        self.status_text.clear();
        self.flash_until = None;
        eprintln!("--- СБРОС ---");
    }

    fn fire_beep(&mut self, now: Instant) {
        self.loop_period = (self.interval - self.offset()).max(0.1);
        let mut deadline =
            self.next_deadline.unwrap_or(now) + Duration::from_secs_f64(self.loop_period);
        while deadline <= now {
            deadline += Duration::from_secs_f64(self.loop_period);
        }
        self.next_deadline = Some(deadline);

        if self.sound_enabled {
            play_beep(SOUNDS[self.sound_idx].1);
        }
        if self.reaction_enabled {
            self.flash_until = Some(now + Duration::from_millis(300));
            self.flash_color = self.reaction_color;
        }

        let fmt = if self.hide_ms {
            format!("{:.0}", self.interval)
        } else {
            format!("{:.3}", self.interval)
        };
        self.status_text = format!("Интервал: {} сек | Отступ: {}s", fmt, self.offset());
        self.status_color = egui::Color32::from_rgb(0xdc, 0x35, 0x45);
        self.show_interval_until = Some(now + Duration::from_millis(100));
        eprintln!("--- ПОВТОР + ЗВУК (период {:.3}с) ---", self.loop_period);
    }

    fn tick(&mut self, now: Instant) {
        if self.is_running {
            if let Some(st) = self.start_time {
                self.elapsed = now.duration_since(st).as_secs_f64();
            }
        }
        if self.is_looping {
            if let Some(dl) = self.next_deadline {
                if now >= dl {
                    self.fire_beep(now);
                }
            }
        }
        if let Some(until) = self.flash_until {
            if now >= until {
                self.flash_until = None;
            }
        }
    }

    fn progress_pct(&self, now: Instant) -> f32 {
        if !self.is_looping || self.loop_period <= 0.0 {
            return 0.0;
        }
        let Some(dl) = self.next_deadline else {
            return 0.0;
        };
        let remaining = dl.saturating_duration_since(now).as_secs_f64();
        ((remaining / self.loop_period) * 100.0).clamp(0.0, 100.0) as f32
    }

    fn bg_color(&self) -> egui::Color32 {
        if self.flash_until.is_some() {
            self.flash_color
        } else {
            egui::Color32::from_rgb(0xf0, 0xf0, 0xf0)
        }
    }

    fn draw_ui(&mut self, ui: &mut egui::Ui, ctx: &egui::Context, now: Instant) {
        if ui.input(|i| i.pointer.button_double_clicked(egui::PointerButton::Primary))
            || ui.input(|i| i.pointer.button_pressed(egui::PointerButton::Secondary))
        {
            self.toggle_settings();
        }

        let avail_w = ui.available_width();
        let center_pad = |content_w: f32, avail_w: f32| -> f32 {
            ((avail_w - content_w) / 2.0).max(0.0)
        };

        ui.add_space(10.0);

        let text = format_time(self.elapsed, self.hide_ms);
        ui.horizontal(|ui| {
            let pad = center_pad(220.0, avail_w);
            ui.add_space(pad);
            ui.label(
                egui::RichText::new(text)
                    .size(48.0)
                    .color(egui::Color32::from_rgb(0x00, 0x7b, 0xff)),
            );
        });
        ui.add_space(5.0);

        let pct = self.progress_pct(now) / 100.0;
        ui.horizontal(|ui| {
            let pad = center_pad(250.0, avail_w);
            ui.add_space(pad);
            ui.add(
                egui::ProgressBar::new(pct)
                    .desired_width(250.0)
                    .fill(self.progress_color),
            );
        });
        ui.add_space(10.0);

        ui.horizontal(|ui| {
            let btn_total = 100.0 + 10.0 + 100.0;
            let pad = center_pad(btn_total, avail_w);
            ui.add_space(pad);
            let start_enabled = !self.is_running && !self.is_looping;
            let start = egui::Button::new(
                egui::RichText::new("🟢 СТАРТ").color(egui::Color32::WHITE),
            )
            .min_size(egui::vec2(100.0, 40.0))
            .fill(egui::Color32::from_rgb(0x28, 0xa7, 0x45));
            if ui.add_enabled(start_enabled, start).clicked() {
                self.start();
            }
            ui.add_space(10.0);
            let stop = egui::Button::new(
                egui::RichText::new("🔴 СТОП").color(egui::Color32::WHITE),
            )
            .min_size(egui::vec2(100.0, 40.0))
            .fill(egui::Color32::from_rgb(0xdc, 0x35, 0x45));
            if ui.add_enabled(self.is_running, stop).clicked() {
                self.stop();
            }
        });

        ui.add_space(10.0);

        ui.horizontal(|ui| {
            let pad = center_pad(100.0, avail_w);
            ui.add_space(pad);
            if ui.button("🔄 СБРОС").clicked() {
                self.reset();
            }
        });

        ui.add_space(10.0);

        if self.is_looping {
            let show_interval = self.show_interval_until.map_or(false, |t| now < t);
            if !show_interval {
                if let Some(dl) = self.next_deadline {
                    let remaining = dl.saturating_duration_since(now).as_secs_f64();
                    if remaining > 0.0 {
                        self.status_text = format!("⏱️ Обратный отсчёт: {}s", remaining as i64);
                        self.status_color = self.countdown_color;
                    }
                }
            }
        }
        ui.horizontal(|ui| {
            let pad = center_pad(300.0, avail_w);
            ui.add_space(pad);
            ui.label(
                egui::RichText::new(&self.status_text)
                    .size(18.0)
                    .color(self.status_color),
            );
        });

        if self.show_settings {
            ui.add_space(5.0);

            let block_w = 330.0;
            ui.horizontal(|ui| {
                let pad = center_pad(block_w, avail_w);
                ui.add_space(pad);
                ui.vertical(|ui| {
                    ui.set_width(block_w);

                    ui.horizontal(|ui| {
                        ui.label("Звук:");
                        egui::ComboBox::from_id_salt("sound")
                            .selected_text(SOUNDS[self.sound_idx].0)
                            .show_ui(ui, |ui| {
                                for (i, (name, _)) in SOUNDS.iter().enumerate() {
                                    ui.selectable_value(&mut self.sound_idx, i, *name);
                                }
                            });
                        ui.checkbox(&mut self.sound_enabled, "Звук включен");
                    });

                    ui.checkbox(&mut self.hide_ms, "Убрать мс");

                    ui.horizontal(|ui| {
                        ui.label("Отступ (сек):");
                        ui.add(
                            egui::TextEdit::singleline(&mut self.offset_text).desired_width(60.0),
                        );
                    });

                    ui.horizontal(|ui| {
                        ui.label("Прозрачность:");
                        let resp =
                            ui.add(egui::Slider::new(&mut self.opacity, 0.3..=1.0).step_by(0.05));
                        if resp.changed() {
                            set_window_opacity("Bot Tracker", self.opacity);
                        }
                    });

                    color_combo(
                        ui,
                        "Цвет отсчёта:",
                        &mut self.countdown_color,
                        &COUNTDOWN_COLORS,
                        "cd_color",
                    );
                    color_combo(
                        ui,
                        "Цвет прогресса:",
                        &mut self.progress_color,
                        &PROGRESS_COLORS,
                        "pr_color",
                    );

                    ui.checkbox(&mut self.reaction_enabled, "Реакция светом");
                    color_combo(
                        ui,
                        "Цвет реакции:",
                        &mut self.reaction_color,
                        &REACTION_COLORS,
                        "rx_color",
                    );

                    if ui.checkbox(&mut self.topmost, "Поверх всех окон").changed() {
                        ctx.send_viewport_cmd(egui::ViewportCommand::WindowLevel(
                            if self.topmost {
                                egui::WindowLevel::AlwaysOnTop
                            } else {
                                egui::WindowLevel::Normal
                            },
                        ));
                    }
                });
            });
        }
    }
}

impl eframe::App for StopwatchApp {
    fn logic(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        let now = Instant::now();
        self.tick(now);
        if self.is_running || self.is_looping {
            ctx.request_repaint_after(Duration::from_millis(50));
        }
    }

    fn ui(&mut self, ui: &mut egui::Ui, _frame: &mut eframe::Frame) {
        let now = Instant::now();
        let ctx = ui.ctx().clone();
        let bg = self.bg_color();

        ui.set_visuals(egui::Visuals::light());

        let mut content_size = egui::vec2(0.0, 0.0);
        egui::CentralPanel::default()
            .frame(egui::Frame::new().fill(bg).inner_margin(10.0))
            .show(ui, |ui| {
                ui.scope(|ui| {
                    self.draw_ui(ui, &ctx, now);
                    content_size = ui.min_size();
                });
            });

        if self.pending_resize && content_size.x > 0.0 && content_size.y > 0.0 {
            ctx.send_viewport_cmd(egui::ViewportCommand::InnerSize(
                content_size + egui::vec2(20.0, 20.0),
            ));
            self.pending_resize = false;
        }
    }

    fn clear_color(&self, _visuals: &egui::Visuals) -> [f32; 4] {
        self.bg_color().to_normalized_gamma_f32()
    }
}

fn load_icon() -> egui::IconData {
    let bytes = include_bytes!("../bot_icon.ico");
    let img = image::load_from_memory(bytes)
        .expect("failed to load icon")
        .to_rgba8();
    let (w, h) = img.dimensions();
    egui::IconData {
        rgba: img.into_raw(),
        width: w,
        height: h,
    }
}

fn main() -> eframe::Result {
    let options = eframe::NativeOptions {
        viewport: egui::ViewportBuilder::default()
            .with_title("Bot Tracker")
            .with_inner_size([450.0, 500.0])
            .with_min_inner_size([300.0, 200.0])
            .with_icon(load_icon()),
        ..Default::default()
    };
    eframe::run_native(
        "Bot Tracker",
        options,
        Box::new(|cc| Ok(Box::new(StopwatchApp::new(cc)))),
    )
}
