use crate::config::Config;
use std::ffi::c_void;
use std::time::Instant;
use windows::core::w;
use windows::Win32::Foundation::{COLORREF, HWND, LPARAM, LRESULT, POINT, SIZE, WPARAM};
use windows::Win32::Graphics::Gdi::{
    CreateCompatibleDC, CreateDIBSection, DeleteDC, DeleteObject, GetDC, ReleaseDC, SelectObject,
    AC_SRC_ALPHA, AC_SRC_OVER, BITMAPINFO, BITMAPINFOHEADER, BLENDFUNCTION, BI_RGB, DIB_RGB_COLORS,
    HBITMAP, HDC, HGDIOBJ,
};
use windows::Win32::System::LibraryLoader::GetModuleHandleW;
use windows::Win32::UI::WindowsAndMessaging::{
    CreateWindowExW, DefWindowProcW, GetSystemMetrics, KillTimer, RegisterClassW,
    SetProcessDPIAware, SetTimer, SetWindowPos, ShowWindow, UpdateLayeredWindow, HWND_TOPMOST,
    SM_CXSCREEN, SM_CYSCREEN, SWP_NOACTIVATE, SWP_SHOWWINDOW, SW_HIDE, ULW_ALPHA,
    WM_USER, WNDCLASSW, WS_EX_LAYERED, WS_EX_NOACTIVATE, WS_EX_TOOLWINDOW, WS_EX_TOPMOST,
    WS_EX_TRANSPARENT, WS_POPUP,
};

pub const WM_TRIGGER_GLOW: u32 = WM_USER + 101;
pub const WM_RELOAD_CONFIG: u32 = WM_USER + 102;
pub const TIMER_ID_PULSE: usize = 1;

pub unsafe extern "system" fn overlay_wndproc(
    hwnd: HWND,
    msg: u32,
    wparam: WPARAM,
    lparam: LPARAM,
) -> LRESULT {
    DefWindowProcW(hwnd, msg, wparam, lparam)
}

pub struct Overlay {
    pub hwnd: HWND,
    pub width: i32,
    pub height: i32,
    pub mem_dc: HDC,
    pub bitmap: HBITMAP,
    pub old_bmp: HGDIOBJ,
    pub pixel_ptr: *mut u8,
    pub is_pulsing: bool,
    pub pulse_start: Option<Instant>,
    pub current_duration: f32,
    pub current_color: (u8, u8, u8),
    pub current_border_width: u32,
}

impl Overlay {
    pub fn new(config: &Config) -> Result<Self, windows::core::Error> {
        let class_name = w!("ScreenGlowOverlayClass");

        unsafe {
            let _ = SetProcessDPIAware();
            let hinstance = GetModuleHandleW(None)?;

            let wc = WNDCLASSW {
                lpfnWndProc: Some(overlay_wndproc),
                hInstance: hinstance.into(),
                lpszClassName: class_name,
                ..Default::default()
            };
            let _ = RegisterClassW(&wc);

            let width = GetSystemMetrics(SM_CXSCREEN);
            let height = GetSystemMetrics(SM_CYSCREEN);

            let hwnd = CreateWindowExW(
                WS_EX_LAYERED
                    | WS_EX_TRANSPARENT
                    | WS_EX_TOPMOST
                    | WS_EX_TOOLWINDOW
                    | WS_EX_NOACTIVATE,
                class_name,
                w!("ScreenGlowOverlay"),
                WS_POPUP,
                0,
                0,
                width,
                height,
                None,
                None,
                hinstance,
                None,
            )?;

            let mem_dc = CreateCompatibleDC(None);

            let mut bmi = BITMAPINFO::default();
            bmi.bmiHeader.biSize = std::mem::size_of::<BITMAPINFOHEADER>() as u32;
            bmi.bmiHeader.biWidth = width;
            bmi.bmiHeader.biHeight = -height; // Top-down
            bmi.bmiHeader.biPlanes = 1;
            bmi.bmiHeader.biBitCount = 32;
            bmi.bmiHeader.biCompression = BI_RGB.0;

            let mut ppv_bits: *mut c_void = std::ptr::null_mut();
            let bitmap = CreateDIBSection(
                mem_dc,
                &bmi,
                DIB_RGB_COLORS,
                &mut ppv_bits,
                None,
                0,
            )?;

            let old_bmp = SelectObject(mem_dc, bitmap);
            let pixel_ptr = ppv_bits as *mut u8;

            let rgb = config.rgb();
            let mut overlay = Self {
                hwnd,
                width,
                height,
                mem_dc,
                bitmap,
                old_bmp,
                pixel_ptr,
                is_pulsing: false,
                pulse_start: None,
                current_duration: config.glow.duration_secs,
                current_color: rgb,
                current_border_width: config.glow.border_width,
            };

            overlay.render_gradient(rgb, config.glow.border_width);
            overlay.commit_alpha(0);

            Ok(overlay)
        }
    }

