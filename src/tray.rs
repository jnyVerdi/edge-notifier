use std::process::Command;
use windows::core::w;
use windows::Win32::Foundation::{HWND, LPARAM, POINT, WPARAM};
use windows::Win32::UI::Shell::{
    Shell_NotifyIconW, NIM_ADD, NIM_DELETE, NOTIFYICONDATAW, NIF_ICON, NIF_MESSAGE, NIF_TIP,
};
use windows::Win32::UI::WindowsAndMessaging::{
    CreatePopupMenu, DestroyMenu, GetCursorPos, LoadIconW, SetForegroundWindow,
    TrackPopupMenu, AppendMenuW, IDI_APPLICATION, MF_SEPARATOR, MF_STRING, TPM_BOTTOMALIGN,
    TPM_RIGHTBUTTON, WM_USER,
};

pub const WM_TRAYICON: u32 = WM_USER + 201;

pub const ID_TRAY_TEST: usize = 1001;
pub const ID_TRAY_RELOAD: usize = 1002;
pub const ID_TRAY_OPEN_CONFIG: usize = 1003;
pub const ID_TRAY_EXIT: usize = 1004;

pub struct TrayIcon {
    nid: NOTIFYICONDATAW,
}

impl TrayIcon {
    pub fn new(hwnd: HWND) -> Self {
        let mut nid = NOTIFYICONDATAW::default();
        nid.cbSize = std::mem::size_of::<NOTIFYICONDATAW>() as u32;
        nid.hWnd = hwnd;
        nid.uID = 1;
        nid.uFlags = NIF_MESSAGE | NIF_ICON | NIF_TIP;
        nid.uCallbackMessage = WM_TRAYICON;

        unsafe {
            if let Ok(icon) = LoadIconW(None, IDI_APPLICATION) {
                nid.hIcon = icon;
            }

            let tip = "Screen Glow Notifier";
            let mut sz_tip = [0u16; 128];
            for (i, c) in tip.encode_utf16().enumerate().take(127) {
                sz_tip[i] = c;
            }
            nid.szTip = sz_tip;

            let _ = Shell_NotifyIconW(NIM_ADD, &nid);
        }

        Self { nid }
    }

    pub fn show_menu(&self, hwnd: HWND) {
        unsafe {
            let mut pt = POINT::default();
            let _ = GetCursorPos(&mut pt);

            if let Ok(hmenu) = CreatePopupMenu() {
                let _ = AppendMenuW(hmenu, MF_STRING, ID_TRAY_TEST, w!("Trigger Glow Test"));
                let _ = AppendMenuW(hmenu, MF_STRING, ID_TRAY_RELOAD, w!("Reload Config"));
                let _ = AppendMenuW(hmenu, MF_STRING, ID_TRAY_OPEN_CONFIG, w!("Open config.toml"));
                let _ = AppendMenuW(hmenu, MF_SEPARATOR, 0, None);
                let _ = AppendMenuW(hmenu, MF_STRING, ID_TRAY_EXIT, w!("Exit"));

                let _ = SetForegroundWindow(hwnd);
                let cmd = TrackPopupMenu(
                    hmenu,
                    TPM_BOTTOMALIGN | TPM_RIGHTBUTTON | windows::Win32::UI::WindowsAndMessaging::TPM_RETURNCMD,
                    pt.x,
                    pt.y,
                    0,
                    hwnd,
                    None,
                );
                let _ = DestroyMenu(hmenu);

                match cmd.0 as usize {
                    ID_TRAY_TEST => {
                        let _ = windows::Win32::UI::WindowsAndMessaging::PostMessageW(
                            hwnd,
                            crate::overlay::WM_TRIGGER_GLOW,
                            WPARAM(0),
                            LPARAM(0),
                        );
                    }
                    ID_TRAY_RELOAD => {
                        let _ = windows::Win32::UI::WindowsAndMessaging::PostMessageW(
                            hwnd,
                            crate::overlay::WM_RELOAD_CONFIG,
                            WPARAM(0),
                            LPARAM(0),
                        );
                    }
                    ID_TRAY_OPEN_CONFIG => {
                        let _ = Command::new("notepad.exe").arg("config.toml").spawn();
                    }
                    ID_TRAY_EXIT => {
                        let _ = windows::Win32::UI::WindowsAndMessaging::PostMessageW(
                            hwnd,
                            windows::Win32::UI::WindowsAndMessaging::WM_CLOSE,
                            WPARAM(0),
                            LPARAM(0),
                        );
                    }
                    _ => {}
                }
            }
        }
    }
}

impl Drop for TrayIcon {
    fn drop(&mut self) {
        unsafe {
            let _ = Shell_NotifyIconW(NIM_DELETE, &self.nid);
        }
    }
}
