use eframe::egui;
use notify::{Config, RecommendedWatcher, RecursiveMode, Watcher};
use serde::{Deserialize, Serialize};
use std::path::Path;
use std::sync::mpsc::{channel, Receiver};
use std::sync::{Arc, Mutex};

#[derive(Debug, Clone, Serialize, Deserialize)]
struct SubtitleEntry {
    text: String,
    start_time: f64,
    #[serde(skip_serializing_if = "Option::is_none")]
    end_time: Option<f64>,
    timestamp: i64,
}

struct SubtitleViewer {
    subtitles: Arc<Mutex<Vec<SubtitleEntry>>>,
    rx: Receiver<notify::Result<notify::Event>>,
    subtitle_file: String,
    display_count: usize,
    always_on_top: bool,
    file_exists: bool,
    script_installed: bool,
}

impl SubtitleViewer {
    fn new() -> Self {
        let (tx, rx) = channel();
        let subtitle_file = "/tmp/mpv-subtitles.json".to_string();
        
        // Set up file watcher
        let mut watcher = RecommendedWatcher::new(tx, Config::default()).unwrap();
        watcher
            .watch(Path::new(&subtitle_file), RecursiveMode::NonRecursive)
            .unwrap_or_else(|_| {
                eprintln!("Warning: Could not watch subtitle file. Will attempt to read it anyway.");
            });
        
        // Keep watcher alive
        Box::leak(Box::new(watcher));
        
        let mut viewer = Self {
            subtitles: Arc::new(Mutex::new(Vec::new())),
            rx,
            subtitle_file,
            display_count: 10,
            always_on_top: true,
            file_exists: false,
            script_installed: false,
        };
        
        // Load initial content
        viewer.load_subtitles();
        
        viewer
    }
    
    fn load_subtitles(&mut self) {
        self.file_exists = std::path::Path::new(&self.subtitle_file).exists();
        self.script_installed = self.check_script_installed();
        if let Ok(content) = std::fs::read_to_string(&self.subtitle_file) {
            if let Ok(subs) = serde_json::from_str::<Vec<SubtitleEntry>>(&content) {
                let mut subtitles = self.subtitles.lock().unwrap();
                *subtitles = subs;
            }
        }
    }
    
    fn check_script_installed(&self) -> bool {
        let home_dir = std::env::var("HOME").unwrap_or_else(|_| "/tmp".to_string());
        let script_path = format!("{}/.config/mpv/scripts/subtitle-monitor.lua", home_dir);
        std::path::Path::new(&script_path).exists()
    }
    
    fn install_lua_script(&self) -> Result<(), std::io::Error> {
        let home_dir = std::env::var("HOME").unwrap_or_else(|_| "/tmp".to_string());
        let mpv_scripts_dir = format!("{}/.config/mpv/scripts", home_dir);
        
        // Create directory if it doesn't exist
        std::fs::create_dir_all(&mpv_scripts_dir)?;
        
        // Copy the script
        let script_content = std::fs::read_to_string("subtitle-monitor.lua")?;
        let target_path = format!("{}/subtitle-monitor.lua", mpv_scripts_dir);
        std::fs::write(target_path, script_content)?;
        
        Ok(())
    }
}