    pub fn render_gradient(&mut self, (r, g, b): (u8, u8, u8), border_width: u32) {
        if self.pixel_ptr.is_null() || border_width == 0 {
            return;
        }

        self.current_color = (r, g, b);
        self.current_border_width = border_width;

        let w = self.width;
        let h = self.height;
        let bw = border_width as i32;
        let bw_f = border_width as f32;

        unsafe {
            let total_pixels = (w * h) as usize;
            let slice = std::slice::from_raw_parts_mut(self.pixel_ptr, total_pixels * 4);

            for y in 0..h {
                let dist_y = y.min(h - 1 - y);
                let row_offset = (y * w * 4) as usize;

                if dist_y >= bw {
                    // Left border (0..bw)
                    for x in 0..bw {
                        let t = 1.0 - (x as f32 / bw_f);
                        let smooth = t * t * (3.0 - 2.0 * t);
                        let a = (255.0 * smooth).clamp(0.0, 255.0) as u32;

                        let pr = ((r as u32 * a) / 255) as u8;
                        let pg = ((g as u32 * a) / 255) as u8;
                        let pb = ((b as u32 * a) / 255) as u8;

                        let idx = row_offset + (x as usize * 4);
                        slice[idx] = pb;
                        slice[idx + 1] = pg;
                        slice[idx + 2] = pr;
                        slice[idx + 3] = a as u8;
                    }

                    // Middle is completely transparent
                    let mid_start = row_offset + (bw as usize * 4);
                    let mid_len = ((w - 2 * bw).max(0) as usize) * 4;
                    if mid_len > 0 && mid_start + mid_len <= slice.len() {
                        slice[mid_start..mid_start + mid_len].fill(0);
                    }

                    // Right border (w - bw .. w)
                    for x in (w - bw)..w {
                        let dist_x = w - 1 - x;
                        let t = 1.0 - (dist_x as f32 / bw_f);
                        let smooth = t * t * (3.0 - 2.0 * t);
                        let a = (255.0 * smooth).clamp(0.0, 255.0) as u32;

                        let pr = ((r as u32 * a) / 255) as u8;
                        let pg = ((g as u32 * a) / 255) as u8;
                        let pb = ((b as u32 * a) / 255) as u8;

                        let idx = row_offset + (x as usize * 4);
                        slice[idx] = pb;
                        slice[idx + 1] = pg;
                        slice[idx + 2] = pr;
                        slice[idx + 3] = a as u8;
                    }
                } else {
                    // Top or Bottom border rows
                    for x in 0..w {
                        let dist_x = x.min(w - 1 - x);
                        let dist = dist_x.min(dist_y);

                        if dist < bw {
                            let t = 1.0 - (dist as f32 / bw_f);
                            let smooth = t * t * (3.0 - 2.0 * t);
                            let a = (255.0 * smooth).clamp(0.0, 255.0) as u32;

                            let pr = ((r as u32 * a) / 255) as u8;
                            let pg = ((g as u32 * a) / 255) as u8;
                            let pb = ((b as u32 * a) / 255) as u8;

                            let idx = row_offset + (x as usize * 4);
                            slice[idx] = pb;
                            slice[idx + 1] = pg;
                            slice[idx + 2] = pr;
                            slice[idx + 3] = a as u8;
                        } else {
                            let idx = row_offset + (x as usize * 4);
                            slice[idx] = 0;
                            slice[idx + 1] = 0;
                            slice[idx + 2] = 0;
                            slice[idx + 3] = 0;
                        }
                    }
                }
            }
        }
    }

