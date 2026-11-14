use std::env;
use std::fs;
use std::io::{Cursor, Read, Write};
use std::net::{TcpListener, UdpSocket};
use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;
use std::thread;

use tungstenite::Message;

struct PrefixedStream<S> {
    prefix: Cursor<Vec<u8>>,
    stream: S,
}

impl<S: Read> Read for PrefixedStream<S> {
    fn read(&mut self, buf: &mut [u8]) -> std::io::Result<usize> {
        let n = self.prefix.read(buf)?;
        if n > 0 {
            Ok(n)
        } else {
            self.stream.read(buf)
        }
    }
}

impl<S: Write> Write for PrefixedStream<S> {
    fn write(&mut self, buf: &[u8]) -> std::io::Result<usize> {
        self.stream.write(buf)
    }
    fn flush(&mut self) -> std::io::Result<()> {
        self.stream.flush()
    }
}

fn vic_base_dir() -> PathBuf {
    let home = env::var("HOME")
        .or_else(|_| env::var("USERPROFILE"))
        .unwrap_or_else(|_| ".".to_string());
    PathBuf::from(home).join(".vic")
}

fn cmd_exists(cmd: &str) -> bool {
    let checker = if cfg!(target_os = "windows") { "where" } else { "which" };
    Command::new(checker)
        .arg(cmd)
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .status()
        .map(|s| s.success())
        .unwrap_or(false)
}

fn get_output(cmd: &str, args: &[&str]) -> String {
    Command::new(cmd)
        .args(args)
        .output()
        .map(|o| String::from_utf8_lossy(&o.stdout).trim().to_string())
        .unwrap_or_default()
}

fn get_ips() -> (String, String) {
    let lan_ip = UdpSocket::bind("0.0.0.0:0")
        .and_then(|s| {
            s.connect("8.8.8.8:80")?;
            s.local_addr()
        })
        .map(|addr| addr.ip().to_string())
        .unwrap_or_else(|_| "127.0.0.1".to_string());

    let ts_ip = if cmd_exists("tailscale") {
        let ip = get_output("tailscale", &["ip", "-4"]);
        if ip.is_empty() {
            "Offline".to_string()
        } else {
            ip
        }
    } else {
        "Not installed".to_string()
    };

    (lan_ip, ts_ip)
}

fn get_ydotool_cmd() -> Command {
    let mut cmd = Command::new("ydotool");
    if env::var("YDOTOOL_SOCKET").is_err() {
        if let Ok(runtime) = env::var("XDG_RUNTIME_DIR") {
            let p = Path::new(&runtime).join(".ydotool_socket");
            if p.exists() {
                cmd.env("YDOTOOL_SOCKET", p);
                return cmd;
            }
        }
        if Path::new("/run/user/1000/.ydotool_socket").exists() {
            cmd.env("YDOTOOL_SOCKET", "/run/user/1000/.ydotool_socket");
        }
    }
    cmd
}

fn mouse_move(dx: i32, dy: i32) {
    if cfg!(target_os = "linux") {
        let _ = get_ydotool_cmd()
            .args(["mousemove", "-x", &dx.to_string(), "-y", &dy.to_string()])
            .stdout(Stdio::null())
            .stderr(Stdio::null())
            .status();
    } else if cfg!(target_os = "windows") {
        let script = format!(
            "[void][System.Reflection.Assembly]::LoadWithPartialName('System.Windows.Forms'); \
             [System.Windows.Forms.Cursor]::Position = New-Object System.Drawing.Point(([System.Windows.Forms.Cursor]::Position.X + {}), ([System.Windows.Forms.Cursor]::Position.Y + {}))",
            dx, dy
        );
        let _ = Command::new("powershell")
            .args(["-NoProfile", "-Command", &script])
            .stdout(Stdio::null())
            .stderr(Stdio::null())
            .status();
    }
}

fn mouse_move_abs(x: i32, y: i32) {
    if cfg!(target_os = "linux") {
        let _ = get_ydotool_cmd()
            .args(["mousemove", "-a", "-x", &x.to_string(), "-y", &y.to_string()])
            .stdout(Stdio::null())
            .stderr(Stdio::null())
            .status();
    } else if cfg!(target_os = "windows") {
        let script = format!(
            "[void][System.Reflection.Assembly]::LoadWithPartialName('System.Windows.Forms'); \
             [System.Windows.Forms.Cursor]::Position = New-Object System.Drawing.Point({}, {})",
            x, y
        );
        let _ = Command::new("powershell")
            .args(["-NoProfile", "-Command", &script])
            .stdout(Stdio::null())
            .stderr(Stdio::null())
            .status();
    }
}

fn mouse_center() {
    if cfg!(target_os = "linux") {
        let _ = get_ydotool_cmd()
            .args(["mousemove", "-a", "-x", "683", "-y", "384"])
            .stdout(Stdio::null())
            .stderr(Stdio::null())
            .status();
    }
}

fn mouse_scroll(dy: i32) {
    if cfg!(target_os = "linux") {
        let _ = get_ydotool_cmd()
            .args(["mousemove", "-w", "-x", "0", "-y", &dy.to_string()])
            .stdout(Stdio::null())
            .stderr(Stdio::null())
            .status();
    }
}

fn mouse_click(btn: &str) {
    if cfg!(target_os = "linux") {
        let hex = match btn {
            "right" | "r" => "0xC1",
            "middle" | "m" => "0xC2",
            _ => "0xC0",
        };
        let _ = get_ydotool_cmd()
            .args(["click", hex])
            .stdout(Stdio::null())
            .stderr(Stdio::null())
            .status();
    }
}

fn mouse_down(btn: &str) {
    if cfg!(target_os = "linux") {
        let hex = match btn {
            "right" | "r" => "0x41",
            "middle" | "m" => "0x42",
            _ => "0x40",
        };
        let _ = get_ydotool_cmd()
            .args(["click", hex])
            .stdout(Stdio::null())
            .stderr(Stdio::null())
            .status();
    }
}

fn mouse_up(btn: &str) {
    if cfg!(target_os = "linux") {
        let hex = match btn {
            "right" | "r" => "0x81",
            "middle" | "m" => "0x82",
            _ => "0x80",
        };
        let _ = get_ydotool_cmd()
            .args(["click", hex])
            .stdout(Stdio::null())
            .stderr(Stdio::null())
            .status();
    }
}

fn key_press(k: &str) {
    if cfg!(target_os = "linux") {
        let code = match k {
            "esc" | "escape" => "1:1 1:0",
            "enter" | "return" => "28:1 28:0",
            "space" => "57:1 57:0",
            "backspace" => "14:1 14:0",
            "f5" => "63:1 63:0",
            _ => "",
        };
        if !code.is_empty() {
            let mut args = vec!["key"];
            args.extend(code.split_whitespace());
            let _ = get_ydotool_cmd()
                .args(&args)
                .stdout(Stdio::null())
                .stderr(Stdio::null())
                .status();
        }
    }
}

fn handle_command(cmd: &str) {
    let parts: Vec<&str> = cmd.trim().split_whitespace().collect();
    if parts.is_empty() {
        return;
    }
    match parts[0] {
        "m" => {
            if parts.len() > 1 {
                let coords: Vec<&str> = if parts[1].contains(';') {
                    parts[1].split(';').collect()
                } else if parts.len() > 2 {
                    vec![parts[1], parts[2]]
                } else {
                    vec![]
                };
                if coords.len() >= 2 {
                    if let (Ok(dx), Ok(dy)) = (coords[0].parse::<i32>(), coords[1].parse::<i32>()) {
                        mouse_move(dx, dy);
                    }
                }
            }
        }
        "ma" => {
            if parts.len() > 1 {
                let coords: Vec<&str> = if parts[1].contains(';') {
                    parts[1].split(';').collect()
                } else if parts.len() > 2 {
                    vec![parts[1], parts[2]]
                } else {
                    vec![]
                };
                if coords.len() >= 2 {
                    if let (Ok(x), Ok(y)) = (coords[0].parse::<i32>(), coords[1].parse::<i32>()) {
                        mouse_move_abs(x, y);
                    }
                }
            }
        }
        "center" => {
            mouse_center();
        }
        "s" | "scroll" => {
            let dy = parts.get(1).and_then(|s| s.parse::<i32>().ok()).unwrap_or(0);
            if dy != 0 {
                mouse_scroll(dy);
            }
        }
        "click" => {
            let btn = parts.get(1).unwrap_or(&"left");
            mouse_click(btn);
        }
        "down" => {
            let btn = parts.get(1).unwrap_or(&"left");
            mouse_down(btn);
        }
        "up" => {
            let btn = parts.get(1).unwrap_or(&"left");
            mouse_up(btn);
        }
        "key" => {
            let k = parts.get(1).unwrap_or(&"");
            key_press(k);
        }
        _ => {}
    }
}

