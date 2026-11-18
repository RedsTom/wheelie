//! Removes the system frame, border and shadow from popup windows.
//!
//! Windows draws a 1 px border and a drop shadow around `WS_POPUP` windows
//! through the desktop window manager (DWM). The attributes below turn both
//! off so that only the content rendered by the window is visible.

use crate::platform::error::PlatformError;
use raw_window_handle::HasWindowHandle;

/// Strips the system frame from the native window behind `window`.
///
/// Platforms without DWM have no system frame, so this does nothing there.
#[cfg(target_os = "windows")]
pub fn strip_frame<W: HasWindowHandle>(window: &W) -> Result<(), PlatformError> {
    use raw_window_handle::RawWindowHandle;
    use std::ffi::c_void;
    use std::mem::{size_of, size_of_val};
    use windows::Win32::Foundation::HWND;
    use windows::Win32::Graphics::Dwm::{
        DWM_WINDOW_CORNER_PREFERENCE, DWMNCRENDERINGPOLICY, DWMNCRP_DISABLED, DWMWA_BORDER_COLOR,
        DWMWA_NCRENDERING_POLICY, DWMWA_WINDOW_CORNER_PREFERENCE, DWMWCP_DONOTROUND,
        DwmSetWindowAttribute,
    };

    /// `DWMWA_COLOR_NONE`: removes the 1 px border colour.
    const COLOR_NONE: u32 = 0xFFFF_FFFE;

    let handle = window
        .window_handle()
        .map_err(|error| PlatformError::Other(error.to_string()))?;
    let RawWindowHandle::Win32(win32) = handle.as_raw() else {
        return Err(PlatformError::Other(
            "window is not a Win32 window".to_owned(),
        ));
    };
    let hwnd = HWND(win32.hwnd.get() as *mut c_void);

    let map = |error: windows::core::Error| PlatformError::Other(error.to_string());

    let border: u32 = COLOR_NONE;
    let corner: DWM_WINDOW_CORNER_PREFERENCE = DWMWCP_DONOTROUND;
    let policy: DWMNCRENDERINGPOLICY = DWMNCRP_DISABLED;

    // SAFETY: `hwnd` comes from the live window handle and every value
    // pointer refers to a local of the size passed alongside it.
    let results = unsafe {
        [
            DwmSetWindowAttribute(
                hwnd,
                DWMWA_BORDER_COLOR,
                (&border as *const u32).cast(),
                size_of::<u32>() as u32,
            ),
            DwmSetWindowAttribute(
                hwnd,
                DWMWA_WINDOW_CORNER_PREFERENCE,
                (&corner as *const DWM_WINDOW_CORNER_PREFERENCE).cast(),
                size_of_val(&corner) as u32,
            ),
            DwmSetWindowAttribute(
                hwnd,
                DWMWA_NCRENDERING_POLICY,
                (&policy as *const DWMNCRENDERINGPOLICY).cast(),
                size_of_val(&policy) as u32,
            ),
        ]
    };

    // Each attribute is applied on its own: border colour and corner
    // preference only exist on Windows 11 and must not block the shadow fix.
    results
        .into_iter()
        .filter_map(Result::err)
        .map(map)
        .next()
        .map_or(Ok(()), Err)
}

/// Hides the native window immediately, before gpui removes it.
///
/// gpui dispatches active-status callbacks asynchronously. Removing a window
/// while it is still active makes that callback update a window handle gpui has
/// already dropped, which logs a spurious "window not found" error. Hiding the
/// native window first delivers the deactivation while the handle is still
/// live, so the deferred callback resolves cleanly.
#[cfg(target_os = "windows")]
pub fn hide_window<W: HasWindowHandle>(window: &W) -> Result<(), PlatformError> {
    use raw_window_handle::RawWindowHandle;
    use std::ffi::c_void;
    use windows::Win32::Foundation::HWND;
    use windows::Win32::UI::WindowsAndMessaging::{SW_HIDE, ShowWindow};

    let handle = window
        .window_handle()
        .map_err(|error| PlatformError::Other(error.to_string()))?;
    let RawWindowHandle::Win32(win32) = handle.as_raw() else {
        return Err(PlatformError::Other(
            "window is not a Win32 window".to_owned(),
        ));
    };
    // SAFETY: `hwnd` comes from the live window handle passed by the caller.
    let _ = unsafe { ShowWindow(HWND(win32.hwnd.get() as *mut c_void), SW_HIDE) };
    Ok(())
}

/// No native hide needed outside Windows.
#[cfg(not(target_os = "windows"))]
pub fn hide_window<W: HasWindowHandle>(_window: &W) -> Result<(), PlatformError> {
    Ok(())
}

/// No system frame to strip outside Windows.
#[cfg(not(target_os = "windows"))]
pub fn strip_frame<W: HasWindowHandle>(_window: &W) -> Result<(), PlatformError> {
    Ok(())
}
