use egui::{Align, Color32, CornerRadius, Frame, Layout, RichText, ScrollArea, Stroke, Ui, Vec2};

use crate::app::MediaForgeApp;
use crate::config::*;
use crate::converter::ffmpeg::{self, FormatCategory, OutputFormat};
use crate::converter::job::{self, FileStatus, InputFile};
use crate::converter::progress;
use crate::media::detect::MediaType;
use crate::ui::{theme, widgets};

const INPUT_RATIO: f32 = 0.55;
const CARD_MARGIN: i8 = 10;

pub fn show(app: &mut MediaForgeApp, ui: &mut Ui) {
    // Only recompute formats when the file list or selections changed
    if app.formats_dirty {
        app.cached_formats = compute_available_formats(app);
        app.formats_dirty = false;
    }

    if app.selected_format.is_none() {
        app.selected_format = app.cached_formats.first().cloned();
    } else if let Some(selected) = &app.selected_format {
        if !app.cached_formats.iter().any(|f| f.label == selected.label) {
            app.selected_format = app.cached_formats.first().cloned();
        }
    }

    ui.spacing_mut().item_spacing = egui::vec2(6.0, 4.0);

    let content_width = ui.available_width();
    // Clone cached formats to avoid borrow issues
    let formats = app.cached_formats.clone();
    show_content(app, ui, &formats, content_width);
}

fn show_content(
    app: &mut MediaForgeApp,
    ui: &mut Ui,
    formats: &[OutputFormat],
    content_width: f32,
) {
    show_header(app, ui);
    ui.add_space(2.0);

    if content_width < 780.0 {
        // Narrow: stack vertically
        show_input_panel(app, ui);
        ui.add_space(2.0);
        show_output_panel(app, ui, formats);
    } else {
        // Wide: weighted two-column
        ui.horizontal_top(|ui| {
            let space = ui.available_width();
            let spacing = ui.spacing().item_spacing.x;
            let w1 = (space - spacing) * INPUT_RATIO;
            let w2 = space - spacing - w1;

            ui.allocate_ui_with_layout(
                egui::vec2(w1, ui.available_height()),
                Layout::top_down(Align::Min),
                |ui| {
                    show_input_panel(app, ui);
                },
            );

            ui.allocate_ui_with_layout(
                egui::vec2(w2, ui.available_height()),
                Layout::top_down(Align::Min),
                |ui| {
                    show_output_panel(app, ui, formats);
                },
            );
        });
    }
}

fn show_header(app: &mut MediaForgeApp, ui: &mut Ui) {
    let dark_mode = ui.visuals().dark_mode;
    card_frame(theme::surface_secondary(dark_mode), CARD_MARGIN).show(ui, |ui| {
        ui.horizontal(|ui| {
            ui.vertical(|ui| {
                ui.label(
                    RichText::new("MediaForge")
                        .size(20.0)
                        .strong()
                        .color(theme::text_primary(dark_mode)),
                );
                ui.horizontal_wrapped(|ui| {
                    ui.label(
                        RichText::new("Portable media converter")
                            .size(11.0)
                            .color(theme::text_secondary(dark_mode)),
                    );
                    ui.label(
                        RichText::new("(Made with Rust:")
                            .size(10.0)
                            .color(theme::ACCENT_LIGHT),
                    );
                    ui.hyperlink_to(
                        RichText::new("github.com/Hamza-op)")
                            .size(10.0)
                            .color(theme::text_secondary(dark_mode)),
                        "https://github.com/Hamza-op",
                    );
                });
            });

            ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
                if widgets::outline_button(ui, "\u{2699} Settings").clicked() {
                    app.show_settings = !app.show_settings;
                }

                let (status_text, status_color) = if app.progress.is_running {
                    ("Working", theme::ACCENT_WARM)
                } else if app.progress.is_complete {
                    ("Complete", theme::SUCCESS)
                } else {
                    ("Ready", theme::ACCENT_LIGHT)
                };

                pill(ui, status_text, status_color, color_with_alpha(status_color, 24));
            });
        });

        ui.add_space(4.0);
        ui.horizontal_wrapped(|ui| {
            compact_stat(ui, "Files", &app.files.len().to_string());
            compact_stat(
                ui,
                "Selected",
                &app.files.iter().filter(|f| f.selected).count().to_string(),
            );
            compact_stat(ui, "Format", selected_format_label(app));
            compact_stat(
                ui,
                "Engine",
                if app.ffmpeg_version.lock().is_some() {
                    "FFmpeg"
                } else {
                    "Bundled"
                },
            );
        });

        if app.progress.is_running || app.progress.is_complete {
            ui.add_space(4.0);
            let label = if app.progress.is_running {
                if let Some(eta) = app.progress.eta_secs {
                    format!(
                        "{:.0}% \u{2022} {} \u{2022} ETA {}",
                        app.progress.overall_pct,
                        app.progress.current_file_name,
                        progress::format_eta(eta)
                    )
                } else {
                    format!(
                        "{:.0}% \u{2022} {}",
                        app.progress.overall_pct, app.progress.current_file_name
                    )
                }
            } else {
                format!(
                    "Complete \u{2022} {} done, {} failed",
                    app.progress.succeeded, app.progress.failed
                )
            };
            widgets::smooth_progress(
                ui,
                (app.progress.overall_pct as f32 / 100.0).clamp(0.0, 1.0),
                &label,
            );
        }
    });
}