const HTML_CONTENT: &str = r#"<!DOCTYPE html>
<html lang="vi">
<head>
<meta charset="utf-8">
<meta name="viewport" content="width=device-width, initial-scale=1.0, maximum-scale=1.0, user-scalable=no, viewport-fit=cover">
<title>VIC Air Mouse & Trackpad</title>
<style>
* {
    box-sizing: border-box;
    margin: 0;
    padding: 0;
    -webkit-tap-highlight-color: transparent;
    -webkit-touch-callout: none;
    -webkit-user-select: none;
    user-select: none;
}
html, body {
    width: 100%;
    height: 100%;
    overflow: hidden;
    background-color: #0c0d12;
    color: #e2e8f0;
    font-family: -apple-system, BlinkMacSystemFont, "Segoe UI", Roboto, Helvetica, Arial, sans-serif;
    touch-action: none;
}
#app {
    display: flex;
    flex-direction: column;
    width: 100vw;
    height: 100vh;
    height: 100dvh;
    padding-top: env(safe-area-inset-top, 0px);
    padding-bottom: env(safe-area-inset-bottom, 0px);
}
header {
    display: flex;
    justify-content: space-between;
    align-items: center;
    padding: 8px 14px;
    background: #111522;
    border-bottom: 1px solid rgba(255, 255, 255, 0.08);
    font-size: 13px;
    z-index: 10;
}
.brand {
    font-weight: 700;
    letter-spacing: 0.5px;
    display: flex;
    align-items: center;
    gap: 8px;
}
.status-pill {
    display: inline-block;
    width: 8px;
    height: 8px;
    border-radius: 50%;
    background: #ef4444;
    box-shadow: 0 0 6px #ef4444;
}
.status-pill.online {
    background: #10b981;
    box-shadow: 0 0 6px #10b981;
}
.header-actions {
    display: flex;
    align-items: center;
    gap: 6px;
}
.btn-tag {
    background: rgba(255, 255, 255, 0.06);
    border: 1px solid rgba(255, 255, 255, 0.12);
    color: #cbd5e1;
    padding: 4px 8px;
    border-radius: 6px;
    font-size: 11px;
    cursor: pointer;
    font-weight: 600;
}
.btn-tag:active { background: rgba(255, 255, 255, 0.15); }
.btn-tag.active {
    background: #3b82f6;
    color: #fff;
    border-color: #60a5fa;
}

#mode-selector {
    display: flex;
    background: #090b10;
    padding: 4px;
    gap: 4px;
    border-bottom: 1px solid rgba(255, 255, 255, 0.06);
}
.mode-tab {
    flex: 1;
    padding: 8px 0;
    text-align: center;
    font-size: 12px;
    font-weight: 600;
    background: transparent;
    border: none;
    color: #94a3b8;
    border-radius: 6px;
    cursor: pointer;
    transition: all 0.2s ease;
}
.mode-tab.active {
    background: #1e293b;
    color: #38bdf8;
    box-shadow: 0 2px 6px rgba(0,0,0,0.3);
}

.mode-pane {
    display: none !important;
    flex: 1;
    position: relative;
    overflow: hidden;
}
.mode-pane.active {
    display: flex !important;
}

/* DANH SÁCH THẢ CÀI ĐẶT */
#settings-dropdown {
    display: none;
    background: #0b0f1a;
    border-bottom: 1px solid rgba(255, 255, 255, 0.1);
    padding: 12px 14px;
    font-size: 12px;
    z-index: 100;
}
.setting-row {
    margin-bottom: 10px;
}
.setting-title {
    font-weight: 600;
    color: #cbd5e1;
    margin-bottom: 6px;
    display: flex;
    justify-content: space-between;
    align-items: center;
}

/* CHẾ ĐỘ 1: CHUỘT BAY AIR MOUSE */
#pane-air {
    flex-direction: row;
    padding: 8px;
    gap: 8px;
    width: 100%;
    height: 100%;
    box-sizing: border-box;
    overflow: hidden;
}
#col-func {
    flex: 2.2;
    display: flex;
    flex-direction: column;
    gap: 8px;
    height: 100%;
    min-width: 0;
}
#col-check {
    flex: 1;
    background: #0a0d18;
    border: 1px solid rgba(255, 255, 255, 0.08);
    border-radius: 12px;
    padding: 8px;
    display: flex;
    flex-direction: column;
    gap: 6px;
    overflow-y: auto;
    -webkit-overflow-scrolling: touch;
    min-width: 120px;
}
.check-card {
    background: rgba(255, 255, 255, 0.03);
    border: 1px solid rgba(255, 255, 255, 0.07);
    border-radius: 8px;
    padding: 6px 8px;
    display: flex;
    align-items: center;
    gap: 8px;
    cursor: pointer;
    transition: all 0.25s ease;
}
.check-card:active {
    background: rgba(255, 255, 255, 0.08);
}
.check-card.ok {
    border-color: rgba(16, 185, 129, 0.4);
    background: rgba(16, 185, 129, 0.08);
}
.check-led {
    width: 10px;
    height: 10px;
    border-radius: 50%;
    background: #475569;
    flex-shrink: 0;
    transition: all 0.3s ease;
}
/* SÁNG LÊN KHI ĐIỀU KIỆN OK */
.check-card.ok .check-led {
    background: #10b981;
    box-shadow: 0 0 8px #10b981, 0 0 16px rgba(16, 185, 129, 0.6);
}
.check-text {
    overflow: hidden;
    flex: 1;
}
.check-name {
    font-size: 11px;
    font-weight: 700;
    color: #94a3b8;
    white-space: nowrap;
    text-overflow: ellipsis;
    overflow: hidden;
}
.check-card.ok .check-name {
    color: #34d399;
}
.check-sub {
    font-size: 9px;
    color: #64748b;
    white-space: nowrap;
    text-overflow: ellipsis;
    overflow: hidden;
}
.check-card.ok .check-sub {
    color: #a7f3d0;
}
#air-aim-box {
    flex: 1;
    width: 100%;
    background: radial-gradient(circle at center, #161e33 0%, #0c0f1a 100%);
    border: 2px dashed rgba(56, 189, 248, 0.3);
    border-radius: 14px;
    display: flex;
    flex-direction: column;
    align-items: center;
    justify-content: center;
    text-align: center;
    padding: 12px;
    cursor: pointer;
    position: relative;
    transition: all 0.15s ease;
}
#air-aim-box:active, #air-aim-box.active {
    background: radial-gradient(circle at center, #1e3a8a 0%, #0f172a 100%);
    border-color: #38bdf8;
    box-shadow: 0 0 24px rgba(56, 189, 248, 0.4);
    transform: scale(0.99);
}
.aim-crosshair {
    width: 50px;
    height: 50px;
    border: 2px solid #38bdf8;
    border-radius: 50%;
    position: relative;
    margin-bottom: 8px;
    opacity: 0.7;
    transition: all 0.2s ease;
}
#air-aim-box.active .aim-crosshair {
    opacity: 1;
    transform: scale(1.15);
    border-color: #10b981;
    box-shadow: 0 0 16px rgba(16, 185, 129, 0.6);
}
.aim-crosshair::before, .aim-crosshair::after {
    content: "";
    position: absolute;
    background: #38bdf8;
}
.aim-crosshair::before { width: 100%; height: 2px; top: 50%; left: 0; transform: translateY(-50%); }
.aim-crosshair::after { width: 2px; height: 100%; left: 50%; top: 0; transform: translateX(-50%); }
#air-aim-box.active .aim-crosshair::before,
#air-aim-box.active .aim-crosshair::after { background: #10b981; }

