use eframe::{egui, App, Frame};
use std::path::PathBuf;
use std::sync::mpsc::{channel, Receiver};

enum InstallStatus {
    Success(String),
    Error(String),
}

struct ModInstallerApp {
    dll_url: String,
    status_message: String,
    is_installing: bool,
    steam_path: Option<PathBuf>,
    game_path: Option<PathBuf>,
    runtime: tokio::runtime::Runtime,
    status_rx: Option<Receiver<InstallStatus>>,
}

impl Default for ModInstallerApp {
    fn default() -> Self {
        let mut app = Self {
            dll_url: String::from("https://github.com/alvindimas05/ThatNoobSkyMod/releases/latest/download/TNSM.dll"),
            status_message: String::from("Ready to install"),
            is_installing: false,
            steam_path: None,
            game_path: None,
            runtime: tokio::runtime::Runtime::new().unwrap(),
            status_rx: None,
        };
        app.detect_steam_path();
        app
    }
}

impl ModInstallerApp {
    fn detect_steam_path(&mut self) {
        // Common Steam installation paths
        let possible_paths = vec![
            PathBuf::from("C:\\Program Files (x86)\\Steam"),
            PathBuf::from("C:\\Program Files\\Steam"),
        ];

        for path in possible_paths {
            if path.exists() {
                self.steam_path = Some(path.clone());
                self.find_game_directory(&path);
                break;
            }
        }

        if self.steam_path.is_none() {
            self.status_message = "⚠ Steam directory not found. Please install manually.".to_string();
        }
    }

    fn find_game_directory(&mut self, steam_path: &PathBuf) {
        // Check common Steam library folders
        let library_folders = vec![
            steam_path.join("steamapps\\common\\Sky Children of the Light"),
            PathBuf::from("D:\\SteamLibrary\\steamapps\\common\\Sky Children of the Light"),
            PathBuf::from("E:\\SteamLibrary\\steamapps\\common\\Sky Children of the Light"),
        ];

        for folder in library_folders {
            if folder.exists() {
                self.game_path = Some(folder);
                self.status_message = format!("✓ Game found: {}", self.game_path.as_ref().unwrap().display());
                return;
            }
        }

        self.status_message = "⚠ Sky Children of the Light not found in Steam directories".to_string();
    }

    fn install_mod(&mut self, ctx: egui::Context) {
        if self.dll_url.is_empty() {
            self.status_message = "❌ Please enter a DLL URL first".to_string();
            return;
        }

        if self.game_path.is_none() {
            self.status_message = "❌ Game directory not found. Cannot install.".to_string();
            return;
        }

        self.is_installing = true;
        self.status_message = "⏳ Downloading and installing...".to_string();

        let game_path = self.game_path.as_ref().unwrap().clone();
        let dll_url = self.dll_url.clone();

        let (tx, rx) = channel();
        self.status_rx = Some(rx);

        self.runtime.spawn(async move {
            let result = download_and_install_async(&dll_url, &game_path).await;

            let status = match result {
                Ok(_) => InstallStatus::Success("✅ Mod installed successfully! Launch the game to use it.".to_string()),
                Err(e) => InstallStatus::Error(format!("❌ Installation failed: {}", e)),
            };

            let _ = tx.send(status);
            ctx.request_repaint();
        });
    }

    fn check_install_status(&mut self) {
        if let Some(rx) = &self.status_rx {
            if let Ok(status) = rx.try_recv() {
                match status {
                    InstallStatus::Success(msg) => {
                        self.status_message = msg;
                        self.is_installing = false;
                        self.status_rx = None;
                    }
                    InstallStatus::Error(msg) => {
                        self.status_message = msg;
                        self.is_installing = false;
                        self.status_rx = None;
                    }
                }
            }
        }
    }
}