fn show_input_panel(app: &mut MediaForgeApp, ui: &mut Ui) {
    let dark_mode = ui.visuals().dark_mode;
    card_frame(theme::surface_primary(dark_mode), CARD_MARGIN).show(ui, |ui| {
        ui.set_min_width(ui.available_width());
        ui.set_min_height(ui.available_height());
        let subtitle = if app.is_importing {
            "Scanning folder in background. Files will appear as they are discovered."
        } else {
            "Add files and review the queue."
        };
        section_heading(ui, "Input", subtitle);

        let drop = widgets::drop_zone(ui, app.drop_hover, !app.files.is_empty());
        if drop.clicked() {
            open_file_dialog(app);
        }

        ui.horizontal(|ui| {
            if widgets::accent_button(ui, "+ Files").clicked() {
                open_file_dialog(app);
            }
            if widgets::outline_button(ui, "+ Folder").clicked() {
                open_folder_dialog(app);
            }
            if widgets::outline_button(ui, "Clear").clicked() {
                app.files.clear();
                app.formats_dirty = true;
            }

            ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
                let all_selected =
                    !app.files.is_empty() && app.files.iter().all(|f| f.selected);
                let label = if all_selected {
                    "Deselect All"
                } else {
                    "Select All"
                };
                if ui
                    .add(
                        egui::Button::new(RichText::new(label).size(11.0))
                            .fill(theme::inactive_chip(dark_mode))
                            .stroke(Stroke::new(1.0, theme::PANEL_STROKE))
                            .corner_radius(CornerRadius::same(10))
                            .min_size(Vec2::new(94.0, 28.0)),
                    )
                    .clicked()
                {
                    let next = !all_selected;
                    for file in &mut app.files {
                        file.selected = next;
                    }
                    app.formats_dirty = true;
                }
            });
        });

        ui.label(
            RichText::new(format!(
                "{} files  \u{2022}  {} selected  \u{2022}  {}",
                app.files.len(),
                app.files.iter().filter(|f| f.selected).count(),
                total_size_label(&app.files)
            ))
            .size(11.0)
            .color(theme::text_secondary(dark_mode)),
        );

        if app.is_importing {
            pill(
                ui,
                &format!("Importing {} discovered", app.files.len()),
                theme::ACCENT_LIGHT,
                color_with_alpha(theme::ACCENT_LIGHT, 24),
            );
        }

        show_file_list(app, ui);
    });
}

