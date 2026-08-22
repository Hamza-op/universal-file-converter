use std::process::{Command, Stdio};

use crate::platform::CommandExt;

pub fn show_completion(succeeded: usize, failed: usize, cancelled: usize, play_sound: bool) {
    let body = format!("{succeeded} succeeded, {failed} failed, {cancelled} cancelled.");

    #[cfg(target_os = "windows")]
    show_windows(&body, play_sound);

    #[cfg(target_os = "macos")]
    show_macos(&body, play_sound);

    #[cfg(all(unix, not(target_os = "macos")))]
    show_linux(&body, play_sound);
}

#[cfg(target_os = "windows")]
fn show_windows(body: &str, play_sound: bool) {
    const SCRIPT: &str = r#"
param([string]$Body, [string]$PlaySound)
[Windows.UI.Notifications.ToastNotificationManager, Windows.UI.Notifications, ContentType = WindowsRuntime] > $null
[Windows.UI.Notifications.ToastNotification, Windows.UI.Notifications, ContentType = WindowsRuntime] > $null
[Windows.Data.Xml.Dom.XmlDocument, Windows.Data.Xml.Dom.XmlDocument, ContentType = WindowsRuntime] > $null
$xml = New-Object Windows.Data.Xml.Dom.XmlDocument
$escaped = [System.Security.SecurityElement]::Escape($Body)
$xml.LoadXml("<toast><visual><binding template='ToastGeneric'><text>MediaForge</text><text>$escaped</text></binding></visual></toast>")
$toast = New-Object Windows.UI.Notifications.ToastNotification $xml
[Windows.UI.Notifications.ToastNotificationManager]::CreateToastNotifier('MediaForge').Show($toast)
if ($PlaySound -eq 'true') { [System.Media.SystemSounds]::Asterisk.Play() }
"#;

    let _ = Command::new("powershell.exe")
        .args([
            "-NoLogo",
            "-NoProfile",
            "-NonInteractive",
            "-WindowStyle",
            "Hidden",
            "-Command",
            SCRIPT,
            body,
            if play_sound { "true" } else { "false" },
        ])
        .stdin(Stdio::null())
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .creation_flags(0x08000000)
        .spawn();
}

#[cfg(target_os = "macos")]
fn show_macos(body: &str, play_sound: bool) {
    let script = if play_sound {
        format!(
            "display notification {:?} with title \"MediaForge\" sound name \"Glass\"",
            body
        )
    } else {
        format!("display notification {:?} with title \"MediaForge\"", body)
    };
    let _ = Command::new("osascript")
        .args(["-e", &script])
        .stdin(Stdio::null())
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .spawn();
}

#[cfg(all(unix, not(target_os = "macos")))]
fn show_linux(body: &str, _play_sound: bool) {
    let _ = Command::new("notify-send")
        .args(["MediaForge", body])
        .stdin(Stdio::null())
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .spawn();
}
