use std::io::{Read, Write};
use std::net::{TcpListener, TcpStream};
use std::sync::mpsc::Sender;
use std::thread;
use windows::Win32::Foundation::{HWND, LPARAM, WPARAM};
use windows::Win32::UI::WindowsAndMessaging::{PostMessageW, WM_CLOSE};

#[derive(Debug, Clone)]
pub struct NotifyRequest {
    pub color: Option<String>,
    pub duration_secs: Option<f32>,
    pub border_width: Option<u32>,
}

pub fn start_server(listener: TcpListener, tx: Sender<NotifyRequest>, hwnd_val: usize) {
    thread::spawn(move || {
        for stream in listener.incoming() {
            if let Ok(stream) = stream {
                let tx_conn = tx.clone();
                handle_connection(stream, tx_conn, hwnd_val);
            }
        }
    });
}

fn handle_connection(mut stream: TcpStream, tx: Sender<NotifyRequest>, hwnd_val: usize) {
    let mut buffer = [0u8; 2048];
    let bytes_read = match stream.read(&mut buffer) {
        Ok(n) if n > 0 => n,
        _ => return,
    };

    let request_str = String::from_utf8_lossy(&buffer[..bytes_read]);
    let first_line = request_str.lines().next().unwrap_or("");
    let parts: Vec<&str> = first_line.split_whitespace().collect();

    if parts.len() < 2 {
        let _ = stream.write_all(b"HTTP/1.1 400 Bad Request\r\n\r\n");
        return;
    }

    let target = parts[1];

    if target.starts_with("/ping") {
        let body = r#"{"status":"ok"}"#;
        let response = format!(
            "HTTP/1.1 200 OK\r\nContent-Type: application/json\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{}",
            body.len(),
            body
        );
        let _ = stream.write_all(response.as_bytes());
        return;
    }

    if target.starts_with("/quit") {
        let body = r#"{"status":"quitting"}"#;
        let response = format!(
            "HTTP/1.1 200 OK\r\nContent-Type: application/json\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{}",
            body.len(),
            body
        );
        let _ = stream.write_all(response.as_bytes());
        unsafe {
            let hwnd = HWND(hwnd_val as *mut std::ffi::c_void);
            let _ = PostMessageW(hwnd, WM_CLOSE, WPARAM(0), LPARAM(0));
        }
        return;
    }

    if target.starts_with("/notify") {
        let mut req = NotifyRequest {
            color: None,
            duration_secs: None,
            border_width: None,
        };

        if let Some(query_idx) = target.find('?') {
            let query = &target[query_idx + 1..];
            for pair in query.split('&') {
                let mut kv = pair.split('=');
                if let (Some(key), Some(val)) = (kv.next(), kv.next()) {
                    let decoded_val = url_decode(val);
                    match key {
                        "color" => req.color = Some(decoded_val),
                        "duration" => req.duration_secs = decoded_val.parse::<f32>().ok(),
                        "width" => req.border_width = decoded_val.parse::<u32>().ok(),
                        _ => {}
                    }
                }
            }
        }

        let _ = tx.send(req);

        // Notify main thread window
        unsafe {
            let hwnd = HWND(hwnd_val as *mut std::ffi::c_void);
            let _ = PostMessageW(hwnd, crate::WM_SERVER_NOTIFY, WPARAM(0), LPARAM(0));
        }

        let body = r#"{"status":"triggered"}"#;
        let response = format!(
            "HTTP/1.1 200 OK\r\nAccess-Control-Allow-Origin: *\r\nContent-Type: application/json\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{}",
            body.len(),
            body
        );
        let _ = stream.write_all(response.as_bytes());
        return;
    }

    let _ = stream.write_all(b"HTTP/1.1 404 Not Found\r\n\r\n");
}

fn url_decode(s: &str) -> String {
    let mut result = String::with_capacity(s.len());
    let mut chars = s.chars();
    while let Some(ch) = chars.next() {
        if ch == '%' {
            let h1 = chars.next().unwrap_or('0');
            let h2 = chars.next().unwrap_or('0');
            if let Ok(byte) = u8::from_str_radix(&format!("{}{}", h1, h2), 16) {
                result.push(byte as char);
            }
        } else if ch == '+' {
            result.push(' ');
        } else {
            result.push(ch);
        }
    }
    result
}
