//! Detached "FloatBar" window: a small always-on-top transparent strip
//! that shows remaining capacity per provider. Runs as an auxiliary
//! Tauri window labeled `floatbar`, independent of the main surface
//! state machine.

use std::sync::atomic::{AtomicBool, Ordering};
use std::time::Duration;

use tauri::{LogicalPosition, LogicalSize, Manager, PhysicalPosition, WebviewUrl};

use crate::geometry_store;

pub const FLOATBAR_LABEL: &str = "floatbar";
pub const FLOAT_BAR_CONFIG_CHANGED_EVENT: &str = "float-bar-config-changed";
const FLOATBAR_DEFAULT_WIDTH_H: f64 = 360.0;
const FLOATBAR_DEFAULT_HEIGHT_H: f64 = 36.0;
const FLOATBAR_DEFAULT_WIDTH_V: f64 = 195.0;
const FLOATBAR_DEFAULT_HEIGHT_V: f64 = 420.0;
static ATTACH_LOOP_STARTED: AtomicBool = AtomicBool::new(false);

/// Initial dimensions (logical pixels) for the floating bar given an
/// orientation string. Unknown values fall back to horizontal so callers
/// don't have to pre-validate.
pub fn initial_size(orientation: &str) -> (f64, f64) {
    match orientation {
        "vertical" => (FLOATBAR_DEFAULT_WIDTH_V, FLOATBAR_DEFAULT_HEIGHT_V),
        _ => (FLOATBAR_DEFAULT_WIDTH_H, FLOATBAR_DEFAULT_HEIGHT_H),
    }
}

/// Convert a 0..=100 opacity value to a Win32 SetLayeredWindowAttributes
/// alpha byte (0..=255). Values below 30 are clamped so the bar is never
/// fully invisible — that would be a usability footgun.
#[cfg_attr(not(windows), allow(dead_code))]
pub fn opacity_to_alpha(opacity: u8) -> u8 {
    let clamped = opacity.clamp(30, 100);
    ((clamped as u32) * 255 / 100) as u8
}

/// Open the floating-bar window, or focus + reapply attributes if already
/// open. Position is restored from the geometry store keyed by
/// `floatbar`; on first launch the window is centered horizontally near
/// the top of the primary monitor.
pub fn show(
    app: &tauri::AppHandle,
    opacity: u8,
    orientation: &str,
    style: &str,
    click_through: bool,
) -> Result<(), String> {
    if let Some(window) = app.get_webview_window(FLOATBAR_LABEL) {
        apply_no_activate(&window);
        apply_opacity(&window, opacity);
        apply_click_through(&window, click_through);
        window.show().map_err(|e| e.to_string())?;
        start_codex_attach_loop(&window);
        return Ok(());
    }

    let (w, h) = initial_size(orientation);
    let url =
        WebviewUrl::App(format!("index.html?window=floatbar&orientation={orientation}").into());

    let builder = tauri::WebviewWindowBuilder::new(app, FLOATBAR_LABEL, url)
        .title("Codex 余量条")
        .inner_size(w, h)
        .decorations(false)
        .shadow(false)
        .resizable(false)
        .always_on_top(true)
        .skip_taskbar(true);

    // WebView2 only honors an alpha (transparent) background when the native
    // window is itself created transparent. Tauri cfg-gates this builder API
    // off on macOS unless `macos-private-api` is enabled, so keep the Windows
    // fix out of the macOS validation path.
    #[cfg(windows)]
    let builder = builder.transparent(true);

    let win = builder
        .background_color(tauri::utils::config::Color(0, 0, 0, 0))
        .visible(false)
        .build()
        .map_err(|e| e.to_string())?;

    // Restore prior geometry if we have one. Otherwise, taskbar style opens
    // near the bottom while the original floating style keeps its top-center
    // placement.
    if let Some(g) = geometry_store::load_entry(FLOATBAR_LABEL) {
        let _ = win.set_position(LogicalPosition::new(g.x as f64, g.y as f64));
        if let (Some(w), Some(h)) = (g.width, g.height) {
            let _ = win.set_size(LogicalSize::new(w as f64, h as f64));
        }
    } else if let Ok(Some(monitor)) = win.primary_monitor() {
        let scale = win.scale_factor().unwrap_or(1.0);
        let mon_x = monitor.position().x as f64 / scale;
        let mon_y = monitor.position().y as f64 / scale;
        let mon_w = monitor.size().width as f64 / scale;
        let mon_h = monitor.size().height as f64 / scale;
        let x = mon_x + (mon_w - w) / 2.0;
        let y = if style == "taskbar" {
            mon_y + mon_h - h - 8.0
        } else {
            mon_y + 8.0
        };
        let _ = win.set_position(LogicalPosition::new(x.max(mon_x), y.max(mon_y)));
    }

    apply_no_activate(&win);
    apply_opacity(&win, opacity);
    apply_click_through(&win, click_through);
    win.show().map_err(|e| e.to_string())?;
    start_codex_attach_loop(&win);
    Ok(())
}

