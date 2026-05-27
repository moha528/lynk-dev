//! Windows-only native title bar styling.
//!
//! Two passes:
//! 1. Try Mica via [`window_vibrancy::apply_mica`] — requires Windows 11 21H2+.
//!    On success the OS draws a translucent backdrop behind the title bar
//!    (and any transparent webview region), matching modern File Explorer.
//! 2. If Mica is unavailable (Windows 10), fall back to painting the title
//!    bar manually via `DwmSetWindowAttribute(DWMWA_CAPTION_COLOR ...)` so it
//!    stops looking like a stock light-themed strip on top of a dark app.
//!
//! Both attempts are best-effort: any error is logged and the window falls
//! through to the OS default.

use tauri::WebviewWindow;
use tracing::{debug, warn};
use window_vibrancy::apply_mica;
use windows_sys::Win32::Graphics::Dwm::{
    DwmSetWindowAttribute, DWMWA_BORDER_COLOR, DWMWA_CAPTION_COLOR, DWMWA_TEXT_COLOR,
};

/// Try Mica first; fall back to caption painting.
pub fn style_titlebar(window: &WebviewWindow) {
    match apply_mica(window, Some(true)) {
        Ok(()) => debug!("Mica applied to main window"),
        Err(e) => {
            warn!("Mica unavailable ({e}); falling back to caption color");
            apply_caption_color(window);
        }
    }
}

/// Paint title bar / window border / title text via DWM attributes.
///
/// Colors are encoded as `0x00BBGGRR` (the COLORREF layout) and tuned to match
/// the in-app palette (`--color-panel`, `--color-border-strong`, `--color-text`).
fn apply_caption_color(window: &WebviewWindow) {
    let Ok(hwnd) = window.hwnd() else {
        warn!("no HWND for main window — skipping caption colors");
        return;
    };

    // COLORREF layout: 0x00BBGGRR. Tuned to match the in-app palette
    // (--color-panel, --color-border-strong, --color-text).
    let caption: u32 = 0x002E2E2E;
    let border: u32 = 0x003B3B3B;
    let text: u32 = 0x00E6E6E6;

    let raw = hwnd.0 as windows_sys::Win32::Foundation::HWND;
    let size = std::mem::size_of::<u32>() as u32;

    // SAFETY: `raw` is a valid live HWND; `DwmSetWindowAttribute` reads
    // exactly `size` bytes from `pvAttribute`, which we point at u32 values.
    unsafe {
        let _ = DwmSetWindowAttribute(
            raw,
            DWMWA_CAPTION_COLOR as u32,
            std::ptr::from_ref(&caption).cast(),
            size,
        );
        let _ = DwmSetWindowAttribute(
            raw,
            DWMWA_BORDER_COLOR as u32,
            std::ptr::from_ref(&border).cast(),
            size,
        );
        let _ = DwmSetWindowAttribute(
            raw,
            DWMWA_TEXT_COLOR as u32,
            std::ptr::from_ref(&text).cast(),
            size,
        );
    }
}
