/// Parse FFmpeg `-progress pipe:1` output to extract progress information
#[derive(Debug, Clone, Default)]
pub struct FfmpegProgress {
    pub frame: u64,
    pub fps: f64,
    pub total_size: u64,
    pub out_time_us: u64,
    pub speed: f64,
    pub progress_state: ProgressState,
}

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub enum ProgressState {
    #[default]
    Continue,
    End,
}

impl FfmpegProgress {
    /// Calculate progress percentage given total duration in microseconds
    pub fn percentage(&self, total_duration_us: u64) -> f64 {
        if total_duration_us == 0 {
            return 0.0;
        }
        let pct = (self.out_time_us as f64 / total_duration_us as f64) * 100.0;
        pct.clamp(0.0, 100.0)
    }
}

/// Parse a chunk of FFmpeg progress output, returning the latest parsed state
pub fn parse_progress(data: &str) -> FfmpegProgress {
    let mut progress = FfmpegProgress::default();

    for line in data.lines() {
        let line = line.trim();
        if let Some((key, value)) = line.split_once('=') {
            let key = key.trim();
            let value = value.trim();
            match key {
                "frame" => {
                    progress.frame = value.parse().unwrap_or(0);
                }
                "fps" => {
                    progress.fps = value.parse().unwrap_or(0.0);
                }
                "total_size" => {
                    progress.total_size = value.parse().unwrap_or(0);
                }
                "out_time_us" => {
                    progress.out_time_us = value.parse().unwrap_or(0);
                }
                "out_time_ms" => {
                    // FFmpeg's -progress output uses `out_time_ms` as a legacy key name,
                    // but the value is in microseconds (same unit as out_time_us).
                    if let Ok(ms) = value.parse::<u64>() {
                        progress.out_time_us = ms;
                    }
                }
                "speed" => {
                    // Format: "2.3x" or "N/A"
                    // Parse slices directly instead of allocating
                    let speed_val = value.trim_end_matches('x').trim();
                    progress.speed = speed_val.parse().unwrap_or(0.0);
                }
                "progress" => {
                    progress.progress_state = if value == "end" {
                        ProgressState::End
                    } else {
                        ProgressState::Continue
                    };
                }
                _ => {}
            }
        }
    }

    progress
}

/// Calculate ETA in seconds
pub fn calculate_eta(elapsed_secs: f64, progress_pct: f64) -> Option<f64> {
    if progress_pct <= 0.0 || progress_pct >= 100.0 {
        return None;
    }
    let total_estimated = elapsed_secs / (progress_pct / 100.0);
    let remaining = total_estimated - elapsed_secs;
    Some(remaining.max(0.0))
}

/// Format ETA as human-readable string
pub fn format_eta(eta_secs: f64) -> String {
    let total = eta_secs as u64;
    let hours = total / 3600;
    let mins = (total % 3600) / 60;
    let secs = total % 60;

    if hours > 0 {
        format!("{hours}h {mins}m")
    } else if mins > 0 {
        format!("{mins}m {secs}s")
    } else {
        format!("{secs}s")
    }
}