fn show_output_panel(app: &mut MediaForgeApp, ui: &mut Ui, formats: &[OutputFormat]) {
    let dark_mode = ui.visuals().dark_mode;
    card_frame(theme::surface_primary(dark_mode), CARD_MARGIN).show(ui, |ui| {
        ui.set_min_width(ui.available_width());
        ui.set_min_height(ui.available_height());
        ScrollArea::vertical()
            .id_salt("output_panel_scroll")
            .auto_shrink([false, false])
            .show(ui, |ui| {
                section_heading(ui, "Output", "Format, destination, and progress.");
                show_format_picker(ui, app, formats);
                output_settings(app, ui);
                show_convert_button(app, ui);
                ui.add_space(2.0);

                if let Some(format) = &app.selected_format {
                    show_quality_settings(ui, &format.category, &mut app.config);
                }

                if app.progress.is_running {
                    show_progress_hint(ui, app);
                } else if app.progress.is_complete {
                    show_complete(ui, app);
                } else {
                    idle_preview(app, ui);
                }
            });
    });
}

fn show_file_list(app: &mut MediaForgeApp, ui: &mut Ui) {
    let dark_mode = ui.visuals().dark_mode;
    let desired_height = ui.available_height().max(176.0);
    Frame::default()
        .fill(theme::surface_secondary(dark_mode))
        .stroke(Stroke::new(1.0, theme::PANEL_STROKE))
        .corner_radius(CornerRadius::same(14))
        .inner_margin(egui::Margin::same(10))
        .show(ui, |ui| {
            ui.set_min_height(desired_height);
            if app.files.is_empty() {
                ui.vertical_centered(|ui| {
                    ui.add_space(46.0);
                    ui.label(RichText::new("Nothing queued").size(18.0).strong());
                    ui.label(
                        RichText::new(
                            "Add files or a folder and everything stays on this screen.",
                        )
                        .size(11.0)
                        .color(theme::text_secondary(dark_mode)),
                    );
                });
                return;
            }

            ScrollArea::vertical()
                .id_salt("file_list")
                .auto_shrink([false, false])
                .max_height(desired_height - 10.0)
                .show(ui, |ui| {
                    let mut remove_idx = None;
                    for (idx, file) in app.files.iter_mut().enumerate() {
                        queue_row(ui, file, &mut remove_idx, idx);
                    }
                    if let Some(idx) = remove_idx {
                        app.files.remove(idx);
                        app.formats_dirty = true;
                    }
                });
        });
}

fn queue_row(ui: &mut Ui, file: &mut InputFile, remove_idx: &mut Option<usize>, idx: usize) {
    let dark_mode = ui.visuals().dark_mode;
    let row_fill = match &file.status {
        FileStatus::Converting => {
            if dark_mode {
                Color32::from_rgb(24, 48, 78)
            } else {
                Color32::from_rgb(217, 235, 252)
            }
        }
        FileStatus::Done => {
            if dark_mode {
                Color32::from_rgb(20, 45, 37)
            } else {
                Color32::from_rgb(221, 244, 232)
            }
        }
        FileStatus::Failed(_) => {
            if dark_mode {
                Color32::from_rgb(56, 28, 34)
            } else {
                Color32::from_rgb(251, 225, 229)
            }
        }
        FileStatus::Pending => theme::soft_fill(dark_mode),
    };

    Frame::default()
        .fill(row_fill)
        .stroke(Stroke::new(1.0, theme::PANEL_STROKE))
        .corner_radius(CornerRadius::same(12))
        .inner_margin(egui::Margin::same(10))
        .show(ui, |ui| {
            ui.set_min_width(ui.available_width());
            ui.horizontal(|ui| {
                ui.checkbox(&mut file.selected, "");
                ui.label(
                    RichText::new(media_icon(file.media_type))
                        .size(16.0)
                        .color(theme::ACCENT_LIGHT),
                );
                ui.vertical(|ui| {
                    let name_color = match &file.status {
                        FileStatus::Done => theme::SUCCESS,
                        FileStatus::Failed(_) => theme::ERROR,
                        FileStatus::Converting => theme::ACCENT_LIGHT,
                        FileStatus::Pending => theme::text_primary(dark_mode),
                    };
                    ui.label(
                        RichText::new(file.filename())
                            .size(12.0)
                            .strong()
                            .color(name_color),
                    );
                    let meta = file.metadata.info_string();
                    let sub = if meta.is_empty() {
                        file.size_string().to_string()
                    } else {
                        format!("{} \u{2022} {}", file.size_string(), meta)
                    };
                    ui.label(
                        RichText::new(sub)
                            .size(10.0)
                            .color(theme::text_secondary(dark_mode)),
                    );
                });

                ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
                    if ui
                        .add(
                            egui::Button::new(RichText::new("\u{2715}").size(10.0))
                                .fill(theme::inactive_chip(dark_mode))
                                .stroke(Stroke::new(1.0, theme::PANEL_STROKE))
                                .corner_radius(CornerRadius::same(10))
                                .min_size(Vec2::new(28.0, 24.0)),
                        )
                        .clicked()
                    {
                        *remove_idx = Some(idx);
                    }
                    let (label, color) = match &file.status {
                        FileStatus::Pending => ("Queued", theme::TEXT_DIM),
                        FileStatus::Converting => ("Working", theme::ACCENT_LIGHT),
                        FileStatus::Done => ("Done", theme::SUCCESS),
                        FileStatus::Failed(_) => ("Failed", theme::ERROR),
                    };
                    pill(ui, label, color, color_with_alpha(color, 20));
                });
            });
        });
}

