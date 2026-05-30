//! System tray icon: shows idle/recording state via tooltip.

use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;
use std::thread;

pub struct TrayHandle {
    visible: Arc<AtomicBool>,
}

impl TrayHandle {
    pub fn set_recording(&self, active: bool) {
        self.visible.store(active, Ordering::Relaxed);
    }
}

impl Drop for TrayHandle {
    fn drop(&mut self) {
        self.set_recording(false);
    }
}

pub fn spawn() -> TrayHandle {
    let visible = Arc::new(AtomicBool::new(false));

    #[cfg(windows)]
    {
        let vis = visible.clone();
        thread::spawn(move || {
            win::run(vis);
        });
        thread::sleep(std::time::Duration::from_millis(200));
    }

    TrayHandle { visible }
}

#[cfg(windows)]
mod win {
    use std::sync::atomic::{AtomicBool, Ordering};
    use std::sync::Arc;

    use windows::core::w;
    use windows::Win32::Foundation::{HINSTANCE, HWND, LPARAM, LRESULT, WPARAM};
    use windows::Win32::System::LibraryLoader::GetModuleHandleW;
    use windows::Win32::UI::Shell::{
        Shell_NotifyIconW, NIF_ICON, NIF_MESSAGE, NIF_TIP, NIM_ADD, NIM_DELETE,
        NIM_MODIFY, NOTIFYICONDATAW,
    };
    use windows::Win32::UI::WindowsAndMessaging::{
        CreateWindowExW, DefWindowProcW, DispatchMessageW, LoadIconW,
        PeekMessageW, PostQuitMessage, RegisterClassW, TranslateMessage,
        MSG, PM_REMOVE, WM_APP, WM_DESTROY, WNDCLASSW, WS_OVERLAPPED,
        IDI_APPLICATION,
    };

    const TRAY_MSG: u32 = WM_APP + 1;

    pub fn run(visible: Arc<AtomicBool>) {
        unsafe {
            let hmodule = GetModuleHandleW(None).unwrap_or_default();
            let hi = HINSTANCE(hmodule.0);

            let class_name_raw: Vec<u16> = "PushToTalkTray\0".encode_utf16().collect();
            let class_pcwstr = windows::core::PCWSTR(class_name_raw.as_ptr());

            let wc = WNDCLASSW {
                style: Default::default(),
                lpfnWndProc: Some(wndproc),
                hInstance: hi,
                lpszClassName: class_pcwstr,
                ..Default::default()
            };
            RegisterClassW(&wc);

            let hwnd = CreateWindowExW(
                Default::default(),
                class_pcwstr,
                w!(""),
                WS_OVERLAPPED,
                0,
                0,
                0,
                0,
                None,
                None,
                hi,
                None,
            );

            let icon = LoadIconW(None, IDI_APPLICATION).unwrap_or_default();

            let idle_tip: Vec<u16> = "Push-to-Talk (idle)\0".encode_utf16().collect();
            let rec_tip: Vec<u16> = "● RECORDING — Push-to-Talk\0".encode_utf16().collect();

            let mut nid = NOTIFYICONDATAW {
                cbSize: std::mem::size_of::<NOTIFYICONDATAW>() as u32,
                hWnd: hwnd,
                uID: 1,
                uFlags: NIF_ICON | NIF_MESSAGE | NIF_TIP,
                uCallbackMessage: TRAY_MSG,
                hIcon: icon,
                ..Default::default()
            };
            nid.szTip[..idle_tip.len()].copy_from_slice(&idle_tip);
            let _ = Shell_NotifyIconW(NIM_ADD, &nid);

            let mut last_state = false;

            let mut msg = MSG::default();
            loop {
                let has_msg = PeekMessageW(&mut msg, None, 0, 0, PM_REMOVE).as_bool();

                if has_msg {
                    if msg.message == WM_DESTROY {
                        break;
                    }
                    let _ = TranslateMessage(&msg);
                    DispatchMessageW(&msg);
                }

                let is_rec = visible.load(Ordering::Relaxed);
                if is_rec != last_state {
                    last_state = is_rec;
                    let tip = if is_rec { &rec_tip } else { &idle_tip };
                    let mut update = NOTIFYICONDATAW {
                        cbSize: std::mem::size_of::<NOTIFYICONDATAW>() as u32,
                        hWnd: hwnd,
                        uID: 1,
                        uFlags: NIF_TIP,
                        ..Default::default()
                    };
                    update.szTip[..tip.len()].copy_from_slice(tip);
                    let _ = Shell_NotifyIconW(NIM_MODIFY, &update);
                }

                if !has_msg {
                    std::thread::sleep(std::time::Duration::from_millis(200));
                }
            }

            let _ = Shell_NotifyIconW(NIM_DELETE, &nid);
        }
    }

    unsafe extern "system" fn wndproc(
        hwnd: HWND,
        msg: u32,
        wparam: WPARAM,
        lparam: LPARAM,
    ) -> LRESULT {
        unsafe {
            match msg {
                WM_DESTROY => {
                    PostQuitMessage(0);
                    LRESULT(0)
                }
                _ => DefWindowProcW(hwnd, msg, wparam, lparam),
            }
        }
    }
}
