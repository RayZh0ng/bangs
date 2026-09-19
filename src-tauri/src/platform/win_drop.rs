//! Catching file drops on Windows.
//!
//! The window under the pointer over the notch belongs to WebView2's own
//! process, which refuses external drops, and the drag never reaches this
//! process at all — the webview's drag-drop events simply never arrive
//! (verified: nothing is reported for a real drag, on any window style).
//!
//! So the notch catches drops itself. While a drag that began somewhere else
//! passes over the notch, an invisible window of ours is put on top of it and
//! acts as the drop target; it hands the paths to the webview, which shelves
//! them exactly as it does on macOS. It only exists during such a drag, so it
//! never swallows an ordinary click.

use std::path::PathBuf;
use std::sync::atomic::{AtomicIsize, Ordering};

use tauri::{AppHandle, Emitter};
use windows::core::{implement, w, Ref, Result as WinResult};
use windows::Win32::Foundation::{HWND, LPARAM, LRESULT, WPARAM};
use windows::Win32::System::Com::{IDataObject, DVASPECT_CONTENT, FORMATETC, TYMED_HGLOBAL};
use windows::Win32::System::LibraryLoader::GetModuleHandleW;
use windows::Win32::System::Ole::{
    IDropTarget, IDropTarget_Impl, RegisterDragDrop, ReleaseStgMedium, CF_HDROP, DROPEFFECT,
    DROPEFFECT_COPY, DROPEFFECT_NONE,
};
use windows::Win32::System::SystemServices::MODIFIERKEYS_FLAGS;
use windows::Win32::UI::Shell::{DragQueryFileW, HDROP};
use windows::Win32::UI::WindowsAndMessaging::{
    CreateWindowExW, DefWindowProcW, RegisterClassW, SetLayeredWindowAttributes, SetWindowPos,
    ShowWindow, HWND_TOPMOST, LWA_ALPHA, SWP_NOACTIVATE, SW_HIDE, SW_SHOWNOACTIVATE, WNDCLASSW,
    WS_EX_LAYERED, WS_EX_NOACTIVATE, WS_EX_TOOLWINDOW, WS_EX_TOPMOST, WS_POPUP,
};

use windows::Win32::Foundation::POINTL;

/// The catcher window, once it exists.
static CATCHER: AtomicIsize = AtomicIsize::new(0);
/// Whether it is currently over the notch.
static SHOWN: AtomicIsize = AtomicIsize::new(0);

/// Creates the catcher up front, so the first drag does not have to wait for
/// it and any failure is reported once, at start-up.
pub fn prepare(app: &AppHandle) {
    let handle = app.clone();
    let _ = app.run_on_main_thread(move || {
        catcher(&handle);
    });
}

/// Shows or hides the catcher over `rect` (physical pixels). Called from the
/// cursor tracker: a drag is in progress when the button went down outside the
/// notch and the pointer is now inside it.
pub fn set_catching(app: &AppHandle, catching: bool, rect: (i32, i32, i32, i32)) {
    let shown = SHOWN.load(Ordering::Relaxed) != 0;
    // Hiding what is already hidden is the common case, every poll.
    if !catching && !shown {
        return;
    }
    SHOWN.store(catching as isize, Ordering::Relaxed);

    eprintln!("[drop] catching={catching} rect={rect:?}");
    let handle = app.clone();
    let _ = app.run_on_main_thread(move || {
        let hwnd = match catcher(&handle) {
            Some(hwnd) => hwnd,
            None => return,
        };
        unsafe {
            if catching {
                let (x, y, width, height) = rect;
                let _ = SetWindowPos(hwnd, Some(HWND_TOPMOST), x, y, width, height, SWP_NOACTIVATE);
                let _ = ShowWindow(hwnd, SW_SHOWNOACTIVATE);
            } else {
                let _ = ShowWindow(hwnd, SW_HIDE);
            }
        }
    });
}