impl eframe::App for SubtitleViewer {
    fn update(&mut self, ctx: &egui::Context, frame: &mut eframe::Frame) {
        // Check for file changes
        while let Ok(event) = self.rx.try_recv() {
            if let Ok(_) = event {
                self.load_subtitles();
            }
        }
        
        // Request repaint for continuous updates
        ctx.request_repaint_after(std::time::Duration::from_millis(100));
        
        egui::CentralPanel::default().show(ctx, |ui| {
            ui.vertical(|ui| {
                ui.heading("MPV Subtitle History");
                ui.separator();
                
                // Get subtitles (keep original order for bottom alignment)
                let display_subs: Vec<_> = {
                    let subtitles = self.subtitles.lock().unwrap();
                    let len = subtitles.len();
                    if len > self.display_count {
                        subtitles[(len - self.display_count)..]
                            .iter()
                            .cloned()
                            .collect()
                    } else {
                        subtitles.iter().cloned().collect()
                    }
                };
                
                // Show script installation status
                if !self.script_installed {
                    ui.horizontal(|ui| {
                        ui.colored_label(
                            egui::Color32::from_rgb(255, 165, 0),
                            "⚠️ Script not installed:"
                        );
                        if ui.button("Install Script").clicked() {
                            match self.install_lua_script() {
                                Ok(_) => {
                                    self.script_installed = true;
                                }
                                Err(_) => {}
                            }
                        }
                    });
                } else {
                    ui.colored_label(
                        egui::Color32::from_rgb(0, 200, 0),
                        "✓ Script installed"
                    );
                }
                
                // Show file status warning
                if !self.file_exists {
                    ui.colored_label(
                        egui::Color32::from_rgb(255, 165, 0),
                        "⚠️ No subtitle data (maybe mpv isn't running?)"
                    );
                    ui.separator();
                }
                
                // Subtitle area with bottom alignment
                let remaining_height = ui.available_height() - 50.0; // Reserve space for controls
                
                if display_subs.is_empty() {
                    ui.allocate_ui_with_layout(
                        egui::vec2(ui.available_width(), remaining_height),
                        egui::Layout::bottom_up(egui::Align::Center),
                        |ui| {
                            if self.file_exists {
                                ui.label("No subtitles yet...");
                            } else if self.script_installed {
                                ui.label("Start mpv to see subtitles here.");
                            } else {
                                ui.label("Install the script and start mpv to see subtitles.");
                            }
                        }
                    );
                } else {
                    ui.allocate_ui_with_layout(
                        egui::vec2(ui.available_width(), remaining_height),
                        egui::Layout::bottom_up(egui::Align::LEFT),
                        |ui| {
                            // Reverse order since bottom_up layout displays from bottom
                            for (i, sub) in display_subs.iter().rev().enumerate() {
                                ui.group(|ui| {
                                    ui.horizontal(|ui| {
                                        // Time stamp
                                        ui.label(
                                            egui::RichText::new(format!(
                                                "[{:.1}s]",
                                                sub.start_time
                                            ))
                                            .small()
                                            .color(egui::Color32::from_gray(128)),
                                        );
                                        
                                        // Subtitle text
                                        ui.label(&sub.text);
                                    });
                                });
                                
                                if i < display_subs.len() - 1 {
                                    ui.add_space(2.0);
                                }
                            }
                        }
                    );
                }
                
                ui.separator();
                
                // Controls
                ui.horizontal(|ui| {
                    ui.label("Show last:");
                    if ui.button("-").clicked() && self.display_count > 1 {
                        self.display_count -= 1;
                    }
                    ui.label(format!("{}", self.display_count));
                    if ui.button("+").clicked() && self.display_count < 50 {
                        self.display_count += 1;
                    }
                    
                    ui.separator();
                    
                    if ui.button("Clear").clicked() {
                        self.subtitles.lock().unwrap().clear();
                    }
                    
                    ui.separator();
                    
                    if ui.button(if self.always_on_top { "Always On Top ✓" } else { "Always On Top" }).clicked() {
                        self.always_on_top = !self.always_on_top;
                        ctx.send_viewport_cmd(egui::ViewportCommand::WindowLevel(
                            if self.always_on_top {
                                egui::WindowLevel::AlwaysOnTop
                            } else {
                                egui::WindowLevel::Normal
                            }
                        ));
                    }
                });
            });
        });
    }
}

fn main() -> Result<(), eframe::Error> {
    let options = eframe::NativeOptions {
        viewport: egui::ViewportBuilder::default()
            .with_inner_size([500.0, 600.0])
            .with_always_on_top(),
        ..Default::default()
    };
    
    eframe::run_native(
        "ScriptView",
        options,
        Box::new(|_cc| Ok(Box::new(SubtitleViewer::new()))),
    )
}