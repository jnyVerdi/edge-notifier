mod config;
mod overlay;
mod server;
mod tray;

use config::Config;
use overlay::{Overlay, TIMER_ID_PULSE, WM_RELOAD_CONFIG, WM_TRIGGER_GLOW};
use server::{start_server, NotifyRequest};
use std::env;
use std::io::{Read, Write};
use std::net::TcpStream;
use std::sync::mpsc::{channel, Receiver, Sender};
use std::time::Duration;
use tray::{TrayIcon, WM_TRAYICON};
use windows::Win32::UI::WindowsAndMessaging::{
    DispatchMessageW, GetMessageW, PostQuitMessage, TranslateMessage, MSG,
    WM_CLOSE, WM_DESTROY, WM_LBUTTONUP, WM_QUIT, WM_RBUTTONUP, WM_TIMER,
};

pub const WM_SERVER_NOTIFY: u32 = windows::Win32::UI::WindowsAndMessaging::WM_USER + 103;

fn main() {
    let args: Vec<String> = env::args().collect();
    println!("[Main] Starting with args: {:?}", args);

    if args.len() > 1 && (args[1] == "notify" || args[1] == "--trigger" || args[1] == "-t") {
        let config = Config::load_or_create("config.toml");
        trigger_via_cli(&args[2..], config.server.port);
        return;
    }

    run_daemon();
}

fn trigger_via_cli(args: &[String], port: u16) {
    let mut color = None;
    let mut duration = None;
    let mut width = None;

    let mut i = 0;
    while i < args.len() {
        match args[i].as_str() {
            "--color" | "-c" if i + 1 < args.len() => {
                color = Some(args[i + 1].clone());
                i += 2;
            }
            "--duration" | "-d" if i + 1 < args.len() => {
                duration = Some(args[i + 1].clone());
                i += 2;
            }
            "--width" | "-w" if i + 1 < args.len() => {
                width = Some(args[i + 1].clone());
                i += 2;
            }
            _ => {
                i += 1;
            }
        }
    }

    let mut query_params = Vec::new();
    if let Some(c) = color {
        query_params.push(format!("color={}", c.replace('#', "%23")));
    }
    if let Some(d) = duration {
        query_params.push(format!("duration={}", d));
    }
    if let Some(w) = width {
        query_params.push(format!("width={}", w));
    }

    let path = if query_params.is_empty() {
        "/notify".to_string()
    } else {
        format!("/notify?{}", query_params.join("&"))
    };

    let addr = format!("127.0.0.1:{}", port);

    if TcpStream::connect(&addr).is_err() {
        println!("[CLI] Daemon not found, starting...");
        if let Ok(exe_path) = env::current_exe() {
            let _ = std::process::Command::new(exe_path).spawn();
            std::thread::sleep(Duration::from_millis(600));
        }
    }

    if let Ok(mut stream) = TcpStream::connect(&addr) {
        let req = format!(
            "GET {} HTTP/1.1\r\nHost: {}\r\nConnection: close\r\n\r\n",
            path, addr
        );
        let _ = stream.write_all(req.as_bytes());
        let mut resp = String::new();
        let _ = stream.read_to_string(&mut resp);
        println!("✨ Edge glow notification triggered successfully!");
    } else {
        eprintln!("❌ Failed to connect to notifier on {}", addr);
    }
}

fn run_daemon() {
    println!("[Daemon] Initializing...");
    let config = Config::load_or_create("config.toml");
    let addr = format!("127.0.0.1:{}", config.server.port);

    if config.server.enabled && TcpStream::connect(&addr).is_ok() {
        println!("[Daemon] Another instance is already listening on {}. Triggering it...", addr);
        trigger_via_cli(&[], config.server.port);
        return;
    }

    let mut config = config;
    let mut overlay = match Overlay::new(&config) {
        Ok(o) => {
            println!("[Daemon] Overlay created successfully (hwnd: {:?})", o.hwnd);
            o
        }
        Err(e) => {
            eprintln!("[Error] Failed to initialize overlay: {:?}", e);
            return;
        }
    };

    let _tray = TrayIcon::new(overlay.hwnd);
    println!("[Daemon] Tray icon created.");

    let (tx, rx): (Sender<NotifyRequest>, Receiver<NotifyRequest>) = channel();

    if config.server.enabled {
        start_server(config.server.port, tx, overlay.hwnd.0 as usize);
        println!("[Daemon] Server started on port {}", config.server.port);
    }

    println!("[Daemon] Triggering welcome glow...");
    overlay.trigger_glow(&config, None, Some(1.5), None);

    println!("[Daemon] Entering message loop...");
    unsafe {
        let mut msg = MSG::default();
        loop {
            let ret = GetMessageW(&mut msg, None, 0, 0);
            if ret.0 == 0 || ret.0 == -1 {
                println!("[Daemon] GetMessageW returned {}, exiting loop", ret.0);
                break;
            }

            match msg.message {
                WM_TIMER => {
                    if msg.wParam.0 == TIMER_ID_PULSE {
                        overlay.on_timer(&config);
                    }
                }
                WM_TRIGGER_GLOW => {
                    println!("[Daemon] WM_TRIGGER_GLOW received");
                    overlay.trigger_glow(&config, None, None, None);
                }
                WM_SERVER_NOTIFY => {
                    println!("[Daemon] WM_SERVER_NOTIFY received");
                    while let Ok(req) = rx.try_recv() {
                        overlay.trigger_glow(
                            &config,
                            req.color,
                            req.duration_secs,
                            req.border_width,
                        );
                    }
                }
                WM_RELOAD_CONFIG => {
                    println!("[Daemon] Reloading config");
                    config = Config::load_or_create("config.toml");
                    let rgb = config.rgb();
                    overlay.render_gradient(rgb, config.glow.border_width);
                }
                WM_TRAYICON => {
                    let event = msg.lParam.0 as u32;
                    if event == WM_RBUTTONUP || event == WM_LBUTTONUP {
                        _tray.show_menu(overlay.hwnd);
                    }
                }
                WM_CLOSE | WM_DESTROY => {
                    println!("[Daemon] WM_CLOSE/DESTROY received");
                    PostQuitMessage(0);
                }
                WM_QUIT => {
                    println!("[Daemon] WM_QUIT received");
                    break;
                }
                _ => {
                    let _ = TranslateMessage(&msg);
                    DispatchMessageW(&msg);
                }
            }
        }
    }
    println!("[Daemon] Exited.");
}