async fn download_and_install_async(dll_url: &str, game_path: &PathBuf) -> Result<(), String> {
    let response = reqwest::get(dll_url)
        .await
        .map_err(|e| format!("Download failed: {}", e))?;

    let dll_bytes = response.bytes()
        .await
        .map_err(|e| format!("Failed to read response: {}", e))?;

    let dll_path = game_path.join("powrprof.dll");
    tokio::fs::write(&dll_path, dll_bytes)
        .await
        .map_err(|e| format!("Failed to write DLL: {}", e))?;

    Ok(())
}

impl App for ModInstallerApp {
    fn update(&mut self, ctx: &egui::Context, _frame: &mut Frame) {
        // Check for status updates from async task
        self.check_install_status();

        let mut style = (*ctx.style()).clone();
        style.visuals.widgets.noninteractive.bg_fill = egui::Color32::from_rgb(30, 30, 40);
        style.visuals.widgets.inactive.bg_fill = egui::Color32::from_rgb(50, 50, 65);
        style.visuals.widgets.hovered.bg_fill = egui::Color32::from_rgb(70, 70, 90);
        style.visuals.widgets.active.bg_fill = egui::Color32::from_rgb(90, 90, 120);
        ctx.set_style(style);

        egui::CentralPanel::default().show(ctx, |ui| {
            ui.vertical_centered(|ui| {
                ui.add_space(20.0);

                // Title
                ui.heading(egui::RichText::new("🌟 ThatNoobSkyApp")
                    .size(28.0)
                    .color(egui::Color32::from_rgb(135, 206, 250)));

                ui.add_space(10.0);
                ui.label(egui::RichText::new("Installer for ThatNoobSkyMod")
                    .size(14.0)
                    .color(egui::Color32::GRAY));

                ui.add_space(30.0);
            });

            // Status Information
            ui.group(|ui| {
                ui.set_width(470.0);
                ui.vertical(|ui| {
                    ui.label(egui::RichText::new("📍 Installation Status:").strong());
                    ui.add_space(5.0);

                    let status_color = if self.status_message.contains("✓") || self.status_message.contains("✅") {
                        egui::Color32::from_rgb(100, 255, 100)
                    } else if self.status_message.contains("⚠") {
                        egui::Color32::from_rgb(255, 200, 100)
                    } else if self.status_message.contains("❌") {
                        egui::Color32::from_rgb(255, 100, 100)
                    } else {
                        egui::Color32::WHITE
                    };

                    ui.label(egui::RichText::new(&self.status_message)
                        .color(status_color));

                    if let Some(game_path) = &self.game_path {
                        ui.add_space(5.0);
                        ui.label(egui::RichText::new(format!("📂 {}", game_path.display()))
                            .size(11.0)
                            .color(egui::Color32::GRAY));
                    }
                });
            });

            ui.add_space(30.0);

            // Install Button
            ui.vertical_centered(|ui| {
                let install_button = egui::Button::new(
                    egui::RichText::new("⚡ Install Mod")
                        .size(18.0)
                        .strong()
                ).min_size(egui::vec2(200.0, 45.0));

                ui.add_enabled_ui(!self.is_installing, |ui| {
                    if ui.add(install_button).clicked() {
                        self.install_mod(ctx.clone());
                    }
                });

                if self.is_installing {
                    ui.add_space(10.0);
                    ui.spinner();
                    ctx.request_repaint(); // Keep UI responsive during async operation
                }
            });

            ui.add_space(20.0);

            ui.vertical_centered(|ui| {
                ui.label(egui::RichText::new("⚠ Note: Run the game as usual to activate the mod")
                    .size(12.0)
                    .color(egui::Color32::from_rgb(255, 200, 100)));
            });
        });
    }
}

fn main() -> eframe::Result<()> {
    let options = eframe::NativeOptions {
        viewport: egui::ViewportBuilder::default()
            .with_inner_size([500.0, 400.0])
            .with_resizable(false),
        ..Default::default()
    };

    eframe::run_native(
        "ThatNoobSkyApp",
        options,
        Box::new(|_| Ok(Box::new(ModInstallerApp::default()))),
    )
}