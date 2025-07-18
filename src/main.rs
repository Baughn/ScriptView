use eframe::egui;
use notify::{Config, RecommendedWatcher, RecursiveMode, Watcher};
use serde::{Deserialize, Serialize};
use std::path::Path;
use std::sync::mpsc::{channel, Receiver};
use std::sync::{Arc, Mutex};
use std::time::{Duration, Instant};

const LUA_SCRIPT: &str = include_str!("../subtitle-monitor.lua");

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
    always_on_top: bool,
    file_exists: bool,
    script_installed: bool,
    script_install_time: Option<Instant>,
    font_size: f32,
}

fn format_timestamp(seconds: f64) -> String {
    let total_seconds = seconds as u64;
    let hours = total_seconds / 3600;
    let minutes = (total_seconds % 3600) / 60;
    let secs = total_seconds % 60;
    let millis = ((seconds - total_seconds as f64) * 10.0) as u64;
    
    if hours > 0 {
        format!("{}:{:02}:{:02}.{}", hours, minutes, secs, millis)
    } else {
        format!("{}:{:02}.{}", minutes, secs, millis)
    }
}

fn filter_prefix_subtitles(subtitles: Vec<SubtitleEntry>) -> Vec<SubtitleEntry> {
    let mut filtered = Vec::new();
    for i in 0..subtitles.len() {
        let should_include = if i < subtitles.len() - 1 {
            // Check if current subtitle is a prefix of the next one
            !subtitles[i + 1].text.starts_with(&subtitles[i].text)
        } else {
            // Always include the last subtitle
            true
        };
        
        if should_include {
            filtered.push(subtitles[i].clone());
        }
    }
    filtered
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
            always_on_top: true,
            file_exists: false,
            script_installed: false,
            script_install_time: None,
            font_size: 14.0,
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
                let filtered_subs = filter_prefix_subtitles(subs);
                let mut subtitles = self.subtitles.lock().unwrap();
                *subtitles = filtered_subs;
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
        
        // Write the embedded script
        let target_path = format!("{}/subtitle-monitor.lua", mpv_scripts_dir);
        std::fs::write(target_path, LUA_SCRIPT)?;
        
        Ok(())
    }
}

impl eframe::App for SubtitleViewer {
    fn update(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
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
                // Controls at the top
                ui.horizontal(|ui| {
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
                    ui.separator();
                    ui.label("Font size:");
                    if ui.button("−").clicked() && self.font_size > 8.0 {
                        self.font_size -= 1.0;
                    }
                    ui.label(format!("{:.0}", self.font_size));
                    if ui.button("+").clicked() && self.font_size < 32.0 {
                        self.font_size += 1.0;
                    }
                });
                ui.separator();
                
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
                                    self.script_install_time = Some(Instant::now());
                                }
                                Err(_) => {}
                            }
                        }
                    });
                } else if let Some(install_time) = self.script_install_time {
                    if install_time.elapsed() < Duration::from_secs(5) {
                        ui.colored_label(
                            egui::Color32::from_rgb(0, 200, 0),
                            "✓ Script installed"
                        );
                    }
                }
                
                // Show file status warning
                if !self.file_exists {
                    ui.colored_label(
                        egui::Color32::from_rgb(255, 165, 0),
                        "⚠️ No subtitle data (maybe mpv isn't running?)"
                    );
                    ui.separator();
                }
                
                // Subtitle area with automatic scrolling
                let subtitles = self.subtitles.lock().unwrap();
                
                if subtitles.is_empty() {
                    ui.centered_and_justified(|ui| {
                        if self.file_exists {
                            ui.label("No subtitles yet...");
                        } else if self.script_installed {
                            ui.label("Start mpv to see subtitles here.");
                        } else {
                            ui.label("Install the script and start mpv to see subtitles.");
                        }
                    });
                } else {
                    egui::ScrollArea::vertical()
                        .stick_to_bottom(true)
                        .show(ui, |ui| {
                            ui.set_width(ui.available_width());
                            for sub in subtitles.iter() {
                                ui.allocate_ui_with_layout(
                                    egui::vec2(ui.available_width(), 0.0),
                                    egui::Layout::top_down(egui::Align::LEFT),
                                    |ui| {
                                        ui.group(|ui| {
                                            ui.set_width(ui.available_width());
                                            ui.horizontal_wrapped(|ui| {
                                                ui.label(
                                                    egui::RichText::new(format!("[{}]", format_timestamp(sub.start_time)))
                                                        .small()
                                                        .color(egui::Color32::from_gray(128)),
                                                );
                                                ui.label(egui::RichText::new(&sub.text.replace('\n', " ")).size(self.font_size));
                                            });
                                        });
                                    }
                                );
                                ui.add_space(4.0);
                            }
                        });
                }
                
                ui.separator();
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

