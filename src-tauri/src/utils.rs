use crate::managers::audio::AudioRecordingManager;
use crate::managers::transcription::TranscriptionManager;
use crate::shortcut;
use crate::TranscriptionCoordinator;
use log::info;
use std::process::Command;
use std::sync::Arc;
use tauri::{AppHandle, Manager};

// Re-export all utility modules for easy access
// pub use crate::audio_feedback::*;
pub use crate::clipboard::*;
pub use crate::overlay::*;
pub use crate::tray::*;

/// Centralized cancellation function that can be called from anywhere in the app.
/// Handles cancelling both recording and transcription operations and updates UI state.
pub fn cancel_current_operation(app: &AppHandle) {
    info!("Initiating operation cancellation...");

    // Unregister the cancel shortcut asynchronously
    shortcut::unregister_cancel_shortcut(app);

    // Cancel any ongoing recording
    let audio_manager = app.state::<Arc<AudioRecordingManager>>();
    let recording_was_active = audio_manager.is_recording();
    audio_manager.cancel_recording();

    // Update tray icon and hide overlay
    change_tray_icon(app, crate::tray::TrayIconState::Idle);
    hide_recording_overlay(app);

    // Unload model if immediate unload is enabled
    let tm = app.state::<Arc<TranscriptionManager>>();
    tm.maybe_unload_immediately("cancellation");

    // Notify coordinator so it can keep lifecycle state coherent.
    if let Some(coordinator) = app.try_state::<TranscriptionCoordinator>() {
        coordinator.notify_cancel(recording_was_active);
    }

    info!("Operation cancellation completed - returned to idle state");
}

/// Check if using the Wayland display server protocol
#[cfg(target_os = "linux")]
pub fn is_wayland() -> bool {
    std::env::var("WAYLAND_DISPLAY").is_ok()
        || std::env::var("XDG_SESSION_TYPE")
            .map(|v| v.to_lowercase() == "wayland")
            .unwrap_or(false)
}

/// Check if running on KDE Plasma desktop environment
#[cfg(target_os = "linux")]
pub fn is_kde_plasma() -> bool {
    std::env::var("XDG_CURRENT_DESKTOP")
        .map(|v| v.to_uppercase().contains("KDE"))
        .unwrap_or(false)
        || std::env::var("KDE_SESSION_VERSION").is_ok()
}

/// Check if running on KDE Plasma with Wayland
#[cfg(target_os = "linux")]
pub fn is_kde_wayland() -> bool {
    is_wayland() && is_kde_plasma()
}

/// Check if running on GNOME desktop environment
#[cfg(target_os = "linux")]
pub fn is_gnome() -> bool {
    std::env::var("XDG_CURRENT_DESKTOP")
        .map(|v| v.to_uppercase().contains("GNOME"))
        .unwrap_or(false)
}

/// XKB `ctrl:` swap options can remap which physical key produces the Control
/// modifier (e.g. `ctrl:swap_lalt_lctl_lwin`). Since ydotool sends raw evdev
/// keycodes that pass through XKB remapping, we must send the keycode that
/// *resolves to* Control, not assume KEY_LEFTCTRL (29).
///
/// Reference: `/usr/include/linux/input-event-codes.h`
///   KEY_LEFTCTRL  = 29    KEY_LEFTALT  = 56
///   KEY_LEFTMETA  = 125   KEY_LEFTSHIFT = 42
static CTRL_KEYCODE: std::sync::atomic::AtomicU32 = std::sync::atomic::AtomicU32::new(29);

#[cfg(target_os = "linux")]
pub fn detect_and_cache_ctrl_keycode() {
    let code = detect_ctrl_keycode_from_xkb();
    CTRL_KEYCODE.store(code, std::sync::atomic::Ordering::Relaxed);
    info!("Detected Ctrl keycode: {code} (default=29, swap_lalt=56, swap_lwin=125)");
}

#[cfg(target_os = "linux")]
fn detect_ctrl_keycode_from_xkb() -> u32 {
    // Try gsettings first (GNOME)
    if let Ok(output) = Command::new("gsettings")
        .arg("get")
        .arg("org.gnome.desktop.input-sources")
        .arg("xkb-options")
        .output()
    {
        let s = String::from_utf8_lossy(&output.stdout);
        // ctrl:swap_lalt_lctl_lwin — Left Alt position produces Control
        if s.contains("swap_lalt_lctl_lwin") || s.contains("swap_ralt_rctl_rwin") {
            return 56; // KEY_LEFTALT / KEY_RIGHTALT
        }
        // ctrl:swap_lwin_lctl — Left Win position produces Control
        if s.contains("swap_lwin_lctl") || s.contains("swap_rwin_rctl") {
            return 125; // KEY_LEFTMETA / KEY_RIGHTMETA
        }
    }

    // Fallback: try setxkbmap -query
    if let Ok(output) = Command::new("setxkbmap").arg("-query").output() {
        let s = String::from_utf8_lossy(&output.stdout);
        if s.contains("swap_lalt_lctl_lwin") || s.contains("swap_ralt_rctl_rwin") {
            return 56;
        }
        if s.contains("swap_lwin_lctl") || s.contains("swap_rwin_rctl") {
            return 125;
        }
    }

    29 // KEY_LEFTCTRL — default
}

/// Retrieve the cached Ctrl keycode (set by detect_and_cache_ctrl_keycode).
#[cfg(target_os = "linux")]
pub fn get_cached_ctrl_keycode() -> u32 {
    CTRL_KEYCODE.load(std::sync::atomic::Ordering::Relaxed)
}
