use egui::RichText;

use crate::app::MediaForgeApp;
use crate::config::*;
use crate::platform::context_menu;
use crate::ui::{theme, widgets};

pub fn show(app: &mut MediaForgeApp, ctx: &egui::Context) {
    egui::Window::new("Settings")
        .open(&mut app.show_settings)
        .resizable(true)
        .default_width(450.0)
        .default_height(500.0)
        .show(ctx, |ui| {
            egui::ScrollArea::vertical().show(ui, |ui| {
                // General
                egui::CollapsingHeader::new(RichText::new("\u{2699} General").size(15.0).strong())
                    .default_open(true)
                    .show(ui, |ui| {
                        ui.horizontal(|ui| {
                            ui.label("Theme:");
                            if ui
                                .selectable_label(app.config.theme == Theme::Dark, "Dark")
                                .clicked()
                            {
                                app.config.theme = Theme::Dark;
                                ctx.set_visuals(theme::dark_theme());
                            }
                            if ui
                                .selectable_label(app.config.theme == Theme::Light, "Light")
                                .clicked()
                            {
                                app.config.theme = Theme::Light;
                                ctx.set_visuals(theme::light_theme());
                            }
                        });

                        ui.add_space(4.0);

                        ui.horizontal(|ui| {
                            ui.label("Default suffix:");
                            ui.text_edit_singleline(&mut app.config.default_suffix);
                        });

                        ui.checkbox(
                            &mut app.config.show_notification,
                            "Show system notification on completion",
                        );
                        ui.checkbox(
                            &mut app.config.play_sound_on_complete,
                            "Play sound on completion",
                        );

                        ui.horizontal(|ui| {
                            ui.label("Max folder scan depth:");
                            ui.add(
                                egui::DragValue::new(&mut app.config.max_folder_scan_depth)
                                    .range(1..=50),
                            );
                        });
                    });

                ui.add_space(8.0);

                // Performance
                egui::CollapsingHeader::new(
                    RichText::new("\u{26A1} Performance").size(15.0).strong(),
                )
                .default_open(false)
                .show(ui, |ui| {
                    ui.horizontal(|ui| {
                        ui.label("Max concurrent conversions:");
                        ui.add(
                            egui::DragValue::new(&mut app.config.max_concurrent_conversions)
                                .range(1..=32),
                        );
                    });

                    ui.horizontal(|ui| {
                        ui.label("Hardware acceleration:");
                        egui::ComboBox::from_id_salt("settings_hw_accel")
                            .selected_text(app.config.hw_accel.label())
                            .show_ui(ui, |ui| {
                                for hw in HwAccel::ALL {
                                    ui.selectable_value(
                                        &mut app.config.hw_accel,
                                        *hw,
                                        hw.label(),
                                    );
                                }
                            });
                    });

                    ui.add_space(4.0);

                    ui.horizontal(|ui| {
                        let mut use_custom_threads = app.config.ffmpeg_threads.is_some();
                        if ui
                            .checkbox(&mut use_custom_threads, "Override FFmpeg threads:")
                            .changed()
                        {
                            if use_custom_threads {
                                app.config.ffmpeg_threads = Some(0);
                            } else {
                                app.config.ffmpeg_threads = None;
                            }
                        }
                        if let Some(threads) = &mut app.config.ffmpeg_threads {
                            ui.add(egui::DragValue::new(threads).range(0..=64));
                            ui.label("(0 = auto)");
                        }
                    });

                    ui.add_space(4.0);

                    ui.horizontal(|ui| {
                        let temp_dir_str = app
                            .config
                            .temp_dir
                            .as_ref()
                            .map(|p| p.to_string_lossy().to_string())
                            .unwrap_or_else(|| "Default OS temp".to_string());
                        ui.label(format!("Temp dir: {temp_dir_str}"));
                        if ui.small_button("Browse").clicked() {
                            if let Some(folder) = rfd::FileDialog::new().pick_folder() {
                                app.config.temp_dir = Some(folder);
                            }
                        }
                        if app.config.temp_dir.is_some() && ui.small_button("Reset").clicked() {
                            app.config.temp_dir = None;
                        }
                    });
                });

                ui.add_space(8.0);

                // Context Menu
                egui::CollapsingHeader::new(
                    RichText::new("\u{1F4C2} Context Menu").size(15.0).strong(),
                )
                .default_open(false)
                .show(ui, |ui| {
                    let is_registered = context_menu::is_registered();

                    ui.label(
                        RichText::new(if is_registered {
                            "\u{2705} Context menu is registered"
                        } else {
                            "\u{274C} Context menu is not registered"
                        })
                        .size(13.0),
                    );

                    ui.add_space(4.0);

                    ui.horizontal(|ui| {
                        if !is_registered {
                            if widgets::accent_button(ui, "Register").clicked() {
                                match context_menu::register_context_menu() {
                                    Ok(_) => {
                                        app.status_message =
                                            "Context menu registered successfully!".to_string()
                                    }
                                    Err(e) => {
                                        app.status_message =
                                            format!("Failed to register: {e}")
                                    }
                                }
                            }
                        } else if widgets::danger_button(ui, "Unregister").clicked() {
                            match context_menu::unregister_context_menu() {
                                Ok(_) => {
                                    app.status_message =
                                        "Context menu unregistered".to_string()
                                }
                                Err(e) => {
                                    app.status_message =
                                        format!("Failed to unregister: {e}")
                                }
                            }
                        }
                    });
                });

                ui.add_space(8.0);

                // About
                egui::CollapsingHeader::new(
                    RichText::new("\u{2139} About").size(15.0).strong(),
                )
                .default_open(false)
                .show(ui, |ui| {
                    ui.label(
                        RichText::new("MediaForge")
                            .size(18.0)
                            .strong()
                            .color(theme::ACCENT),
                    );
                    ui.label("All-in-One Media Converter");
                    ui.label(format!("Version: {}", env!("CARGO_PKG_VERSION")));

                    if let Some(ref ver) = *app.ffmpeg_version.lock() {
                        ui.label(
                            RichText::new(format!("Engine: {ver}"))
                                .size(12.0)
                                .color(theme::TEXT_DIM),
                        );
                    } else {
                        ui.label(
                            RichText::new("Engine: Bundled FFmpeg (version check on demand)")
                                .size(12.0)
                                .color(theme::TEXT_DIM),
                        );
                    }

                    ui.add_space(4.0);
                    ui.label(
                        RichText::new("Built with Rust + egui")
                            .size(11.0)
                            .color(theme::TEXT_DIM),
                    );
                });

                ui.add_space(12.0);

                // Save button
                ui.horizontal(|ui| {
                    if widgets::accent_button(ui, "Save Settings").clicked() {
                        app.config.save();
                        app.status_message = "Settings saved".to_string();
                    }
                });

                if app.status_message == "Settings saved" {
                    ui.add_space(6.0);
                    ui.label(
                        RichText::new("Settings saved")
                            .size(12.0)
                            .color(theme::SUCCESS),
                    );
                }
            });
        });
}