/// Hide / destroy the floating bar.
pub fn hide(app: &tauri::AppHandle) -> Result<(), String> {
    if let Some(window) = app.get_webview_window(FLOATBAR_LABEL) {
        // Persist position before closing so it reopens in place.
        remember_geometry(&window);
        window.close().map_err(|e| e.to_string())?;
    }
    Ok(())
}

/// Capture current position into the geometry store under the floatbar key.
///
/// Accepts any Tauri window handle (`Window` from event callbacks or
/// `WebviewWindow` from `get_webview_window`), since `WindowEvent`
/// callbacks deliver a `&Window` while imperative call sites have a
/// `&WebviewWindow`.
pub fn remember_geometry<R: tauri::Runtime, M: WindowGeometry<R>>(window: &M) {
    let Ok(pos) = window.outer_position() else {
        return;
    };
    let Ok(size) = window.outer_size() else {
        return;
    };
    let scale = window.scale_factor().unwrap_or(1.0);
    geometry_store::save_entry(
        FLOATBAR_LABEL,
        geometry_store::StoredGeometry {
            x: (pos.x as f64 / scale).round() as i32,
            y: (pos.y as f64 / scale).round() as i32,
            width: Some((size.width as f64 / scale).round() as u32),
            height: Some((size.height as f64 / scale).round() as u32),
        },
    );
}

/// Subset of `tauri::WebviewWindow` / `tauri::Window` used by
/// [`remember_geometry`]. Both types implement the underlying methods, but
/// they don't share a public trait — this private trait bridges them so we
/// can be called from `WindowEvent` (which delivers `&Window`) and from
/// imperative paths (which hold `&WebviewWindow`).
pub trait WindowGeometry<R: tauri::Runtime> {
    fn outer_position(&self) -> tauri::Result<tauri::PhysicalPosition<i32>>;
    fn outer_size(&self) -> tauri::Result<tauri::PhysicalSize<u32>>;
    fn scale_factor(&self) -> tauri::Result<f64>;
}

impl<R: tauri::Runtime> WindowGeometry<R> for tauri::WebviewWindow<R> {
    fn outer_position(&self) -> tauri::Result<tauri::PhysicalPosition<i32>> {
        tauri::WebviewWindow::outer_position(self)
    }
    fn outer_size(&self) -> tauri::Result<tauri::PhysicalSize<u32>> {
        tauri::WebviewWindow::outer_size(self)
    }
    fn scale_factor(&self) -> tauri::Result<f64> {
        tauri::WebviewWindow::scale_factor(self)
    }
}

impl<R: tauri::Runtime> WindowGeometry<R> for tauri::Window<R> {
    fn outer_position(&self) -> tauri::Result<tauri::PhysicalPosition<i32>> {
        tauri::Window::outer_position(self)
    }
    fn outer_size(&self) -> tauri::Result<tauri::PhysicalSize<u32>> {
        tauri::Window::outer_size(self)
    }
    fn scale_factor(&self) -> tauri::Result<f64> {
        tauri::Window::scale_factor(self)
    }
}

/// Resize the floatbar to the given logical dimensions and re-assert the
/// native interaction invariants in the same step.
///
/// A resize goes through `SetWindowPos`/frame changes, which can drop the
/// extended window styles, so the no-activate and click-through flags must be
/// re-applied afterwards. Keeping both halves here gives callers (including the
/// webview) a single canonical "the bar changed size" entry point instead of
/// pairing a JS `setSize` with a separate native repair command.
pub fn resize(
    window: &tauri::WebviewWindow,
    width: f64,
    height: f64,
    click_through: bool,
) -> Result<(), String> {
    window
        .set_size(LogicalSize::new(width, height))
        .map_err(|e| e.to_string())?;
    apply_no_activate(window);
    apply_click_through(window, click_through);
    Ok(())
}