.aim-title {
    font-size: 14px;
    font-weight: 700;
    color: #f8fafc;
    letter-spacing: 0.5px;
    margin-bottom: 4px;
}
.aim-desc {
    font-size: 11px;
    color: #94a3b8;
    line-height: 1.35;
    max-width: 220px;
}
.aim-sub-bar {
    display: flex;
    width: 100%;
    gap: 6px;
}
.aim-toggle-btn {
    flex: 1;
    background: rgba(255, 255, 255, 0.05);
    border: 1px solid rgba(255, 255, 255, 0.1);
    color: #cbd5e1;
    padding: 8px 0;
    font-size: 11px;
    font-weight: 600;
    border-radius: 8px;
    text-align: center;
    cursor: pointer;
}
.aim-toggle-btn.active {
    background: #059669;
    border-color: #10b981;
    color: white;
}

/* CHẾ ĐỘ 2: BÀN DI TOUCHPAD */
#pane-touch {
    flex-direction: row;
}
#trackpad {
    flex: 1;
    height: 100%;
    background: radial-gradient(circle at center, #141724 0%, #0a0c12 100%);
    position: relative;
    cursor: grab;
    display: flex;
    flex-direction: column;
    align-items: center;
    justify-content: center;
}
#trackpad:active { cursor: grabbing; }
.hint-text {
    font-size: 12px;
    color: rgba(255, 255, 255, 0.25);
    pointer-events: none;
    text-align: center;
    line-height: 1.6;
}
#scroll-strip {
    width: 50px;
    height: 100%;
    background: rgba(20, 24, 36, 0.95);
    border-left: 1px solid rgba(255, 255, 255, 0.08);
    display: flex;
    flex-direction: column;
    align-items: center;
    justify-content: center;
    color: rgba(255, 255, 255, 0.4);
    font-size: 11px;
    font-weight: 700;
    letter-spacing: 1px;
}

/* KHUNG NÚT DƯỚI CỐ ĐỊNH */
#bottom-bar {
    display: flex;
    height: 80px;
    background: #0f131f;
    border-top: 1px solid rgba(255, 255, 255, 0.08);
    gap: 8px;
    padding: 8px 12px;
}
.mouse-btn {
    flex: 1;
    border: 1px solid rgba(255, 255, 255, 0.1);
    background: rgba(30, 36, 52, 0.7);
    color: #e2e8f0;
    font-size: 14px;
    font-weight: 700;
    border-radius: 10px;
    display: flex;
    align-items: center;
    justify-content: center;
    transition: all 0.05s ease;
}
.mouse-btn:active, .mouse-btn.active {
    background: #3b82f6;
    color: #ffffff;
    border-color: #60a5fa;
    transform: scale(0.98);
}
.mouse-btn.primary { flex: 1.3; }

#extra-bar {
    display: flex;
    gap: 6px;
    padding: 6px 12px;
    background: #07090e;
    border-top: 1px solid rgba(255, 255, 255, 0.05);
}
.extra-btn {
    flex: 1;
    background: rgba(255, 255, 255, 0.05);
    border: 1px solid rgba(255, 255, 255, 0.08);
    color: #94a3b8;
    padding: 6px 0;
    font-size: 11px;
    font-weight: 600;
    border-radius: 6px;
    text-align: center;
}
.extra-btn:active {
    background: rgba(255, 255, 255, 0.15);
    color: #fff;
}
</style>
</head>
<body>
<div id="app">
    <header>
        <div class="brand">
            <span class="status-pill" id="status-dot"></span>
            <span>VIC Air Mouse</span>
        </div>
        <div class="header-actions">
            <button class="btn-tag" id="center-btn" onclick="send('center')">Giữa màn hình</button>
            <button class="btn-tag" id="fs-btn">Toàn màn hình</button>
            <button class="btn-tag active" id="btn-settings-toggle" onclick="toggleSettingsMenu()">Cài đặt [v]</button>
        </div>
    </header>

    <!-- DANH SÁCH THẢ (SETTINGS DROPDOWN) -->
    <div id="settings-dropdown">
        <!-- THANH KÉO ĐỘ NHẠY VÀ NÚT MẶC ĐỊNH -->
        <div class="setting-row">
            <div class="setting-title">
                <span>Độ nhạy con trỏ: <b id="sens-val" style="color:#38bdf8;">1.8x</b></span>
                <button class="btn-tag" onclick="resetSensDefault()">Mặc định</button>
            </div>
            <input type="range" id="sens-slider" min="0.5" max="4.0" step="0.1" value="1.8" style="width:100%;" oninput="setSensitivity(this.value)">
        </div>

        <!-- ĐẢO NGƯỢC TRỤC XY -->
        <div class="setting-row" style="padding-top:8px;border-top:1px solid rgba(255,255,255,0.06);">
            <div class="setting-title">Đảo ngược trục di chuyển:</div>
            <div style="display:flex;gap:18px;">
                <label style="display:flex;align-items:center;gap:6px;cursor:pointer;color:#cbd5e1;">
                    <input type="checkbox" id="chk-inv-x" onchange="toggleInvertX(this.checked)">
                    <span>Đảo trục X</span>
                </label>
                <label style="display:flex;align-items:center;gap:6px;cursor:pointer;color:#cbd5e1;">
                    <input type="checkbox" id="chk-inv-y" onchange="toggleInvertY(this.checked)">
                    <span>Đảo trục Y</span>
                </label>
            </div>
        </div>

        <!-- CHẾ ĐỘ CUỘN CHUỘT -->
        <div style="padding-top:8px;border-top:1px solid rgba(255,255,255,0.06);display:flex;justify-content:space-between;align-items:center;">
            <span style="font-weight:600;color:#cbd5e1;">Chế độ cuộn chuột:</span>
            <select id="scroll-mode-select" onchange="setScrollMode(this.value)" style="background:#1e293b;color:#38bdf8;border:1px solid #334155;padding:4px 8px;border-radius:6px;font-size:12px;font-weight:600;outline:none;">
                <option value="normal">Cuộn Chuẩn (Classic)</option>
                <option value="natural">Cuộn Tự nhiên (Natural)</option>
            </select>
        </div>
    </div>

    <div id="mode-selector">
        <button class="mode-tab active" data-pane="air" onclick="switchMode('air')">Chuột Bay (Air Mouse)</button>
        <button class="mode-tab" data-pane="touch" onclick="switchMode('touch')">Bàn Di (Trackpad)</button>
    </div>

    <!-- PANE 1: CHUỘT BAY (AIR MOUSE) -->
    <div id="pane-air" class="mode-pane active">
        <!-- CỘT 1: DANH MỤC CHỨC NĂNG (TO HƠN) -->
        <div id="col-func">
            <div id="air-aim-box">
                <div class="aim-crosshair"></div>
                <div class="aim-title" id="aim-status-text">CHẠM GIỮ ĐỂ LIA CHUỘT</div>
                <div class="aim-desc">Vuốt ngón tay hoặc lia cổ tay để di chuyển con trỏ</div>
                <button id="btn-request-perm" style="display:none;margin-top:10px;padding:6px 12px;background:#3b82f6;border:none;border-radius:6px;color:white;font-weight:600;font-size:11px;" onclick="requestSensorPermission()">Cấp quyền con quay (iOS)</button>
            </div>
            <div class="aim-sub-bar">
                <button class="aim-toggle-btn" id="btn-toggle-aim" onclick="toggleAlwaysAim()">Lia liên tục: TẮT</button>
                <button class="aim-toggle-btn" onclick="send('center');vibrate(10);">Về giữa</button>
            </div>
        </div>

        <!-- CỘT 2: DANH MỤC CHECK ĐIỀU KIỆN (NHỎ HƠN, CẠNH NHAU) -->
        <div id="col-check">
            <div style="font-size:10px;font-weight:700;color:#94a3b8;font-family:monospace;letter-spacing:0.5px;text-align:center;border-bottom:1px solid rgba(255,255,255,0.08);padding-bottom:4px;">
                ĐIỀU KIỆN
            </div>

            <div class="check-card" id="check-api" onclick="showCheckTip('api')">
                <div class="check-led" id="led-api"></div>
                <div class="check-text">
                    <div class="check-name">API Sensor</div>
                    <div class="check-sub" id="sub-api">Đang kiểm tra</div>
                </div>
            </div>

            <div class="check-card" id="check-secure" onclick="showCheckTip('secure')">
                <div class="check-led" id="led-secure"></div>
                <div class="check-text">
                    <div class="check-name">HTTPS / Cờ</div>
                    <div class="check-sub" id="sub-secure">Đang kiểm tra</div>
                </div>
            </div>

            <div class="check-card" id="check-perm" onclick="showCheckTip('perm')">
                <div class="check-led" id="led-perm"></div>
                <div class="check-text">
                    <div class="check-name">Quyền Sensor</div>
                    <div class="check-sub" id="sub-perm">Đang kiểm tra</div>
                </div>
            </div>

            <div class="check-card" id="check-gyro" onclick="showCheckTip('gyro')">
                <div class="check-led" id="led-gyro"></div>
                <div class="check-text">
                    <div class="check-name">Tín hiệu Gyro</div>
                    <div class="check-sub" id="sub-gyro">Đang đợi...</div>
                </div>
            </div>

            <div id="check-tip-box" style="margin-top:auto;font-size:10px;color:#94a3b8;line-height:1.35;background:rgba(255,255,255,0.03);padding:6px;border-radius:6px;border:1px dashed rgba(255,255,255,0.1);">
                Nhấn vào từng mục để xem chi tiết
            </div>
        </div>
    </div>

    <!-- PANE 2: BÀN DI TOUCHPAD -->
    <div id="pane-touch" class="mode-pane">
        <div id="trackpad">
            <div class="hint-text">
                1 ngón vuốt: Di chuyển con trỏ<br>
                1 ngón chạm nhẹ: Chuột Trái<br>
                2 ngón chạm nhẹ: Chuột Phải<br>
                2 ngón vuốt: Cuộn trang
            </div>
        </div>
        <div id="scroll-strip" title="Vuốt dọc để cuộn">
            CUỘN
        </div>
    </div>

    <!-- PHÍM NHANH HỆ THỐNG -->
    <div id="extra-bar">
        <button class="extra-btn" onclick="sendKey('space')">Space (Play)</button>
        <button class="extra-btn" onclick="sendKey('f5')">F5 (Chiếu)</button>
        <button class="extra-btn" onclick="sendKey('enter')">Enter</button>
        <button class="extra-btn" onclick="sendKey('esc')">Esc</button>
    </div>

    <!-- NÚT CHUỘT DƯỚI -->
    <div id="bottom-bar">
        <button class="mouse-btn primary" id="btn-left">CHUỘT TRÁI</button>
        <button class="mouse-btn" id="btn-right">CHUỘT PHẢI</button>
    </div>
