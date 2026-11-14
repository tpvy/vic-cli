use std::env;
use std::process::Command;

fn cmd_exists(cmd: &str) -> bool {
    Command::new("which")
        .arg(cmd)
        .output()
        .map(|o| o.status.success())
        .unwrap_or(false)
}

fn check_deps() -> bool {
    if cfg!(target_os = "linux") && !cmd_exists("playerctl") {
        println!("[-] Thiếu công cụ 'playerctl'. Hãy cài đặt: sudo pacman -S playerctl (hoặc sudo apt install playerctl)");
        return false;
    }
    true
}

fn main() {
    let args: Vec<String> = env::args().collect();
    if args.len() < 2 {
        println!("Usage: vic media <play|pause|play-pause|next|prev>");
        return;
    }

    if !check_deps() {
        return;
    }

    let action = args[1].to_lowercase();
    match action.as_str() {
        "play" | "play-pause" | "toggle" => media_play_pause(),
        "pause" => media_pause(),
        "next" => media_next(),
        "prev" | "previous" => media_prev(),
        _ => println!("[-] Unknown action '{}'. Usage: vic media <play|pause|next|prev>", action),
    }
}

fn media_play_pause() {
    if cfg!(target_os = "linux") {
        match Command::new("playerctl").arg("play-pause").status() {
            Ok(s) if s.success() => println!("[+] Media: Play/Pause"),
            Ok(_) => println!("[!] Không tìm thấy trình phát media nào đang mở (Spotify, trình duyệt, v.v.)."),
            Err(e) => println!("[-] Lỗi gọi playerctl: {}", e),
        }
    } else if cfg!(target_os = "windows") {
        let ps = "$wscript = New-Object -ComObject Wscript.Shell; $wscript.SendKeys([char]179)";
        let _ = Command::new("powershell").args(["-NoProfile", "-Command", ps]).status();
        println!("[+] Media: Play/Pause");
    } else if cfg!(target_os = "macos") {
        let _ = Command::new("osascript").args(["-e", "tell application \"Music\" to playpause"]).status();
        println!("[+] Media: Play/Pause");
    }
}

fn media_pause() {
    if cfg!(target_os = "linux") {
        let _ = Command::new("playerctl").arg("pause").status();
    } else if cfg!(target_os = "windows") {
        let ps = "$wscript = New-Object -ComObject Wscript.Shell; $wscript.SendKeys([char]179)";
        let _ = Command::new("powershell").args(["-NoProfile", "-Command", ps]).status();
    } else if cfg!(target_os = "macos") {
        let _ = Command::new("osascript").args(["-e", "tell application \"Music\" to pause"]).status();
    }
    println!("[+] Media: Pause");
}

fn media_next() {
    if cfg!(target_os = "linux") {
        let _ = Command::new("playerctl").arg("next").status();
    } else if cfg!(target_os = "windows") {
        let ps = "$wscript = New-Object -ComObject Wscript.Shell; $wscript.SendKeys([char]176)";
        let _ = Command::new("powershell").args(["-NoProfile", "-Command", ps]).status();
    } else if cfg!(target_os = "macos") {
        let _ = Command::new("osascript").args(["-e", "tell application \"Music\" to next track"]).status();
    }
    println!("[+] Media: Next");
}

fn media_prev() {
    if cfg!(target_os = "linux") {
        let _ = Command::new("playerctl").arg("previous").status();
    } else if cfg!(target_os = "windows") {
        let ps = "$wscript = New-Object -ComObject Wscript.Shell; $wscript.SendKeys([char]177)";
        let _ = Command::new("powershell").args(["-NoProfile", "-Command", ps]).status();
    } else if cfg!(target_os = "macos") {
        let _ = Command::new("osascript").args(["-e", "tell application \"Music\" to previous track"]).status();
    }
    println!("[+] Media: Previous");
}