fn start_codex_attach_loop(window: &tauri::WebviewWindow) {
    if ATTACH_LOOP_STARTED.swap(true, Ordering::SeqCst) {
        return;
    }
    let window = window.clone();
    tauri::async_runtime::spawn(async move {
        loop {
            #[cfg(windows)]
            attach_to_foreground_codex(&window);
            #[cfg(not(windows))]
            {
                let _ = window.show();
            }
            tokio::time::sleep(Duration::from_millis(220)).await;
        }
    });
}

#[cfg(windows)]
fn attach_to_foreground_codex(window: &tauri::WebviewWindow) {
    let Some(rect) = active_codex_window_rect() else {
        let _ = window.hide();
        return;
    };

    let Ok(size) = window.outer_size() else {
        return;
    };
    let height = size.height as i32;
    let codex_width = rect.right - rect.left;
    let codex_height = rect.bottom - rect.top;
    if codex_width <= 0 || codex_height <= 0 {
        let _ = window.hide();
        return;
    }

    let gap = 10;
    let x = rect.right + gap;
    let y = rect.top + ((codex_height - height) / 2).max(12);
    let _ = window.set_position(PhysicalPosition::new(x, y));
    let _ = window.show();
    apply_no_activate(window);
}

#[cfg(windows)]
fn active_codex_window_rect() -> Option<Rect> {
    unsafe {
        let hwnd = GetForegroundWindow();
        if hwnd == 0 || IsWindowVisible(hwnd) == 0 || IsIconic(hwnd) != 0 {
            return None;
        }
        if !is_codex_window(hwnd) {
            return None;
        }
        let mut rect = Rect::default();
        if GetWindowRect(hwnd, &mut rect) == 0 {
            return None;
        }
        Some(rect)
    }
}

#[cfg(windows)]
unsafe fn is_codex_window(hwnd: isize) -> bool {
    let mut process_id = 0u32;
    unsafe {
        GetWindowThreadProcessId(hwnd, &mut process_id);
    }
    if process_id == 0 {
        return false;
    }
    if process_id == unsafe { GetCurrentProcessId() } {
        return false;
    }
    let exe_name = unsafe { process_exe_name(process_id) };
    let exe_lower = exe_name.to_ascii_lowercase();
    if exe_lower.contains("codexbar") || exe_lower.contains("codex-usage-bar") {
        return false;
    }
    if exe_name.eq_ignore_ascii_case("codex.exe")
        || exe_name.eq_ignore_ascii_case("codex")
        || exe_name.eq_ignore_ascii_case("codex desktop.exe")
    {
        return true;
    }
    let title = unsafe { window_title(hwnd) };
    is_probable_codex_desktop_title(&title)
}

#[cfg(windows)]
fn is_probable_codex_desktop_title(title: &str) -> bool {
    let normalized = title.to_ascii_lowercase();
    if !normalized.contains("codex") {
        return false;
    }
    let blocked_titles = [
        "codexbar",
        "codex bar",
        "codex usage bar",
        "codex quota",
        "codex 余量条",
    ];
    !blocked_titles
        .iter()
        .any(|blocked| normalized.contains(blocked))
}

#[cfg(windows)]
unsafe fn process_exe_name(process_id: u32) -> String {
    const PROCESS_QUERY_LIMITED_INFORMATION: u32 = 0x1000;
    let handle = unsafe { OpenProcess(PROCESS_QUERY_LIMITED_INFORMATION, 0, process_id) };
    if handle == 0 {
        return String::new();
    }
    let mut buffer = vec![0u16; 1024];
    let mut size = buffer.len() as u32;
    let ok = unsafe { QueryFullProcessImageNameW(handle, 0, buffer.as_mut_ptr(), &mut size) };
    unsafe {
        CloseHandle(handle);
    }
    if ok == 0 || size == 0 {
        return String::new();
    }
    let path = String::from_utf16_lossy(&buffer[..size as usize]);
    std::path::Path::new(&path)
        .file_name()
        .and_then(|name| name.to_str())
        .unwrap_or("")
        .to_string()
}