</div>

<script>
let ws = null;
const statusDot = document.getElementById("status-dot");
const trackpad = document.getElementById("trackpad");
const scrollStrip = document.getElementById("scroll-strip");
const btnLeft = document.getElementById("btn-left");
const btnRight = document.getElementById("btn-right");
const fsBtn = document.getElementById("fs-btn");
const airAimBox = document.getElementById("air-aim-box");
const aimStatusText = document.getElementById("aim-status-text");
const btnToggleAim = document.getElementById("btn-toggle-aim");
const btnRequestPerm = document.getElementById("btn-request-perm");

// --- CÀI ĐẶT & DANH SÁCH THẢ ---
let sensitivity = 1.8;
let invertX = false;
let invertY = false;
let scrollMode = 'normal'; // 'normal' | 'natural'

function toggleSettingsMenu() {
    const el = document.getElementById("settings-dropdown");
    if (el) {
        el.style.display = (el.style.display === "none" || el.style.display === "") ? "block" : "none";
    }
}

function setSensitivity(val) {
    sensitivity = parseFloat(val);
    const valEl = document.getElementById("sens-val");
    if (valEl) valEl.textContent = sensitivity.toFixed(1) + "x";
    try { localStorage.vic_mouse_sens = sensitivity; } catch (_) {}
}

function resetSensDefault() {
    setSensitivity(1.8);
    const slider = document.getElementById("sens-slider");
    if (slider) slider.value = 1.8;
    vibrate(10);
}

function toggleInvertX(checked) {
    invertX = checked;
    try { localStorage.vic_mouse_inv_x = checked ? "1" : "0"; } catch (_) {}
}

function toggleInvertY(checked) {
    invertY = checked;
    try { localStorage.vic_mouse_inv_y = checked ? "1" : "0"; } catch (_) {}
}

function setScrollMode(mode) {
    scrollMode = mode;
    try { localStorage.vic_mouse_scroll = mode; } catch (_) {}
}

// Khôi phục cài đặt đã lưu
try {
    if (localStorage.vic_mouse_sens) {
        setSensitivity(localStorage.vic_mouse_sens);
        const slider = document.getElementById("sens-slider");
        if (slider) slider.value = sensitivity;
    }
    if (localStorage.vic_mouse_inv_x === "1") {
        invertX = true;
        const chk = document.getElementById("chk-inv-x");
        if (chk) chk.checked = true;
    }
    if (localStorage.vic_mouse_inv_y === "1") {
        invertY = true;
        const chk = document.getElementById("chk-inv-y");
        if (chk) chk.checked = true;
    }
    if (localStorage.vic_mouse_scroll) {
        scrollMode = localStorage.vic_mouse_scroll;
        const sel = document.getElementById("scroll-mode-select");
        if (sel) sel.value = scrollMode;
    }
} catch (_) {}

function applyTransform(x, y) {
    let tx = invertX ? -x : x;
    let ty = invertY ? -y : y;
    return { x: tx, y: ty };
}

// --- CHECKLIST ĐIỀU KIỆN SỬ DỤNG AIR MOUSE ---
function markCheck(id, ok, subText) {
    const card = document.getElementById("check-" + id);
    const sub = document.getElementById("sub-" + id);
    if (card) {
        if (ok) card.classList.add("ok");
        else card.classList.remove("ok");
    }
    if (sub && subText) sub.textContent = subText;
}

function showCheckTip(id) {
    const tip = document.getElementById("check-tip-box");
    if (!tip) return;
    if (id === 'api') {
        tip.innerHTML = "<b>API Sensor:</b> Kiểm tra trình duyệt có hỗ trợ DeviceMotionEvent / DeviceOrientationEvent.";
    } else if (id === 'secure') {
        tip.innerHTML = "<b>HTTPS / Cờ:</b> Chrome chặn cảm biến trên HTTP thường.<br>Để mở: vào <code>chrome://flags</code> &gt; tìm <b>Insecure origins treated as secure</b> &gt; thêm URL máy tính &gt; Relaunch.";
    } else if (id === 'perm') {
        tip.innerHTML = "<b>Quyền Sensor:</b> Trên iOS cần bấm vào đây để cấp quyền truy cập con quay.";
        requestSensorPermission();
    } else if (id === 'gyro') {
        tip.innerHTML = "<b>Tín hiệu Gyro:</b> Luồng dữ liệu vận tốc góc đang truyền về máy tính.";
    }
}

