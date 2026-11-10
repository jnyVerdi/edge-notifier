use serde::{Deserialize, Serialize};
use std::fs;
use std::path::Path;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GlowConfig {
    #[serde(default = "default_color")]
    pub color: String,

    #[serde(default = "default_border_width")]
    pub border_width: u32,

    #[serde(default = "default_pulse_speed")]
    pub pulse_speed: f32,

    #[serde(default = "default_duration_secs")]
    pub duration_secs: f32,

    #[serde(default = "default_max_opacity")]
    pub max_opacity: f32,

    #[serde(default = "default_min_opacity")]
    pub min_opacity: f32,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ServerConfig {
    #[serde(default = "default_server_enabled")]
    pub enabled: bool,

    #[serde(default = "default_port")]
    pub port: u16,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Config {
    #[serde(default)]
    pub glow: GlowConfig,

    #[serde(default)]
    pub server: ServerConfig,
}

fn default_color() -> String {
    "#FF2222".to_string()
}
fn default_border_width() -> u32 {
    80
}
fn default_pulse_speed() -> f32 {
    1.5
}
fn default_duration_secs() -> f32 {
    5.0
}
fn default_max_opacity() -> f32 {
    0.95
}
fn default_min_opacity() -> f32 {
    0.25
}
fn default_server_enabled() -> bool {
    true
}
fn default_port() -> u16 {
    4545
}

impl Default for GlowConfig {
    fn default() -> Self {
        Self {
            color: default_color(),
            border_width: default_border_width(),
            pulse_speed: default_pulse_speed(),
            duration_secs: default_duration_secs(),
            max_opacity: default_max_opacity(),
            min_opacity: default_min_opacity(),
        }
    }
}

impl Default for ServerConfig {
    fn default() -> Self {
        Self {
            enabled: default_server_enabled(),
            port: default_port(),
        }
    }
}

impl Default for Config {
    fn default() -> Self {
        Self {
            glow: GlowConfig::default(),
            server: ServerConfig::default(),
        }
    }
}

impl Config {
    pub fn load_or_create<P: AsRef<Path>>(path: P) -> Self {
        let path = path.as_ref();
        if path.exists() {
            if let Ok(content) = fs::read_to_string(path) {
                if let Ok(cfg) = toml::from_str::<Config>(&content) {
                    return cfg;
                }
            }
        }

        let default_cfg = Config::default();
        if let Ok(serialized) = toml::to_string_pretty(&default_cfg) {
            let _ = fs::write(path, serialized);
        }
        default_cfg
    }

    /// Parse HEX color string to (R, G, B)
    pub fn rgb(&self) -> (u8, u8, u8) {
        Self::parse_hex_color(&self.glow.color)
    }

    pub fn parse_hex_color(hex: &str) -> (u8, u8, u8) {
        let hex = hex.trim().trim_start_matches('#');
        if hex.len() == 6 {
            let r = u8::from_str_radix(&hex[0..2], 16).unwrap_or(255);
            let g = u8::from_str_radix(&hex[2..4], 16).unwrap_or(0);
            let b = u8::from_str_radix(&hex[4..6], 16).unwrap_or(0);
            (r, g, b)
        } else if hex.len() == 3 {
            let r = u8::from_str_radix(&hex[0..1].repeat(2), 16).unwrap_or(255);
            let g = u8::from_str_radix(&hex[1..2].repeat(2), 16).unwrap_or(0);
            let b = u8::from_str_radix(&hex[2..3].repeat(2), 16).unwrap_or(0);
            (r, g, b)
        } else {
            (255, 34, 34) // Fallback bright red
        }
    }
}