#[cfg(windows)]
unsafe fn window_title(hwnd: isize) -> String {
    let len = unsafe { GetWindowTextLengthW(hwnd) };
    if len <= 0 {
        return String::new();
    }
    let mut buffer = vec![0u16; len as usize + 1];
    let copied = unsafe { GetWindowTextW(hwnd, buffer.as_mut_ptr(), buffer.len() as i32) };
    if copied <= 0 {
        return String::new();
    }
    String::from_utf16_lossy(&buffer[..copied as usize])
}

/// Apply the current opacity setting to an existing floatbar window via
/// `SetLayeredWindowAttributes`. No-op on non-Windows platforms.
pub fn apply_opacity(window: &tauri::WebviewWindow, opacity: u8) {
    let _ = (window, opacity);
    #[cfg(windows)]
    {
        use raw_window_handle::HasWindowHandle;
        let alpha = opacity_to_alpha(opacity);
        let Ok(handle) = window.window_handle() else {
            return;
        };
        let raw_window_handle::RawWindowHandle::Win32(h) = handle.as_raw() else {
            return;
        };
        unsafe {
            // Ensure WS_EX_LAYERED is set so SetLayeredWindowAttributes works.
            const WS_EX_LAYERED: isize = 0x00080000;
            let ex = GetWindowLongPtrW(h.hwnd.get(), GWL_EXSTYLE);
            if ex & WS_EX_LAYERED == 0 {
                set_extended_style(h.hwnd.get(), ex | WS_EX_LAYERED);
            }
            const LWA_ALPHA: u32 = 0x00000002;
            SetLayeredWindowAttributes(h.hwnd.get(), 0, alpha, LWA_ALPHA);
        }
    }
}

/// Keep the floatbar from activating when it is shown or clicked. This makes
/// it behave like a desktop widget that visually sits above the taskbar without
/// stealing focus from the active app.
pub fn apply_no_activate(window: &tauri::WebviewWindow) {
    let _ = window;
    #[cfg(windows)]
    {
        use raw_window_handle::HasWindowHandle;
        let Ok(handle) = window.window_handle() else {
            return;
        };
        let raw_window_handle::RawWindowHandle::Win32(h) = handle.as_raw() else {
            return;
        };
        unsafe {
            const WS_EX_NOACTIVATE: isize = 0x08000000;
            let ex = GetWindowLongPtrW(h.hwnd.get(), GWL_EXSTYLE);
            if ex & WS_EX_NOACTIVATE == 0 {
                set_extended_style(h.hwnd.get(), ex | WS_EX_NOACTIVATE);
            }
        }
    }
}

/// Toggle click-through (`WS_EX_TRANSPARENT`). When enabled, mouse events
/// pass through to the window beneath — true overlay mode.
pub fn apply_click_through(window: &tauri::WebviewWindow, click_through: bool) {
    let _ = (window, click_through);
    #[cfg(windows)]
    {
        use raw_window_handle::HasWindowHandle;
        let Ok(handle) = window.window_handle() else {
            return;
        };
        let raw_window_handle::RawWindowHandle::Win32(h) = handle.as_raw() else {
            return;
        };
        unsafe {
            const WS_EX_LAYERED: isize = 0x00080000;
            const WS_EX_TRANSPARENT: isize = 0x00000020;
            let ex = GetWindowLongPtrW(h.hwnd.get(), GWL_EXSTYLE);
            let mut new_ex = ex | WS_EX_LAYERED;
            if click_through {
                new_ex |= WS_EX_TRANSPARENT;
            } else {
                new_ex &= !WS_EX_TRANSPARENT;
            }
            if new_ex != ex {
                set_extended_style(h.hwnd.get(), new_ex);
            }
        }
    }
}

#[cfg(windows)]
const GWL_EXSTYLE: i32 = -20;

#[cfg(windows)]
#[derive(Clone, Copy, Default)]
#[repr(C)]
struct Rect {
    left: i32,
    top: i32,
    right: i32,
    bottom: i32,
}

