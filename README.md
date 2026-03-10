# MediaForge

**All-in-One Portable Media Converter** — A standalone media converter for Windows, macOS, and Linux that handles video, audio, and image conversions.

![Rust](https://img.shields.io/badge/Rust-2021-orange?logo=rust)
![Platform](https://img.shields.io/badge/Platform-Win%20%7C%20Mac%20%7C%20Linux-blue?logo=windows)
![License](https://img.shields.io/badge/License-MIT-green)

---

## Features

- **Cross-platform** — works on Windows, macOS, and Linux
- **Single portable `.exe` (Windows)** — copy anywhere and run, zero installation
- **Bundled FFmpeg (Windows)** — no external downloads needed on Windows
- **Video conversion** — MP4, MKV, AVI, MOV, WebM, WMV, FLV, MPEG, 3GP, TS, GIF, OGV
- **Audio conversion** — MP3, WAV, FLAC, AAC, OGG, OPUS, WMA, AIFF, AC3, M4A
- **Image conversion** — PNG, JPG, WebP, BMP, TIFF, GIF, ICO, AVIF
- **Drag & drop** — drop files or folders directly onto the window
- **Batch conversion** — convert hundreds of files at once with real-time progress
- **Hardware acceleration** — NVIDIA NVENC, Intel QSV, AMD AMF auto-detection
- **Quality controls** — CRF, preset, resolution, bitrate, sample rate, channels
- **Windows context menu** — right-click → "Convert with MediaForge" on any media file
- **Single instance** — launching from context menu sends files to the running window
- **Desktop notifications** — get notified when batch conversions complete
- **Dark & Light themes** — toggle in settings

## Screenshot

> *Launch the app to see the UI — it features a modern card-based layout with drag-and-drop, format picker tabs, and a smooth progress bar.*

## Getting Started

### Windows Portable

Grab the latest `mediaforge.exe` from the **Releases** page and run it. That's it. FFmpeg is embedded inside the executable.

### macOS & Linux Requirements

On macOS and Linux, MediaForge requires `ffmpeg` and `ffprobe` to be installed on your system and available in your `PATH`.
- **macOS:** `brew install ffmpeg`
- **Ubuntu/Debian:** `sudo apt install ffmpeg`
- **Arch:** `sudo pacman -S ffmpeg`

### Build from Source

**Prerequisites:**
- [Rust toolchain](https://rustup.rs/) (stable)
- **Windows only:** FFmpeg binaries placed in `bin/` (`ffmpeg.exe`, `ffprobe.exe`) prior to building.

```bash
# Clone the repo
git clone https://github.com/Hamza-op/MediaForge.git
cd MediaForge

# Build release (optimized, stripped)
cargo build --release
```

The release profile is configured for maximum optimization:
```toml
[profile.release]
opt-level = "z"      # Optimize for size
lto = true           # Link-time optimization
codegen-units = 1    # Max optimization
panic = "abort"      # Remove unwinding overhead
strip = true         # Strip debug symbols
```

## Usage

### GUI

Just run `mediaforge.exe`. Drag files onto the window or click **+ Files** / **+ Folder**.

### Command Line

```bash
# Open with specific files
mediaforge.exe --files "video.mp4" "photo.png"

# Open with a folder (recursively scans for media)
mediaforge.exe --folder "C:\Users\Photos"

# Unregister context menu entries
mediaforge.exe --unregister
```

### Context Menu Integration

Open **Settings → Context Menu → Register** to add "Convert with MediaForge" to the Windows right-click menu for all supported file types and folders.

## Project Structure

```
src/
├── main.rs                 # Entry point, CLI parsing, single-instance check
├── app.rs                  # App state, message processing, eframe integration
├── config.rs               # Settings, serialization, enums
├── converter/
│   ├── ffmpeg.rs           # Output format definitions, FFmpeg arg builders
│   ├── job.rs              # Conversion pipeline, progress messages
│   ├── image_conv.rs       # Native image conversion (no FFmpeg)
│   ├── progress.rs         # FFmpeg progress output parser
│   └── embed.rs            # Embedded FFmpeg binary extraction
├── media/
│   ├── detect.rs           # Media type detection (extension + magic bytes)
│   └── metadata.rs         # FFprobe / image metadata extraction
├── platform/
│   ├── single_instance.rs  # Named mutex + named pipe IPC
│   └── context_menu.rs     # Windows registry shell extension
└── ui/
    ├── main_view.rs        # Main layout, file list, format picker
    ├── settings.rs         # Settings window
    ├── theme.rs            # Dark/light themes, color palette
    └── widgets.rs          # Custom widgets (progress bar, drop zone, buttons)
```

## Configuration

Settings are stored in `mediaforge.toml` next to the exe (portable) or in `%APPDATA%\MediaForge\` as a fallback.

| Setting | Default | Description |
|---|---|---|
| `theme` | `Dark` | Dark or Light |
| `add_suffix` | `true` | Append "(converted)" to output names |
| `overwrite_existing` | `false` | Overwrite files with same name |
| `max_concurrent_conversions` | CPU count | Parallel conversion limit |
| `hw_accel` | `Auto` | GPU acceleration preference |
| `video_crf` | `23` | Video quality (0 = lossless, 51 = worst) |
| `video_preset` | `Medium` | Encoding speed vs quality tradeoff |
| `image_quality` | `85` | JPEG/WebP quality (1–100) |
| `audio_bitrate` | `192` | Audio bitrate in kbps |
| `show_notification` | `true` | Desktop notification on completion |

## Tech Stack

| Component | Crate |
|---|---|
| GUI | `egui` + `eframe` |
| File dialogs | `rfd` |
| Media processing | Bundled FFmpeg + `image` crate |
| Config | `serde` + `toml` |
| CLI | `clap` |
| File type detection | `infer` |
| Windows APIs | `windows-sys` + `winreg` |
| Concurrency | `crossbeam-channel` + `parking_lot` |
| Logging | `tracing` |
| Notifications | `notify-rust` |

## License

MIT

---

**Made with Rust by [Hamza-op](https://github.com/Hamza-op)**