/// The catcher window, created on first use. Must run on the main thread,
/// which is the one with a message loop and OLE already initialised.
fn catcher(app: &AppHandle) -> Option<HWND> {
    let existing = CATCHER.load(Ordering::Relaxed);
    if existing != 0 {
        return Some(HWND(existing as *mut _));
    }

    unsafe {
        let instance = match GetModuleHandleW(None) {
            Ok(instance) => instance,
            Err(error) => {
                eprintln!("[drop] no module handle: {error}");
                return None;
            }
        };
        let class = WNDCLASSW {
            lpfnWndProc: Some(catcher_proc),
            hInstance: instance.into(),
            lpszClassName: w!("BangsDropCatcher"),
            ..Default::default()
        };
        // A second registration is harmless; the class already exists then.
        if RegisterClassW(&class) == 0 {
            eprintln!("[drop] window class: {}", windows::core::Error::from_win32());
        }

        let hwnd = CreateWindowExW(
            WS_EX_LAYERED | WS_EX_TOPMOST | WS_EX_NOACTIVATE | WS_EX_TOOLWINDOW,
            w!("BangsDropCatcher"),
            w!("Bangs drop"),
            WS_POPUP,
            0,
            0,
            0,
            0,
            None,
            None,
            Some(instance.into()),
            None,
        );
        let hwnd = match hwnd {
            Ok(hwnd) => hwnd,
            Err(error) => {
                eprintln!("[drop] no catcher window: {error}");
                return None;
            }
        };

        // Almost transparent: invisible to the eye, still a drop target.
        let _ = SetLayeredWindowAttributes(hwnd, Default::default(), 1, LWA_ALPHA);

        let target: IDropTarget = Catcher { app: app.clone() }.into();
        if let Err(error) = RegisterDragDrop(hwnd, &target) {
            eprintln!("[drop] could not register the drop target: {error}");
            return None;
        }
        eprintln!("[drop] catcher ready");
        // The target has to outlive this call; the window owns it from here.
        std::mem::forget(target);

        CATCHER.store(hwnd.0 as isize, Ordering::Relaxed);
        Some(hwnd)
    }
}

unsafe extern "system" fn catcher_proc(
    hwnd: HWND,
    message: u32,
    wparam: WPARAM,
    lparam: LPARAM,
) -> LRESULT {
    DefWindowProcW(hwnd, message, wparam, lparam)
}

#[implement(IDropTarget)]
struct Catcher {
    app: AppHandle,
}

/// The paths a drag carries, or none when it carries something else. The
/// medium belongs to the drag source, so it is released, never freed with
/// `DragFinish` — doing that during a drag would pull the data out from under
/// the drop that follows.
unsafe fn dragged_paths(data: Ref<'_, IDataObject>) -> Option<Vec<PathBuf>> {
    unsafe {
        let format = FORMATETC {
            cfFormat: CF_HDROP.0,
            ptd: std::ptr::null_mut(),
            dwAspect: DVASPECT_CONTENT.0,
            lindex: -1,
            tymed: TYMED_HGLOBAL.0 as u32,
        };
        let mut medium = data.as_ref()?.GetData(&format).ok()?;
        let drop = HDROP(medium.u.hGlobal.0 as _);

        let count = DragQueryFileW(drop, u32::MAX, None);
        let mut paths = Vec::with_capacity(count as usize);
        for index in 0..count {
            let length = DragQueryFileW(drop, index, None) as usize;
            if length == 0 {
                continue;
            }
            let mut buffer = vec![0u16; length + 1];
            let written = DragQueryFileW(drop, index, Some(&mut buffer)) as usize;
            paths.push(PathBuf::from(String::from_utf16_lossy(&buffer[..written])));
        }
        ReleaseStgMedium(&mut medium);
        Some(paths)
    }
}

impl Catcher {
    fn tell(&self, event: &str, paths: &[PathBuf]) {
        let paths: Vec<String> =
            paths.iter().map(|path| path.to_string_lossy().into_owned()).collect();
        let _ = self.app.emit(event, paths);
    }
}

impl IDropTarget_Impl for Catcher_Impl {
    fn DragEnter(
        &self,
        data: Ref<'_, IDataObject>,
        _keys: MODIFIERKEYS_FLAGS,
        _point: &POINTL,
        effect: *mut DROPEFFECT,
    ) -> WinResult<()> {
        let paths = unsafe { dragged_paths(data) }.unwrap_or_default();
        eprintln!("[drop] enter with {} path(s)", paths.len());
        unsafe { *effect = if paths.is_empty() { DROPEFFECT_NONE } else { DROPEFFECT_COPY } };
        if !paths.is_empty() {
            self.tell("bangs://drag-enter", &paths);
        }
        Ok(())
    }

    fn DragOver(
        &self,
        _keys: MODIFIERKEYS_FLAGS,
        _point: &POINTL,
        effect: *mut DROPEFFECT,
    ) -> WinResult<()> {
        unsafe { *effect = DROPEFFECT_COPY };
        Ok(())
    }

    fn DragLeave(&self) -> WinResult<()> {
        let _ = self.app.emit("bangs://drag-leave", ());
        Ok(())
    }

    fn Drop(
        &self,
        data: Ref<'_, IDataObject>,
        _keys: MODIFIERKEYS_FLAGS,
        _point: &POINTL,
        effect: *mut DROPEFFECT,
    ) -> WinResult<()> {
        let paths = unsafe { dragged_paths(data) }.unwrap_or_default();
        eprintln!("[drop] dropped {} path(s)", paths.len());
        unsafe { *effect = DROPEFFECT_COPY };
        self.tell("bangs://drop", &paths);
        Ok(())
    }
}
