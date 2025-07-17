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
        };
        
        // Load initial content
        viewer.load_subtitles();
        
        viewer
    }
    
    fn load_subtitles(&mut self) {
        if let Ok(content) = std::fs::read_to_string(&self.subtitle_file) {
            if let Ok(subs) = serde_json::from_str::<Vec<SubtitleEntry>>(&content) {
                let mut subtitles = self.subtitles.lock().unwrap();
                *subtitles = subs;
            }
        }
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
                
                // Get subtitles
                let subtitles = self.subtitles.lock().unwrap();
                let display_subs: Vec<_> = subtitles
                    .iter()
                    .rev()
                    .take(self.display_count)
                    .collect();
                
                if display_subs.is_empty() {
                    ui.label("No subtitles yet...");
                } else {
                    egui::ScrollArea::vertical()
                        .auto_shrink([false; 2])
                        .show(ui, |ui| {
                            for (i, sub) in display_subs.iter().enumerate() {
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
                        });
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