#![windows_subsystem = "windows"]

mod app;
mod config;
mod converter;
mod media;
mod platform;
mod ui;

use clap::Parser;

#[derive(Parser, Debug)]
#[command(name = "MediaForge", about = "All-in-One Media Converter")]
struct Cli {
    /// Files to convert
    #[arg(long, num_args = 0..)]
    files: Vec<String>,

    /// Folder to scan for media files
    #[arg(long)]
    folder: Option<String>,

    /// Unregister context menu entries
    #[arg(long)]
    unregister: bool,
}

fn main() -> eframe::Result<()> {
    // Initialize logging
    tracing_subscriber::fmt()
        .with_env_filter(
            tracing_subscriber::EnvFilter::from_default_env()
                .add_directive(tracing::Level::WARN.into()),
        )
        .init();

    // Set custom panic hook to log panics
    std::panic::set_hook(Box::new(|panic_info| {
        tracing::error!("Application panicked: {}", panic_info);
    }));

    let cli = Cli::parse();

    // Handle --unregister
    if cli.unregister {
        if let Err(e) = platform::context_menu::unregister_context_menu() {
            tracing::error!("Failed to unregister context menu: {}", e);
            std::process::exit(1);
        }
        tracing::info!("Successfully unregistered context menu.");
        std::process::exit(0);
    }

    // Collect initial files from CLI
    let mut initial_files: Vec<String> = cli.files;
    if let Some(folder) = cli.folder {
        initial_files.push(folder);
    }

    // Single instance check
    let ipc_receiver = match platform::single_instance::try_acquire(&initial_files) {
        Ok(Some(_guard)) => {
            let (sender, receiver) = crossbeam_channel::unbounded();
            platform::single_instance::start_pipe_listener(sender);
            Some(receiver)
        }
        Ok(None) => {
            tracing::info!("Another instance is running. Forwarded argument(s). Exiting.");
            std::process::exit(0);
        }
        Err(e) => {
            tracing::error!("Single instance check failed: {}. Proceeding without single-instance guarantee.", e);
            None
        }
    };

    // Launch GUI
    let options = eframe::NativeOptions {
        viewport: egui::ViewportBuilder::default()
            .with_inner_size([1120.0, 700.0])
            .with_min_inner_size([820.0, 580.0])
            .with_title("MediaForge")
            .with_drag_and_drop(true),
        ..Default::default()
    };

    eframe::run_native(
        "MediaForge",
        options,
        Box::new(move |cc| {
            Ok(Box::new(app::MediaForgeApp::new(cc, initial_files, ipc_receiver)))
        }),
    )
}