fsBtn.addEventListener("click", () => {
    if (!document.fullscreenElement) {
        document.documentElement.requestFullscreen().catch(() => {});
    } else {
        document.exitFullscreen().catch(() => {});
    }
});

function switchMode(mode) {
    document.querySelectorAll(".mode-tab").forEach(t => {
        if (t.getAttribute("data-pane") === mode) t.classList.add("active");
        else t.classList.remove("active");
    });
    document.querySelectorAll(".mode-pane").forEach(p => {
        if (p.id === "pane-" + mode) p.classList.add("active");
        else p.classList.remove("active");
    });
    if (mode === "air") {
        checkSensorSupport();
    }
}

function connect() {
    const proto = location.protocol === "https:" ? "wss:" : "ws:";
    const pin = new URLSearchParams(location.search).get("pin") || "";
    const url = `${proto}//${location.host}/ws` + (pin ? `?pin=${encodeURIComponent(pin)}` : "");
    ws = new WebSocket(url);

    ws.onopen = () => {
        statusDot.classList.add("online");
    };

    ws.onclose = () => {
        statusDot.classList.remove("online");
        setTimeout(connect, 1000);
    };

    ws.onerror = () => {
        ws.close();
    };
}
connect();

function send(msg) {
    if (ws && ws.readyState === WebSocket.OPEN) {
        ws.send(msg);
    }
}

function sendKey(k) {
    send(`key ${k}`);
    vibrate(10);
}

function vibrate(ms) {
    if (navigator.vibrate) {
        try { navigator.vibrate(ms); } catch (e) {}
    }
}

// --- AIR MOUSE SENSOR ENGINE ---
let isAiming = false;
let alwaysAim = false;
let sensorListening = false;
let smoothX = 0;
let smoothY = 0;
let lastMotionSend = 0;
const deadzone = 0.4;
let receivedSensorData = false;

function checkSensorSupport() {
    const hasApi = (typeof DeviceMotionEvent !== "undefined" || typeof DeviceOrientationEvent !== "undefined");
    markCheck("api", hasApi, hasApi ? "Hỗ trợ [OK]" : "Không hỗ trợ");

    const isSecure = window.isSecureContext || location.hostname === "localhost" || location.hostname === "127.0.0.1";
    markCheck("secure", isSecure, isSecure ? "Bảo mật [OK]" : "Chặn HTTP");

    if (typeof DeviceMotionEvent !== "undefined" && typeof DeviceMotionEvent.requestPermission === "function") {
        btnRequestPerm.style.display = "inline-block";
        markCheck("perm", false, "Cần cấp quyền");
    } else {
        markCheck("perm", true, "Tự động [OK]");
        startMotionListener();
    }
}

function requestSensorPermission() {
    if (typeof DeviceMotionEvent !== "undefined" && typeof DeviceMotionEvent.requestPermission === "function") {
        DeviceMotionEvent.requestPermission().then(state => {
            if (state === "granted") {
                btnRequestPerm.style.display = "none";
                markCheck("perm", true, "Đã cấp [OK]");
                startMotionListener();
                vibrate(20);
            } else {
                markCheck("perm", false, "Bị từ chối");
                alert("Cần cấp quyền Motion Sensor để dùng tính năng Chuột Bay.");
            }
        }).catch(err => {
            console.error(err);
        });
    }
}

function startMotionListener() {
    if (sensorListening) return;
    if (window.DeviceMotionEvent) {
        window.addEventListener("devicemotion", handleDeviceMotion, { passive: true });
        sensorListening = true;
    }
    if (window.DeviceOrientationEvent) {
        window.addEventListener("deviceorientation", handleDeviceOrientation, { passive: true });
    }
}

let lastOriTime = 0;
let lastOriGamma = null;
let lastOriBeta = null;

function handleDeviceOrientation(e) {
    if (receivedSensorData) return;
    if (e.gamma === null || e.beta === null) return;
    const now = performance.now();
    if (lastOriTime === 0) {
        lastOriTime = now;
        lastOriGamma = e.gamma;
        lastOriBeta = e.beta;
        return;
    }
    const dt = (now - lastOriTime) / 1000.0;
    if (dt < 0.016) return;

    let dGamma = e.gamma - lastOriGamma;
    let dBeta = e.beta - lastOriBeta;
    lastOriGamma = e.gamma;
    lastOriBeta = e.beta;
    lastOriTime = now;

    if (Math.abs(dGamma) > 40 || Math.abs(dBeta) > 40) return;
    processMotionInput(-dGamma * 2.2, -dBeta * 2.2);
}

function handleDeviceMotion(e) {
    let rx = 0;
    let ry = 0;
    let hasData = false;

    if (e.rotationRate && e.rotationRate.gamma !== null && e.rotationRate.beta !== null) {
        rx = -e.rotationRate.gamma;
        ry = -e.rotationRate.beta;
        hasData = true;
    } else if (e.accelerationIncludingGravity && e.accelerationIncludingGravity.x !== null) {
        rx = -e.accelerationIncludingGravity.x * 2.0;
        ry = e.accelerationIncludingGravity.y * 2.0;
        hasData = true;
    }

    if (hasData) {
        receivedSensorData = true;
        processMotionInput(rx, ry);
    }
}

function processMotionInput(rawRx, rawRy) {
    markCheck("gyro", true, "Đang nhận [OK]");
    markCheck("secure", true, "Cờ đã bật [OK]");

    if (!isAiming && !alwaysAim) return;

    const tf = applyTransform(rawRx, rawRy);
    let rx = tf.x;
    let ry = tf.y;

    if (Math.abs(rx) < deadzone) rx = 0;
    if (Math.abs(ry) < deadzone) ry = 0;

    smoothX = smoothX * 0.25 + rx * 0.75;
    smoothY = smoothY * 0.25 + ry * 0.75;

    const now = performance.now();
    if (now - lastMotionSend >= 16) {
        const speed = Math.hypot(smoothX, smoothY);
        let accel = 1.0;
        if (speed > 12) {
            accel = 1.0 + Math.min(2.5, (speed - 12) / 25.0);
        }

        const dx = Math.round(smoothX * sensitivity * accel);
        const dy = Math.round(smoothY * sensitivity * accel);

        if (dx !== 0 || dy !== 0) {
            send(`m ${dx};${dy}`);
        }
        lastMotionSend = now;
    }
}


function setAimActive(active) {
    isAiming = active;
    if (active) {
        airAimBox.classList.add("active");
        aimStatusText.textContent = "ĐANG LIA CHUỘT (AIM ACTIVE)";
        vibrate(15);
        startMotionListener();
    } else {
        if (!alwaysAim) {
            airAimBox.classList.remove("active");
            aimStatusText.textContent = "CHẠM GIỮ ĐỂ LIA CHUỘT";
        }
    }
}

function toggleAlwaysAim() {
    alwaysAim = !alwaysAim;
    if (alwaysAim) {
        btnToggleAim.classList.add("active");
        btnToggleAim.textContent = "Lia liên tục: BẬT";
        airAimBox.classList.add("active");
        aimStatusText.textContent = "LIA LIÊN TỤC ĐANG BẬT";
        startMotionListener();
        vibrate(25);
    } else {
        btnToggleAim.classList.remove("active");
        btnToggleAim.textContent = "Lia liên tục: TẮT";
        if (!isAiming) {
            airAimBox.classList.remove("active");
            aimStatusText.textContent = "CHẠM GIỮ ĐỂ LIA CHUỘT";
        }
        vibrate(10);
    }
}

// Touch lia truc tiep tren Aim Box
let aimTouchLastX = 0;
let aimTouchLastY = 0;
let aimTouching = false;

airAimBox.addEventListener("touchstart", (e) => {
    e.preventDefault();
    aimTouching = true;
    if (e.touches.length > 0) {
        aimTouchLastX = e.touches[0].pageX;
        aimTouchLastY = e.touches[0].pageY;
    }
    setAimActive(true);
}, { passive: false });