    pub fn commit_alpha(&self, alpha: u8) {
        unsafe {
            let screen_dc = GetDC(None);
            let pt_dst = POINT { x: 0, y: 0 };
            let sz_dst = SIZE {
                cx: self.width,
                cy: self.height,
            };
            let pt_src = POINT { x: 0, y: 0 };
            let blend = BLENDFUNCTION {
                BlendOp: AC_SRC_OVER as u8,
                BlendFlags: 0,
                SourceConstantAlpha: alpha,
                AlphaFormat: AC_SRC_ALPHA as u8,
            };

            let _ = UpdateLayeredWindow(
                self.hwnd,
                screen_dc,
                Some(&pt_dst),
                Some(&sz_dst),
                self.mem_dc,
                Some(&pt_src),
                COLORREF(0),
                Some(&blend),
                ULW_ALPHA,
            );
            let _ = ReleaseDC(None, screen_dc);
        }
    }

    pub fn trigger_glow(
        &mut self,
        config: &Config,
        custom_color: Option<String>,
        custom_duration: Option<f32>,
        custom_width: Option<u32>,
    ) {
        let color = if let Some(hex) = custom_color {
            Config::parse_hex_color(&hex)
        } else {
            config.rgb()
        };

        let border_width = custom_width.unwrap_or(config.glow.border_width);

        if color != self.current_color || border_width != self.current_border_width {
            self.render_gradient(color, border_width);
        }

        self.current_duration = custom_duration.unwrap_or(config.glow.duration_secs);
        self.pulse_start = Some(Instant::now());
        self.is_pulsing = true;

        unsafe {
            let _ = SetWindowPos(
                self.hwnd,
                HWND_TOPMOST,
                0,
                0,
                self.width,
                self.height,
                SWP_SHOWWINDOW | SWP_NOACTIVATE,
            );
            let _ = SetTimer(self.hwnd, TIMER_ID_PULSE, 16, None);
        }

        let initial_alpha = (config.glow.max_opacity * 255.0) as u8;
        self.commit_alpha(initial_alpha);
    }

    pub fn on_timer(&mut self, config: &Config) {
        if !self.is_pulsing {
            return;
        }

        if let Some(start) = self.pulse_start {
            let elapsed = start.elapsed().as_secs_f32();

            if self.current_duration > 0.0 && elapsed >= self.current_duration {
                self.stop_glow();
                return;
            }

            let speed = config.glow.pulse_speed;
            let t = elapsed * speed * std::f32::consts::TAU;
            let factor = 0.5 * (1.0 + (t - std::f32::consts::FRAC_PI_2).sin());

            let min_a = config.glow.min_opacity;
            let max_a = config.glow.max_opacity;
            let current_a = min_a + (max_a - min_a) * factor;

            let master_alpha = (current_a * 255.0).clamp(0.0, 255.0) as u8;
            self.commit_alpha(master_alpha);
        }
    }

    pub fn stop_glow(&mut self) {
        self.is_pulsing = false;
        self.pulse_start = None;
        unsafe {
            let _ = KillTimer(self.hwnd, TIMER_ID_PULSE);
            let _ = ShowWindow(self.hwnd, SW_HIDE);
        }
        self.commit_alpha(0);
    }
}

impl Drop for Overlay {
    fn drop(&mut self) {
        unsafe {
            let _ = SelectObject(self.mem_dc, self.old_bmp);
            let _ = DeleteObject(self.bitmap);
            let _ = DeleteDC(self.mem_dc);
        }
    }
}
