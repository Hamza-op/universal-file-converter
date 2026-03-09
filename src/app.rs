use std::path::PathBuf;
use std::sync::Arc;

use crossbeam_channel::{Receiver, Sender};
use eframe::egui;
use parking_lot::Mutex;

use crate::config::{MediaForgeConfig, Theme};
use crate::converter::ffmpeg::{self, OutputFormat};
use crate::converter::job::{self, ConversionMessage, ConversionProgress, FileStatus, InputFile};
use crate::media::detect::{self, MediaType};
use crate::media::metadata;
use crate::ui::{main_view, settings, theme};

pub enum ImportMessage {
    FileDiscovered(InputFile),
    BatchDone { added: usize },
}

pub struct MediaForgeApp {
    // Input
    pub files: Vec<InputFile>,
    pub drop_hover: bool,

    // Output
    pub selected_format: Option<OutputFormat>,
    pub custom_output_dir: Option<PathBuf>,

    // Conversion
    pub progress: ConversionProgress,
    pub cancel_flag: Arc<Mutex<bool>>,
    pub msg_sender: Sender<ConversionMessage>,
    pub msg_receiver: Receiver<ConversionMessage>,

    // UI state
    pub show_settings: bool,
    pub status_message: String,
    pub ffmpeg_version: Arc<Mutex<Option<String>>>,

    // Config
    pub config: MediaForgeConfig,

    // IPC
    pub ipc_receiver: Option<Receiver<Vec<String>>>,
    pub import_sender: Sender<ImportMessage>,
    pub import_receiver: Receiver<ImportMessage>,

    // Init
    pub fonts_configured: bool,
    pub ffmpeg_check_requested: bool,
    pub is_importing: bool,

    // Cached format list — dirty flag avoids recomputing every frame
    pub cached_formats: Vec<OutputFormat>,
    pub formats_dirty: bool,
}

impl MediaForgeApp {
    pub fn new(
        _cc: &eframe::CreationContext<'_>,
        initial_files: Vec<String>,
        ipc_receiver: Option<Receiver<Vec<String>>>,
    ) -> Self {
        let config = MediaForgeConfig::load();
        let (msg_sender, msg_receiver) = crossbeam_channel::unbounded();
        let (import_sender, import_receiver) = crossbeam_channel::unbounded();
        let ffmpeg_version: Arc<Mutex<Option<String>>> = Arc::new(Mutex::new(None));

        let mut app = Self {
            files: Vec::new(),
            drop_hover: false,
            selected_format: None,
            custom_output_dir: None,
            progress: ConversionProgress::default(),
            cancel_flag: Arc::new(Mutex::new(false)),
            msg_sender,
            msg_receiver,
            show_settings: false,
            status_message: "Ready".to_string(),
            ffmpeg_version,
            config,
            ipc_receiver,
            import_sender,
            import_receiver,
            fonts_configured: false,
            ffmpeg_check_requested: false,
            is_importing: false,
            cached_formats: Vec::new(),
            formats_dirty: true,
        };

        for path_str in initial_files {
            app.add_path(&PathBuf::from(path_str));
        }

        app
    }

    pub fn start_conversion(&mut self) {
        let Some(format) = self.selected_format.clone() else {
            return;
        };

        *self.cancel_flag.lock() = false;
        self.progress = ConversionProgress {
            is_running: true,
            total_files: self.files.iter().filter(|f| f.selected).count(),
            ..Default::default()
        };

        for file in &mut self.files {
            if file.selected {
                file.status = FileStatus::Pending;
            }
        }

        self.status_message = "Converting...".to_string();

        let tasks: Vec<job::JobTask> = self.files.iter().enumerate().filter_map(|(idx, f)| {
            if f.selected {
                Some(job::JobTask {
                    index: idx,
                    path: f.path.clone(),
                    filename: f.cached_filename.clone(),
                    metadata: f.metadata.clone(),
                })
            } else {
                None
            }
        }).collect();

        let config = self.config.clone();
        let output_dir = self.custom_output_dir.clone();
        let sender = self.msg_sender.clone();
        let cancel_flag = self.cancel_flag.clone();

        job::start_conversion(tasks, format, config, output_dir, sender, cancel_flag);
    }