#[cfg(windows)]
unsafe fn set_extended_style(hwnd: isize, ex_style: isize) {
    unsafe {
        SetWindowLongPtrW(hwnd, GWL_EXSTYLE, ex_style);
        const SWP_NOSIZE: u32 = 0x0001;
        const SWP_NOMOVE: u32 = 0x0002;
        const SWP_NOZORDER: u32 = 0x0004;
        const SWP_NOACTIVATE: u32 = 0x0010;
        const SWP_FRAMECHANGED: u32 = 0x0020;
        let flags = SWP_NOSIZE | SWP_NOMOVE | SWP_NOZORDER | SWP_NOACTIVATE | SWP_FRAMECHANGED;
        SetWindowPos(hwnd, 0, 0, 0, 0, 0, flags);
    }
}

#[cfg(windows)]
#[link(name = "user32")]
unsafe extern "system" {
    fn GetForegroundWindow() -> isize;
    fn GetWindowLongPtrW(hwnd: isize, index: i32) -> isize;
    fn GetWindowRect(hwnd: isize, rect: *mut Rect) -> i32;
    fn GetWindowTextLengthW(hwnd: isize) -> i32;
    fn GetWindowTextW(hwnd: isize, text: *mut u16, max_count: i32) -> i32;
    fn GetWindowThreadProcessId(hwnd: isize, process_id: *mut u32) -> u32;
    fn IsIconic(hwnd: isize) -> i32;
    fn IsWindowVisible(hwnd: isize) -> i32;
    fn SetWindowLongPtrW(hwnd: isize, index: i32, new: isize) -> isize;
    fn SetLayeredWindowAttributes(hwnd: isize, color_key: u32, alpha: u8, flags: u32) -> i32;
    fn SetWindowPos(
        hwnd: isize,
        hwnd_insert_after: isize,
        x: i32,
        y: i32,
        cx: i32,
        cy: i32,
        flags: u32,
    ) -> i32;
}

#[cfg(windows)]
#[link(name = "kernel32")]
unsafe extern "system" {
    fn CloseHandle(handle: isize) -> i32;
    fn GetCurrentProcessId() -> u32;
    fn OpenProcess(desired_access: u32, inherit_handle: i32, process_id: u32) -> isize;
    fn QueryFullProcessImageNameW(
        process: isize,
        flags: u32,
        exe_name: *mut u16,
        size: *mut u32,
    ) -> i32;
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn opacity_to_alpha_clamps_low_values() {
        assert_eq!(opacity_to_alpha(0), opacity_to_alpha(30));
        assert_eq!(opacity_to_alpha(10), opacity_to_alpha(30));
    }

    #[test]
    fn opacity_to_alpha_full_is_255() {
        assert_eq!(opacity_to_alpha(100), 255);
    }

    #[test]
    fn opacity_to_alpha_is_monotonic() {
        let a = opacity_to_alpha(30);
        let b = opacity_to_alpha(60);
        let c = opacity_to_alpha(100);
        assert!(a < b);
        assert!(b < c);
    }

    #[test]
    fn opacity_to_alpha_midpoint() {
        // 50% should be roughly half of 255.
        let alpha = opacity_to_alpha(50);
        assert!((125..=130).contains(&alpha), "got {alpha}");
    }

    #[test]
    fn initial_size_picks_orientation() {
        assert_eq!(
            initial_size("horizontal"),
            (FLOATBAR_DEFAULT_WIDTH_H, FLOATBAR_DEFAULT_HEIGHT_H)
        );
        assert_eq!(
            initial_size("vertical"),
            (FLOATBAR_DEFAULT_WIDTH_V, FLOATBAR_DEFAULT_HEIGHT_V)
        );
        // Unknown values fall through to horizontal so a corrupted setting
        // can't yield an unreadable strip.
        assert_eq!(
            initial_size("diagonal"),
            (FLOATBAR_DEFAULT_WIDTH_H, FLOATBAR_DEFAULT_HEIGHT_H)
        );
    }

    #[cfg(windows)]
    #[test]
    fn codex_title_match_excludes_our_own_windows() {
        assert!(!is_probable_codex_desktop_title("CodexBar"));
        assert!(!is_probable_codex_desktop_title("Codex 余量条"));
        assert!(!is_probable_codex_desktop_title("Codex Usage Bar"));
    }

    #[cfg(windows)]
    #[test]
    fn codex_title_match_allows_desktop_threads() {
        assert!(is_probable_codex_desktop_title("添加 Codex 余量监控插件"));
        assert!(is_probable_codex_desktop_title("Codex"));
    }
}