fn output_settings(app: &mut MediaForgeApp, ui: &mut Ui) {
    subtle_panel(ui, |ui| {
        ui.horizontal(|ui| {
            ui.vertical(|ui| {
                ui.label(RichText::new("Destination").size(11.0).strong());
                ui.label(
                    RichText::new(output_dir_label(app))
                        .size(11.0)
                        .color(theme::TEXT_DIM),
                );
            });

            ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
                if widgets::outline_button(ui, "Browse").clicked() {
                    if let Some(folder) = rfd::FileDialog::new().pick_folder() {
                        app.custom_output_dir = Some(folder);
                    }
                }
            });
        });

        ui.horizontal_wrapped(|ui| {
            ui.checkbox(&mut app.config.add_suffix, "Add suffix");
            ui.checkbox(&mut app.config.overwrite_existing, "Overwrite");
        });
    });
}

fn show_quality_settings(ui: &mut Ui, category: &FormatCategory, config: &mut MediaForgeConfig) {
    subtle_panel(ui, |ui| {
        ui.label(RichText::new("Quality").size(11.0).strong());
        match category {
            FormatCategory::Image => {
                ui.add(egui::Slider::new(&mut config.image_quality, 1..=100).text("Quality"));
            }
            FormatCategory::Video => {
                ui.add(egui::Slider::new(&mut config.video_crf, 0..=51).text("CRF"));
                ui.horizontal_wrapped(|ui| {
                    ui.label("Preset");
                    egui::ComboBox::from_id_salt("preset")
                        .selected_text(config.video_preset.as_str())
                        .show_ui(ui, |ui| {
                            for preset in VideoPreset::ALL {
                                ui.selectable_value(
                                    &mut config.video_preset,
                                    *preset,
                                    preset.as_str(),
                                );
                            }
                        });
                    ui.label("Resolution");
                    egui::ComboBox::from_id_salt("res")
                        .selected_text(config.video_resolution.label())
                        .show_ui(ui, |ui| {
                            for resolution in ResolutionPreset::ALL {
                                ui.selectable_value(
                                    &mut config.video_resolution,
                                    *resolution,
                                    resolution.label(),
                                );
                            }
                        });
                });
            }
            FormatCategory::Audio => {
                ui.horizontal_wrapped(|ui| {
                    ui.add(
                        egui::Slider::new(&mut config.audio_bitrate, 64..=320).text("Bitrate"),
                    );
                    ui.label("Sample");
                    egui::ComboBox::from_id_salt("sr")
                        .selected_text(format!("{} Hz", config.audio_sample_rate))
                        .show_ui(ui, |ui| {
                            for rate in &[22050_u32, 44100, 48000, 96000] {
                                ui.selectable_value(
                                    &mut config.audio_sample_rate,
                                    *rate,
                                    format!("{rate} Hz"),
                                );
                            }
                        });
                    ui.label("Channels");
                    egui::ComboBox::from_id_salt("channels")
                        .selected_text(config.audio_channels.label())
                        .show_ui(ui, |ui| {
                            for channels in AudioChannels::ALL {
                                ui.selectable_value(
                                    &mut config.audio_channels,
                                    *channels,
                                    channels.label(),
                                );
                            }
                        });
                });
            }
        }
    });
}