    pub fn cancel_conversion(&mut self) {
        *self.cancel_flag.lock() = true;
        self.status_message = "Cancelling...".to_string();
    }

    fn process_messages(&mut self, ctx: &egui::Context) {
        let mut received = false;
        while let Ok(msg) = self.msg_receiver.try_recv() {
            received = true;
            match msg {
                ConversionMessage::Started { total_files } => {
                    self.progress.total_files = total_files;
                    self.progress.is_running = true;
                    self.progress.is_complete = false;
                }
                ConversionMessage::FileStarted { index, name } => {
                    self.progress.current_file_index = index;
                    self.progress.current_file_name = name;
                    self.progress.current_file_pct = 0.0;

                    if let Some(file) = self.files.get_mut(index) {
                        file.status = FileStatus::Converting;
                    }
                }
                ConversionMessage::FileProgress { index, pct, speed, eta } => {
                    self.progress.current_file_pct = pct;
                    self.progress.speed_str = speed;
                    self.progress.eta_secs = eta;

                    let total = self.progress.total_files as f64;
                    if total > 0.0 {
                        self.progress.overall_pct =
                            (index as f64 + pct / 100.0) / total * 100.0;
                    }
                }
                ConversionMessage::FileDone { index, success, error } => {
                    if let Some(file) = self.files.get_mut(index) {
                        file.status = if success {
                            FileStatus::Done
                        } else {
                            FileStatus::Failed(
                                error.unwrap_or_else(|| "Unknown error".to_string()),
                            )
                        };
                    }

                    if success {
                        self.progress.succeeded += 1;
                    } else {
                        self.progress.failed += 1;
                    }

                    self.progress.current_file_pct = 100.0;
                }
                ConversionMessage::AllDone { succeeded, failed } => {
                    self.progress.is_running = false;
                    self.progress.is_complete = true;
                    self.progress.overall_pct = 100.0;
                    self.progress.current_file_pct = 100.0;
                    self.progress.succeeded = succeeded;
                    self.progress.failed = failed;

                    self.status_message = format!("Done — {succeeded} ok, {failed} failed");

                    if self.config.show_notification {
                        let mut notification = notify_rust::Notification::new();
                        notification
                            .appname("MediaForge")
                            .summary("Conversion Complete")
                            .body(&format!("{succeeded} succeeded, {failed} failed."));
                        if self.config.play_sound_on_complete {
                            #[cfg(target_os = "windows")]
                            {
                                notification.sound_name("Mail");
                            }
                        }
                        let _ = notification.show();
                    }

                    self.config.save();
                }
                ConversionMessage::Log(line) => {
                    self.progress.log_lines.push_back(line);
                    if self.progress.log_lines.len() > 500 {
                        self.progress.log_lines.pop_front();
                    }
                }
            }
        }

        if received {
            ctx.request_repaint();
        }
    }

    fn process_imports(&mut self, ctx: &egui::Context) {
        let mut received = false;
        while let Ok(msg) = self.import_receiver.try_recv() {
            received = true;
            match msg {
                ImportMessage::FileDiscovered(file) => {
                    if !self.files.iter().any(|existing| existing.path == file.path) {
                        self.files.push(file);
                        self.formats_dirty = true;
                    }
                    self.status_message =
                        format!("Importing... {} file(s) added", self.files.len());
                }
                ImportMessage::BatchDone { added } => {
                    self.is_importing = false;
                    self.status_message = if added == 0 {
                        "No supported media found".to_string()
                    } else {
                        format!("Ready \u{2022} imported {added} file(s)")
                    };
                }
            }
        }

        if received {
            ctx.request_repaint();
        }
    }

    fn process_dropped_files(&mut self, ctx: &egui::Context) {
        let dropped_files: Vec<PathBuf> = ctx.input(|i| {
            i.raw
                .dropped_files
                .iter()
                .filter_map(|f| f.path.clone())
                .collect()
        });

        self.drop_hover = ctx.input(|i| !i.raw.hovered_files.is_empty());

        if !dropped_files.is_empty() {
            self.formats_dirty = true;
        }
        for path in dropped_files {
            self.add_path(&path);
        }
    }