airAimBox.addEventListener("touchmove", (e) => {
    e.preventDefault();
    if (!aimTouching || e.touches.length === 0) return;
    const cx = e.touches[0].pageX;
    const cy = e.touches[0].pageY;
    const rawDx = cx - aimTouchLastX;
    const rawDy = cy - aimTouchLastY;
    aimTouchLastX = cx;
    aimTouchLastY = cy;

    const tf = applyTransform(rawDx, rawDy);
    const sendX = Math.round(tf.x * sensitivity * 1.5);
    const sendY = Math.round(tf.y * sensitivity * 1.5);
    if (sendX !== 0 || sendY !== 0) {
        send(`m ${sendX};${sendY}`);
    }
}, { passive: false });

airAimBox.addEventListener("touchend", (e) => {
    e.preventDefault();
    aimTouching = false;
    setAimActive(false);
}, { passive: false });

airAimBox.addEventListener("touchcancel", (e) => {
    aimTouching = false;
    setAimActive(false);
});

// Mouse fallback tren PC/browser test
airAimBox.addEventListener("mousedown", (e) => {
    e.preventDefault();
    setAimActive(true);
});
window.addEventListener("mouseup", () => {
    if (isAiming) setAimActive(false);
});

// Khoi dong cam bien
checkSensorSupport();

// --- TRACKPAD TOUCH ENGINE ---
function calcAccel(speed) {
    if (speed < 120) return 1.0;
    if (speed < 600) return 1.0 + (speed - 120) / 480.0 * 1.1;
    return 2.2;
}

let ongoingTouches = {};
let touchStartTime = 0;
let touchMoved = false;
let maxFingerCount = 0;
let dragMode = false;
let lastTapReleaseTime = 0;
let padScrollAcc = 0;

trackpad.addEventListener("touchstart", (e) => {
    e.preventDefault();
    const now = Date.now();
    if (e.touches.length === 1) {
        touchStartTime = now;
        touchMoved = false;
        maxFingerCount = 1;

        if (now - lastTapReleaseTime < 260) {
            dragMode = true;
            send("down left");
            vibrate(15);
        }
    } else {
        maxFingerCount = Math.max(maxFingerCount, e.touches.length);
    }

    for (let i = 0; i < e.changedTouches.length; i++) {
        const t = e.changedTouches[i];
        ongoingTouches[t.identifier] = {
            x: t.pageX,
            y: t.pageY,
            time: now
        };
    }
}, { passive: false });

trackpad.addEventListener("touchmove", (e) => {
    e.preventDefault();
    const now = Date.now();
    const touches = e.touches;
    maxFingerCount = Math.max(maxFingerCount, touches.length);

    if (touches.length === 1) {
        const t = touches[0];
        const prev = ongoingTouches[t.identifier];
        if (prev) {
            const rawDx = t.pageX - prev.x;
            const rawDy = t.pageY - prev.y;
            const dist = Math.hypot(rawDx, rawDy);

            if (dist > 3) {
                touchMoved = true;
            }

            const dt = Math.max(1, now - prev.time);
            const speed = (dist / dt) * 1000;
            const mult = calcAccel(speed) * sensitivity;

            const tf = applyTransform(rawDx, rawDy);
            const sendX = Math.round(tf.x * mult);
            const sendY = Math.round(tf.y * mult);
            if (sendX !== 0 || sendY !== 0) {
                send(`m ${sendX};${sendY}`);
            }
            ongoingTouches[t.identifier] = { x: t.pageX, y: t.pageY, time: now };
        }
    } else if (touches.length === 2) {
        touchMoved = true;
        const t0 = touches[0];
        const t1 = touches[1];
        const prev0 = ongoingTouches[t0.identifier];
        const prev1 = ongoingTouches[t1.identifier];

        if (prev0 && prev1) {
            const dy0 = t0.pageY - prev0.y;
            const dy1 = t1.pageY - prev1.y;
            const avgDy = (dy0 + dy1) / 2.0;

            const scrollDir = (scrollMode === 'natural' ? -1 : 1);
            padScrollAcc += avgDy * scrollDir * 0.9;
            const STEP = 12;
            while (Math.abs(padScrollAcc) >= STEP) {
                const notch = padScrollAcc > 0 ? -1 : 1;
                send(`scroll ${notch}`);
                padScrollAcc -= (padScrollAcc > 0 ? STEP : -STEP);
            }
            ongoingTouches[t0.identifier] = { x: t0.pageX, y: t0.pageY, time: now };
            ongoingTouches[t1.identifier] = { x: t1.pageX, y: t1.pageY, time: now };
        }
    }
}, { passive: false });

trackpad.addEventListener("touchend", (e) => {
    e.preventDefault();
    const now = Date.now();

    for (let i = 0; i < e.changedTouches.length; i++) {
        delete ongoingTouches[e.changedTouches[i].identifier];
    }

    if (e.touches.length === 0) {
        padScrollAcc = 0;
        if (dragMode) {
            dragMode = false;
            send("up left");
            vibrate(10);
        } else if (!touchMoved && (now - touchStartTime < 280)) {
            if (maxFingerCount === 1) {
                send("click left");
                vibrate(15);
                lastTapReleaseTime = now;
            } else if (maxFingerCount === 2) {
                send("click right");
                vibrate(20);
            } else if (maxFingerCount === 3) {
                send("click middle");
                vibrate(20);
            }
        }
        maxFingerCount = 0;
        touchMoved = false;
    }
}, { passive: false });

trackpad.addEventListener("touchcancel", (e) => {
    ongoingTouches = {};
    padScrollAcc = 0;
    if (dragMode) {
        dragMode = false;
        send("up left");
    }
    maxFingerCount = 0;
    touchMoved = false;
});

// Scroll strip
let scrollStartY = 0;
let stripScrollAcc = 0;
scrollStrip.addEventListener("touchstart", (e) => {
    e.preventDefault();
    if (e.touches.length > 0) {
        scrollStartY = e.touches[0].pageY;
        stripScrollAcc = 0;
        vibrate(5);
    }
}, { passive: false });

scrollStrip.addEventListener("touchmove", (e) => {
    e.preventDefault();
    if (e.touches.length > 0) {
        const currentY = e.touches[0].pageY;
        const dy = currentY - scrollStartY;
        scrollStartY = currentY;

        const scrollDir = (scrollMode === 'natural' ? -1 : 1);
        stripScrollAcc += dy * scrollDir * 1.0;
        const STEP = 10;
        while (Math.abs(stripScrollAcc) >= STEP) {
            const notch = stripScrollAcc > 0 ? -1 : 1;
            send(`scroll ${notch}`);
            stripScrollAcc -= (stripScrollAcc > 0 ? STEP : -STEP);
        }
    }
}, { passive: false });

// Nút bấm cứng
btnLeft.addEventListener("touchstart", (e) => {
    e.preventDefault();
    btnLeft.classList.add("active");
    send("down left");
    vibrate(15);
}, { passive: false });

btnLeft.addEventListener("touchend", (e) => {
    e.preventDefault();
    btnLeft.classList.remove("active");
    send("up left");
}, { passive: false });

btnRight.addEventListener("touchstart", (e) => {
    e.preventDefault();
    btnRight.classList.add("active");
    send("click right");
    vibrate(20);
}, { passive: false });

btnRight.addEventListener("touchend", (e) => {
    e.preventDefault();
    btnRight.classList.remove("active");
}, { passive: false });
</script>
</body>
</html>
"#;

fn find_available_port(bind_ip: &str, start_port: u16) -> u16 {
    for port in start_port..start_port + 20 {
        if TcpListener::bind(format!("{}:{}", bind_ip, port)).is_ok() {
            return port;
        }
    }
    start_port
}