fn show_format_picker(ui: &mut Ui, app: &mut MediaForgeApp, formats: &[OutputFormat]) {
    let dark_mode = ui.visuals().dark_mode;
    let active_category = app
        .selected_format
        .as_ref()
        .map(|f| f.category)
        .unwrap_or(FormatCategory::Image);

    ui.horizontal(|ui| {
        for (label, category) in [
            ("Video", FormatCategory::Video),
            ("Audio", FormatCategory::Audio),
            ("Image", FormatCategory::Image),
        ] {
            let has_any = formats.iter().any(|f| f.category == category);
            if !has_any {
                continue;
            }

            let selected = active_category == category;
            let fill = if selected {
                theme::ACCENT
            } else {
                theme::inactive_tab(dark_mode)
            };
            let text_color = if selected {
                Color32::WHITE
            } else {
                theme::text_primary(dark_mode)
            };
            if ui
                .add(
                    egui::Button::new(RichText::new(label).size(11.0).color(text_color))
                        .fill(fill)
                        .stroke(Stroke::new(
                            1.0,
                            if selected {
                                theme::ACCENT_HI
                            } else {
                                theme::PANEL_STROKE
                            },
                        ))
                        .corner_radius(CornerRadius::same(14))
                        .min_size(Vec2::new(70.0, 24.0)),
                )
                .clicked()
            {
                app.selected_format = formats.iter().find(|f| f.category == category).cloned();
            }
        }
    });

    ui.horizontal_wrapped(|ui| {
        for format in formats.iter().filter(|f| f.category == active_category) {
            let selected = app
                .selected_format
                .as_ref()
                .map(|current| current.label == format.label)
                .unwrap_or(false);
            let fill = if selected {
                theme::ACCENT
            } else {
                theme::inactive_chip(dark_mode)
            };
            let stroke = if selected {
                theme::ACCENT_HI
            } else {
                theme::PANEL_STROKE
            };
            let text_color = if selected {
                Color32::WHITE
            } else {
                theme::text_primary(dark_mode)
            };
            if ui
                .add(
                    egui::Button::new(RichText::new(format.label).size(11.0).color(text_color))
                        .fill(fill)
                        .stroke(Stroke::new(1.0, stroke))
                        .corner_radius(CornerRadius::same(18))
                        .min_size(Vec2::new(0.0, 24.0)),
                )
                .clicked()
            {
                app.selected_format = Some(format.clone());
            }
        }
    });
}

fn show_progress_hint(ui: &mut Ui, app: &mut MediaForgeApp) {
    subtle_panel(ui, |ui| {
        ui.label(RichText::new("Converting...").size(11.0).strong());
        if app.progress.current_file_name.is_empty() {
            ui.label(
                RichText::new("Overall progress is shown in the top header.")
                    .size(11.0)
                    .color(theme::TEXT_DIM),
            );
        } else {
            ui.label(
                RichText::new(format!(
                    "Processing {} \u{2022} {}/{} (overall bar in header)",
                    app.progress.current_file_name,
                    app.progress.current_file_index.saturating_add(1),
                    app.progress.total_files
                ))
                .size(11.0)
                .color(theme::TEXT_DIM),
            );
        }
        ui.horizontal(|ui| {
            if widgets::danger_button(ui, "Cancel").clicked() {
                app.cancel_conversion();
            }
            if !app.progress.speed_str.is_empty() {
                ui.label(
                    RichText::new(format!("Speed {}", app.progress.speed_str))
                        .size(11.0)
                        .color(theme::ACCENT_LIGHT),
                );
            }
        });
    });
}