    pub fn add_path(&mut self, path: &PathBuf) {
        if path.is_dir() {
            self.start_async_import(path.clone());
        } else if path.is_file() {
            let mt = detect::detect_media_type(path);
            if mt != MediaType::Unknown {
                if self.contains_path(path) {
                    return;
                }
                let mut f = InputFile::new(path.clone(), mt);
                if mt == MediaType::Image {
                    f.metadata = metadata::probe_image(path);
                } else {
                    f.metadata = metadata::probe_media(path, &self.config.ffprobe_path());
                }
                self.files.push(f);
                self.formats_dirty = true;
                self.status_message = "Ready".to_string();
            }
        }
    }

    fn start_async_import(&mut self, path: PathBuf) {
        self.is_importing = true;
        self.status_message = "Scanning folder...".to_string();

        let sender = self.import_sender.clone();
        let ffprobe_path = self.config.ffprobe_path();
        let max_depth = self.config.max_folder_scan_depth;

        std::thread::spawn(move || {
            let found = detect::scan_directory(&path, max_depth);
            let mut added = 0usize;

            for fp in found {
                let mt = detect::detect_media_type(&fp);
                if mt == MediaType::Unknown {
                    continue;
                }

                let mut file = InputFile::new(fp.clone(), mt);
                if mt == MediaType::Image {
                    file.metadata = metadata::probe_image(&fp);
                } else {
                    file.metadata = metadata::probe_media(&fp, &ffprobe_path);
                }

                added += 1;
                let _ = sender.send(ImportMessage::FileDiscovered(file));
            }

            let _ = sender.send(ImportMessage::BatchDone { added });
        });
    }

    fn process_ipc(&mut self) {
        let mut paths_to_add = Vec::new();
        if let Some(ref receiver) = self.ipc_receiver {
            while let Ok(files) = receiver.try_recv() {
                for path_str in files {
                    paths_to_add.push(PathBuf::from(&path_str));
                }
            }
        }
        for path in &paths_to_add {
            self.add_path(path);
        }
    }

    fn request_ffmpeg_version_check(&mut self, ctx: &egui::Context) {
        if self.ffmpeg_check_requested {
            return;
        }
        self.ffmpeg_check_requested = true;
        let ver = self.ffmpeg_version.clone();
        let ctx = ctx.clone();
        let path = self.config.ffmpeg_path();
        std::thread::spawn(move || {
            let result = ffmpeg::get_ffmpeg_version(&path);
            *ver.lock() = result;
            ctx.request_repaint();
        });
    }

    fn contains_path(&self, path: &PathBuf) -> bool {
        self.files.iter().any(|f| f.path == *path)
    }
}

impl eframe::App for MediaForgeApp {
    fn update(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        // One-time setup
        if !self.fonts_configured {
            match self.config.theme {
                Theme::Dark => ctx.set_visuals(theme::dark_theme()),
                Theme::Light => ctx.set_visuals(theme::light_theme()),
            }
            theme::configure_fonts(ctx);
            self.fonts_configured = true;
        }

        if self.show_settings {
            self.request_ffmpeg_version_check(ctx);
        }

        // Process background events
        self.process_messages(ctx);
        self.process_imports(ctx);
        self.process_dropped_files(ctx);
        self.process_ipc();

        // Continuous repaint during conversion
        if self.progress.is_running || self.is_importing {
            ctx.request_repaint_after(std::time::Duration::from_millis(50));
        }

        // Settings window
        if self.show_settings {
            settings::show(self, ctx);
        }

        // Main layout — tight margins to eliminate edge gaps
        let panel_fill = ctx.style().visuals.panel_fill;
        egui::CentralPanel::default()
            .frame(egui::Frame::default()
                .fill(panel_fill)
                .inner_margin(egui::Margin::same(4)))
            .show(ctx, |ui| {
                main_view::show(self, ui);
            });
    }
}
