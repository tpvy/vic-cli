use std::env;
use std::fs;
use std::path::PathBuf;
use std::process::{Command, Stdio};

struct ProcItem {
    pid: u32,
    cpu: String,
    mem: String,
    name: String,
}

fn vic_base_dir() -> PathBuf {
    let home = env::var("HOME")
        .or_else(|_| env::var("USERPROFILE"))
        .unwrap_or_else(|_| ".".to_string());
    PathBuf::from(home).join(".vic")
}

fn cache_file() -> PathBuf {
    vic_base_dir().join("top_cache.txt")
}

fn get_top_processes() -> Vec<ProcItem> {
    let mut list = Vec::new();

    if cfg!(target_os = "linux") || cfg!(target_os = "macos") {
        let args = if cfg!(target_os = "macos") {
            vec!["-eo", "pid,%cpu,%mem,comm", "-r"]
        } else {
            vec!["-eo", "pid,%cpu,%mem,comm", "--sort=-%cpu"]
        };

        if let Ok(output) = Command::new("ps").args(&args).output() {
            let text = String::from_utf8_lossy(&output.stdout);
            for line in text.lines().skip(1) {
                let parts: Vec<&str> = line.split_whitespace().collect();
                if parts.len() >= 4 {
                    if let Ok(pid) = parts[0].parse::<u32>() {
                        let comm = parts[3..].join(" ");
                        if comm == "ps" || comm.contains("vic") {
                            continue;
                        }
                        list.push(ProcItem {
                            pid,
                            cpu: format!("{}%", parts[1]),
                            mem: format!("{}%", parts[2]),
                            name: comm,
                        });
                        if list.len() >= 5 {
                            break;
                        }
                    }
                }
            }
        }
    } else if cfg!(target_os = "windows") {
        let ps_cmd = "Get-Process | Sort-Object CPU -Descending | Select-Object -First 5 -Property Id, ProcessName, @{Name='CPU';Expression={[math]::Round($_.CPU,1)}}, @{Name='MEM';Expression={[math]::Round($_.WorkingSet/1MB,1)}} | ForEach-Object { \"$($_.Id) $($_.CPU)% $($_.MEM)MB $($_.ProcessName)\" }";
        if let Ok(output) = Command::new("powershell").args(["-NoProfile", "-Command", ps_cmd]).output() {
            let text = String::from_utf8_lossy(&output.stdout);
            for line in text.lines() {
                let parts: Vec<&str> = line.split_whitespace().collect();
                if parts.len() >= 4 {
                    if let Ok(pid) = parts[0].parse::<u32>() {
                        list.push(ProcItem {
                            pid,
                            cpu: parts[1].to_string(),
                            mem: parts[2].to_string(),
                            name: parts[3..].join(" "),
                        });
                    }
                }
            }
        }
    }

    list
}

fn save_cache(items: &[ProcItem]) {
    let mut content = String::new();
    for item in items {
        content.push_str(&format!("{}:{}\n", item.pid, item.name));
    }
    let _ = fs::write(cache_file(), content);
}

fn load_cache() -> Vec<(u32, String)> {
    let mut list = Vec::new();
    if let Ok(text) = fs::read_to_string(cache_file()) {
        for line in text.lines() {
            if let Some((p, n)) = line.split_once(':') {
                if let Ok(pid) = p.parse::<u32>() {
                    list.push((pid, n.to_string()));
                }
            }
        }
    }
    list
}

fn kill_pid(pid: u32) -> bool {
    if cfg!(target_os = "windows") {
        Command::new("taskkill")
            .args(["/F", "/PID", &pid.to_string()])
            .stdout(Stdio::null())
            .stderr(Stdio::null())
            .status()
            .map(|s| s.success())
            .unwrap_or(false)
    } else {
        Command::new("kill")
            .args(["-9", &pid.to_string()])
            .stdout(Stdio::null())
            .stderr(Stdio::null())
            .status()
            .map(|s| s.success())
            .unwrap_or(false)
    }
}

fn main() {
    let args: Vec<String> = env::args().collect();

    if args.len() >= 2 && args[1].to_lowercase() == "kill" {
        if args.len() < 3 {
            println!("Usage: vic top kill <1-5 | PID>");
            return;
        }

        let target_str = &args[2];
        let cache = load_cache();

        if let Ok(idx) = target_str.parse::<usize>() {
            if idx >= 1 && idx <= cache.len() {
                let (pid, name) = &cache[idx - 1];
                if kill_pid(*pid) {
                    println!("[+] Đã tắt tiến trình [#{}] {} (PID: {})", idx, name, pid);
                } else {
                    println!("[-] Không thể tắt tiến trình (PID: {}). Thử chạy với sudo?", pid);
                }
                return;
            }
        }

        if let Ok(pid) = target_str.parse::<u32>() {
            if kill_pid(pid) {
                println!("[+] Đã tắt tiến trình (PID: {})", pid);
            } else {
                println!("[-] Không thể tắt tiến trình (PID: {}).", pid);
            }
            return;
        }

        println!("[-] Mục tiêu không hợp lệ. Hãy chạy 'vic top' để xem danh sách.");
        return;
    }

    let items = get_top_processes();
    save_cache(&items);

    println!("=== TOP 5 TIẾN TRÌNH CHIẾM DỤNG TÀI NGUYÊN ===");
    println!("{:<4} {:<8} {:<9} {:<9} {}", "STT", "PID", "CPU", "RAM", "TÊN TIẾN TRÌNH");
    println!("--------------------------------------------------");
    for (i, it) in items.iter().enumerate() {
        println!("[{}]  {:<8} {:<9} {:<9} {}", i + 1, it.pid, it.cpu, it.mem, it.name);
    }
    println!("\n>> Tắt nhanh: 'vic top kill <1-5>'  (VD: vic top kill 1)");
}