fn show_complete(ui: &mut Ui, app: &mut MediaForgeApp) {
    subtle_panel(ui, |ui| {
        ui.horizontal(|ui| {
            pill(
                ui,
                &format!("{} done", app.progress.succeeded),
                theme::SUCCESS,
                color_with_alpha(theme::SUCCESS, 20),
            );
            pill(
                ui,
                &format!("{} failed", app.progress.failed),
                theme::ERROR,
                color_with_alpha(theme::ERROR, 20),
            );
        });
        if widgets::outline_button(ui, "Open Folder").clicked() {
            if let Some(dir) = &app.custom_output_dir {
                let _ = open::that(dir);
            } else if let Some(first) = app.files.first() {
                if let Some(parent) = first.path.parent() {
                    let _ = open::that(parent);
                }
            }
        }
    });
}

fn idle_preview(app: &MediaForgeApp, ui: &mut Ui) {
    subtle_panel(ui, |ui| {
        ui.label(RichText::new("Ready").size(11.0).strong());
        let selected = app.files.iter().filter(|f| f.selected).count();
        let text = if let Some(format) = &app.selected_format {
            format!("{selected} file(s) will convert to {}", format.label)
        } else {
            "Choose a format to enable conversion".to_string()
        };
        ui.label(RichText::new(text).size(11.0).color(theme::TEXT_DIM));
    });
}

fn show_convert_button(app: &mut MediaForgeApp, ui: &mut Ui) {
    let can_convert = !app.files.is_empty()
        && app.selected_format.is_some()
        && app.files.iter().any(|f| f.selected)
        && !app.progress.is_running;

    let btn_width = ui.available_width();

    ui.add_enabled_ui(can_convert, |ui| {
        let label = if app.progress.is_complete {
            "Convert Again"
        } else {
            "Start Conversion"
        };
        if ui
            .add(
                egui::Button::new(
                    RichText::new(format!("\u{25B6}  {label}"))
                        .size(14.0)
                        .strong()
                        .color(Color32::WHITE),
                )
                .fill(theme::ACCENT)
                .stroke(Stroke::new(1.0, theme::ACCENT_HI))
                .corner_radius(CornerRadius::same(14))
                .min_size(Vec2::new(btn_width, 34.0)),
            )
            .clicked()
        {
            if app.progress.is_complete {
                app.progress = Default::default();
            }
            app.start_conversion();
        }
    });
}

// ── Dialogs ──

fn open_file_dialog(app: &mut MediaForgeApp) {
    let extensions: Vec<&str> = crate::media::detect::supported_extensions();
    if let Some(paths) = rfd::FileDialog::new()
        .add_filter("Media Files", &extensions)
        .pick_files()
    {
        for path in paths {
            app.add_path(&path);
        }
    }
}

fn open_folder_dialog(app: &mut MediaForgeApp) {
    if let Some(folder) = rfd::FileDialog::new().pick_folder() {
        app.add_path(&folder);
    }
}

// ── Data helpers ──

/// Compute available formats based on current file selections.
/// This is only called when formats_dirty is true.
fn compute_available_formats(app: &MediaForgeApp) -> Vec<OutputFormat> {
    if app.files.is_empty() {
        let mut all = Vec::with_capacity(31);
        all.extend_from_slice(ffmpeg::image_output_formats());
        all.extend_from_slice(ffmpeg::video_output_formats());
        all.extend_from_slice(ffmpeg::audio_output_formats());
        return all;
    }

    let mut has_image = false;
    let mut has_video = false;
    let mut has_audio = false;

    for f in &app.files {
        if f.selected {
            match f.media_type {
                MediaType::Image => has_image = true,
                MediaType::Video => has_video = true,
                MediaType::Audio => has_audio = true,
                _ => {}
            }
            if has_image && has_video && has_audio {
                break;
            }
        }
    }

    let no_selection = !has_image && !has_video && !has_audio;
    let mut formats = Vec::with_capacity(31);

    if has_image || no_selection {
        formats.extend_from_slice(ffmpeg::image_output_formats());
    }
    if has_video || no_selection {
        formats.extend_from_slice(ffmpeg::video_output_formats());
        formats.extend_from_slice(ffmpeg::audio_output_formats());
    }
    if has_audio || no_selection {
        formats.extend_from_slice(ffmpeg::audio_output_formats());
    }

    let mut seen = std::collections::HashSet::new();
    formats.retain(|f| seen.insert(f.label));
    formats
}