fn is_auth_valid(token: &str) -> bool {
    if token.is_empty() {
        return false;
    }
    let pin_file = vic_base_dir().join("ui.pin");
    if let Ok(p) = fs::read_to_string(pin_file) {
        if token == p.trim() {
            return true;
        }
    }
    let cfg = vic_base_dir().join("config.toml");
    if let Ok(content) = fs::read_to_string(cfg) {
        for line in content.lines() {
            if let Some((k, v)) = line.split_once('=') {
                let key = k.trim();
                if key == "password" || key == "pin" {
                    let val = v.trim().trim_matches('"');
                    if !val.is_empty() && token == val {
                        return true;
                    }
                }
            }
        }
    }
    false
}

fn get_display_pin() -> String {
    let pin_file = vic_base_dir().join("ui.pin");
    if let Ok(p) = fs::read_to_string(pin_file) {
        let trimmed = p.trim().to_string();
        if !trimmed.is_empty() {
            return trimmed;
        }
    }
    let cfg = vic_base_dir().join("config.toml");
    if let Ok(content) = fs::read_to_string(cfg) {
        for line in content.lines() {
            if let Some((k, v)) = line.split_once('=') {
                let key = k.trim();
                if key == "pin" || key == "password" {
                    let val = v.trim().trim_matches('"').to_string();
                    if !val.is_empty() {
                        return val;
                    }
                }
            }
        }
    }
    "123456".to_string()
}

fn get_lang() -> String {
    let cfg = vic_base_dir().join("config.toml");
    if let Ok(content) = fs::read_to_string(cfg) {
        for line in content.lines() {
            if let Some((k, v)) = line.split_once('=') {
                if k.trim() == "lang" {
                    return v.trim().trim_matches('"').to_string();
                }
            }
        }
    }
    "vi".to_string()
}

fn stop_daemon() {
    let vi = get_lang() == "vi";
    let pid_file = vic_base_dir().join("mouse.pid");
    let port_file = vic_base_dir().join("mouse.port");
    if let Ok(pid_str) = fs::read_to_string(&pid_file) {
        if let Ok(pid) = pid_str.trim().parse::<u32>() {
            #[cfg(windows)]
            let _ = Command::new("taskkill").args(&["/F", "/PID", &pid.to_string()]).status();
            #[cfg(not(windows))]
            let _ = Command::new("kill").arg("-9").arg(pid.to_string()).status();

            println!("{}", if vi {
                format!("[+] Đã dừng tiến trình Air Mouse & Trackpad (PID: {}).", pid)
            } else {
                format!("[+] Stopped Air Mouse & Trackpad daemon (PID: {}).", pid)
            });
        }
    }
    let _ = fs::remove_file(&pid_file);
    let _ = fs::remove_file(&port_file);
}

fn status_daemon() {
    let vi = get_lang() == "vi";
    let pid_file = vic_base_dir().join("mouse.pid");
    let port_file = vic_base_dir().join("mouse.port");
    if let (Ok(pid_str), Ok(port_str)) = (fs::read_to_string(&pid_file), fs::read_to_string(&port_file)) {
        let (lan_ip, ts_ip) = get_ips();
        let port = port_str.trim();
        let pin = get_display_pin();
        let q = format!("?pin={}", pin);
        println!("=== VIC AIR MOUSE & TRACKPAD STATUS ===");
        println!(" -> {}   : {}", if vi { "Trạng thái" } else { "Status    " }, if vi { "Đang chạy (Active)" } else { "Running (Active)" });
        println!(" -> Process PID  : {}", pid_str.trim());
        println!(" -> {} : {}", if vi { "Cổng kết nối" } else { "Server Port " }, port);
        if ts_ip.starts_with("100.") {
            println!(" -> Tailscale URL: http://{}:{}/{}", ts_ip, port, q);
        }
        println!(" -> LAN URL      : http://{}:{}/{}", lan_ip, port, q);
    } else {
        println!("{}", if vi {
            "[-] Air Mouse / Trackpad hiện đang tắt (không có tiến trình chạy ngầm).\nGõ 'vic mouse' hoặc 'vic mouse -d' để khởi chạy."
        } else {
            "[-] Air Mouse / Trackpad is currently stopped (no background daemon running).\nType 'vic mouse' or 'vic mouse -d' to start."
        });
    }
}

fn run_server(bind_ip: &str, requested_port: u16, is_daemon: bool, is_child: bool) {
    let vi = get_lang() == "vi";
    let target_port = find_available_port(bind_ip, requested_port);

    if is_daemon && !is_child {
        stop_daemon();
        let exe = env::current_exe().unwrap_or_else(|_| PathBuf::from("vic-mouse"));
        let mut child_args = vec![
            target_port.to_string(),
            "--internal-worker".to_string(),
        ];
        if bind_ip == "0.0.0.0" {
            child_args.push("--lan".to_string());
        }

        let mut cmd = Command::new(exe);
        cmd.args(&child_args)
            .stdin(Stdio::null())
            .stdout(Stdio::null())
            .stderr(Stdio::null());

        #[cfg(unix)]
        {
            use std::os::unix::process::CommandExt;
            cmd.process_group(0);
        }

        match cmd.spawn() {
            Ok(child) => {
                let pid = child.id();
                let _ = fs::create_dir_all(vic_base_dir());
                let _ = fs::write(vic_base_dir().join("mouse.pid"), pid.to_string());
                let _ = fs::write(vic_base_dir().join("mouse.port"), target_port.to_string());

                let (lan_ip, ts_ip) = get_ips();
                let pin = get_display_pin();
                let q = format!("?pin={}", pin);
                println!("=== VIC AIR MOUSE & TRACKPAD SERVER (DAEMON) ===");
                println!(" -> Process PID   : {}", pid);
                if ts_ip.starts_with("100.") {
                    println!(" -> Tailscale URL : http://{}:{}/{}", ts_ip, target_port, q);
                }
                println!(" -> LAN URL       : http://{}:{}/{}", lan_ip, target_port, q);
                println!(" -> {}     : {}", if vi { "Hướng dẫn" } else { "Usage" }, if vi { "Mở liên kết trên bằng trình duyệt điện thoại để bắt đầu lia chuột bay." } else { "Open the above link in your mobile browser to start air mouse." });
                println!("{}", if vi { "[+] Đã khởi chạy Air Mouse ngầm. Dùng 'vic mouse stop' để dừng." } else { "[+] Air Mouse daemon started. Use 'vic mouse stop' to stop." });
                return;
            }
            Err(e) => {
                eprintln!("{}: {}", if vi { "[-] Lỗi khởi chạy daemon" } else { "[-] Failed to start daemon" }, e);
                return;
            }
        }
    }

    let bind_addr = format!("{}:{}", bind_ip, target_port);
    let listener = match TcpListener::bind(&bind_addr) {
        Ok(l) => l,
        Err(e) => {
            eprintln!("{}: {}: {}", if vi { "[-] Không thể lắng nghe trên" } else { "[-] Failed to bind on" }, bind_addr, e);
            return;
        }
    };

    if is_child {
        let pid = std::process::id();
        let _ = fs::create_dir_all(vic_base_dir());
        let _ = fs::write(vic_base_dir().join("mouse.pid"), pid.to_string());
        let _ = fs::write(vic_base_dir().join("mouse.port"), target_port.to_string());
    } else {
        let (lan_ip, ts_ip) = get_ips();
        let pin = get_display_pin();
        let q = format!("?pin={}", pin);
        println!("=== VIC AIR MOUSE & TRACKPAD SERVER ===");
        if ts_ip.starts_with("100.") {
            println!(" -> Tailscale URL : http://{}:{}/{}", ts_ip, target_port, q);
        }
        println!(" -> LAN URL       : http://{}:{}/{}", lan_ip, target_port, q);
        println!(" -> {}     : {}", if vi { "Hướng dẫn" } else { "Usage" }, if vi { "Mở liên kết trên bằng điện thoại để bắt đầu lia chuột bay." } else { "Open the above link on your phone to start air mouse." });
        println!("{}", if vi { "Nhấn Ctrl+C để dừng." } else { "Press Ctrl+C to stop." });
    }

    let running = Arc::new(AtomicBool::new(true));

    for stream in listener.incoming() {
        if !running.load(Ordering::Relaxed) {
            break;
        }
        let mut stream = match stream {
            Ok(s) => s,
            Err(_) => continue,
        };

        let _ = stream.set_nodelay(true);

        thread::spawn(move || {
            let mut buf = [0u8; 2048];
            let n = match stream.read(&mut buf) {
                Ok(n) if n > 0 => n,
                _ => return,
            };
            let req_str = String::from_utf8_lossy(&buf[..n]);

            // Trích xuất mã PIN từ query (?pin=...) hoặc header X-VIC-AUTH hoặc Cookie
            let mut req_token = String::new();
            if let Some(pos) = req_str.find("pin=") {
                let after = &req_str[pos + 4..];
                let token = after.split(|c| c == '&' || c == ' ' || c == '\r' || c == '\n').next().unwrap_or("");
                req_token = token.to_string();
            } else if let Some(pos) = req_str.find("vic_auth=") {
                let after = &req_str[pos + 9..];
                let token = after.split(|c| c == ';' || c == ' ' || c == '\r' || c == '\n').next().unwrap_or("");
                req_token = token.to_string();
            }

            let auth_ok = is_auth_valid(&req_token);

            if !auth_ok {
                let body_str = if get_lang() == "vi" {
                    "401 Unauthorized: Vui lòng sử dụng URL có kèm mã PIN hợp lệ (?pin=...)"
                } else {
                    "401 Unauthorized: Please use URL with valid PIN (?pin=...)"
                };
                let body = body_str.as_bytes();
                let resp_head = format!(
                    "HTTP/1.1 401 Unauthorized\r\nContent-Type: text/plain; charset=utf-8\r\nContent-Length: {}\r\nConnection: close\r\nAccess-Control-Allow-Origin: *\r\n\r\n",
                    body.len()
                );
                let _ = stream.write_all(resp_head.as_bytes());
                let _ = stream.write_all(body);
                let _ = stream.flush();
                return;
            }

            if req_str.to_lowercase().contains("upgrade: websocket") {
                let prefixed = PrefixedStream {
                    prefix: Cursor::new(buf[..n].to_vec()),
                    stream,
                };
                if let Ok(mut ws) = tungstenite::accept(prefixed) {
                    while let Ok(msg) = ws.read() {
                        match msg {
                            Message::Text(txt) => {
                                handle_command(&txt);
                            }
                            Message::Close(_) => break,
                            Message::Ping(p) => {
                                let _ = ws.write(Message::Pong(p));
                            }
                            _ => {}
                        }
                    }
                }
            } else {
                let is_root = req_str.starts_with("GET / ")
                    || req_str.starts_with("GET /?")
                    || req_str.starts_with("GET /index.html")
                    || req_str.starts_with("HEAD /");
                let (status, body) = if is_root {
                    ("200 OK", HTML_CONTENT.as_bytes())
                } else {
                    ("404 Not Found", b"Not Found".as_slice())
                };

                let resp_head = format!(
                    "HTTP/1.1 {}\r\nContent-Type: text/html; charset=utf-8\r\nContent-Length: {}\r\nConnection: close\r\nAccess-Control-Allow-Origin: *\r\n\r\n",
                    status,
                    body.len()
                );
                let _ = stream.write_all(resp_head.as_bytes());
                let _ = stream.write_all(body);
                let _ = stream.flush();
            }
        });
    }
}

