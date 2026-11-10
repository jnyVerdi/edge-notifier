# 🌟 Screen Glow Notifier (Edge Notifier)

A minimalist, ultra-lightweight ambient desktop notifier for Windows written in **Rust**.
It displays a glowing, pulsating gradient along the edges of your screen whenever a notification is triggered.

Designed especially for developers and users who keep laptop audio muted and need an eye-friendly, non-intrusive visual cue when tasks (such as **Antigravity AI** tasks, long builds, or test suites) finish.

---

## ✨ Features

- **Ultra-lightweight**: Built with native Win32 APIs (~5 MB RAM, 0% CPU when idle).
- **Click-through & Non-intrusive**: Clicks pass through transparently (`WS_EX_TRANSPARENT`), so it never interferes with typing, clicking, or gaming.
- **Smooth Gradient Pulse**: Hardware-accelerated alpha pulsing with customizable speed, opacity, and border thickness.
- **Multi-source Triggering**:
  - **Local HTTP API** (`POST /notify` or `GET /notify` on `http://127.0.0.1:4545/notify`).
  - **Windows Notification Listener** (captures OS toast notifications).
- **Customizable**: Configured via a simple `config.toml`.
- **System Tray Integration**: Run in the background with convenient tray options (Test Glow, Reload Config, Quit).

---

## 🚀 Quick Start

### 1. Build and Run
```bash
cargo build --release
cargo run --release
```

### 2. Triggering a Notification
Trigger the edge glow from anywhere (terminal, bash script, python, curl, agent hook):
```powershell
curl.exe http://127.0.0.1:4545/notify
```
Or customize on the fly:
```powershell
curl.exe "http://127.0.0.1:4545/notify?color=%2300FF88&duration=6&width=60"
```

---

## ⚙️ Configuration (`config.toml`)

```toml
[glow]
color = "#FF2222"         # Hex color (e.g. #FF0000, #00FF88, #3388FF)
border_width = 45         # Thickness of the edge gradient in pixels
pulse_speed = 1.8         # Pulsation speed (cycles per second)
duration_secs = 5.0       # Auto-dismiss duration in seconds (0 = pulse indefinitely until click)
max_opacity = 0.85        # Peak opacity during pulse (0.1 to 1.0)
min_opacity = 0.15        # Minimum opacity during pulse

[server]
port = 4545               # Local HTTP trigger port
enabled = true
```

---

## 🛠️ GitHub Repository Setup

To push this repository to your GitHub account:

```bash
# 1. Log in to GitHub CLI (one time only)
gh auth login

# 2. Create the GitHub repository and push
gh repo create edge-notifier --public --source=. --remote=origin --push
```

---

## 📄 License
MIT License