// ── UI primitives ──

fn card_frame(fill: Color32, margin: i8) -> Frame {
    Frame::default()
        .fill(fill)
        .stroke(Stroke::new(1.0, theme::PANEL_STROKE))
        .corner_radius(CornerRadius::same(18))
        .inner_margin(egui::Margin::same(margin))
}

fn subtle_panel(ui: &mut Ui, add: impl FnOnce(&mut Ui)) {
    let dark_mode = ui.visuals().dark_mode;
    let w = ui.available_width();
    Frame::default()
        .fill(theme::surface_tertiary(dark_mode))
        .stroke(Stroke::new(1.0, theme::PANEL_STROKE))
        .corner_radius(CornerRadius::same(14))
        .inner_margin(egui::Margin::same(12))
        .show(ui, |ui| {
            ui.set_min_width(w - 26.0);
            add(ui);
        });
}

fn section_heading(ui: &mut Ui, title: &str, subtitle: &str) {
    let dark_mode = ui.visuals().dark_mode;
    ui.label(
        RichText::new(title)
            .size(14.0)
            .strong()
            .color(theme::text_primary(dark_mode)),
    );
    ui.label(
        RichText::new(subtitle)
            .size(10.0)
            .color(theme::text_secondary(dark_mode)),
    );
}

fn compact_stat(ui: &mut Ui, label: &str, value: &str) {
    let dark_mode = ui.visuals().dark_mode;
    Frame::default()
        .fill(theme::soft_fill(dark_mode))
        .stroke(Stroke::new(1.0, theme::PANEL_STROKE))
        .corner_radius(CornerRadius::same(12))
        .inner_margin(egui::Margin::symmetric(10, 6))
        .show(ui, |ui| {
            ui.vertical(|ui| {
                ui.label(
                    RichText::new(label)
                        .size(9.0)
                        .color(theme::text_secondary(dark_mode)),
                );
                ui.label(
                    RichText::new(value)
                        .size(11.0)
                        .strong()
                        .color(theme::text_primary(dark_mode)),
                );
            });
        });
}

fn pill(ui: &mut Ui, text: &str, text_color: Color32, fill: Color32) {
    Frame::default()
        .fill(fill)
        .stroke(Stroke::new(1.0, color_with_alpha(text_color, 40)))
        .corner_radius(CornerRadius::same(255))
        .inner_margin(egui::Margin::symmetric(9, 4))
        .show(ui, |ui| {
            ui.label(RichText::new(text).size(9.5).strong().color(text_color));
        });
}

fn color_with_alpha(c: Color32, alpha: u8) -> Color32 {
    Color32::from_rgba_unmultiplied(c.r(), c.g(), c.b(), alpha)
}

fn media_icon(media_type: MediaType) -> &'static str {
    match media_type {
        MediaType::Image => "\u{1F5BC}",
        MediaType::Video => "\u{1F3AC}",
        MediaType::Audio => "\u{1F3B5}",
        MediaType::Unknown => "\u{2022}",
    }
}

fn total_size_label(files: &[InputFile]) -> String {
    let total = files.iter().map(|f| f.file_size).sum();
    job::format_size(total)
}

fn selected_format_label(app: &MediaForgeApp) -> &str {
    app.selected_format
        .as_ref()
        .map(|f| f.label)
        .unwrap_or("Choose")
}

fn output_dir_label(app: &MediaForgeApp) -> String {
    app.custom_output_dir
        .as_ref()
        .map(|dir| dir.to_string_lossy().to_string())
        .unwrap_or_else(|| "Same folder as source files".to_string())
}