fn main() {
    let args: Vec<String> = env::args().skip(1).collect();
    let vi = get_lang() == "vi";

    if args.iter().any(|a| a == "-h" || a == "--help" || a == "help") {
        if vi {
            println!("VIC MOUSE - Mobile Air Mouse & Virtual Trackpad");
            println!("Cách dùng:");
            println!("  vic mouse                 : Khởi chạy Web Air Mouse server (hiển thị trực tiếp)");
            println!("  vic mouse -d              : Khởi chạy Web Air Mouse server ngầm (Daemon)");
            println!("  vic mouse stop            : Dừng tiến trình Air Mouse ngầm");
            println!("  vic mouse status          : Xem trạng thái Air Mouse server");
            println!("  vic mouse [--lan] [cổng]  : Chọn cổng và mở cho mạng LAN");
            println!("  vic mouse click [l|r|m]   : Click chuột trái/phải/giữa từ dòng lệnh");
            println!("  vic mouse move <dx> <dy>  : Di chuyển con trỏ chuột từ dòng lệnh");
            println!("  vic mouse center          : Đưa chuột về giữa màn hình");
            println!("  vic mouse scroll <dy>     : Cuộn con lăn chuột từ dòng lệnh");
        } else {
            println!("VIC MOUSE - Mobile Air Mouse & Virtual Trackpad");
            println!("Usage:");
            println!("  vic mouse                 : Start Web Air Mouse server in foreground");
            println!("  vic mouse -d              : Start Web Air Mouse server in background (Daemon)");
            println!("  vic mouse stop            : Stop background Air Mouse daemon");
            println!("  vic mouse status          : Show Air Mouse server status");
            println!("  vic mouse [--lan] [port]  : Select port and expose to LAN");
            println!("  vic mouse click [l|r|m]   : Click mouse button (left/right/middle) from CLI");
            println!("  vic mouse move <dx> <dy>  : Move cursor by delta dx/dy from CLI");
            println!("  vic mouse center          : Center cursor on primary screen");
            println!("  vic mouse scroll <dy>     : Scroll mouse wheel from CLI");
        }
        return;
    }

    if args.first().map(|s| s.as_str()) == Some("stop") {
        stop_daemon();
        return;
    }

    if args.first().map(|s| s.as_str()) == Some("status") {
        status_daemon();
        return;
    }

    if args.first().map(|s| s.as_str()) == Some("click") {
        let btn = args.get(1).map(|s| s.as_str()).unwrap_or("left");
        mouse_click(btn);
        println!("{}", if vi { format!("[+] Đã click chuột: {}", btn) } else { format!("[+] Clicked mouse button: {}", btn) });
        return;
    }

    if args.first().map(|s| s.as_str()) == Some("center") {
        mouse_center();
        println!("{}", if vi { "[+] Đã đưa con trỏ chuột về giữa màn hình." } else { "[+] Centered cursor on screen." });
        return;
    }

    if args.first().map(|s| s.as_str()) == Some("move") {
        if args.len() >= 3 {
            if let (Ok(dx), Ok(dy)) = (args[1].parse::<i32>(), args[2].parse::<i32>()) {
                mouse_move(dx, dy);
                println!("{}", if vi { format!("[+] Đã di chuyển con trỏ: dx={}, dy={}", dx, dy) } else { format!("[+] Moved cursor: dx={}, dy={}", dx, dy) });
                return;
            }
        }
        eprintln!("Usage: vic mouse move <dx> <dy>");
        return;
    }

    if args.first().map(|s| s.as_str()) == Some("scroll") {
        if args.len() >= 2 {
            if let Ok(dy) = args[1].parse::<i32>() {
                mouse_scroll(dy);
                println!("{}", if vi { format!("[+] Đã cuộn con lăn: dy={}", dy) } else { format!("[+] Scrolled wheel: dy={}", dy) });
                return;
            }
        }
        eprintln!("Usage: vic mouse scroll <dy>");
        return;
    }

    let is_daemon = args.iter().any(|a| a == "-d" || a == "bg" || a == "--daemon");
    let is_child = args.iter().any(|a| a == "--internal-worker");

    let mut requested_port: u16 = 3669;
    for a in &args {
        if a.starts_with('-') || a == "bg" {
            continue;
        }
        if let Ok(p) = a.parse::<u16>() {
            requested_port = p;
            break;
        }
    }

    // Mặc định bind 0.0.0.0 để vào được từ cả Wi-Fi LAN và Tailscale VPN trên điện thoại
    let bind_ip = "0.0.0.0";

    run_server(bind_ip, requested_port, is_daemon, is_child);
}