#[cfg(test)]
mod tests {
    use super::*;

    fn create_subtitle(text: &str, start_time: f64) -> SubtitleEntry {
        SubtitleEntry {
            text: text.to_string(),
            start_time,
            end_time: None,
            timestamp: 0,
        }
    }

    #[test]
    fn test_filter_no_prefixes() {
        let subtitles = vec![
            create_subtitle("Hello world", 1.0),
            create_subtitle("Goodbye world", 2.0),
            create_subtitle("Another subtitle", 3.0),
        ];
        
        let filtered = filter_prefix_subtitles(subtitles.clone());
        assert_eq!(filtered.len(), 3);
        assert_eq!(filtered[0].text, "Hello world");
        assert_eq!(filtered[1].text, "Goodbye world");
        assert_eq!(filtered[2].text, "Another subtitle");
    }

    #[test]
    fn test_filter_single_prefix() {
        let subtitles = vec![
            create_subtitle("Hello", 1.0),
            create_subtitle("Hello world", 2.0),
            create_subtitle("Goodbye", 3.0),
        ];
        
        let filtered = filter_prefix_subtitles(subtitles);
        assert_eq!(filtered.len(), 2);
        assert_eq!(filtered[0].text, "Hello world");
        assert_eq!(filtered[1].text, "Goodbye");
    }

    #[test]
    fn test_filter_multiple_prefixes() {
        let subtitles = vec![
            create_subtitle("H", 1.0),
            create_subtitle("He", 1.5),
            create_subtitle("Hel", 2.0),
            create_subtitle("Hell", 2.5),
            create_subtitle("Hello", 3.0),
            create_subtitle("Hello world", 3.5),
            create_subtitle("Next subtitle", 4.0),
        ];
        
        let filtered = filter_prefix_subtitles(subtitles);
        assert_eq!(filtered.len(), 2);
        assert_eq!(filtered[0].text, "Hello world");
        assert_eq!(filtered[1].text, "Next subtitle");
    }

    #[test]
    fn test_filter_keeps_last_subtitle() {
        let subtitles = vec![
            create_subtitle("Hello", 1.0),
            create_subtitle("World", 2.0),
        ];
        
        let filtered = filter_prefix_subtitles(subtitles);
        assert_eq!(filtered.len(), 2);
        assert_eq!(filtered[1].text, "World");
    }

    #[test]
    fn test_filter_empty_list() {
        let subtitles = vec![];
        let filtered = filter_prefix_subtitles(subtitles);
        assert_eq!(filtered.len(), 0);
    }

    #[test]
    fn test_filter_single_subtitle() {
        let subtitles = vec![create_subtitle("Only one", 1.0)];
        let filtered = filter_prefix_subtitles(subtitles);
        assert_eq!(filtered.len(), 1);
        assert_eq!(filtered[0].text, "Only one");
    }

    #[test]
    fn test_filter_progressive_typing() {
        // Simulates progressive typing/display of a subtitle
        let subtitles = vec![
            create_subtitle("I", 1.0),
            create_subtitle("I a", 1.1),
            create_subtitle("I am", 1.2),
            create_subtitle("I am t", 1.3),
            create_subtitle("I am ty", 1.4),
            create_subtitle("I am typ", 1.5),
            create_subtitle("I am typi", 1.6),
            create_subtitle("I am typin", 1.7),
            create_subtitle("I am typing", 1.8),
            create_subtitle("I am typing this", 1.9),
            create_subtitle("I am typing this message", 2.0),
            create_subtitle("Next subtitle", 3.0),
        ];
        
        let filtered = filter_prefix_subtitles(subtitles);
        assert_eq!(filtered.len(), 2);
        assert_eq!(filtered[0].text, "I am typing this message");
        assert_eq!(filtered[1].text, "Next subtitle");
    }

    #[test]
    fn test_filter_non_prefix_similar_start() {
        // These start similarly but aren't prefixes
        let subtitles = vec![
            create_subtitle("Hello world", 1.0),
            create_subtitle("Hello there", 2.0),
            create_subtitle("Helicopter", 3.0),
        ];
        
        let filtered = filter_prefix_subtitles(subtitles);
        assert_eq!(filtered.len(), 3);
        assert_eq!(filtered[0].text, "Hello world");
        assert_eq!(filtered[1].text, "Hello there");
        assert_eq!(filtered[2].text, "Helicopter");
    }
}
