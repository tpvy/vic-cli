use chrono::Local;
use sha2::{Digest, Sha256};
use std::env;
use std::fs;
use std::io::{self, Read, Write};
use std::net::UdpSocket;
use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};
use std::time::Duration;
use tiny_http::{Header, Method, Response, Server};

const DEFAULT_MODULE_MD: &str = include_str!("../module.md");

#[derive(Clone)]
struct EntryInfo {
    id: usize,
    name: String,
    entry_type: String,
    vi_short: String,
    en_short: String,
    vi_desc: String,
    en_desc: String,
    usage: String,
    linux_url: String,
    windows_url: String,
    mac_url: String,
    sha256_linux: String,
    sha256_windows: String,
    sha256_mac: String,
    ui_actions: String,
}

fn main() {
    setup_linux_gui_env();
    init_dirs();
    let lang = get_cfg("lang", "vi");
    let args: Vec<String> = env::args().collect();

    if args.len() < 2 {
        cmd_list(&[], &lang);
        return;
    }

    let cmd = args[1].to_lowercase();
    let sub_args = &args[2..];

    match cmd.as_str() {
        "list" | "m" | "module" => cmd_list(sub_args, &lang),
        "sync" => {
            sync_remote_md(true, &lang);
            cmd_list(&[], &lang);
        }
        "update" | "upgrade" => {
            sync_remote_md(false, &lang);
            cmd_upgrade(sub_args, &lang);
        }
        "?" | "help" | "-h" | "--help" => cmd_help_detail(sub_args, &lang),
        "version" | "-v" | "--version" => {
            println!("vic-cli 1.0.0");
            return;
        }
        "pass" | "password" => cmd_password(sub_args, &lang),
        "install" => {
            let _ = cmd_install(sub_args, &lang);
        }
        "lang" => cmd_lang(sub_args),
        "setup" => cmd_setup(sub_args, &lang),
        "doctor" => cmd_doctor(&lang),
        "sc" | "screenshot" => {
            let _ = cmd_screenshot(sub_args, &lang);
        }
        "fast" => cmd_fast(),
        "share" => cmd_share(sub_args, &lang),
        "light" => cmd_light(sub_args, &lang),
        "au" | "audio" => cmd_audio(sub_args, &lang),
        "clip" => cmd_clip(sub_args, &lang),
        "lock" | "sleep" => cmd_power(&cmd),
        "ui" => cmd_ui(sub_args, &lang),
        _ => run_external_module(&cmd, sub_args, &lang),
    }
}


fn setup_linux_gui_env() {
    if !cfg!(target_os = "linux") {
        return;
    }
    let uid = get_output("id", &["-u"]);
    if uid.is_empty() {
        return;
    }
    let runtime_dir = format!("/run/user/{}", uid);
    if env::var("XDG_RUNTIME_DIR").is_err() && Path::new(&runtime_dir).exists() {
        env::set_var("XDG_RUNTIME_DIR", &runtime_dir);
    }
    if env::var("DBUS_SESSION_BUS_ADDRESS").is_err() {
        let bus_path = format!("{}/bus", runtime_dir);
        if Path::new(&bus_path).exists() {
            env::set_var("DBUS_SESSION_BUS_ADDRESS", format!("unix:path={}", bus_path));
        }
    }
    if env::var("WAYLAND_DISPLAY").is_err() {
        if let Ok(entries) = fs::read_dir(&runtime_dir) {
            for entry in entries.flatten() {
                let fname = entry.file_name().to_string_lossy().to_string();
                if fname.starts_with("wayland-") && !fname.ends_with(".lock") {
                    env::set_var("WAYLAND_DISPLAY", fname);
                    break;
                }
            }
        }
    }
    if env::var("DISPLAY").is_err() {
        env::set_var("DISPLAY", ":0");
    }
}


fn vic_base_dir() -> PathBuf {
    let home = env::var("HOME")
        .or_else(|_| env::var("USERPROFILE"))
        .unwrap_or_else(|_| ".".to_string());
    Path::new(&home).join(".vic")
}

fn generate_secure_pin() -> String {
    #[cfg(unix)]
    {
        if let Ok(mut f) = fs::File::open("/dev/urandom") {
            let mut buf = [0u8; 4];
            if f.read_exact(&mut buf).is_ok() {
                let val = u32::from_ne_bytes(buf);
                return format!("{:06}", (val % 900_000) + 100_000);
            }
        }
    }
    let nanos = Local::now().timestamp_subsec_nanos();
    let ptr_mix = (&nanos as *const u32 as usize) as u32;
    let val = nanos.wrapping_add(ptr_mix).wrapping_mul(1103515245).wrapping_add(12345);
    format!("{:06}", (val % 900_000) + 100_000)
}

fn init_dirs() {
    let base = vic_base_dir();
    for sub in ["bin", "modules", "archive", "share"] {
        let _ = fs::create_dir_all(base.join(sub));
    }
    let cfg = base.join("config.toml");
    if !cfg.exists() {
        let pin = generate_secure_pin();
        let _ = fs::write(cfg, format!("lang = \"vi\"\npin = \"{}\"\n", pin));
    }
    let md = base.join("module.md");
    if !md.exists() {
        let _ = fs::write(md, DEFAULT_MODULE_MD);
    }
}

fn get_cfg(key: &str, default: &str) -> String {
    let cfg = vic_base_dir().join("config.toml");
    if let Ok(content) = fs::read_to_string(cfg) {
        for line in content.lines() {
            if let Some((k, v)) = line.split_once('=') {
                if k.trim() == key {
                    return v.trim().trim_matches('"').to_string();
                }
            }
        }
    }
    default.to_string()
}

fn set_cfg(key: &str, val: &str) {
    let cfg_path = vic_base_dir().join("config.toml");
    let content = fs::read_to_string(&cfg_path).unwrap_or_default();
    let mut lines: Vec<String> = Vec::new();
    let mut found = false;
    for line in content.lines() {
        if let Some((k, _)) = line.split_once('=') {
            if k.trim() == key {
                lines.push(format!("{} = \"{}\"", key, val));
                found = true;
                continue;
            }
        }
        lines.push(line.to_string());
    }
    if !found {
        lines.push(format!("{} = \"{}\"", key, val));
    }
    let _ = fs::write(cfg_path, lines.join("\n") + "\n");
}

fn cmd_lang(args: &[String]) {
    let Some(l) = args.first() else {
        println!("Current language: {} (Usage: vic lang vi | en)", get_cfg("lang", "vi"));
        return;
    };
    let target = l.to_lowercase();
    if target == "vi" || target == "en" {
        set_cfg("lang", &target);
        println!(
            "{}",
            if target == "vi" {
                "[+] Đã chuyển ngôn ngữ sang Tiếng Việt."
            } else {
                "[+] Language switched to English."
            }
        );
    } else {
        println!("[-] Chỉ hỗ trợ 'vi' hoặc 'en' / Only 'vi' or 'en' supported.");
    }
}

fn cmd_password(args: &[String], lang: &str) {
    let vi = lang == "vi";
    if let Some(val) = args.first() {
        if val == "clear" || val == "remove" || val == "rm" || val == "none" {
            set_cfg("password", "");
            println!("{}", if vi {
                "[+] Đã xóa mật khẩu riêng. Web UI sẽ chỉ sử dụng mã PIN ngẫu nhiên sinh mới mỗi phiên."
            } else {
                "[+] Custom password removed. Web UI will use random session PIN only."
            });
            return;
        }
        if val.len() < 4 {
            println!("{}", if vi {
                "[-] Mật khẩu quá ngắn. Vui lòng đặt tối thiểu 4 ký tự (chữ và số)."
            } else {
                "[-] Password too short. Please provide at least 4 characters."
            });
            return;
        }
        set_cfg("password", val);
        println!("{}", if vi {
            format!("[+] Đã lưu mật khẩu riêng thành công (độ dài: {} ký tự).", val.len())
        } else {
            format!("[+] Custom password set successfully (length: {} chars).", val.len())
        });
        println!("{}", if vi {
            "[*] Bạn có thể đăng nhập Web UI bằng mật khẩu này hoặc bằng mã PIN ngẫu nhiên mỗi phiên."
        } else {
            "[*] You can log in to Web UI using this password or the session random PIN."
        });
    } else {
        let cur = get_cfg("password", "");
        if cur.is_empty() {
            println!("{}", if vi {
                "[*] Chưa đặt mật khẩu riêng (Web UI đang dùng mã PIN ngẫu nhiên sinh mới mỗi khi mở)."
            } else {
                "[*] No custom password set (Web UI is using a fresh random PIN per session)."
            });
            println!("{}", if vi {
                "[*] Cú pháp đặt mật khẩu: vic pass <mật_khẩu_của_bạn> (hoặc: vic ui pass <mật_khẩu>)"
            } else {
                "[*] Set password: vic pass <your_password> (or: vic ui pass <your_password>)"
            });
        } else {
            println!("{}", if vi {
                format!("[+] Mật khẩu riêng hiện tại: [Đã đặt - {} ký tự]", cur.len())
            } else {
                format!("[+] Current custom password: [Active - {} chars]", cur.len())
            });
            println!("{}", if vi {
                "[*] Đổi mật khẩu: vic pass <mật_khẩu_mới> | Xóa mật khẩu: vic pass clear"
            } else {
                "[*] Change password: vic pass <new_password> | Remove: vic pass clear"
            });
        }
    }
}


const REGISTRY_URL: &str = "https://fastly.jsdelivr.net/gh/tpvy/vic-cli@main/module.md";
const REGISTRY_FALLBACK_URL: &str = "https://vic-cli.phuongvy.xyz/module.md";

fn sync_remote_md(verbose: bool, lang: &str) {
    let agent = ureq::AgentBuilder::new()
        .timeout(Duration::from_secs(3))
        .build();
    let cache_buster = format!("{}?t={}", REGISTRY_URL, Local::now().timestamp());
    let fetch_result = agent.get(&cache_buster).call().or_else(|_| agent.get(REGISTRY_FALLBACK_URL).call());
    match fetch_result {
        Ok(resp) => {
            if let Ok(text) = resp.into_string() {
                if text.contains("## [") {
                    let _ = fs::write(vic_base_dir().join("module.md"), text);
                    if verbose {
                        println!(
                            "{}",
                            if lang == "vi" {
                                "[+] Đã cập nhật module.md từ máy chủ."
                            } else {
                                "[+] Updated module.md from server."
                            }
                        );
                    }
                }
            }
        }
        Err(_) => {
            if verbose {
                println!(
                    "{}",
                    if lang == "vi" {
                        "[-] Không thể kết nối tới vic-cli.phuongvy.xyz, đang dùng dữ liệu offline."
                    } else {
                        "[-] Could not reach vic-cli.phuongvy.xyz, using cached local module.md."
                    }
                );
            }
        }
    }
}

fn parse_module_md() -> Vec<EntryInfo> {
    let md_path = vic_base_dir().join("module.md");
    let content = fs::read_to_string(&md_path).unwrap_or_else(|_| DEFAULT_MODULE_MD.to_string());
    let mut list = Vec::new();
    let mut cur: Option<EntryInfo> = None;

    for line in content.lines() {
        let t = line.trim();
        if t.starts_with("## [") {
            if let Some(item) = cur.take() {
                list.push(item);
            }
            if let Some(end_idx) = t.find(']') {
                let id_str = &t[4..end_idx];
                let name = t[end_idx + 1..].trim().to_string();
                if let Ok(id) = id_str.parse::<usize>() {
                    cur = Some(EntryInfo {
                        id,
                        name,
                        entry_type: "module".to_string(),
                        vi_short: String::new(),
                        en_short: String::new(),
                        vi_desc: String::new(),
                        en_desc: String::new(),
                        usage: String::new(),
                        linux_url: String::new(),
                        windows_url: String::new(),
                        mac_url: String::new(),
                        sha256_linux: String::new(),
                        sha256_windows: String::new(),
                        sha256_mac: String::new(),
                        ui_actions: String::new(),
                    });
                }
            }
        } else if t.starts_with("- ") {
            if let Some(ref mut item) = cur {
                if let Some((k, v)) = t[2..].split_once(':') {
                    let key = k.trim().trim_matches('*');
                    let val = v.trim().trim_matches('`').to_string();
                    match key {
                        "type" => item.entry_type = val,
                        "vi_short" => item.vi_short = val,
                        "en_short" => item.en_short = val,
                        "vi_desc" => item.vi_desc = val,
                        "en_desc" => item.en_desc = val,
                        "usage" => item.usage = val,
                        "ui_actions" => item.ui_actions = val,
                        "linux_url" => item.linux_url = val,
                        "windows_url" => item.windows_url = val,
                        "mac_url" => item.mac_url = val,
                        "sha256_linux" => item.sha256_linux = val,
                        "sha256_windows" => item.sha256_windows = val,
                        "sha256_mac" => item.sha256_mac = val,
                        _ => {}
                    }
                }
            }
        }
    }
    if let Some(item) = cur {
        list.push(item);
    }
    list
}

fn is_module_installed(name: &str) -> bool {
    let bin_name = if cfg!(target_os = "windows") {
        format!("{}.exe", name)
    } else {
        name.to_string()
    };
    let path = vic_base_dir().join("modules").join(bin_name);
    match fs::metadata(path) {
        Ok(m) => m.len() > 0,
        Err(_) => false,
    }
}


fn cmd_list(args: &[String], lang: &str) {
    let vi = lang == "vi";
    let show_only_installed = args.iter().any(|a| a == "-i" || a == "--installed");
    let show_only_uninstalled = args.iter().any(|a| a == "-u" || a == "--uninstalled" || a == "--available");

    let entries = parse_module_md();
    let cores: Vec<&EntryInfo> = entries.iter().filter(|e| e.entry_type == "core").collect();
    let mut mods: Vec<&EntryInfo> = entries.iter().filter(|e| e.entry_type != "core").collect();

    // Sắp xếp đưa module đã cài đặt lên đầu danh sách, sau đó theo ID
    mods.sort_by(|a, b| {
        let a_inst = is_module_installed(&a.name);
        let b_inst = is_module_installed(&b.name);
        match (a_inst, b_inst) {
            (true, false) => std::cmp::Ordering::Less,
            (false, true) => std::cmp::Ordering::Greater,
            _ => a.id.cmp(&b.id),
        }
    });

    if !show_only_installed && !show_only_uninstalled {
        println!(
            "{}",
            if vi {
                "=== LỆNH CƠ BẢN (CORE) ==="
            } else {
                "=== CORE COMMANDS ==="
            }
        );

        let half = (cores.len() + 1) / 2;
        for i in 0..half {
            let left = cores[i];
            let l_desc = if vi { &left.vi_short } else { &left.en_short };
            let l_str = format!("[{}] {:<7} {}", left.id, left.name, l_desc);

            if i + half < cores.len() {
                let right = cores[i + half];
                let r_desc = if vi { &right.vi_short } else { &right.en_short };
                println!("{:<34} | [{}] {:<7} {}", l_str, right.id, right.name, r_desc);
            } else {
                println!("{}", l_str);
            }
        }
    }

    let filtered_mods: Vec<&EntryInfo> = mods
        .into_iter()
        .filter(|m| {
            let inst = is_module_installed(&m.name);
            if show_only_installed {
                inst
            } else if show_only_uninstalled {
                !inst
            } else {
                true
            }
        })
        .collect();

    let header = if show_only_installed {
        if vi { "=== MODULE ĐÃ CÀI ĐẶT (INSTALLED) ===" } else { "=== INSTALLED EXTENSION MODULES ===" }
    } else if show_only_uninstalled {
        if vi { "=== MODULE CHƯA CÀI ĐẶT (AVAILABLE) ===" } else { "=== AVAILABLE EXTENSION MODULES ===" }
    } else {
        if vi { "=== MODULE MỞ RỘNG (vic-cli.phuongvy.xyz) ===" } else { "=== EXTENSION MODULES (vic-cli.phuongvy.xyz) ===" }
    };

    println!("\n{}", header);

    if filtered_mods.is_empty() {
        println!("{}", if vi { "  (Không có module nào phù hợp)" } else { "  (No matching modules found)" });
    } else {
        for m in filtered_mods {
            let installed = is_module_installed(&m.name);
            let status = if installed { "[OK]" } else { "" };
            let desc = if vi { &m.vi_short } else { &m.en_short };
            println!("[{:>2}] {:<10} {:<6} - {}", m.id, m.name, status, desc);
        }
    }

    println!(
        "\n{}",
        if vi {
            "Tra cứu: 'vic ? <số>' | Cài: 'vic install <số>' | Lọc: 'vic list -i' (đã cài), 'vic list -u' (chưa cài)"
        } else {
            "Help: 'vic ? <id>' | Install: 'vic install <id>' | Filter: 'vic list -i' (installed), 'vic list -u' (available)"
        }
    );
}

fn cmd_help_detail(args: &[String], lang: &str) {
    let vi = lang == "vi";
    let Some(target) = args.first() else {
        cmd_list(&[], lang);
        return;
    };

    let entries = parse_module_md();
    let found = if let Ok(num) = target.parse::<usize>() {
        entries.into_iter().find(|e| e.id == num)
    } else {
        entries.into_iter().find(|e| e.name.eq_ignore_ascii_case(target) || (e.id == 9 && (target.eq_ignore_ascii_case("doctor") || target.eq_ignore_ascii_case("setup"))))
    };

    if let Some(e) = found {
        let title = if vi { &e.vi_short } else { &e.en_short };
        let desc = if vi { &e.vi_desc } else { &e.en_desc };
        println!("[{}] {} - {}", e.id, e.name, title);
        println!("--------------------------------------------------");
        println!("{}", desc);
        println!(
            "{}: {}",
            if vi { "Cú pháp" } else { "Usage" },
            e.usage
        );
    } else {
        println!(
            "{}",
            if vi {
                "[-] Không tìm thấy mục này. Gõ 'vic list' để xem danh sách."
            } else {
                "[-] Entry not found. Type 'vic list' to view all."
            }
        );
    }
}


fn cmd_install(args: &[String], lang: &str) -> String {
    let vi = lang == "vi";
    let is_force = args.iter().any(|a| a == "-f" || a == "--force");
    let is_local = args.iter().any(|a| a == "--local");
    let filter_installed = args.iter().any(|a| a == "-i" || a == "--installed");
    let filter_uninstalled = args.iter().any(|a| a == "-u" || a == "--uninstalled" || a == "--available");
    let is_list = args.is_empty() || args.iter().any(|a| a == "list" || a == "--list" || a == "-l") || filter_installed || filter_uninstalled;

    sync_remote_md(false, lang);
    let entries = parse_module_md();
    let mods: Vec<EntryInfo> = entries.into_iter().filter(|e| e.entry_type != "core").collect();

    if is_list {
        let mut sorted_mods = mods.clone();
        sorted_mods.sort_by(|a, b| {
            let a_inst = is_module_installed(&a.name);
            let b_inst = is_module_installed(&b.name);
            match (a_inst, b_inst) {
                (true, false) => std::cmp::Ordering::Less,
                (false, true) => std::cmp::Ordering::Greater,
                _ => a.id.cmp(&b.id),
            }
        });
        let display_mods: Vec<&EntryInfo> = sorted_mods
            .iter()
            .filter(|m| {
                let inst = is_module_installed(&m.name);
                if filter_installed {
                    inst
                } else if filter_uninstalled {
                    !inst
                } else {
                    true
                }
            })
            .collect();

        let header = if filter_installed {
            if vi { "=== MODULE MỞ RỘNG ĐÃ CÀI ĐẶT (INSTALLED) ===" } else { "=== INSTALLED EXTENSION MODULES ===" }
        } else if filter_uninstalled {
            if vi { "=== MODULE MỞ RỘNG CHƯA CÀI ĐẶT (AVAILABLE) ===" } else { "=== AVAILABLE EXTENSION MODULES ===" }
        } else {
            if vi { "=== DANH MỤC MODULE MỞ RỘNG (EXTENSION MODULES) ===" } else { "=== EXTENSION MODULES CATALOG ===" }
        };
        println!("{}", header);
        println!();
        if display_mods.is_empty() {
            println!("{}", if vi { "  (Không có module nào phù hợp)" } else { "  (No matching modules found)" });
        } else {
            for m in display_mods {
                let installed = is_module_installed(&m.name);
                let status = if installed { "[OK]" } else { "" };
                let title = if vi { &m.vi_short } else { &m.en_short };
                let desc = if vi { &m.vi_desc } else { &m.en_desc };
                println!("[{:>2}] {:<10} {:<6} - {}", m.id, m.name, status, title);
                println!("     {} {}", if vi { "Cú pháp :" } else { "Usage   :" }, m.usage);
                println!("     {} {}", if vi { "Mô tả   :" } else { "Desc    :" }, desc);
                println!();
            }
        }
        println!("{}", if vi {
            "Cài đặt: 'vic install <id | tên | all>' | Lọc: 'vic install -i' (đã cài), 'vic install -u' (chưa cài)"
        } else {
            "Install: 'vic install <id | name | all>' | Filter: 'vic install -i' (installed), 'vic install -u' (available)"
        });
        return String::new();
    }

    let install_all = args.iter().any(|a| a.eq_ignore_ascii_case("all"));

    let mut target_modules: Vec<&EntryInfo> = Vec::new();

    if install_all {
        target_modules = mods.iter().collect();
    } else {
        for arg in args {
            if arg == "-f" || arg == "--force" || arg == "-y" || arg == "--yes" || arg == "--local"
                || arg == "-i" || arg == "--installed" || arg == "-u" || arg == "--uninstalled" || arg == "--available" {
                continue;
            }
            for token in arg.split(',') {
                let t = token.trim();
                if t.is_empty() {
                    continue;
                }
                if let Ok(id) = t.parse::<usize>() {
                    if let Some(m) = mods.iter().find(|m| m.id == id) {
                        if !target_modules.iter().any(|x| x.id == m.id) {
                            target_modules.push(m);
                        }
                    } else {
                        let msg = if vi {
                            format!("[-] ID [{}] không tồn tại trong danh mục module mở rộng.", id)
                        } else {
                            format!("[-] ID [{}] not found in extension modules.", id)
                        };
                        println!("{}", msg);
                        return msg;
                    }
                } else if let Some(m) = mods.iter().find(|m| m.name.eq_ignore_ascii_case(t)) {
                    if !target_modules.iter().any(|x| x.id == m.id) {
                        target_modules.push(m);
                    }
                } else {
                    let msg = if vi {
                        format!("[-] Module '{}' không tồn tại trong danh mục.", t)
                    } else {
                        format!("[-] Module '{}' not found in catalog.", t)
                    };
                    println!("{}", msg);
                    return msg;
                }
            }
        }
    }

    if target_modules.is_empty() {
        let msg = if vi {
            "[-] Không tìm thấy module hợp lệ nào để cài đặt. Gõ 'vic install list' để xem danh sách."
        } else {
            "[-] No valid module selected to install. Type 'vic install list' to view available modules."
        };
        println!("{}", msg);
        return msg.to_string();
    }

    let to_install: Vec<&EntryInfo> = target_modules
        .into_iter()
        .filter(|m| !is_module_installed(&m.name) || is_force)
        .collect();

    if to_install.is_empty() {
        let msg = if vi {
            "[*] Tất cả các module đã được cài đặt. Thêm '-f' nếu muốn cài đè."
        } else {
            "[*] All requested modules are already installed. Use '-f' to force reinstall."
        };
        println!("{}", msg);
        return msg.to_string();
    }

    println!("{}", if vi {
        "[*] Chuẩn bị tải và cài đặt các module sau:"
    } else {
        "[*] Ready to download and install the following modules:"
    });
    for m in &to_install {
        let (url, expected_sha) = if cfg!(target_os = "windows") {
            (&m.windows_url, &m.sha256_windows)
        } else if cfg!(target_os = "macos") {
            (&m.mac_url, &m.sha256_mac)
        } else {
            (&m.linux_url, &m.sha256_linux)
        };
        let title = if vi { &m.vi_short } else { &m.en_short };
        println!(" -> [{:>2}] {:<10} : {}", m.id, m.name, title);
        println!("    Link : {}", url);
        if !expected_sha.is_empty() && expected_sha != "SKIP" {
            println!("    Hash : {}", expected_sha);
        }
    }

    let is_yes = args.iter().any(|a| a == "-y" || a == "--yes");
    if !is_yes {
        print!("{}", if vi {
            "\n[?] Bạn có đồng ý tải và cài đặt các module trên không? [y/N]: "
        } else {
            "\n[?] Do you agree to download and install the modules above? [y/N]: "
        });
        let _ = io::stdout().flush();
        let mut confirm = String::new();
        if io::stdin().read_line(&mut confirm).is_err() || !confirm.trim().eq_ignore_ascii_case("y") {
            let msg = if vi { "[-] Đã hủy quá trình cài đặt." } else { "[-] Installation canceled by user." };
            println!("{}", msg);
            return msg.to_string();
        }
    }

    let mut result_messages = Vec::new();

    for m in to_install {
        let bin_name = if cfg!(target_os = "windows") {
            format!("{}.exe", m.name)
        } else {
            m.name.clone()
        };

        let bytes = if is_local {
            let possible_paths = [
                PathBuf::from(format!("target/release/{}", bin_name)),
                PathBuf::from(format!("target/debug/{}", bin_name)),
                PathBuf::from(&bin_name),
            ];
            let mut found_bytes = None;
            for p in &possible_paths {
                if let Ok(b) = fs::read(p) {
                    println!("[*] Found local build: {}", p.display());
                    found_bytes = Some(b);
                    break;
                }
            }
            if let Some(b) = found_bytes {
                b
            } else {
                let msg = if vi {
                    format!("[-] Không tìm thấy bản build cục bộ nào cho module '{}' trong target/release/ hoặc target/debug/.", m.name)
                } else {
                    format!("[-] No local build found for module '{}' in target/release/ or target/debug/.", m.name)
                };
                println!("{}", msg);
                result_messages.push(msg);
                continue;
            }
        } else {
            let (url, expected_sha) = if cfg!(target_os = "windows") {
                (&m.windows_url, &m.sha256_windows)
            } else if cfg!(target_os = "macos") {
                (&m.mac_url, &m.sha256_mac)
            } else {
                (&m.linux_url, &m.sha256_linux)
            };

            if url.is_empty() {
                let msg = if vi {
                    format!("[-] Module '{}' chưa có link tải cho hệ điều hành này.", m.name)
                } else {
                    format!("[-] Module '{}' has no download link for this OS.", m.name)
                };
                println!("{}", msg);
                result_messages.push(msg);
                continue;
            }

            println!("[*] Downloading '{}'...", m.name);
            let Ok(resp) = ureq::get(url).timeout(Duration::from_secs(30)).call() else {
                let msg = if vi {
                    format!("[-] Tải thất bại từ: {}", url)
                } else {
                    format!("[-] Download failed from: {}", url)
                };
                println!("{}", msg);
                result_messages.push(msg);
                continue;
            };

            let mut b = Vec::new();
            if resp.into_reader().read_to_end(&mut b).is_err() {
                let msg = if vi { "[-] Lỗi đọc luồng dữ liệu." } else { "[-] Failed reading stream." };
                println!("{}", msg);
                result_messages.push(msg.to_string());
                continue;
            }

            let valid_sha = expected_sha.len() == 64 && expected_sha.chars().all(|c| c.is_ascii_hexdigit());
            if !valid_sha {
                let msg = if vi {
                    format!("[-] CẢNH BÁO BẢO MẬT: Module '{}' không có mã băm SHA256 hợp lệ trong registry (bị thiếu hoặc để SKIP). Đã hủy cài đặt để bảo vệ chuỗi cung ứng.", m.name)
                } else {
                    format!("[-] SECURITY WARNING: Module '{}' lacks a valid SHA256 in the registry. Installation blocked for supply chain integrity.", m.name)
                };
                println!("{}", msg);
                result_messages.push(msg);
                continue;
            }

            let mut hasher = Sha256::new();
            hasher.update(&b);
            let actual_sha = hex::encode(hasher.finalize());
            if actual_sha != expected_sha.to_lowercase() {
                let msg = if vi {
                    format!("[-] CẢNH BÁO BẢO MẬT: Mã SHA256 không khớp cho module '{}'!\n    Mong đợi: {}\n    Thực tế : {}\n    Đã hủy cài đặt.", m.name, expected_sha, actual_sha)
                } else {
                    format!("[-] SECURITY WARNING: SHA256 mismatch for module '{}'!\n    Expected: {}\n    Actual  : {}\n    Installation aborted.", m.name, expected_sha, actual_sha)
                };
                println!("{}", msg);
                result_messages.push(msg);
                continue;
            }
            b
        };

        let dest = vic_base_dir().join("modules").join(&bin_name);
        let tmp_dest = vic_base_dir().join("modules").join(format!("{}.tmp.{}", bin_name, std::process::id()));
        let mut install_ok = false;
        if fs::write(&tmp_dest, &bytes).is_ok() {
            #[cfg(unix)]
            {
                use std::os::unix::fs::PermissionsExt;
                let _ = fs::set_permissions(&tmp_dest, fs::Permissions::from_mode(0o755));
                if fs::rename(&tmp_dest, &dest).is_ok() {
                    install_ok = true;
                } else if fs::copy(&tmp_dest, &dest).is_ok() {
                    let _ = fs::remove_file(&tmp_dest);
                    install_ok = true;
                }
            }
            #[cfg(windows)]
            {
                let old_dest = vic_base_dir().join("modules").join(format!("{}.old", bin_name));
                let _ = fs::remove_file(&old_dest);
                if dest.exists() {
                    let _ = fs::rename(&dest, &old_dest);
                }
                if fs::rename(&tmp_dest, &dest).is_ok() {
                    let _ = fs::remove_file(&old_dest);
                    install_ok = true;
                } else if fs::copy(&tmp_dest, &dest).is_ok() {
                    let _ = fs::remove_file(&tmp_dest);
                    let _ = fs::remove_file(&old_dest);
                    install_ok = true;
                }
            }
        }

        if install_ok {
            let msg = if vi {
                format!("[+] Đã cài đặt module '{}' thành công.", m.name)
            } else {
                format!("[+] Module '{}' installed successfully.", m.name)
            };
            println!("{}", msg);
            result_messages.push(msg);
        } else {
            let _ = fs::remove_file(&tmp_dest);
            let msg = if vi {
                format!("[-] Không thể ghi tệp module '{}' vào thư mục cài đặt.", m.name)
            } else {
                format!("[-] Failed to write module '{}' to install directory.", m.name)
            };
            println!("{}", msg);
            result_messages.push(msg);
        }
    }

    result_messages.join("\n")
}

fn run_external_module(name: &str, args: &[String], lang: &str) {
    let bin_name = if cfg!(target_os = "windows") {
        format!("{}.exe", name)
    } else {
        name.to_string()
    };
    let mod_path = vic_base_dir().join("modules").join(&bin_name);
    if mod_path.exists() {
        let _ = Command::new(mod_path).args(args).status();
    } else {
        println!(
            "{}",
            if lang == "vi" {
                format!("[-] Lệnh hoặc module '{}' chưa được cài. Gõ 'vic list' để kiểm tra.", name)
            } else {
                format!("[-] Command or module '{}' not installed. Type 'vic list' to check.", name)
            }
        );
    }
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
        if ip.is_empty() { "Offline".to_string() } else { ip }
    } else {
        "Not installed".to_string()
    };

    (lan_ip, ts_ip)
}

fn url_decode(input: &str) -> String {
    let mut bytes = Vec::with_capacity(input.len());
    let mut chars = input.as_bytes().iter().copied();
    while let Some(b) = chars.next() {
        if b == b'%' {
            let h1 = chars.next();
            let h2 = chars.next();
            if let (Some(c1), Some(c2)) = (h1, h2) {
                let hex_str = [c1, c2];
                if let Ok(s) = std::str::from_utf8(&hex_str) {
                    if let Ok(val) = u8::from_str_radix(s, 16) {
                        bytes.push(val);
                        continue;
                    }
                }
            }
            bytes.push(b'%');
        } else if b == b'+' {
            bytes.push(b' ');
        } else {
            bytes.push(b);
        }
    }
    String::from_utf8_lossy(&bytes).to_string()
}

fn json_escape(s: &str) -> String {
    let mut out = String::with_capacity(s.len() + 16);
    for c in s.chars() {
        match c {
            '\\' => out.push_str("\\\\"),
            '"' => out.push_str("\\\""),
            '\n' => out.push_str("\\n"),
            '\r' => out.push_str("\\r"),
            '\t' => out.push_str("\\t"),
            '\x08' => out.push_str("\\b"),
            '\x0C' => out.push_str("\\f"),
            c if (c as u32) < 0x20 => {
                out.push_str(&format!("\\u{:04x}", c as u32));
            }
            c => out.push(c),
        }
    }
    out
}

struct SetupItem {
    key: &'static str,
    name: &'static str,
    desc_vi: &'static str,
    desc_en: &'static str,
}

fn get_setup_items() -> Vec<SetupItem> {
    vec![
        SetupItem {
            key: "symlink",
            name: "Symlink toàn cục",
            desc_vi: "Tạo liên kết /usr/local/bin/vic để gõ lệnh 'vic' ở mọi nơi",
            desc_en: "Global symlink /usr/local/bin/vic to run 'vic' everywhere",
        },
        SetupItem {
            key: "udev",
            name: "Quyền độ sáng udev",
            desc_vi: "Phân quyền độ sáng màn hình qua SSH/Web không cần sudo",
            desc_en: "Backlight udev permissions over SSH/Web without sudo",
        },
        SetupItem {
            key: "fastfetch",
            name: "Fastfetch",
            desc_vi: "Hiển thị thông tin phần cứng & hệ điều hành cực nhanh",
            desc_en: "Fast system and hardware information tool",
        },
        SetupItem {
            key: "playerctl",
            name: "Playerctl",
            desc_vi: "Điều khiển trình phát nhạc & video (Spotify, browser...)",
            desc_en: "Media player CLI controller",
        },
        SetupItem {
            key: "brightnessctl",
            name: "Brightnessctl",
            desc_vi: "Điều khiển độ sáng màn hình qua CLI",
            desc_en: "Display brightness controller tool",
        },
        SetupItem {
            key: "clipboard",
            name: "Clipboard",
            desc_vi: "Đồng bộ khay nhớ tạm (wl-clipboard hoặc xclip)",
            desc_en: "Clipboard sync tools (wl-clipboard or xclip)",
        },
        SetupItem {
            key: "screenshot",
            name: "Screenshot",
            desc_vi: "Công cụ chụp màn hình (grim hoặc scrot)",
            desc_en: "Screen capture tools (grim or scrot)",
        },
        SetupItem {
            key: "tailscale",
            name: "Tailscale VPN",
            desc_vi: "Mạng Mesh VPN kết nối thiết bị từ xa an toàn",
            desc_en: "Secure mesh VPN for remote device access",
        },
        SetupItem {
            key: "ydotool",
            name: "Ydotool",
            desc_vi: "Mô phỏng chuột & bàn phím qua daemon (dành cho mouse)",
            desc_en: "Mouse and keyboard simulation CLI daemon",
        },
    ]
}

fn check_setup_item(key: &str) -> bool {
    match key {
        "symlink" => Path::new("/usr/local/bin/vic").exists(),
        "udev" => Path::new("/etc/udev/rules.d/90-backlight.rules").exists(),
        "fastfetch" => cmd_exists("fastfetch"),
        "playerctl" => cmd_exists("playerctl"),
        "brightnessctl" => cmd_exists("brightnessctl"),
        "clipboard" => cmd_exists("wl-copy") || cmd_exists("xclip"),
        "screenshot" => cmd_exists("grim") || cmd_exists("scrot"),
        "tailscale" => cmd_exists("tailscale"),
        "ydotool" => cmd_exists("ydotool"),
        _ => false,
    }
}

fn get_item_packages<'a>(key: &str, is_wayland: bool) -> Vec<&'a str> {
    match key {
        "fastfetch" => vec!["fastfetch"],
        "playerctl" => vec!["playerctl"],
        "brightnessctl" => vec!["brightnessctl"],
        "clipboard" => {
            if is_wayland {
                vec!["wl-clipboard"]
            } else {
                vec!["xclip"]
            }
        }
        "screenshot" => {
            if is_wayland {
                vec!["grim"]
            } else {
                vec!["scrot"]
            }
        }
        "tailscale" => vec!["tailscale"],
        "ydotool" => vec!["ydotool"],
        _ => vec![],
    }
}

fn run_sudo_sh(script: &str) -> bool {
    let mut cmd = Command::new("sudo");
    cmd.args(["sh", "-c", script]);
    if let Ok(tty) = fs::File::open("/dev/tty") {
        cmd.stdin(Stdio::from(tty));
    }
    cmd.status().map(|s| s.success()).unwrap_or(false)
}

fn confirm_action(prompt_vi: &str, prompt_en: &str, is_yes: bool, vi: bool) -> bool {
    if is_yes {
        return true;
    }
    print!("{}", if vi { prompt_vi } else { prompt_en });
    let _ = io::stdout().flush();
    let mut confirm = String::new();
    if io::stdin().read_line(&mut confirm).is_err() || !confirm.trim().eq_ignore_ascii_case("y") {
        println!("{}", if vi { "[-] Đã hủy thao tác." } else { "[-] Canceled." });
        return false;
    }
    true
}

fn get_pkg_install_cmd(distro: &str, pkgs: &[&str]) -> Option<String> {
    if pkgs.is_empty() {
        return None;
    }
    let pkg_list = pkgs.join(" ");
    match distro {
        "arch" | "manjaro" | "endeavouros" | "garuda" | "artix" => {
            Some(format!("pacman -S --needed --noconfirm {}", pkg_list))
        }
        "debian" | "ubuntu" | "linuxmint" | "pop" | "kali" | "raspbian" => {
            Some(format!("apt-get update -y && apt-get install -y {}", pkg_list))
        }
        "fedora" | "rhel" | "centos" | "rocky" | "almalinux" => {
            Some(format!("dnf install -y {}", pkg_list))
        }
        "alpine" => Some(format!("apk add {}", pkg_list)),
        "opensuse" | "opensuse-tumbleweed" | "opensuse-leap" | "sles" => {
            Some(format!("zypper install -y {}", pkg_list))
        }
        "void" => Some(format!("xbps-install -Sy {}", pkg_list)),
        _ => None,
    }
}

fn get_pkg_remove_cmd(distro: &str, pkgs: &[&str]) -> Option<String> {
    if pkgs.is_empty() {
        return None;
    }
    let pkg_list = pkgs.join(" ");
    match distro {
        "arch" | "manjaro" | "endeavouros" | "garuda" | "artix" => {
            Some(format!("pacman -R --noconfirm {}", pkg_list))
        }
        "debian" | "ubuntu" | "linuxmint" | "pop" | "kali" | "raspbian" => {
            Some(format!("apt-get remove -y {}", pkg_list))
        }
        "fedora" | "rhel" | "centos" | "rocky" | "almalinux" => {
            Some(format!("dnf remove -y {}", pkg_list))
        }
        "alpine" => Some(format!("apk del {}", pkg_list)),
        "opensuse" | "opensuse-tumbleweed" | "opensuse-leap" | "sles" => {
            Some(format!("zypper remove -y {}", pkg_list))
        }
        "void" => Some(format!("xbps-remove -Ry {}", pkg_list)),
        _ => None,
    }
}

fn install_setup_item(key: &str, distro: &str, is_wayland: bool, is_yes: bool, vi: bool) -> bool {
    if cfg!(target_os = "windows") {
        if key == "symlink" {
            let bin_dir = vic_base_dir().join("bin");
            let ps = format!(
                "$p = [Environment]::GetEnvironmentVariable('Path', 'User'); \
                 if ($p -notlike '*{}*') {{ [Environment]::SetEnvironmentVariable('Path', \"$p;{}\", 'User') }}",
                bin_dir.display(),
                bin_dir.display()
            );
            let _ = Command::new("powershell").args(["-NoProfile", "-Command", &ps]).status();
            println!("{}", if vi { format!("[+] Đã thêm {} vào User PATH.", bin_dir.display()) } else { format!("[+] Added {} to User PATH.", bin_dir.display()) });
            return true;
        }
        println!("{}", if vi { "[-] Mục này chỉ áp dụng cho Linux." } else { "[-] This component is only for Linux." });
        return false;
    }

    match key {
        "symlink" => {
            let vic_bin = vic_base_dir().join("bin").join("vic");
            if !vic_bin.exists() {
                println!(
                    "{}",
                    if vi {
                        "[-] Không tìm thấy tệp nhị phân ~/.vic/bin/vic để tạo symlink."
                    } else {
                        "[-] Binary ~/.vic/bin/vic not found to create symlink."
                    }
                );
                return false;
            }
            if !confirm_action(
                "[?] Tạo liên kết /usr/local/bin/vic (cần sudo)? [y/N]: ",
                "[?] Create symlink /usr/local/bin/vic (requires sudo)? [y/N]: ",
                is_yes,
                vi,
            ) {
                return false;
            }
            let script = format!("ln -sf '{}' /usr/local/bin/vic", vic_bin.display());
            let ok = run_sudo_sh(&script);
            if ok {
                println!("{}", if vi { "[+] Đã tạo symlink /usr/local/bin/vic thành công." } else { "[+] Successfully created symlink /usr/local/bin/vic." });
            } else {
                println!("{}", if vi { "[-] Tạo symlink thất bại hoặc bị hủy." } else { "[-] Symlink creation failed or cancelled." });
            }
            ok
        }
        "udev" => {
            if !confirm_action(
                "[?] Thiết lập quyền udev độ sáng màn hình (cần sudo)? [y/N]: ",
                "[?] Configure backlight udev permissions (requires sudo)? [y/N]: ",
                is_yes,
                vi,
            ) {
                return false;
            }
            let user = env::var("USER").unwrap_or_else(|_| get_output("whoami", &[]));
            let rule = r#"ACTION=="add", SUBSYSTEM=="backlight", RUN+="/bin/chgrp video /sys/class/backlight/%k/brightness", RUN+="/bin/chmod g+w /sys/class/backlight/%k/brightness""#;
            let script = format!(
                "usermod -aG video {user} 2>/dev/null || true; \
                 printf '%s\\n' '{rule}' > /etc/udev/rules.d/90-backlight.rules; \
                 udevadm control --reload-rules && udevadm trigger; \
                 chgrp video /sys/class/backlight/*/brightness 2>/dev/null || true; \
                 chmod g+w /sys/class/backlight/*/brightness 2>/dev/null || true",
                user = user,
                rule = rule
            );
            let ok = run_sudo_sh(&script);
            if ok {
                println!("{}", if vi { "[+] Đã thiết lập quyền udev độ sáng thành công." } else { "[+] Successfully configured backlight udev permissions." });
            } else {
                println!("{}", if vi { "[-] Thiết lập udev thất bại hoặc bị hủy." } else { "[-] Udev configuration failed or cancelled." });
            }
            ok
        }
        _ => {
            let pkgs = get_item_packages(key, is_wayland);
            if pkgs.is_empty() {
                println!("{}", if vi { format!("[-] Không tìm thấy cấu hình cài đặt cho mục '{}'.", key) } else { format!("[-] No install configuration for item '{}'.", key) });
                return false;
            }
            let Some(cmd_str) = get_pkg_install_cmd(distro, &pkgs) else {
                println!("{}", if vi { format!("[-] Distro '{}' chưa hỗ trợ tự động cài gói. Vui lòng cài thủ công: {}", distro, pkgs.join(" ")) } else { format!("[-] Distro '{}' does not support auto-install. Please install manually: {}", distro, pkgs.join(" ")) });
                return false;
            };
            if !confirm_action(
                &format!("[?] Cài đặt '{}' bằng lệnh 'sudo {}'? [y/N]: ", pkgs.join(" "), cmd_str),
                &format!("[?] Install '{}' via 'sudo {}'? [y/N]: ", pkgs.join(" "), cmd_str),
                is_yes,
                vi,
            ) {
                return false;
            }
            println!("{}", if vi { format!("[*] Đang cài đặt gói: {}", pkgs.join(" ")) } else { format!("[*] Installing package: {}", pkgs.join(" ")) });
            let ok = run_sudo_sh(&cmd_str);
            if ok {
                println!("{}", if vi { format!("[+] Cài đặt '{}' thành công.", key) } else { format!("[+] Successfully installed '{}'.", key) });
            } else {
                println!("{}", if vi { format!("[-] Cài đặt '{}' thất bại hoặc bị hủy.", key) } else { format!("[-] Installation of '{}' failed or cancelled.", key) });
            }
            ok
        }
    }
}

fn remove_setup_item(key: &str, distro: &str, is_wayland: bool, is_yes: bool, vi: bool) -> bool {
    if cfg!(target_os = "windows") {
        println!("{}", if vi { "[-] Mục này chỉ áp dụng cho Linux." } else { "[-] This component is only for Linux." });
        return false;
    }

    match key {
        "symlink" => {
            if !confirm_action(
                "[?] Xóa liên kết /usr/local/bin/vic (cần sudo)? [y/N]: ",
                "[?] Remove symlink /usr/local/bin/vic (requires sudo)? [y/N]: ",
                is_yes,
                vi,
            ) {
                return false;
            }
            let ok = run_sudo_sh("rm -f /usr/local/bin/vic");
            if ok {
                println!("{}", if vi { "[+] Đã xóa symlink /usr/local/bin/vic." } else { "[+] Removed symlink /usr/local/bin/vic." });
            } else {
                println!("{}", if vi { "[-] Xóa symlink thất bại." } else { "[-] Failed to remove symlink." });
            }
            ok
        }
        "udev" => {
            if !confirm_action(
                "[?] Xóa cấu hình udev độ sáng /etc/udev/rules.d/90-backlight.rules (cần sudo)? [y/N]: ",
                "[?] Remove backlight udev rules /etc/udev/rules.d/90-backlight.rules (requires sudo)? [y/N]: ",
                is_yes,
                vi,
            ) {
                return false;
            }
            let ok = run_sudo_sh("rm -f /etc/udev/rules.d/90-backlight.rules && udevadm control --reload-rules && udevadm trigger");
            if ok {
                println!("{}", if vi { "[+] Đã gỡ bỏ cấu hình udev độ sáng." } else { "[+] Removed backlight udev rules." });
            } else {
                println!("{}", if vi { "[-] Gỡ bỏ udev thất bại." } else { "[-] Failed to remove udev rules." });
            }
            ok
        }
        _ => {
            let pkgs = get_item_packages(key, is_wayland);
            if pkgs.is_empty() {
                println!("{}", if vi { format!("[-] Không tìm thấy gói cần gỡ cho mục '{}'.", key) } else { format!("[-] No package found to remove for item '{}'.", key) });
                return false;
            }
            let Some(cmd_str) = get_pkg_remove_cmd(distro, &pkgs) else {
                println!("{}", if vi { format!("[-] Distro '{}' chưa hỗ trợ tự động gỡ gói. Vui lòng gỡ thủ công: {}", distro, pkgs.join(" ")) } else { format!("[-] Distro '{}' does not support auto-remove. Please remove manually: {}", distro, pkgs.join(" ")) });
                return false;
            };
            if !confirm_action(
                &format!("[?] Gỡ bỏ '{}' bằng lệnh 'sudo {}'? [y/N]: ", pkgs.join(" "), cmd_str),
                &format!("[?] Remove '{}' via 'sudo {}'? [y/N]: ", pkgs.join(" "), cmd_str),
                is_yes,
                vi,
            ) {
                return false;
            }
            println!("{}", if vi { format!("[*] Đang gỡ bỏ gói: {}", pkgs.join(" ")) } else { format!("[*] Removing package: {}", pkgs.join(" ")) });
            let ok = run_sudo_sh(&cmd_str);
            if ok {
                println!("{}", if vi { format!("[+] Đã gỡ bỏ '{}' thành công.", key) } else { format!("[+] Successfully removed '{}'.", key) });
            } else {
                println!("{}", if vi { format!("[-] Gỡ bỏ '{}' thất bại hoặc bị hủy.", key) } else { format!("[-] Removal of '{}' failed or cancelled.", key) });
            }
            ok
        }
    }
}

fn setup_clean(vi: bool) {
    let base = vic_base_dir();
    let mut count = 0;
    let mut bytes = 0u64;

    println!(
        "{}",
        if vi {
            "=== DỌN DẸP BỘ NHỚ ĐỆM VÀ TỆP TIN TẠM (VIC CLEAN) ==="
        } else {
            "=== CLEANING VIC CACHE AND TEMPORARY FILES ==="
        }
    );

    if let Ok(entries) = fs::read_dir(&base) {
        for entry in entries.flatten() {
            let path = entry.path();
            if let Some(name) = path.file_name().and_then(|n| n.to_str()) {
                if name.ends_with(".pid")
                    || name.ends_with(".port")
                    || name == "top_cache.txt"
                    || name.starts_with("temp_")
                {
                    if let Ok(meta) = fs::metadata(&path) {
                        bytes += meta.len();
                    }
                    if fs::remove_file(&path).is_ok() {
                        count += 1;
                        println!("[-] Đã xóa: {}", name);
                    }
                }
            }
        }
    }

    let share_dir = base.join("share");
    if let Ok(entries) = fs::read_dir(&share_dir) {
        for entry in entries.flatten() {
            let path = entry.path();
            if let Some(name) = path.file_name().and_then(|n| n.to_str()) {
                if name.starts_with("temp_") || name.ends_with(".tmp") {
                    if let Ok(meta) = fs::metadata(&path) {
                        bytes += meta.len();
                    }
                    if fs::remove_file(&path).is_ok() {
                        count += 1;
                        println!("[-] Đã xóa tệp tạm: share/{}", name);
                    }
                }
            }
        }
    }

    let kb = bytes as f64 / 1024.0;
    println!(
        "{}",
        if vi {
            format!("[+] Hoàn tất! Đã dọn dẹp {} tệp tin tạm ({:.1} KB giải phóng).", count, kb)
        } else {
            format!("[+] Done! Cleaned {} temporary files ({:.1} KB freed).", count, kb)
        }
    );
}

fn cmd_setup(args: &[String], lang: &str) {
    let vi = lang == "vi";
    let is_yes = args.iter().any(|a| a == "-y" || a == "--yes");
    let clean_args: Vec<String> = args
        .iter()
        .filter(|a| *a != "-y" && *a != "--yes")
        .cloned()
        .collect();

    let sub_cmd = clean_args.first().map(|s| s.to_lowercase()).unwrap_or_default();

    if cfg!(target_os = "windows") {
        if sub_cmd == "clean" {
            setup_clean(vi);
            return;
        }
        println!("{}", if vi {
            "=== VIC SETUP (WINDOWS) ==="
        } else {
            "=== VIC SETUP (WINDOWS) ==="
        });
        println!("{}", if vi {
            "[+] Tren Windows, cac lenh he thong cua vic hoat dong truc tiep (Zero-Setup).\n    \
             - Thu muc cai dat: %USERPROFILE%\\.vic\\bin\n    \
             - Bien moi truong PATH: Da tu dong cau hinh\n    \
             - Tailscale: Dam bao app Tailscale da duoc ket noi\n\n    \
             Dung 'vic fast' de kiem tra ket noi, hoac 'vic list' de xem danh sach lenh."
        } else {
            "[+] On Windows, vic core commands operate directly (Zero-Setup).\n    \
             - Install directory: %USERPROFILE%\\.vic\\bin\n    \
             - Environment PATH : Configured automatically\n    \
             - Tailscale        : Ensure Tailscale desktop app is connected\n\n    \
             Run 'vic fast' to check connectivity, or 'vic list' to view available commands."
        });
        return;
    }

    let items = get_setup_items();
    let distro = get_distro_id();
    let is_wayland = env::var("WAYLAND_DISPLAY").is_ok();

    if sub_cmd.is_empty() || sub_cmd == "list" || sub_cmd == "-l" || sub_cmd == "status" {
        println!(
            "{}",
            if vi {
                "=== DANH SÁCH CẤU HÌNH HỆ THỐNG (VIC SETUP) ==="
            } else {
                "=== VIC SYSTEM SETUP CONFIGURATIONS ==="
            }
        );
        println!(
            "{:<16} {:<16} {}",
            if vi { "MỤC" } else { "ITEM" },
            if vi { "TRẠNG THÁI" } else { "STATUS" },
            if vi { "MÔ TẢ" } else { "DESCRIPTION" }
        );
        println!("{}", "-".repeat(78));

        for item in &items {
            let installed = check_setup_item(item.key);
            let status_str = if installed {
                if vi { "[*] [ĐÃ CÓ]" } else { "[*] [READY]" }
            } else {
                if vi { "[ ] [THIẾU]" } else { "[ ] [MISSING]" }
            };
            let desc = if vi { item.desc_vi } else { item.desc_en };
            println!("{:<16} {:<16} {}", item.key, status_str, desc);
        }

        println!("{}", "-".repeat(78));
        println!(
            "{}",
            if vi {
                "Cú pháp sử dụng:\n  \
                 vic setup [list]                Xem danh sách trạng thái các thành phần\n  \
                 vic setup all [-y]              Cài đặt tất cả thành phần còn thiếu\n  \
                 vic setup <mục>                 Cài đặt một thành phần cụ thể (vd: vic setup udev)\n  \
                 vic setup install <mục>         Cài đặt một thành phần cụ thể\n  \
                 vic setup rm <mục>              Gỡ bỏ cấu hình/gói của thành phần (vd: vic setup rm symlink)\n  \
                 vic setup clean                 Dọn dẹp tệp tin tạm, cache và pid cũ"
            } else {
                "Usage:\n  \
                 vic setup [list]                List all setup components and status\n  \
                 vic setup all [-y]              Install all missing components\n  \
                 vic setup <item>                Install a specific component (e.g. vic setup udev)\n  \
                 vic setup install <item>        Install a specific component\n  \
                 vic setup rm <item>             Remove a specific component (e.g. vic setup rm symlink)\n  \
                 vic setup clean                 Clean temporary cache, pid, and port files"
            }
        );
        return;
    }

    if sub_cmd == "all" {
        println!(
            "{}",
            if vi {
                "=== ĐANG TIẾN HÀNH CÀI ĐẶT TOÀN BỘ CẤU HÌNH CẦN THIẾT ==="
            } else {
                "=== INSTALLING ALL REQUIRED SETUP COMPONENTS ==="
            }
        );

        let mut installed_any = false;
        for item in &items {
            if !check_setup_item(item.key) {
                println!(
                    "{}",
                    if vi {
                        format!("--> Đang cài đặt: {} ({})", item.name, item.key)
                    } else {
                        format!("--> Installing: {} ({})", item.name, item.key)
                    }
                );
                install_setup_item(item.key, &distro, is_wayland, is_yes, vi);
                installed_any = true;
            }
        }

        if !installed_any {
            println!(
                "{}",
                if vi {
                    "[+] Tất cả các mục cấu hình cần thiết đều đã có sẵn trên máy của bạn."
                } else {
                    "[+] All required setup components are already installed on your system."
                }
            );
        } else {
            println!(
                "{}",
                if vi {
                    "[+] Hoàn tất quá trình cài đặt setup all!"
                } else {
                    "[+] Completed setup all installation!"
                }
            );
        }
        return;
    }

    if sub_cmd == "rm" || sub_cmd == "remove" || sub_cmd == "uninstall" {
        let target_key = clean_args.get(1).map(|s| s.as_str()).unwrap_or("");
        if target_key.is_empty() {
            println!(
                "{}",
                if vi {
                    "[-] Vui lòng chỉ định mục cần gỡ. Ví dụ: vic setup rm symlink, vic setup rm udev\n    Gõ 'vic setup list' để xem danh sách các mục."
                } else {
                    "[-] Please specify item to remove. Example: vic setup rm symlink, vic setup rm udev\n    Type 'vic setup list' to view all items."
                }
            );
            return;
        }

        if let Some(item) = items.iter().find(|i| i.key == target_key) {
            remove_setup_item(item.key, &distro, is_wayland, is_yes, vi);
        } else {
            println!(
                "{}",
                if vi {
                    format!("[-] Không tìm thấy mục '{}'. Gõ 'vic setup list' để kiểm tra.", target_key)
                } else {
                    format!("[-] Item '{}' not found. Type 'vic setup list' to check.", target_key)
                }
            );
        }
        return;
    }

    let target_key = if sub_cmd == "install" || sub_cmd == "add" {
        clean_args.get(1).map(|s| s.as_str()).unwrap_or("")
    } else {
        &sub_cmd
    };

    if let Some(item) = items.iter().find(|i| i.key == target_key) {
        install_setup_item(item.key, &distro, is_wayland, is_yes, vi);
    } else {
        println!(
            "{}",
            if vi {
                format!("[-] Lệnh setup '{}' không hợp lệ. Gõ 'vic setup list' để xem các mục khả dụng.", target_key)
            } else {
                format!("[-] Invalid setup command '{}'. Type 'vic setup list' to view available items.", target_key)
            }
        );
    }
}

fn get_distro_id() -> String {
    if let Ok(content) = fs::read_to_string("/etc/os-release") {
        for line in content.lines() {
            if let Some(val) = line.strip_prefix("ID=") {
                return val.trim_matches('"').to_lowercase();
            }
        }
    }
    "unknown".to_string()
}

fn doctor_report(lang: &str) -> String {
    let vi = lang == "vi";
    let mut out = String::new();
    out.push_str(if vi { "=== KIỂM TRA CÔNG CỤ HỆ THỐNG ===\n" } else { "=== SYSTEM DEPENDENCY CHECK ===\n" });
    if cfg!(target_os = "linux") {
        out.push_str(&format!("WAYLAND_DISPLAY : {:?}\n", env::var("WAYLAND_DISPLAY").unwrap_or_default()));
        out.push_str(&format!("XDG_RUNTIME_DIR : {:?}\n", env::var("XDG_RUNTIME_DIR").unwrap_or_default()));
    }
    let tools = if cfg!(target_os = "linux") {
        vec!["tailscale", "grim", "scrot", "brightnessctl", "wpctl", "pactl", "wl-copy", "xclip", "fastfetch"]
    } else if cfg!(target_os = "windows") {
        vec!["tailscale", "powershell", "clip"]
    } else {
        vec!["tailscale", "screencapture", "pbcopy", "pbpaste"]
    };
    for t in tools {
        out.push_str(&format!("{:<15} : {}\n", t, if cmd_exists(t) { "[OK]" } else { "[Missing]" }));
    }
    out
}

fn cmd_doctor(lang: &str) {
    print!("{}", doctor_report(lang));
}

fn fetch_latest_version() -> Option<String> {
    let agent = ureq::AgentBuilder::new()
        .timeout(Duration::from_secs(6))
        .build();
    let resp = agent
        .get("https://api.github.com/repos/tpvy/vic-cli/releases/latest")
        .set("User-Agent", "vic-cli")
        .call()
        .ok()?;
    let text = resp.into_string().ok()?;
    if let Some(pos) = text.find("\"tag_name\"") {
        let rest = &text[pos..];
        if let Some(colon) = rest.find(':') {
            let val = rest[colon + 1..].trim();
            if let Some(q1) = val.find('"') {
                if let Some(q2) = val[q1 + 1..].find('"') {
                    return Some(val[q1 + 1..q1 + 1 + q2].to_string());
                }
            }
        }
    }
    None
}

fn is_newer_version(remote: &str, current: &str) -> bool {
    let parse_parts = |s: &str| -> Vec<u32> {
        s.trim_start_matches('v')
            .split('.')
            .filter_map(|p| p.parse::<u32>().ok())
            .collect()
    };
    let r = parse_parts(remote);
    let c = parse_parts(current);
    for (rv, cv) in r.iter().zip(c.iter()) {
        if rv > cv {
            return true;
        } else if rv < cv {
            return false;
        }
    }
    r.len() > c.len()
}

fn cmd_upgrade(args: &[String], lang: &str) {
    let vi = lang == "vi";
    let cur_ver = env!("CARGO_PKG_VERSION");
    let force = args.iter().any(|a| a == "-f" || a == "--force");

    println!("{}", if vi { "[*] Đang kiểm tra phiên bản mới từ máy chủ..." } else { "[*] Checking for updates from server..." });

    let latest_tag = fetch_latest_version();
    if let Some(ref tag) = latest_tag {
        let tag_clean = tag.trim_start_matches('v');
        let has_new = is_newer_version(tag_clean, cur_ver);
        if !force && !has_new {
            println!("[+] {}", if vi {
                format!("VIC đã là phiên bản mới nhất: v{} (Không cần cập nhật).", cur_ver)
            } else {
                format!("VIC is already up-to-date: v{} (No update needed).", cur_ver)
            });
            return;
        }
        println!("[+] {}", if vi {
            format!("Phát hiện phiên bản mới: {} (Hiện tại: v{}).", tag, cur_ver)
        } else {
            format!("New version detected: {} (Current: v{}).", tag, cur_ver)
        });
    } else {
        println!("[*] {}", if vi { "Đang kiểm tra bản cập nhật từ GitHub Releases..." } else { "Checking release from GitHub..." });
    }

    let is_yes = args.iter().any(|a| a == "-y" || a == "--yes");
    if !is_yes {
        let target_name = latest_tag.as_deref().unwrap_or("mới nhất");
        print!("{}", if vi {
            format!("\n[?] Bạn có đồng ý tải và nâng cấp lên phiên bản {} không? [y/N]: ", target_name)
        } else {
            format!("\n[?] Do you agree to download and upgrade to version {}? [y/N]: ", target_name)
        });
        let _ = io::stdout().flush();
        let mut confirm = String::new();
        if io::stdin().read_line(&mut confirm).is_err() || !confirm.trim().eq_ignore_ascii_case("y") {
            println!("{}", if vi { "[-] Đã hủy thao tác nâng cấp." } else { "[-] Upgrade canceled by user." });
            return;
        }
    }

    let target_bin = if cfg!(target_os = "windows") {
        "vic-windows-x86_64.exe"
    } else if cfg!(target_os = "macos") {
        "vic-macos-aarch64"
    } else {
        let arch = get_output("uname", &["-m"]);
        if arch.contains("aarch64") || arch.contains("arm64") {
            "vic-linux-aarch64"
        } else {
            "vic-linux-x86_64"
        }
    };

    let download_url = format!("https://github.com/tpvy/vic-cli/releases/latest/download/{}", target_bin);
    let sha_url = format!("https://github.com/tpvy/vic-cli/releases/latest/download/{}.sha256", target_bin);
    println!(" -> URL: {}", download_url);

    let agent = ureq::AgentBuilder::new()
        .timeout(Duration::from_secs(60))
        .build();

    println!("{}", if vi { "[*] Đang kiểm tra chữ ký xác thực SHA256..." } else { "[*] Checking SHA256 checksum file..." });
    let expected_sha: Option<String> = match agent.get(&sha_url).call() {
        Ok(r) => {
            let mut s = String::new();
            if r.into_reader().read_to_string(&mut s).is_ok() {
                let first = s.split_whitespace().next().unwrap_or("").trim().to_lowercase();
                if first.len() == 64 && first.chars().all(|c| c.is_ascii_hexdigit()) {
                    Some(first)
                } else {
                    None
                }
            } else {
                None
            }
        }
        Err(_) => None,
    };

    let Some(expected) = expected_sha else {
        println!("{}", if vi {
            "[-] CẢNH BÁO BẢO MẬT: Không tìm thấy tệp .sha256 trên Release để đối soát toàn vẹn!\n    Đã hủy nâng cấp để đảm bảo an toàn chuỗi cung ứng."
        } else {
            "[-] SECURITY ERROR: Missing .sha256 checksum file on Release!\n    Upgrade aborted to protect supply chain integrity."
        });
        return;
    };

    println!("{}", if vi { "[*] Đang tải bản cập nhật binary..." } else { "[*] Downloading update binary..." });
    let resp = match agent.get(&download_url).call() {
        Ok(r) => r,
        Err(e) => {
            println!("[-] {}", if vi { format!("Tải bản cập nhật thất bại: {}", e) } else { format!("Failed to download update: {}", e) });
            return;
        }
    };

    let mut bytes = Vec::new();
    if resp.into_reader().read_to_end(&mut bytes).is_err() {
        println!("[-] {}", if vi { "Lỗi đọc dữ liệu bản cập nhật." } else { "Error reading update stream." });
        return;
    }

    let mut hasher = Sha256::new();
    hasher.update(&bytes);
    let actual = hex::encode(hasher.finalize());
    if actual != expected {
        println!("{}", if vi {
            format!("[-] CẢNH BÁO BẢO MẬT: Mã SHA256 không khớp!\n    Mong đợi: {}\n    Thực tế : {}\n    Đã hủy cập nhật để bảo vệ hệ thống.", expected, actual)
        } else {
            format!("[-] SECURITY WARNING: SHA256 checksum mismatch!\n    Expected: {}\n    Actual  : {}\n    Update aborted.", expected, actual)
        });
        return;
    }
    println!("[+] {}", if vi { "Xác minh toàn vẹn SHA256 checksum thành công." } else { "SHA256 checksum verification passed." });

    let bin_name = if cfg!(target_os = "windows") { "vic.exe" } else { "vic" };
    let dest_path = vic_base_dir().join("bin").join(bin_name);
    let tmp_path = vic_base_dir().join("bin").join(format!("{}.tmp", bin_name));

    if fs::write(&tmp_path, &bytes).is_err() {
        println!("[-] {}", if vi { "Không thể ghi tệp tạm thời." } else { "Cannot write temporary file." });
        return;
    }

    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        let _ = fs::set_permissions(&tmp_path, fs::Permissions::from_mode(0o755));
        if fs::rename(&tmp_path, &dest_path).is_err() {
            let _ = fs::copy(&tmp_path, &dest_path);
            let _ = fs::remove_file(&tmp_path);
        }
    }
    #[cfg(windows)]
    {
        let old_path = vic_base_dir().join("bin").join(format!("{}.old", bin_name));
        let _ = fs::remove_file(&old_path);
        if dest_path.exists() {
            let _ = fs::rename(&dest_path, &old_path);
        }
        if fs::rename(&tmp_path, &dest_path).is_err() {
            let _ = fs::copy(&tmp_path, &dest_path);
            let _ = fs::remove_file(&tmp_path);
        }
        let _ = fs::remove_file(&old_path);
    }

    let usr_bin = Path::new("/usr/local/bin/vic");
    if usr_bin.exists() && !usr_bin.is_symlink() {
        let _ = fs::copy(&dest_path, usr_bin);
    }

    sync_remote_md(true, lang);
    println!("{}", if vi { "[+] Cập nhật VIC thành công! Hệ thống đã sẵn sàng." } else { "[+] Successfully updated VIC! Ready to use." });
}

fn format_file_size(bytes: u64) -> String {
    if bytes < 1024 {
        format!("{} B", bytes)
    } else if bytes < 1024 * 1024 {
        format!("{:.1} KB", bytes as f64 / 1024.0)
    } else if bytes < 1024 * 1024 * 1024 {
        format!("{:.1} MB", bytes as f64 / (1024.0 * 1024.0))
    } else {
        format!("{:.2} GB", bytes as f64 / (1024.0 * 1024.0 * 1024.0))
    }
}

fn guess_mime(filename: &str) -> &'static str {
    let lower = filename.to_lowercase();
    if lower.ends_with(".png") {
        "image/png"
    } else if lower.ends_with(".jpg") || lower.ends_with(".jpeg") {
        "image/jpeg"
    } else if lower.ends_with(".gif") {
        "image/gif"
    } else if lower.ends_with(".webp") {
        "image/webp"
    } else if lower.ends_with(".svg") {
        "image/svg+xml"
    } else if lower.ends_with(".bmp") {
        "image/bmp"
    } else if lower.ends_with(".ico") {
        "image/x-icon"
    } else if lower.ends_with(".txt") || lower.ends_with(".log") || lower.ends_with(".md") {
        "text/plain; charset=utf-8"
    } else if lower.ends_with(".html") || lower.ends_with(".htm") {
        "text/html; charset=utf-8"
    } else if lower.ends_with(".json") {
        "application/json"
    } else if lower.ends_with(".pdf") {
        "application/pdf"
    } else if lower.ends_with(".zip") {
        "application/zip"
    } else if lower.ends_with(".mp3") {
        "audio/mpeg"
    } else if lower.ends_with(".mp4") {
        "video/mp4"
    } else {
        "application/octet-stream"
    }
}

fn is_image_ext(filename: &str) -> bool {
    let lower = filename.to_lowercase();
    lower.ends_with(".png")
        || lower.ends_with(".jpg")
        || lower.ends_with(".jpeg")
        || lower.ends_with(".webp")
        || lower.ends_with(".gif")
        || lower.ends_with(".svg")
        || lower.ends_with(".bmp")
        || lower.ends_with(".ico")
}

fn user_home_dir() -> PathBuf {
    let home = env::var("HOME")
        .or_else(|_| env::var("USERPROFILE"))
        .unwrap_or_else(|_| ".".to_string());
    PathBuf::from(home)
}

fn safe_resolve_share_path(p: &str) -> Option<PathBuf> {
    let p = p.trim();
    let home = user_home_dir();
    let canon_home = fs::canonicalize(&home).unwrap_or(home.clone());

    let raw = if p.is_empty() {
        vic_base_dir().join("share")
    } else if p == "~" {
        canon_home.clone()
    } else if let Some(rest) = p.strip_prefix("~/") {
        canon_home.join(rest)
    } else {
        let path = PathBuf::from(p);
        if path.is_relative() {
            vic_base_dir().join("share").join(path)
        } else {
            path
        }
    };

    if raw.exists() {
        let canon = fs::canonicalize(&raw).ok()?;
        if canon.starts_with(&canon_home) {
            Some(canon)
        } else {
            None
        }
    } else if let Some(parent) = raw.parent() {
        let canon_parent = fs::canonicalize(parent).ok()?;
        if canon_parent.starts_with(&canon_home) {
            let fname = raw.file_name()?;
            Some(canon_parent.join(fname))
        } else {
            None
        }
    } else {
        None
    }
}

#[allow(dead_code)]
fn resolve_user_path(p: &str) -> PathBuf {
    safe_resolve_share_path(p).unwrap_or_else(|| vic_base_dir().join("share"))
}

fn is_protected_path(path: &Path) -> bool {
    let home = user_home_dir();
    let canon_home = fs::canonicalize(&home).unwrap_or(home.clone());
    let canon_p = fs::canonicalize(path).unwrap_or_else(|_| {
        if let Some(parent) = path.parent() {
            if let Ok(canon_parent) = fs::canonicalize(parent) {
                if let Some(fname) = path.file_name() {
                    return canon_parent.join(fname);
                }
            }
        }
        path.to_path_buf()
    });

    if canon_p == canon_home {
        return true;
    }

    // Bảo vệ toàn bộ ~/.vic trừ ~/.vic/share
    let vic_base = vic_base_dir();
    let canon_vic = fs::canonicalize(&vic_base).unwrap_or_else(|_| vic_base.clone());
    let vic_share = vic_base.join("share");
    let canon_share = fs::canonicalize(&vic_share).unwrap_or_else(|_| vic_share.clone());

    if (canon_p == canon_vic || canon_p.starts_with(&canon_vic)) && !canon_p.starts_with(&canon_share) {
        return true;
    }

    let sensitive_subdirs = [
        ".ssh", ".gnupg", ".config", ".local/share/keyrings", ".local/bin",
        ".aws", ".kube", ".docker", ".mozilla",
    ];
    for sub in &sensitive_subdirs {
        let sensitive_path = canon_home.join(sub);
        if canon_p == sensitive_path || canon_p.starts_with(&sensitive_path) {
            return true;
        }
    }

    let sensitive_files = [
        ".bashrc", ".bash_profile", ".bash_history", ".profile",
        ".zshrc", ".zprofile", ".zhistory", ".zshenv", ".gitconfig",
        ".netrc", ".npmrc", ".git-credentials", ".pgpass",
    ];
    for f in &sensitive_files {
        let sensitive_path = canon_home.join(f);
        if canon_p == sensitive_path {
            return true;
        }
    }

    if let Some(fname) = canon_p.file_name().and_then(|s| s.to_str()) {
        if fname == ".env" || fname.starts_with(".env.") || fname.ends_with(".env") || fname == ".envrc" {
            return true;
        }
    }

    false
}



fn cmd_screenshot(args: &[String], lang: &str) -> Option<PathBuf> {
    let archive = args.iter().any(|a| a == "-a");
    let send_ts = args.iter().any(|a| a == "-");
    let ts_target_arg: Option<&str> = if let Some(pos) = args.iter().position(|a| a == "-") {
        args.get(pos + 1).map(|s| s.as_str())
    } else {
        None
    };
    let timestamp = Local::now().format("%Y%m%d_%H%M%S");

    let target_dir = if archive {
        vic_base_dir().join("archive")
    } else {
        env::temp_dir()
    };
    let filepath = target_dir.join(format!("sc_{}.png", timestamp));
    let path_str = filepath.to_string_lossy().to_string();

    if cfg!(target_os = "linux") {
        if env::var("WAYLAND_DISPLAY").is_ok() && cmd_exists("grim") {
            let _ = Command::new("grim").arg(&path_str).status();
        } else if cmd_exists("scrot") {
            let _ = Command::new("scrot").arg(&path_str).status();
        } else {
            println!("[-] Cần cài 'grim' (Wayland) hoặc 'scrot' (X11).");
            return None;
        }
    } else if cfg!(target_os = "windows") {
        let ps = format!(
            "Add-Type -AssemblyName System.Windows.Forms,System.Drawing; \
             $b = [System.Windows.Forms.Screen]::PrimaryScreen.Bounds; \
             $bmp = New-Object System.Drawing.Bitmap $b.Width, $b.Height; \
             $g = [System.Drawing.Graphics]::FromImage($bmp); \
             $g.CopyFromScreen($b.Location, [System.Drawing.Point]::Empty, $b.Size); \
             $bmp.Save('{}', [System.Drawing.Imaging.ImageFormat]::Png);",
            path_str.replace('\'', "''")
        );
        let _ = Command::new("powershell").args(["-NoProfile", "-Command", &ps]).status();
    } else if cfg!(target_os = "macos") {
        let _ = Command::new("screencapture").args(["-x", &path_str]).status();
    }

    if filepath.exists() {
        println!(
            "{}: {}",
            if lang == "vi" { "[+] Đã lưu ảnh" } else { "[+] Saved screenshot" },
            path_str
        );
        if send_ts {
            send_via_tailscale(&path_str, ts_target_arg, lang);
            if !archive {
                let _ = fs::remove_file(&filepath);
                return None;
            }
        }
        Some(filepath)
    } else {
        println!("[-] Chụp màn hình thất bại.");
        None
    }
}

fn send_via_tailscale(filepath: &str, target_arg: Option<&str>, lang: &str) {
    if !cmd_exists("tailscale") {
        println!("[-] Chưa cài đặt Tailscale.");
        return;
    }
    let status = get_output("tailscale", &["status"]);
    let mut peers = Vec::new();
    for line in status.lines() {
        let parts: Vec<&str> = line.split_whitespace().collect();
        if parts.len() >= 2 && parts[0].starts_with("100.") {
            peers.push(parts[1].to_string());
        }
    }
    if peers.is_empty() {
        println!("[-] Không tìm thấy thiết bị Tailscale nào đang online.");
        return;
    }

    let target = if let Some(t_arg) = target_arg {
        let trimmed = t_arg.trim();
        if let Ok(idx) = trimmed.parse::<usize>() {
            if idx >= 1 && idx <= peers.len() {
                peers[idx - 1].clone()
            } else {
                trimmed.to_string()
            }
        } else {
            peers
                .iter()
                .find(|p| p.to_lowercase().contains(&trimmed.to_lowercase()))
                .cloned()
                .unwrap_or_else(|| trimmed.to_string())
        }
    } else {
        println!("\n--- Tailscale Peers ---");
        for (i, p) in peers.iter().enumerate() {
            println!("[{}] {}", i + 1, p);
        }
        print!(
            "{}",
            if lang == "vi" {
                "Chọn số thứ tự thiết bị nhận: "
            } else {
                "Select target peer number: "
            }
        );
        let _ = io::stdout().flush();
        let mut input = String::new();
        let _ = io::stdin().read_line(&mut input);
        let choice = input.trim();
        if let Ok(idx) = choice.parse::<usize>() {
            if idx >= 1 && idx <= peers.len() {
                peers[idx - 1].clone()
            } else {
                choice.to_string()
            }
        } else {
            choice.to_string()
        }
    };

    println!("[*] Gửi tới '{}' qua Taildrop...", target);
    let ok = Command::new("tailscale")
        .args(["file", "cp", filepath, &format!("{}:", target)])
        .status()
        .map(|s| s.success())
        .unwrap_or(false);

    println!("{}", if ok { "[+] Đã gửi qua Taildrop!" } else { "[-] Gửi Taildrop thất bại." });
}


fn cmd_fast() {
    let (lan, ts) = get_ips();
    if cmd_exists("fastfetch") {
        let _ = Command::new("fastfetch").status();
    } else if cmd_exists("neofetch") {
        let _ = Command::new("neofetch").status();
    } else {
        println!("OS      : {} ({})", env::consts::OS, env::consts::ARCH);
    }
    println!("LAN IP  : {}", lan);
    println!("TS IP   : {}", ts);
}
fn bind_http_server(bind_ip: &str, requested_port: u16, auto_fallback: bool) -> Result<(Server, u16), String> {
    if auto_fallback {
        for port in requested_port..(requested_port.saturating_add(50)) {
            match Server::http(format!("{}:{}", bind_ip, port)) {
                Ok(srv) => return Ok((srv, port)),
                Err(_) => continue,
            }
        }
        Err(format!("Không thể mở cổng từ {} đến {}", requested_port, requested_port + 50))
    } else {
        match Server::http(format!("{}:{}", bind_ip, requested_port)) {
            Ok(srv) => Ok((srv, requested_port)),
            Err(e) => Err(format!("Không thể mở cổng {}: {}", requested_port, e)),
        }
    }
}

fn find_available_port(bind_ip: &str, requested_port: u16) -> u16 {
    for port in requested_port..(requested_port.saturating_add(50)) {
        if std::net::TcpListener::bind((bind_ip, port)).is_ok() {
            return port;
        }
    }
    requested_port
}

fn get_running_ui() -> Option<(u32, u16)> {
    let pid_file = vic_base_dir().join("ui.pid");
    let port_file = vic_base_dir().join("ui.port");

    if let Ok(pid_str) = fs::read_to_string(&pid_file) {
        if let Ok(pid) = pid_str.trim().parse::<u32>() {
            let is_alive = if cfg!(unix) {
                Command::new("kill")
                    .args(["-0", &pid.to_string()])
                    .status()
                    .map(|s| s.success())
                    .unwrap_or(false)
            } else if cfg!(windows) {
                Command::new("tasklist")
                    .args(["/FI", &format!("PID eq {}", pid)])
                    .output()
                    .map(|o| String::from_utf8_lossy(&o.stdout).contains(&pid.to_string()))
                    .unwrap_or(false)
            } else {
                false
            };

            if is_alive {
                let port = fs::read_to_string(&port_file)
                    .ok()
                    .and_then(|p| p.trim().parse::<u16>().ok())
                    .unwrap_or(3667);
                return Some((pid, port));
            }
        }
    }
    None
}

fn stop_background_ui(lang: &str) -> bool {
    let _ = lang;
    let pid_file = vic_base_dir().join("ui.pid");
    let port_file = vic_base_dir().join("ui.port");
    let mut stopped = false;

    if let Ok(pid_str) = fs::read_to_string(&pid_file) {
        if let Ok(pid) = pid_str.trim().parse::<u32>() {
            if cfg!(unix) {
                let _ = Command::new("kill")
                    .args(["-15", &pid.to_string()])
                    .stdout(Stdio::null())
                    .stderr(Stdio::null())
                    .status();
                std::thread::sleep(std::time::Duration::from_millis(150));
                let _ = Command::new("kill")
                    .args(["-9", &pid.to_string()])
                    .stdout(Stdio::null())
                    .stderr(Stdio::null())
                    .status();
                stopped = true;
            } else if cfg!(windows) {
                let _ = Command::new("taskkill")
                    .args(["/F", "/PID", &pid.to_string()])
                    .stdout(Stdio::null())
                    .stderr(Stdio::null())
                    .status();
                stopped = true;
            }
        }
    }

    if cfg!(unix) {
        let _ = Command::new("pkill")
            .args(["-f", "vic ui.*--internal-worker"])
            .stdout(Stdio::null())
            .stderr(Stdio::null())
            .status();
    }

    let _ = fs::remove_file(&pid_file);
    let _ = fs::remove_file(&port_file);
    let _ = fs::remove_file(vic_base_dir().join("ui.pin"));

    stopped
}

fn status_background_ui(lang: &str) {
    let vi = lang == "vi";
    if let Some((pid, port)) = get_running_ui() {
        let (lan_ip, ts_ip) = get_ips();
        let pin = fs::read_to_string(vic_base_dir().join("ui.pin"))
            .unwrap_or_else(|_| get_cfg("pin", ""))
            .trim()
            .to_string();
        let custom_pass = get_cfg("password", "");
        println!("=== VIC WEB REMOTE DASHBOARD STATUS ===");
        println!(" -> {}   : {}", if vi { "Trạng thái" } else { "Status    " }, if vi { "Đang chạy (Active)" } else { "Running (Active)" });
        println!(" -> Process PID  : {}", pid);
        println!(" -> {} : {}", if vi { "Cổng kết nối" } else { "Server Port " }, port);
        if ts_ip.starts_with("100.") {
            println!(" -> Tailscale URL: http://{}:{}{}", ts_ip, port, if pin.is_empty() { String::new() } else { format!("?pin={}", pin) });
        }
        println!(" -> LAN URL      : http://{}:{}{}", lan_ip, port, if pin.is_empty() { String::new() } else { format!("?pin={}", pin) });
        println!(" -> Security PIN : {} ({})", if pin.is_empty() { if vi { "(Không xác định)" } else { "(Unknown)" } } else { &pin }, if vi { "Mã ngẫu nhiên cho phiên này" } else { "Random session PIN" });
        if !custom_pass.is_empty() {
            println!(" -> {} : {} ({})", if vi { "Mật khẩu riêng" } else { "Custom Pass " }, if vi { "[Đã kích hoạt]" } else { "[Active]" }, if vi { "Có thể dùng thay thế PIN" } else { "Can be used instead of PIN" });
        }
        println!();
        println!("{}", if vi {
            "Dùng lệnh 'vic ui stop' để dừng tiến trình Web UI."
        } else {
            "Use 'vic ui stop' to terminate the background Web UI daemon."
        });
    } else {
        println!("{}", if vi {
            "[-] Web UI hiện đang tắt (không có tiến trình chạy ngầm nào)."
        } else {
            "[-] Web UI is currently stopped (no background daemon running)."
        });
        println!("{}", if vi {
            "Gõ 'vic ui -d' để khởi chạy giao diện Web UI ngầm."
        } else {
            "Run 'vic ui -d' to start Web UI in the background."
        });
    }
}


fn cmd_share(args: &[String], lang: &str) {
    let vi = lang == "vi";
    let use_default = args.iter().any(|a| a == "-d");
    let allow_upload = args.iter().any(|a| a == "-u");

    let mut requested_port: u16 = 3668;
    let mut user_custom_port = false;
    let mut custom_dir: Option<PathBuf> = None;

    for a in args {
        if a.starts_with('-') {
            continue;
        }
        if let Ok(p) = a.parse::<u16>() {
            requested_port = p;
            user_custom_port = true;
        } else {
            custom_dir = Some(PathBuf::from(a));
        }
    }

    let raw_dir = if use_default {
        vic_base_dir().join("share")
    } else {
        custom_dir.unwrap_or_else(|| PathBuf::from("."))
    };

    let Ok(abs_base) = fs::canonicalize(&raw_dir) else {
        println!("[-] Thư mục không tồn tại: {:?}", raw_dir);
        return;
    };

    let (lan_ip, ts_ip) = get_ips();
    println!("{}: {:?}", if vi { "[*] Thư mục chia sẻ" } else { "[*] Sharing folder" }, abs_base);
    println!(
        "{}: {}",
        if vi { " -> Quyền hạn" } else { " -> Permission" },
        if allow_upload { "Đọc + Tải lên (-u)" } else { "Chỉ đọc (Read-Only)" }
    );

    let (server, actual_port) = match bind_http_server("0.0.0.0", requested_port, !user_custom_port) {
        Ok(res) => res,
        Err(e) => {
            eprintln!("[-] {}", e);
            return;
        }
    };
    if actual_port != requested_port {
        println!("[!] Cổng {} đang bận, tự động chuyển sang cổng {}.", requested_port, actual_port);
    }
    println!(" -> LAN URL      : http://{}:{}", lan_ip, actual_port);
    println!(" -> Tailscale URL: http://{}:{}", ts_ip, actual_port);

    for mut request in server.incoming_requests() {
        let raw_url = request.url().to_string();
        let (path_part, query_part) = raw_url.split_once('?').unwrap_or((&raw_url, ""));
        let decoded_rel = url_decode(path_part.trim_start_matches('/'));

        if *request.method() == Method::Post && path_part == "/upload" && allow_upload {
            let mut filename = format!("upload_{}.bin", Local::now().format("%H%M%S"));
            for pair in query_part.split('&') {
                if let Some((k, v)) = pair.split_once('=') {
                    if k == "name" {
                        let dec = url_decode(v);
                        if let Some(safe_name) = Path::new(&dec).file_name() {
                            filename = safe_name.to_string_lossy().to_string();
                        }
                    }
                }
            }
            let dest_file = abs_base.join(&filename);
            if let Ok(mut file) = fs::File::create(&dest_file) {
                let _ = io::copy(request.as_reader(), &mut file);
                let _ = request.respond(Response::from_string("OK"));
            } else {
                let _ = request.respond(Response::from_string("Error").with_status_code(500));
            }
            continue;
        }

        let candidate = abs_base.join(&decoded_rel);
        let Ok(canon_target) = fs::canonicalize(&candidate) else {
            let _ = request.respond(Response::from_string("404 Not Found").with_status_code(404));
            continue;
        };

        if !canon_target.starts_with(&abs_base) {
            let _ = request.respond(Response::from_string("403 Forbidden").with_status_code(403));
            continue;
        }

        if canon_target.is_dir() {
            let upload_ui = if allow_upload {
                r#"<div style="margin:15px 0;padding:12px;border:1px dashed #4b5563;border-radius:8px;">
                    <input type="file" id="f" multiple>
                    <button onclick="up()" style="padding:6px 12px;cursor:pointer;">Upload</button>
                    <span id="st" style="margin-left:10px;"></span>
                   </div>
                   <script>
                   async function up(){
                     const files=document.getElementById('f').files;
                     const st=document.getElementById('st');
                     for(let file of files){
                       st.innerText='Uploading '+file.name+'...';
                       await fetch('/upload?name='+encodeURIComponent(file.name),{method:'POST',body:file});
                     }
                     location.reload();
                   }
                   </script>"#
            } else {
                ""
            };

            let mut html = format!(
                "<html><head><meta charset='utf-8'><meta name='viewport' content='width=device-width,initial-scale=1'>\
                <title>VIC Share</title></head><body style='font-family:sans-serif;background:#111827;color:#f3f4f6;max-width:720px;margin:auto;padding:16px;'>\
                <h3>/{}</h3>{}<hr style='border-color:#374151'><ul style='line-height:1.8'>",
                decoded_rel, upload_ui
            );

            if !decoded_rel.is_empty() {
                html.push_str("<li><a href='..' style='color:#60a5fa'>.. (Back)</a></li>");
            }
            if let Ok(entries) = fs::read_dir(&canon_target) {
                for entry in entries.flatten() {
                    let name = entry.file_name().to_string_lossy().to_string();
                    let is_dir = entry.file_type().map(|t| t.is_dir()).unwrap_or(false);
                    let display = if is_dir { format!("{}/", name) } else { name.clone() };
                    let link = if decoded_rel.is_empty() {
                        format!("/{}", display)
                    } else {
                        format!("/{}/{}", decoded_rel.trim_end_matches('/'), display)
                    };
                    html.push_str(&format!("<li><a href='{}' style='color:#93c5fd;text-decoration:none'>{}</a></li>", link, display));
                }
            }
            html.push_str("</ul></body></html>");

            let header = Header::from_bytes(&b"Content-Type"[..], &b"text/html; charset=utf-8"[..]).unwrap();
            let _ = request.respond(Response::from_string(html).with_header(header));
        } else if canon_target.is_file() {
            if let Ok(file) = fs::File::open(&canon_target) {
                let _ = request.respond(Response::from_file(file));
            }
        }
    }
}


fn cmd_light(args: &[String], lang: &str) {
    let vi = lang == "vi";
    let Some(val) = args.first().and_then(|v| v.parse::<u8>().ok()).filter(|&v| v <= 100) else {
        println!("{}", if vi { "Cú pháp: vic light <0-100>" } else { "Usage: vic light <0-100>" });
        return;
    };
    let mut ok = false;
    if cfg!(target_os = "linux") {
        if cmd_exists("brightnessctl") {
            ok = Command::new("brightnessctl")
                .args(["set", &format!("{}%", val)])
                .status()
                .map(|s| s.success())
                .unwrap_or(false);
        } else {
            println!(
                "{}",
                if vi {
                    "[-] Chưa cài đặt 'brightnessctl'. Hãy chạy 'vic setup' để cài đặt."
                } else {
                    "[-] 'brightnessctl' is not installed. Run 'vic setup' to install."
                }
            );
            return;
        }
    } else if cfg!(target_os = "windows") {
        let ps = format!("(Get-WmiObject -Namespace root/WMI -Class WmiMonitorBrightnessMethods).WmiSetBrightness(1,{})", val);
        ok = Command::new("powershell")
            .args(["-NoProfile", "-Command", &ps])
            .status()
            .map(|s| s.success())
            .unwrap_or(false);
    } else if cfg!(target_os = "macos") {
        if cmd_exists("brightness") {
            let fval = val as f32 / 100.0;
            ok = Command::new("brightness")
                .args([&format!("{:.2}", fval)])
                .status()
                .map(|s| s.success())
                .unwrap_or(false);
        }
    }

    if ok {
        println!("[+] Brightness: {}%", val);
    } else {
        println!(
            "{}",
            if vi {
                "[-] Không thể điều chỉnh độ sáng (thiếu quyền truy cập). Hãy chạy 'vic setup' để cấp quyền."
            } else {
                "[-] Failed to adjust brightness (permission denied). Run 'vic setup' to configure permissions."
            }
        );
    }
}

fn cmd_audio(args: &[String], lang: &str) {
    let Some(val) = args.first() else {
        println!("{}", if lang == "vi" { "Cú pháp: vic au <0-100 | mute>" } else { "Usage: vic au <0-100 | mute>" });
        return;
    };
    if cfg!(target_os = "linux") {
        if cmd_exists("wpctl") {
            if val == "mute" {
                let _ = Command::new("wpctl").args(["set-mute", "@DEFAULT_AUDIO_SINK@", "toggle"]).status();
            } else if let Ok(num) = val.parse::<u8>() {
                let v = num.min(100) as f32 / 100.0;
                let _ = Command::new("wpctl").args(["set-volume", "@DEFAULT_AUDIO_SINK@", &format!("{:.2}", v)]).status();
            }
        } else if cmd_exists("pactl") {
            if val == "mute" {
                let _ = Command::new("pactl").args(["set-sink-mute", "@DEFAULT_SINK@", "toggle"]).status();
            } else if let Ok(num) = val.parse::<u8>() {
                let _ = Command::new("pactl").args(["set-sink-volume", "@DEFAULT_SINK@", &format!("{}%", num.min(100))]).status();
            }
        }
    }
    println!("[+] Audio: {}", val);
}

fn get_clipboard_text() -> String {
    if cfg!(target_os = "linux") {
        if env::var("WAYLAND_DISPLAY").is_ok() && cmd_exists("wl-paste") {
            get_output("wl-paste", &["--no-newline"])
        } else if cmd_exists("xclip") {
            get_output("xclip", &["-selection", "clipboard", "-o"])
        } else {
            String::new()
        }
    } else if cfg!(target_os = "windows") {
        get_output("powershell", &["-NoProfile", "-Command", "Get-Clipboard"])
    } else {
        get_output("pbpaste", &[])
    }
}

fn set_clipboard_text(text: &str) {
    if cfg!(target_os = "linux") {
        if env::var("WAYLAND_DISPLAY").is_ok() && cmd_exists("wl-copy") {
            if let Ok(mut child) = Command::new("wl-copy")
                .stdin(Stdio::piped())
                .stdout(Stdio::null())
                .stderr(Stdio::null())
                .spawn()
            {
                if let Some(mut stdin) = child.stdin.take() {
                    let _ = stdin.write_all(text.as_bytes());
                }
            }
        } else if cmd_exists("xclip") {
            if let Ok(mut child) = Command::new("xclip")
                .args(["-selection", "clipboard"])
                .stdin(Stdio::piped())
                .stdout(Stdio::null())
                .stderr(Stdio::null())
                .spawn()
            {
                if let Some(mut stdin) = child.stdin.take() {
                    let _ = stdin.write_all(text.as_bytes());
                }
            }
        }
    } else if cfg!(target_os = "windows") {
        if let Ok(mut child) = Command::new("clip").stdin(Stdio::piped()).spawn() {
            if let Some(mut stdin) = child.stdin.take() {
                let _ = stdin.write_all(text.as_bytes());
            }
            let _ = child.wait();
        }
    } else if cfg!(target_os = "macos") {
        if let Ok(mut child) = Command::new("pbcopy").stdin(Stdio::piped()).spawn() {
            if let Some(mut stdin) = child.stdin.take() {
                let _ = stdin.write_all(text.as_bytes());
            }
            let _ = child.wait();
        }
    }
}

fn cmd_clip(args: &[String], lang: &str) {
    if args.is_empty() {
        println!("{}", get_clipboard_text());
    } else {
        let text = args.join(" ");
        set_clipboard_text(&text);
        println!("{}", if lang == "vi" { "[+] Đã gửi vào Clipboard." } else { "[+] Copied to Clipboard." });
    }
}

fn cmd_power(action: &str) {
    if action == "lock" {
        if cfg!(target_os = "linux") {
            let _ = Command::new("loginctl").arg("lock-session").status();
        } else if cfg!(target_os = "windows") {
            let _ = Command::new("rundll32.exe").args(["user32.dll,LockWorkStation"]).status();
        } else if cfg!(target_os = "macos") {
            let _ = Command::new("pmset").arg("displaysleepnow").status();
        }
    } else if action == "sleep" {
        if cfg!(target_os = "linux") {
            let _ = Command::new("systemctl").arg("suspend").status();
        } else if cfg!(target_os = "windows") {
            let _ = Command::new("rundll32.exe").args(["powrprof.dll,SetSuspendState", "0,1,0"]).status();
        } else if cfg!(target_os = "macos") {
            let _ = Command::new("pmset").arg("sleepnow").status();
        }
    }
}


fn get_system_status() -> (u32, u32, bool) {
    let mut light = 70;
    let mut volume = 50;
    let mut muted = false;

    if cfg!(target_os = "linux") {
        if cmd_exists("brightnessctl") {
            let cur = get_output("brightnessctl", &["g"]).trim().parse::<f64>().unwrap_or(0.0);
            let max = get_output("brightnessctl", &["m"]).trim().parse::<f64>().unwrap_or(1.0);
            if max > 0.0 {
                light = ((cur / max) * 100.0).round() as u32;
            }
        }
        if cmd_exists("wpctl") {
            let out = get_output("wpctl", &["get-volume", "@DEFAULT_AUDIO_SINK@"]);
            if out.contains("[MUTED]") {
                muted = true;
            }
            for part in out.split_whitespace() {
                if let Ok(v) = part.parse::<f64>() {
                    volume = (v * 100.0).round() as u32;
                    break;
                }
            }
        } else if cmd_exists("pactl") {
            let out = get_output("pactl", &["get-sink-volume", "@DEFAULT_SINK@"]);
            if let Some(pos) = out.find('%') {
                let s = &out[..pos];
                if let Some(word) = s.split_whitespace().last() {
                    if let Ok(v) = word.parse::<u32>() {
                        volume = v;
                    }
                }
            }
        }
    }

    (light, volume, muted)
}

fn cmd_ui(args: &[String], lang: &str) {
    let vi = lang == "vi";

    if let Some(pos) = args.iter().position(|a| a == "pass" || a == "password" || a == "--password") {
        cmd_password(&args[pos + 1..], lang);
        return;
    }

    if args.iter().any(|a| a == "stop" || a == "kill" || a == "--stop") {
        println!("{}", if vi { "[*] Đang dừng Web UI chạy ngầm..." } else { "[*] Stopping background Web UI..." });
        stop_background_ui(lang);
        println!("{}", if vi { "[+] Đã dừng toàn bộ tiến trình Web UI." } else { "[+] Stopped background Web UI." });
        return;
    }

    if args.iter().any(|a| a == "status" || a == "--status") {
        status_background_ui(lang);
        return;
    }

    let is_restart = args.iter().any(|a| a == "restart" || a == "-r" || a == "--restart");
    if is_restart {
        println!("{}", if vi { "[*] Đang khởi động lại Web UI..." } else { "[*] Restarting Web UI..." });
        stop_background_ui(lang);
    }

    let is_daemon = args.iter().any(|a| a == "-d" || a == "bg" || a == "--daemon") || is_restart;
    let is_child = args.iter().any(|a| a == "--internal-worker");
    let bind_all = args.iter().any(|a| a == "--lan");

    let mut requested_port: u16 = 3667;
    let mut user_custom_port = false;

    for a in args {
        if a.starts_with('-') || a == "restart" || a == "bg" {
            continue;
        }
        if let Ok(p) = a.parse::<u16>() {
            requested_port = p;
            user_custom_port = true;
            break;
        }
    }

    let (lan_ip, ts_ip) = get_ips();
    let has_tailscale = ts_ip.starts_with("100.");
    let bind_ip = if bind_all {
        "0.0.0.0"
    } else if has_tailscale {
        &ts_ip
    } else {
        "127.0.0.1"
    };
    let custom_pass = get_cfg("password", "");

    let session_pin = if is_child {
        if let Some(pos) = args.iter().position(|a| a == "--pin") {
            args.get(pos + 1).cloned().unwrap_or_else(|| {
                fs::read_to_string(vic_base_dir().join("ui.pin")).unwrap_or_else(|_| generate_secure_pin())
            })
        } else {
            fs::read_to_string(vic_base_dir().join("ui.pin")).unwrap_or_else(|_| generate_secure_pin())
        }
    } else {
        let p = generate_secure_pin();
        let _ = fs::write(vic_base_dir().join("ui.pin"), &p);
        p
    };
    let pin = session_pin.trim().to_string();

    if is_daemon && !is_child {
        if let Some((old_pid, old_port)) = get_running_ui() {
            println!(
                "{}",
                if vi {
                    format!("[*] Đang dọn dẹp Web UI cũ chạy ngầm (PID: {}, Port: {})...", old_pid, old_port)
                } else {
                    format!("[*] Cleaning up existing Web UI daemon (PID: {}, Port: {})...", old_pid, old_port)
                }
            );
            stop_background_ui(lang);
            std::thread::sleep(std::time::Duration::from_millis(200));
        }

        let target_port = if user_custom_port {
            requested_port
        } else {
            find_available_port(bind_ip, requested_port)
        };

        let exe = env::current_exe().unwrap_or_else(|_| PathBuf::from("vic"));
        let mut child_args = vec![
            "ui".to_string(),
            target_port.to_string(),
            "--internal-worker".to_string(),
            "--pin".to_string(),
            pin.clone(),
        ];
        if bind_all {
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
                let _ = fs::write(vic_base_dir().join("ui.pid"), pid.to_string());
                let _ = fs::write(vic_base_dir().join("ui.port"), target_port.to_string());

                println!("=== VIC WEB REMOTE DASHBOARD (DAEMON) ===");
                if target_port != requested_port {
                    println!(
                        "{}",
                        if vi {
                            format!("[!] Cổng {} đang bận, tự động chuyển sang cổng {}.", requested_port, target_port)
                        } else {
                            format!("[!] Port {} is occupied, automatically assigned port {}.", requested_port, target_port)
                        }
                    );
                }
                println!(" -> Process PID   : {}", pid);
                if bind_all {
                    println!(" -> {}     : {}", if vi { "Lắng nghe" } else { "Listening" }, if vi { "0.0.0.0 (Tất cả card mạng / Mở cho mạng LAN)" } else { "0.0.0.0 (All interfaces / Exposed to LAN)" });
                    println!(
                        "{}",
                        if vi {
                            " -> Cảnh báo     : Bạn đang dùng cờ '--lan'. Không nên dùng trên Wi-Fi công cộng vì Web UI truyền dữ liệu qua HTTP không mã hóa."
                        } else {
                            " -> Warning      : Using '--lan' flag. Avoid public Wi-Fi as Web UI communicates over unencrypted HTTP."
                        }
                    );
                    if has_tailscale {
                        println!(" -> Tailscale URL : http://{}:{}?pin={}", ts_ip, target_port, pin);
                    }
                    println!(" -> LAN URL       : http://{}:{}?pin={}", lan_ip, target_port, pin);
                } else if has_tailscale {
                    println!(" -> {}     : {} ({})", if vi { "Lắng nghe" } else { "Listening" }, ts_ip, if vi { "Chỉ Tailscale - Khóa LAN/Wi-Fi" } else { "Tailscale only - LAN/Wi-Fi blocked" });
                    println!(" -> Tailscale URL : http://{}:{}?pin={}", ts_ip, target_port, pin);
                    println!(" -> {}       : {}", if vi { "Bảo mật" } else { "Security" }, if vi { "Đã chặn truy cập ngoài Tailscale. Thêm cờ '--lan' nếu muốn mở LAN." } else { "External access outside Tailscale blocked. Add '--lan' to expose." });
                } else {
                    println!(" -> {}     : {}", if vi { "Lắng nghe" } else { "Listening" }, if vi { "127.0.0.1 (Chỉ máy cục bộ)" } else { "127.0.0.1 (Localhost only)" });
                    println!(" -> Localhost URL : http://127.0.0.1:{}?pin={}", target_port, pin);
                    println!(" -> {}       : {}", if vi { "Bảo mật" } else { "Security" }, if vi { "Chưa bật Tailscale. Thêm cờ '--lan' nếu muốn mở LAN." } else { "Tailscale not detected. Add '--lan' to expose." });
                }
                println!(" -> Security PIN  : {} ({})", pin, if vi { "Mã ngẫu nhiên cho phiên này" } else { "Random session PIN" });
                if !custom_pass.is_empty() {
                    println!(" -> {} : {} ({})", if vi { "Mật khẩu riêng" } else { "Custom Pass " }, if vi { "[Đã kích hoạt]" } else { "[Active]" }, if vi { "Có thể dùng thay thế PIN" } else { "Can be used instead of PIN" });
                }
                println!(
                    "{}",
                    if vi {
                        "[+] Đã khởi chạy Web UI ngầm thành công. Dùng 'vic ui stop' để dừng."
                    } else {
                        "[+] Web UI running in background. Use 'vic ui stop' to stop."
                    }
                );
                return;
            }
            Err(e) => {
                eprintln!("{}: {}", if vi { "[-] Lỗi khởi chạy daemon" } else { "[-] Failed to start daemon" }, e);
                return;
            }
        }
    }

    if !is_child {
        if let Some((daemon_pid, daemon_port)) = get_running_ui() {
            println!(
                "{}",
                if vi {
                    format!("[!] Cảnh báo: Web UI đang chạy ngầm ở PID {} trên cổng {}.", daemon_pid, daemon_port)
                } else {
                    format!("[!] Warning: Web UI is already running in background (PID: {}, Port: {}).", daemon_pid, daemon_port)
                }
            );
        }
    }

    let (server, actual_port) = match bind_http_server(bind_ip, requested_port, !user_custom_port) {
        Ok(res) => res,
        Err(e) => {
            eprintln!("[-] {}", e);
            return;
        }
    };

    if is_child {
        let pid = std::process::id();
        let _ = fs::write(vic_base_dir().join("ui.pid"), pid.to_string());
        let _ = fs::write(vic_base_dir().join("ui.port"), actual_port.to_string());
    } else {
        println!("=== VIC WEB REMOTE DASHBOARD ===");
        if actual_port != requested_port {
            println!(
                "{}",
                if vi {
                    format!("[!] Cổng {} đang bận, tự động chuyển sang cổng {}.", requested_port, actual_port)
                } else {
                    format!("[!] Port {} is busy, automatically switched to port {}.", requested_port, actual_port)
                }
            );
        }
        if bind_all {
            println!(" -> {}     : {}", if vi { "Lắng nghe" } else { "Listening" }, if vi { "0.0.0.0 (Tất cả card mạng / Mở cho mạng LAN)" } else { "0.0.0.0 (All interfaces / Exposed to LAN)" });
            println!(
                "{}",
                if vi {
                    " -> Cảnh báo     : Bạn đang dùng cờ '--lan'. Không nên dùng trên Wi-Fi công cộng vì Web UI truyền dữ liệu qua HTTP không mã hóa."
                } else {
                    " -> Warning      : Using '--lan' flag. Avoid public Wi-Fi as Web UI communicates over unencrypted HTTP."
                }
            );
            if has_tailscale {
                println!(" -> Tailscale URL : http://{}:{}?pin={}", ts_ip, actual_port, pin);
            }
            println!(" -> LAN URL       : http://{}:{}?pin={}", lan_ip, actual_port, pin);
        } else if has_tailscale {
            println!(" -> {}     : {} ({})", if vi { "Lắng nghe" } else { "Listening" }, ts_ip, if vi { "Chỉ Tailscale - Khóa LAN/Wi-Fi" } else { "Tailscale only - LAN/Wi-Fi blocked" });
            println!(" -> Tailscale URL : http://{}:{}?pin={}", ts_ip, actual_port, pin);
            println!(" -> {}       : {}", if vi { "Bảo mật" } else { "Security" }, if vi { "Đã chặn truy cập ngoài Tailscale. Thêm cờ '--lan' nếu muốn mở LAN." } else { "External access outside Tailscale blocked. Add '--lan' to expose." });
        } else {
            println!(" -> {}     : {}", if vi { "Lắng nghe" } else { "Listening" }, if vi { "127.0.0.1 (Chỉ máy cục bộ)" } else { "127.0.0.1 (Localhost only)" });
            println!(" -> Localhost URL : http://127.0.0.1:{}?pin={}", actual_port, pin);
            println!(" -> {}       : {}", if vi { "Bảo mật" } else { "Security" }, if vi { "Chưa bật Tailscale. Thêm cờ '--lan' nếu muốn mở LAN." } else { "Tailscale not detected. Add '--lan' to expose." });
        }
        println!(" -> Security PIN  : {} ({})", pin, if vi { "Mã ngẫu nhiên cho phiên này" } else { "Random session PIN" });
        if !custom_pass.is_empty() {
            println!(" -> {} : {} ({})", if vi { "Mật khẩu riêng" } else { "Custom Pass " }, if vi { "[Đã kích hoạt]" } else { "[Active]" }, if vi { "Có thể dùng thay thế PIN" } else { "Can be used instead of PIN" });
        }
    }

    struct ClientAttempt {
        fails: u32,
        last_attempt: std::time::Instant,
        locked_until: Option<std::time::Instant>,
    }
    let mut guard: std::collections::HashMap<String, ClientAttempt> = std::collections::HashMap::new();

    for mut req in server.incoming_requests() {
        let client_ip = req.remote_addr().map(|a| a.ip().to_string()).unwrap_or_else(|| "127.0.0.1".to_string());
        let url = req.url().to_string();
        let (path, query) = url.split_once('?').unwrap_or((&url, ""));

        if path == "/" {
            let html = render_dashboard_html();
            let h = Header::from_bytes(&b"Content-Type"[..], &b"text/html; charset=utf-8"[..]).unwrap();
            let _ = req.respond(Response::from_string(html).with_header(h));
            continue;
        }

        let now = std::time::Instant::now();
        if let Some(attempt) = guard.get(&client_ip) {
            if let Some(lock_time) = attempt.locked_until {
                if now < lock_time {
                    let remaining = (lock_time - now).as_secs();
                    let msg = format!(
                        "Quá nhiều lần thử sai. Tạm thời khóa IP {} trong {} giây nữa (Anti-brute-force).",
                        client_ip, remaining
                    );
                    let h = Header::from_bytes(&b"Content-Type"[..], &b"text/plain; charset=utf-8"[..]).unwrap();
                    let _ = req.respond(Response::from_string(msg).with_status_code(429).with_header(h));
                    continue;
                }
            }
        }

        let mut req_token = String::new();
        for h in req.headers() {
            let hname = h.field.as_str().to_ascii_lowercase();
            if hname == "x-vic-auth" || hname == "x-vic-pin" {
                req_token = h.value.as_str().trim().to_string();
                break;
            } else if hname == "authorization" {
                let val = h.value.as_str().trim();
                if let Some(bearer) = val.strip_prefix("Bearer ").or_else(|| val.strip_prefix("bearer ")) {
                    req_token = bearer.trim().to_string();
                    break;
                }
            } else if hname == "cookie" {
                for cookie in h.value.as_str().split(';') {
                    if let Some((k, v)) = cookie.trim().split_once('=') {
                        if k.trim() == "vic_auth" {
                            req_token = url_decode(v.trim());
                            break;
                        }
                    }
                }
            }
        }

        let mut param_val = String::new();
        let mut param_name = String::new();
        let mut param_path = String::new();
        let mut param_dir = String::new();
        let mut param_cmd = String::new();
        let mut param_cwd = String::new();
        for pair in query.split('&') {
            if let Some((k, v)) = pair.split_once('=') {
                if (k == "pin" || k == "pass" || k == "password") && req_token.is_empty() {
                    req_token = url_decode(v);
                } else if k == "v" {
                    param_val = url_decode(v);
                } else if k == "name" {
                    param_name = url_decode(v);
                } else if k == "path" {
                    param_path = url_decode(v);
                } else if k == "dir" {
                    param_dir = url_decode(v);
                } else if k == "cmd" {
                    param_cmd = url_decode(v);
                } else if k == "cwd" {
                    param_cwd = url_decode(v);
                }
            }
        }

        let cur_custom_pass = get_cfg("password", "");
        let auth_ok = !req_token.is_empty() && (req_token == pin || (!cur_custom_pass.is_empty() && req_token == cur_custom_pass));

        if !auth_ok {
            let entry = guard.entry(client_ip.clone()).or_insert(ClientAttempt {
                fails: 0,
                last_attempt: now,
                locked_until: None,
            });

            if now.duration_since(entry.last_attempt) < Duration::from_millis(1000) && entry.fails > 0 {
                let msg = "Vui lòng đợi 1 giây giữa các lần thử đăng nhập (Rate limit).";
                let h = Header::from_bytes(&b"Content-Type"[..], &b"text/plain; charset=utf-8"[..]).unwrap();
                let _ = req.respond(Response::from_string(msg).with_status_code(429).with_header(h));
                continue;
            }
            entry.last_attempt = now;
            entry.fails += 1;

            if entry.fails >= 5 {
                entry.locked_until = Some(now + Duration::from_secs(300));
                entry.fails = 0;
                let msg = "Đã thử sai 5 lần. Tạm khóa IP 5 phút để bảo vệ hệ thống khỏi brute-force.";
                let h = Header::from_bytes(&b"Content-Type"[..], &b"text/plain; charset=utf-8"[..]).unwrap();
                let _ = req.respond(Response::from_string(msg).with_status_code(429).with_header(h));
            } else {
                let msg = format!("Sai PIN hoặc Mật khẩu ({}/5 lần thử trước khi khóa 5 phút).", entry.fails);
                let h = Header::from_bytes(&b"Content-Type"[..], &b"text/plain; charset=utf-8"[..]).unwrap();
                let _ = req.respond(Response::from_string(msg).with_status_code(401).with_header(h));
            }
            continue;
        } else {
            if let Some(entry) = guard.get_mut(&client_ip) {
                entry.fails = 0;
                entry.locked_until = None;
            }
        }

        match path {
            "/api/status" => {
                let (light, volume, muted) = get_system_status();
                let json = format!(
                    r#"{{"light":{},"volume":{},"muted":{}}}"#,
                    light, volume, muted
                );
                let h = Header::from_bytes(&b"Content-Type"[..], &b"application/json; charset=utf-8"[..]).unwrap();
                let _ = req.respond(Response::from_string(json).with_header(h));
            }
            "/api/light" => {
                cmd_light(&[param_val], lang);
                let _ = req.respond(Response::from_string("OK"));
            }
            "/api/au" => {
                cmd_audio(&[param_val], lang);
                let _ = req.respond(Response::from_string("OK"));
            }
            "/api/clip" => {
                if *req.method() == Method::Post {
                    let mut body = String::new();
                    let _ = req.as_reader().read_to_string(&mut body);
                    set_clipboard_text(&body);
                    let _ = req.respond(Response::from_string("OK"));
                } else {
                    let _ = req.respond(Response::from_string(get_clipboard_text()));
                }
            }
            "/api/sc" => {
                if let Some(img_path) = cmd_screenshot(&[], lang) {
                    if let Ok(bytes) = fs::read(&img_path) {
                        let _ = fs::remove_file(&img_path);
                        let h = Header::from_bytes(&b"Content-Type"[..], &b"image/png"[..]).unwrap();
                        let _ = req.respond(Response::from_data(bytes).with_header(h));
                        continue;
                    }
                }
                let _ = req.respond(Response::from_string("Error").with_status_code(500));
            }
            "/api/power" => {
                cmd_power(&param_val);
                let _ = req.respond(Response::from_string("OK"));
            }
            "/api/fast" => {
                let (lan, ts) = get_ips();
                let sys_info = get_output("uname", &["-srmo"]);
                let host = get_output("hostname", &[]);
                let mut out = format!("Host: {}\nOS: {}\nLAN IP: {}\nTailscale IP: {}\n", host, sys_info, lan, ts);
                if cmd_exists("fastfetch") {
                    let ff = get_output("fastfetch", &["--pipe", "true"]);
                    if !ff.is_empty() {
                        out.push_str("\n--- Fastfetch ---\n");
                        out.push_str(&ff);
                    }
                }
                let _ = req.respond(Response::from_string(out));
            }
            "/api/doctor" => {
                let rep = doctor_report(lang);
                let _ = req.respond(Response::from_string(rep));
            }
            "/api/share/list" => {
                let target_dir = if !param_dir.is_empty() {
                    safe_resolve_share_path(&param_dir)
                } else if !param_path.is_empty() {
                    safe_resolve_share_path(&param_path)
                } else if !param_val.is_empty() {
                    safe_resolve_share_path(&param_val)
                } else {
                    None
                }.unwrap_or_else(|| {
                    let share = vic_base_dir().join("share");
                    let _ = fs::create_dir_all(&share);
                    share
                });

                let home = user_home_dir();
                let canon_home = fs::canonicalize(&home).unwrap_or(home);
                if target_dir != canon_home && is_protected_path(&target_dir) {
                    let _ = req.respond(Response::from_string("Thư mục được bảo vệ hệ thống").with_status_code(403));
                    continue;
                }

                let current_str = target_dir.to_string_lossy().to_string();
                let parent_str = target_dir
                    .parent()
                    .map(|p| p.to_string_lossy().to_string())
                    .unwrap_or_default();

                let mut items = Vec::new();
                if let Ok(entries) = fs::read_dir(&target_dir) {
                    let mut dir_items = Vec::new();
                    let mut file_items = Vec::new();

                    for entry in entries.flatten() {
                        let path = entry.path();
                        if is_protected_path(&path) {
                            continue;
                        }
                        if let Ok(meta) = entry.metadata() {
                            let name = entry.file_name().to_string_lossy().to_string();
                            let item_path = path.to_string_lossy().to_string();
                            let is_dir = meta.is_dir();
                            if is_dir {
                                dir_items.push((name, item_path));
                            } else if meta.is_file() {
                                let len = meta.len();
                                let size_str = format_file_size(len);
                                let is_img = is_image_ext(&name);
                                file_items.push((name, item_path, size_str, len, is_img));
                            }
                        }
                    }

                    dir_items.sort_by(|a, b| a.0.to_lowercase().cmp(&b.0.to_lowercase()));
                    file_items.sort_by(|a, b| a.0.to_lowercase().cmp(&b.0.to_lowercase()));

                    for (name, item_path) in dir_items {
                        items.push(format!(
                            r#"{{"name":"{}","path":"{}","is_dir":true,"size":"<DIR>","bytes":0,"is_image":false}}"#,
                            name.replace('\\', "\\\\").replace('"', "\\\""),
                            item_path.replace('\\', "\\\\").replace('"', "\\\"")
                        ));
                    }

                    for (name, item_path, size_str, len, is_img) in file_items {
                        items.push(format!(
                            r#"{{"name":"{}","path":"{}","is_dir":false,"size":"{}","bytes":{},"is_image":{}}}"#,
                            name.replace('\\', "\\\\").replace('"', "\\\""),
                            item_path.replace('\\', "\\\\").replace('"', "\\\""),
                            size_str,
                            len,
                            is_img
                        ));
                    }
                }

                let body = format!(
                    r#"{{"current":"{}","parent":"{}","items":[{}]}}"#,
                    current_str.replace('\\', "\\\\").replace('"', "\\\""),
                    parent_str.replace('\\', "\\\\").replace('"', "\\\""),
                    items.join(",")
                );
                let h = Header::from_bytes(&b"Content-Type"[..], &b"application/json; charset=utf-8"[..]).unwrap();
                let _ = req.respond(Response::from_string(body).with_header(h));
            }
            "/api/share/preview" => {
                let target = if !param_path.is_empty() {
                    param_path
                } else if !param_name.is_empty() {
                    param_name
                } else {
                    param_val
                };
                let Some(file_path) = safe_resolve_share_path(&target) else {
                    let _ = req.respond(Response::from_string("Truy cập bị từ chối").with_status_code(403));
                    continue;
                };
                if is_protected_path(&file_path) {
                    let _ = req.respond(Response::from_string("Tệp tin được bảo vệ hệ thống").with_status_code(403));
                    continue;
                }
                if !file_path.is_file() {
                    let _ = req.respond(Response::from_string("File not found").with_status_code(404));
                    continue;
                }
                if let Ok(data) = fs::read(&file_path) {
                    let fname = file_path.file_name().map(|s| s.to_string_lossy().to_string()).unwrap_or_default();
                    let mime = guess_mime(&fname);
                    let h_type = Header::from_bytes(&b"Content-Type"[..], mime.as_bytes()).unwrap();
                    let h_cache = Header::from_bytes(&b"Cache-Control"[..], &b"no-cache"[..]).unwrap();
                    let _ = req.respond(
                        Response::from_data(data)
                            .with_header(h_type)
                            .with_header(h_cache),
                    );
                    continue;
                } else {
                    let _ = req.respond(Response::from_string("Error reading file").with_status_code(500));
                }
            }
            "/api/share/download" => {
                let target = if !param_path.is_empty() {
                    param_path
                } else if !param_name.is_empty() {
                    param_name
                } else {
                    param_val
                };
                let Some(file_path) = safe_resolve_share_path(&target) else {
                    let _ = req.respond(Response::from_string("Truy cập bị từ chối").with_status_code(403));
                    continue;
                };
                if is_protected_path(&file_path) {
                    let _ = req.respond(Response::from_string("Tệp tin được bảo vệ hệ thống").with_status_code(403));
                    continue;
                }
                if !file_path.is_file() {
                    let _ = req.respond(Response::from_string("File not found").with_status_code(404));
                    continue;
                }
                let safe_name = file_path
                    .file_name()
                    .map(|s| s.to_string_lossy().to_string())
                    .unwrap_or_else(|| "download.bin".to_string());
                if let Ok(data) = fs::read(&file_path) {
                    let mime = guess_mime(&safe_name);
                    let h_type = Header::from_bytes(&b"Content-Type"[..], mime.as_bytes()).unwrap();
                    let disp = format!("attachment; filename=\"{}\"", safe_name.replace('"', ""));
                    let h_disp = Header::from_bytes(&b"Content-Disposition"[..], disp.as_bytes()).unwrap();
                    let _ = req.respond(
                        Response::from_data(data)
                            .with_header(h_type)
                            .with_header(h_disp),
                    );
                    continue;
                } else {
                    let _ = req.respond(Response::from_string("Error reading file").with_status_code(500));
                }
            }
            "/api/share/upload" => {
                let raw_dir = if !param_dir.is_empty() {
                    param_dir
                } else if !param_path.is_empty() {
                    param_path
                } else {
                    String::new()
                };
                let target_dir = safe_resolve_share_path(&raw_dir)
                    .unwrap_or_else(|| vic_base_dir().join("share"));
                let target_dir = if target_dir.is_dir() {
                    target_dir
                } else {
                    target_dir.parent().map(|p| p.to_path_buf()).unwrap_or(target_dir)
                };
                if is_protected_path(&target_dir) {
                    let _ = req.respond(Response::from_string("Thư mục được bảo vệ hệ thống").with_status_code(403));
                    continue;
                }
                let _ = fs::create_dir_all(&target_dir);
                let target_name = if !param_name.is_empty() {
                    param_name
                } else if !param_val.is_empty() {
                    param_val
                } else {
                    format!("upload_{}.bin", Local::now().format("%Y%m%d_%H%M%S"))
                };
                let safe_name = Path::new(&target_name)
                    .file_name()
                    .map(|s| s.to_string_lossy().to_string())
                    .unwrap_or_else(|| format!("upload_{}.bin", Local::now().format("%Y%m%d_%H%M%S")));
                let dest = target_dir.join(&safe_name);
                if is_protected_path(&dest) {
                    let _ = req.respond(Response::from_string("Tệp tin được bảo vệ hệ thống").with_status_code(403));
                    continue;
                }
                if let Ok(mut f) = fs::File::create(&dest) {
                    if io::copy(req.as_reader(), &mut f).is_ok() {
                        let _ = req.respond(Response::from_string("OK"));
                    } else {
                        let _ = req.respond(Response::from_string("Error writing file").with_status_code(500));
                    }
                } else {
                    let _ = req.respond(Response::from_string("Cannot create file").with_status_code(500));
                }
            }
            "/api/share/delete" => {
                let target = if !param_path.is_empty() {
                    param_path
                } else if !param_name.is_empty() {
                    param_name
                } else {
                    param_val
                };
                let Some(target_path) = safe_resolve_share_path(&target) else {
                    let _ = req.respond(Response::from_string("Truy cập bị từ chối").with_status_code(403));
                    continue;
                };
                if is_protected_path(&target_path) {
                    let _ = req.respond(Response::from_string("Tệp tin/thư mục được bảo vệ hệ thống").with_status_code(403));
                    continue;
                }
                if target_path.is_file() {
                    if fs::remove_file(&target_path).is_ok() {
                        let _ = req.respond(Response::from_string("OK"));
                    } else {
                        let _ = req.respond(Response::from_string("Cannot delete file").with_status_code(500));
                    }
                } else if target_path.is_dir() {
                    let share_dir = vic_base_dir().join("share");
                    let canon_share = fs::canonicalize(&share_dir).unwrap_or(share_dir);
                    if target_path.starts_with(&canon_share) && target_path != canon_share {
                        if fs::remove_dir_all(&target_path).is_ok() {
                            let _ = req.respond(Response::from_string("OK"));
                        } else {
                            let _ = req.respond(Response::from_string("Cannot delete directory").with_status_code(500));
                        }
                    } else {
                        if fs::remove_dir(&target_path).is_ok() {
                            let _ = req.respond(Response::from_string("OK"));
                        } else {
                            let _ = req.respond(Response::from_string("Chỉ cho phép xóa thư mục rỗng ngoài ~/.vic/share").with_status_code(400));
                        }
                    }
                } else {
                    let _ = req.respond(Response::from_string("Path not found").with_status_code(404));
                }
            }
            "/api/md" => {
                let md = fs::read_to_string(vic_base_dir().join("module.md"))
                    .unwrap_or_else(|_| DEFAULT_MODULE_MD.to_string());
                let _ = req.respond(Response::from_string(md));
            }
            "/api/catalog" => {
                let entries = parse_module_md();
                let mut json_items = Vec::new();
                for e in entries {
                    let inst = if e.entry_type == "core" {
                        true
                    } else {
                        is_module_installed(&e.name)
                    };
                    let title = if lang == "vi" { &e.vi_short } else { &e.en_short };
                    let desc = if lang == "vi" { &e.vi_desc } else { &e.en_desc };
                    let item = format!(
                        r#"{{"id":{},"name":"{}","type":"{}","installed":{},"title":"{}","desc":"{}","usage":"{}","actions":"{}"}}"#,
                        e.id,
                        e.name.replace('"', "\\\""),
                        e.entry_type.replace('"', "\\\""),
                        inst,
                        title.replace('"', "\\\""),
                        desc.replace('"', "\\\""),
                        e.usage.replace('"', "\\\""),
                        e.ui_actions.replace('"', "\\\"")
                    );
                    json_items.push(item);
                }
                let body = format!("[{}]", json_items.join(","));
                let h = Header::from_bytes(&b"Content-Type"[..], &b"application/json; charset=utf-8"[..]).unwrap();
                let _ = req.respond(Response::from_string(body).with_header(h));
            }
            "/api/modules" => {
                let entries = parse_module_md();
                let mut json_items = Vec::new();
                for e in &entries {
                    if e.entry_type == "core" {
                        continue;
                    }
                    let inst = is_module_installed(&e.name);
                    let title = if lang == "vi" { &e.vi_short } else { &e.en_short };
                    let desc = if lang == "vi" { &e.vi_desc } else { &e.en_desc };
                    let item = format!(
                        r#"{{"id":{},"name":"{}","installed":{},"title":"{}","desc":"{}","usage":"{}","actions":"{}"}}"#,
                        e.id,
                        e.name.replace('"', "\\\""),
                        inst,
                        title.replace('"', "\\\""),
                        desc.replace('"', "\\\""),
                        e.usage.replace('"', "\\\""),
                        e.ui_actions.replace('"', "\\\"")
                    );
                    json_items.push(item);
                }
                if !entries.iter().any(|e| e.name == "mouse") && is_module_installed("mouse") {
                    let item = format!(
                        r#"{{"id":99,"name":"mouse","installed":true,"title":"{}","desc":"{}","usage":"{}","actions":"{}"}}"#,
                        if lang == "vi" { "Điều khiển Chuột & Chuột bay (Thử nghiệm)" } else { "Mobile Air Mouse & Virtual Trackpad (Beta)" },
                        if lang == "vi" { "Điều khiển chuột máy tính bằng cảm ứng và con quay trên điện thoại" } else { "Control mouse with mobile gyro and touchpad" },
                        "vic mouse [-d|stop|status|center|click|scroll]",
                        if lang == "vi" {
                            "btn:Chạy nền=-d|btn:Dừng=stop|btn:Trạng thái=status|btn:Về giữa=center|btn:Click trái=click l|btn:Click phải=click r|btn:Cuộn lên=scroll 2|btn:Cuộn xuống=scroll -2"
                        } else {
                            "btn:Run Daemon=-d|btn:Stop=stop|btn:Status=status|btn:Center=center|btn:Left Click=click l|btn:Right Click=click r|btn:Scroll Up=scroll 2|btn:Scroll Down=scroll -2"
                        }
                    );
                    json_items.push(item);
                }
                let body = format!("[{}]", json_items.join(","));
                let h = Header::from_bytes(&b"Content-Type"[..], &b"application/json; charset=utf-8"[..]).unwrap();
                let _ = req.respond(Response::from_string(body).with_header(h));
            }
            "/api/mouse/info" => {
                let base = vic_base_dir();
                let pid_file = base.join("mouse.pid");
                let port_file = base.join("mouse.port");
                let running = pid_file.exists();
                let port = fs::read_to_string(&port_file).unwrap_or_else(|_| "3669".to_string()).trim().to_string();
                let (lan_ip, ts_ip) = get_ips();
                let body = format!(
                    r#"{{"running":{},"port":"{}","lan_ip":"{}","ts_ip":"{}"}}"#,
                    running,
                    port,
                    lan_ip,
                    ts_ip
                );
                let h = Header::from_bytes(&b"Content-Type"[..], &b"application/json; charset=utf-8"[..]).unwrap();
                let _ = req.respond(Response::from_string(body).with_header(h));
            }
            "/api/install" => {
                let res = cmd_install(&[param_val, "-y".to_string()], lang);
                let _ = req.respond(Response::from_string(res));
            }
            "/api/exec" => {
                let (raw_name, raw_args) = if !param_name.is_empty() {
                    (param_name.as_str(), param_val.as_str())
                } else {
                    param_val.split_once('|').unwrap_or((&param_val, ""))
                };
                let mod_name = raw_name.trim();
                let mod_args_str = raw_args.trim();
                if mod_name.is_empty() || !mod_name.chars().all(|c| c.is_ascii_alphanumeric() || c == '-' || c == '_') {
                    let _ = req.respond(Response::from_string("Tên module không hợp lệ").with_status_code(400));
                    continue;
                }
                let bin_name = if cfg!(target_os = "windows") {
                    format!("{}.exe", mod_name)
                } else {
                    mod_name.to_string()
                };
                let modules_dir = vic_base_dir().join("modules");
                let canon_modules = fs::canonicalize(&modules_dir).unwrap_or(modules_dir);
                let mod_path = canon_modules.join(&bin_name);
                if let Ok(canon_mod_path) = fs::canonicalize(&mod_path) {
                    if !canon_mod_path.starts_with(&canon_modules) {
                        let _ = req.respond(Response::from_string("Truy cập bị từ chối").with_status_code(403));
                        continue;
                    }
                    let args_vec: Vec<&str> = mod_args_str.split_whitespace().collect();
                    let output = Command::new(canon_mod_path).args(&args_vec).output();
                    match output {
                        Ok(o) => {
                            let mut text = String::from_utf8_lossy(&o.stdout).to_string();
                            let err = String::from_utf8_lossy(&o.stderr);
                            if !err.is_empty() {
                                text.push_str(&err);
                            }
                            if text.trim().is_empty() {
                                text = "[OK] Executed.".to_string();
                            }
                            let _ = req.respond(Response::from_string(text));
                        }
                        Err(e) => {
                            let _ = req.respond(
                                Response::from_string(format!("Error: {}", e)).with_status_code(500),
                            );
                        }
                    }
                } else {
                    let _ = req.respond(Response::from_string("Module not installed").with_status_code(404));
                }
            }
            "/api/terminal/exec" => {
                let raw_cmd = if !param_cmd.is_empty() {
                    param_cmd
                } else if !param_val.is_empty() {
                    param_val
                } else {
                    String::new()
                };

                let trimmed_cmd = raw_cmd.trim();
                if trimmed_cmd.is_empty() {
                    let h = Header::from_bytes(&b"Content-Type"[..], &b"application/json; charset=utf-8"[..]).unwrap();
                    let _ = req.respond(Response::from_string(r#"{"output":"","cwd":"","code":0}"#).with_header(h));
                    continue;
                }

                let mut work_dir = if !param_cwd.is_empty() {
                    let p = if param_cwd.starts_with('~') {
                        if let Some(rest) = param_cwd.strip_prefix('~') {
                            let rest_clean = rest.trim_start_matches('/').trim_start_matches('\\');
                            user_home_dir().join(rest_clean)
                        } else {
                            user_home_dir()
                        }
                    } else {
                        PathBuf::from(&param_cwd)
                    };
                    fs::canonicalize(&p).unwrap_or(p)
                } else {
                    user_home_dir()
                };

                if !work_dir.is_dir() {
                    work_dir = user_home_dir();
                }

                // Xử lý lệnh `cd`
                if trimmed_cmd == "cd" || trimmed_cmd.starts_with("cd ") {
                    let dest = trimmed_cmd.strip_prefix("cd").unwrap_or("").trim();
                    let new_path = if dest.is_empty() || dest == "~" {
                        user_home_dir()
                    } else if dest == "-" {
                        user_home_dir()
                    } else if dest.starts_with('~') {
                        let rest = dest.strip_prefix('~').unwrap_or("").trim_start_matches('/').trim_start_matches('\\');
                        user_home_dir().join(rest)
                    } else {
                        let p = Path::new(dest);
                        if p.is_absolute() {
                            p.to_path_buf()
                        } else {
                            work_dir.join(p)
                        }
                    };

                    match fs::canonicalize(&new_path) {
                        Ok(canon) if canon.is_dir() => {
                            let cwd_str = canon.to_string_lossy().to_string();
                            let json = format!(
                                r#"{{"output":"","cwd":"{}","code":0}}"#,
                                json_escape(&cwd_str)
                            );
                            let h = Header::from_bytes(&b"Content-Type"[..], &b"application/json; charset=utf-8"[..]).unwrap();
                            let _ = req.respond(Response::from_string(json).with_header(h));
                        }
                        _ => {
                            let err_msg = format!("cd: {}: Không tìm thấy thư mục\n", dest);
                            let cwd_str = work_dir.to_string_lossy().to_string();
                            let json = format!(
                                r#"{{"output":"{}","cwd":"{}","code":1}}"#,
                                json_escape(&err_msg),
                                json_escape(&cwd_str)
                            );
                            let h = Header::from_bytes(&b"Content-Type"[..], &b"application/json; charset=utf-8"[..]).unwrap();
                            let _ = req.respond(Response::from_string(json).with_header(h));
                        }
                    }
                    continue;
                }

                let (shell_cmd, shell_arg) = if cfg!(target_os = "windows") {
                    ("cmd.exe", "/c")
                } else {
                    ("sh", "-c")
                };

                let mut proc = Command::new(shell_cmd);
                proc.arg(shell_arg)
                    .arg(trimmed_cmd)
                    .current_dir(&work_dir)
                    .stdout(Stdio::piped())
                    .stderr(Stdio::piped());

                if cfg!(target_os = "linux") {
                    if let Ok(w) = env::var("WAYLAND_DISPLAY") {
                        proc.env("WAYLAND_DISPLAY", w);
                    }
                    if let Ok(d) = env::var("DISPLAY") {
                        proc.env("DISPLAY", d);
                    }
                    if let Ok(x) = env::var("XDG_RUNTIME_DIR") {
                        proc.env("XDG_RUNTIME_DIR", x);
                    }
                }

                let out = proc.output();
                let (output_text, exit_code) = match out {
                    Ok(o) => {
                        let mut s = String::from_utf8_lossy(&o.stdout).to_string();
                        let err = String::from_utf8_lossy(&o.stderr);
                        if !err.is_empty() {
                            if !s.is_empty() && !s.ends_with('\n') {
                                s.push('\n');
                            }
                            s.push_str(&err);
                        }
                        (s, o.status.code().unwrap_or(0))
                    }
                    Err(e) => (format!("Lỗi thực thi lệnh: {}\n", e), 127),
                };

                let cwd_str = work_dir.to_string_lossy().to_string();
                let json = format!(
                    r#"{{"output":"{}","cwd":"{}","code":{}}}"#,
                    json_escape(&output_text),
                    json_escape(&cwd_str),
                    exit_code
                );
                let h = Header::from_bytes(&b"Content-Type"[..], &b"application/json; charset=utf-8"[..]).unwrap();
                let _ = req.respond(Response::from_string(json).with_header(h));
            }
            "/api/set-password" => {
                if param_val == "clear" || param_val == "none" {
                    set_cfg("password", "");
                    let _ = req.respond(Response::from_string("Đã xóa mật khẩu riêng, Web UI sẽ dùng PIN ngẫu nhiên"));
                } else if param_val.len() < 4 {
                    let _ = req.respond(Response::from_string("Mật khẩu phải từ 4 ký tự trở lên").with_status_code(400));
                } else {
                    set_cfg("password", &param_val);
                    let _ = req.respond(Response::from_string("Đã cập nhật mật khẩu riêng thành công"));
                }
            }
            _ => {
                let _ = req.respond(Response::from_string("404").with_status_code(404));
            }
        }
    }
}

fn render_dashboard_html() -> &'static str {
    r##"<!DOCTYPE html>
<html lang="vi">
<head>
<meta charset="utf-8">
<meta name="viewport" content="width=device-width,initial-scale=1,maximum-scale=1,user-scalable=no">
<title>VIC Remote C2 - Web Dashboard</title>
<style>
  :root {
    --bg: #090d16;
    --card-bg: #131b2e;
    --card-border: #1e293b;
    --card-hover: #1b2640;
    --primary: #3b82f6;
    --primary-hover: #2563eb;
    --text: #f8fafc;
    --text-muted: #94a3b8;
    --accent: #10b981;
    --danger: #ef4444;
    --danger-hover: #dc2626;
  }
  * { box-sizing: border-box; -webkit-tap-highlight-color: transparent; }
  body {
    font-family: -apple-system, BlinkMacSystemFont, "Segoe UI", Roboto, Helvetica, Arial, sans-serif;
    background: var(--bg);
    color: var(--text);
    margin: 0;
    padding: 14px 10px 60px 10px;
    max-width: 650px;
    margin: auto;
  }
  .header {
    display: flex;
    align-items: center;
    justify-content: space-between;
    padding: 12px 14px;
    background: var(--card-bg);
    border: 1px solid var(--card-border);
    border-radius: 14px;
    margin-bottom: 12px;
  }
  .brand { display: flex; align-items: center; gap: 8px; font-weight: 700; font-size: 16px; }
  .badge { font-size: 11px; padding: 2px 8px; border-radius: 20px; font-weight: 600; }
  .badge-port { background: #1e3a8a; color: #93c5fd; }
  .badge-online { background: #064e3b; color: #6ee7b7; }
  .badge-offline { background: #334155; color: #94a3b8; }

  .pin-bar {
    display: flex;
    align-items: center;
    gap: 6px;
    background: #0b1120;
    padding: 6px 10px;
    border-radius: 10px;
    border: 1px solid var(--card-border);
  }
  .pin-bar input {
    background: transparent;
    border: none;
    color: white;
    font-size: 13px;
    width: 110px;
    outline: none;
    font-family: monospace;
    letter-spacing: 1px;
  }

  .nav-tabs {
    display: flex;
    gap: 4px;
    margin-bottom: 14px;
    background: #0f172a;
    padding: 4px;
    border-radius: 12px;
    border: 1px solid var(--card-border);
    overflow-x: auto;
    -webkit-overflow-scrolling: touch;
  }
  .nav-tabs::-webkit-scrollbar { display: none; }
  .tab-btn {
    flex: 1 0 auto;
    padding: 8px 6px;
    font-size: 12px;
    font-weight: 600;
    background: transparent;
    color: var(--text-muted);
    border: none;
    border-radius: 8px;
    cursor: pointer;
    white-space: nowrap;
    text-align: center;
    transition: all 0.2s ease;
  }
  .tab-btn.active {
    background: var(--primary);
    color: white;
    box-shadow: 0 2px 8px rgba(59, 130, 246, 0.4);
  }

  .card {
    background: var(--card-bg);
    border: 1px solid var(--card-border);
    border-radius: 14px;
    padding: 14px;
    margin-bottom: 12px;
    box-shadow: 0 4px 12px rgba(0,0,0,0.15);
  }
  .card-header {
    display: flex;
    justify-content: space-between;
    align-items: center;
    margin-bottom: 10px;
    font-weight: 600;
    font-size: 14px;
  }
  .row {
    display: flex;
    gap: 8px;
    align-items: center;
    justify-content: space-between;
    margin: 8px 0;
  }
  .btn-grid {
    display: grid;
    grid-template-columns: repeat(auto-fit, minmax(110px, 1fr));
    gap: 8px;
    margin-top: 8px;
  }

  button {
    background: var(--primary);
    color: white;
    border: none;
    padding: 10px 12px;
    border-radius: 8px;
    font-weight: 600;
    font-size: 13px;
    cursor: pointer;
    transition: all 0.15s ease;
    display: inline-flex;
    align-items: center;
    justify-content: center;
    gap: 6px;
  }
  button:active { transform: scale(0.97); }
  button.secondary { background: #334155; color: #e2e8f0; }
  button.secondary:hover { background: #475569; }
  button.danger { background: var(--danger); }
  button.success { background: var(--accent); }
  button.sm { padding: 5px 8px; font-size: 12px; border-radius: 6px; }

  .slider-box {
    display: flex;
    align-items: center;
    gap: 10px;
    background: #0f172a;
    padding: 10px 12px;
    border-radius: 10px;
    border: 1px solid var(--card-border);
    margin-bottom: 8px;
  }
  .slider-val {
    min-width: 44px;
    text-align: right;
    font-weight: 700;
    font-family: monospace;
    color: #60a5fa;
    font-size: 14px;
  }
  input[type=range] {
    flex: 1;
    accent-color: var(--primary);
    cursor: pointer;
  }

  input[type=text], textarea {
    width: 100%;
    box-sizing: border-box;
    background: #0b1120;
    color: white;
    border: 1px solid var(--card-border);
    border-radius: 8px;
    padding: 10px 12px;
    font-size: 13px;
    font-family: inherit;
    outline: none;
  }
  input[type=text]:focus, textarea:focus { border-color: var(--primary); }

  pre {
    background: #060913;
    border: 1px solid #1e293b;
    border-radius: 8px;
    padding: 10px;
    white-space: pre-wrap;
    word-break: break-all;
    font-family: ui-monospace, SFMono-Regular, Menlo, Monaco, Consolas, monospace;
    font-size: 12px;
    color: #38bdf8;
    max-height: 250px;
    overflow-y: auto;
    margin: 8px 0 0 0;
  }

  #file-list-container {
    max-height: 52vh;
    max-height: 52dvh;
    overflow-y: auto;
    -webkit-overflow-scrolling: touch;
    overscroll-behavior: contain;
    padding-right: 4px;
    margin-top: 4px;
    border-radius: 8px;
  }
  #file-list-container::-webkit-scrollbar {
    width: 6px;
  }
  #file-list-container::-webkit-scrollbar-track {
    background: #060913;
    border-radius: 4px;
  }
  #file-list-container::-webkit-scrollbar-thumb {
    background: #334155;
    border-radius: 4px;
  }
  #file-list-container::-webkit-scrollbar-thumb:hover {
    background: #64748b;
  }

  .file-item {
    display: flex;
    align-items: center;
    justify-content: space-between;
    background: #0f172a;
    border: 1px solid var(--card-border);
    border-radius: 10px;
    padding: 10px 12px;
    margin-bottom: 8px;
  }
  .file-info { display: flex; align-items: center; gap: 10px; overflow: hidden; flex: 1; }
  .file-name { font-weight: 600; font-size: 13px; text-overflow: ellipsis; overflow: hidden; white-space: nowrap; max-width: 200px; }
  .file-meta { font-size: 11px; color: var(--text-muted); }

  .file-badge {
    font-size: 10px;
    font-weight: 700;
    padding: 2px 6px;
    border-radius: 4px;
    font-family: ui-monospace, SFMono-Regular, monospace;
    letter-spacing: 0.5px;
    flex-shrink: 0;
  }
  .badge-dir { background: #1e3a8a; color: #93c5fd; }
  .badge-img { background: #065f46; color: #a7f3d0; }
  .badge-file { background: #334155; color: #cbd5e1; }

  #catalog-list {
    max-height: 52vh;
    max-height: 52dvh;
    overflow-y: auto;
    -webkit-overflow-scrolling: touch;
    overscroll-behavior: contain;
    padding-right: 4px;
    margin-top: 10px;
    border-radius: 8px;
  }
  #catalog-list::-webkit-scrollbar { width: 6px; }
  #catalog-list::-webkit-scrollbar-track { background: #060913; border-radius: 4px; }
  #catalog-list::-webkit-scrollbar-thumb { background: #334155; border-radius: 4px; }
  #catalog-list::-webkit-scrollbar-thumb:hover { background: #64748b; }

  #module-space {
    max-height: 56vh;
    max-height: 56dvh;
    overflow-y: auto;
    -webkit-overflow-scrolling: touch;
    overscroll-behavior: contain;
    padding-right: 4px;
    margin-top: 8px;
    border-radius: 8px;
  }
  #module-space::-webkit-scrollbar { width: 6px; }
  #module-space::-webkit-scrollbar-track { background: #060913; border-radius: 4px; }
  #module-space::-webkit-scrollbar-thumb { background: #334155; border-radius: 4px; }
  #module-space::-webkit-scrollbar-thumb:hover { background: #64748b; }

  .cmd-item {
    background: #0f172a;
    border: 1px solid var(--card-border);
    border-radius: 8px;
    padding: 8px 10px;
    margin-bottom: 6px;
  }
  .cmd-title { display: flex; justify-content: space-between; align-items: center; font-weight: 700; font-size: 13px; }
  .cmd-usage {
    background: #060913;
    color: #34d399;
    padding: 3px 6px;
    border-radius: 4px;
    font-family: monospace;
    font-size: 11px;
    margin: 4px 0;
    display: inline-block;
  }
  .cmd-desc { font-size: 12px; color: #cbd5e1; line-height: 1.35; }

  #preview-modal {
    display: none;
    position: fixed;
    top: 0;
    left: 0;
    right: 0;
    bottom: 0;
    background: rgba(4, 7, 13, 0.88);
    backdrop-filter: blur(4px);
    z-index: 9999;
    align-items: center;
    justify-content: center;
    padding: 16px;
  }
  .preview-card {
    background: #131b2e;
    border: 1px solid #1e293b;
    border-radius: 14px;
    max-width: 92vw;
    max-height: 90vh;
    display: flex;
    flex-direction: column;
    overflow: hidden;
    box-shadow: 0 16px 40px rgba(0, 0, 0, 0.6);
  }
  .preview-header {
    display: flex;
    justify-content: space-between;
    align-items: center;
    padding: 10px 14px;
    border-bottom: 1px solid #1e293b;
  }
  .preview-body {
    padding: 12px;
    overflow: auto;
    display: flex;
    align-items: center;
    justify-content: center;
    min-height: 180px;
  }
  .preview-body img {
    max-width: 100%;
    max-height: 65vh;
    object-fit: contain;
    border-radius: 6px;
    display: block;
  }
  .preview-footer {
    display: flex;
    gap: 8px;
    padding: 10px 14px;
    border-top: 1px solid #1e293b;
    justify-content: flex-end;
  }

  #toast {
    position: fixed;
    top: 16px;
    left: 50%;
    transform: translateX(-50%);
    background: #1e293b;
    border: 1px solid #3b82f6;
    color: white;
    padding: 8px 16px;
    border-radius: 20px;
    font-size: 13px;
    font-weight: 600;
    box-shadow: 0 4px 16px rgba(0,0,0,0.4);
    opacity: 0;
    pointer-events: none;
    transition: opacity 0.3s ease, transform 0.3s ease;
    z-index: 99999;
  }
  #toast.show { opacity: 1; transform: translateX(-50%) translateY(5px); }

  .tab-pane { display: none; }
  .tab-pane.active { display: block; }

  #terminal-screen {
    background: #060913;
    border: 1px solid #1e293b;
    border-radius: 8px;
    padding: 12px;
    font-family: ui-monospace, SFMono-Regular, Menlo, Monaco, Consolas, monospace;
    font-size: 12px;
    line-height: 1.45;
    min-height: 280px;
    max-height: 55vh;
    max-height: 55dvh;
    overflow-y: auto;
    white-space: pre-wrap;
    word-break: break-all;
    color: #cbd5e1;
    margin-bottom: 8px;
    -webkit-overflow-scrolling: touch;
  }
  #terminal-screen::-webkit-scrollbar { width: 6px; }
  #terminal-screen::-webkit-scrollbar-track { background: #060913; }
  #terminal-screen::-webkit-scrollbar-thumb { background: #334155; border-radius: 4px; }

  .term-cmd-line {
    color: #38bdf8;
    font-weight: 600;
    margin-top: 6px;
    margin-bottom: 2px;
  }
  .term-prompt {
    color: #34d399;
    font-weight: 700;
  }
  .term-out-line {
    color: #e2e8f0;
    white-space: pre-wrap;
    word-break: break-all;
  }
  .term-err-line {
    color: #f87171;
    white-space: pre-wrap;
    word-break: break-all;
  }
</style>
</head>
<body>
  <div id="toast"></div>

  <!-- IMAGE PREVIEW MODAL -->
  <div id="preview-modal" onclick="if(event.target===this)closePreview()">
    <div class="preview-card">
      <div class="preview-header">
        <span id="preview-title" style="font-weight:600;font-size:13px;overflow:hidden;text-overflow:ellipsis;white-space:nowrap;max-width:65vw;"></span>
        <button class="secondary sm" onclick="closePreview()">Đóng</button>
      </div>
      <div class="preview-body">
        <img id="preview-img" src="" alt="Preview">
      </div>
      <div class="preview-footer">
        <a id="preview-dl" download="" style="text-decoration:none;">
          <button class="sm">Tải ảnh về máy</button>
        </a>
        <button class="secondary sm" onclick="closePreview()">Đóng</button>
      </div>
    </div>
  </div>

  <div class="header">
    <div class="brand">
      <span>VIC REMOTE</span>
      <span class="badge badge-port" id="port-badge">:3667</span>
    </div>
    <div class="pin-bar">
      <span style="font-size:11px;font-weight:700;color:var(--text-muted);font-family:monospace;letter-spacing:1px">AUTH</span>
      <input type="password" id="pin" placeholder="PIN (6 số) / Enter mật khẩu" oninput="onPinInput()" onkeydown="onPinKeyDown(event)">
      <button class="secondary sm" style="padding:2px 6px;font-size:10px" onclick="setCustomPassword()" title="Cài đặt mật khẩu riêng">Pass</button>
    </div>
  </div>

  <div class="nav-tabs">
    <button class="tab-btn active" data-tab="controls" onclick="switchTab('controls')">Điều khiển</button>
    <button class="tab-btn" data-tab="share" onclick="switchTab('share')">Tệp tin</button>
    <button class="tab-btn" data-tab="terminal" onclick="switchTab('terminal')">Terminal</button>
    <button class="tab-btn" data-tab="modules" onclick="switchTab('modules')">Modules</button>
    <button class="tab-btn" data-tab="catalog" onclick="switchTab('catalog')">Danh mục</button>
  </div>

  <!-- TAB 1: CONTROLS -->
  <div id="tab-controls" class="tab-pane active">
    <!-- Light Card -->
    <div class="card">
      <div class="card-header">
        <span>Độ sáng màn hình</span>
        <span class="badge badge-online" id="light-val">70%</span>
      </div>
      <div class="slider-box">
        <input type="range" id="light-range" min="0" max="100" value="70" oninput="document.getElementById('light-val').innerText=this.value+'%'" onchange="setLight(this.value)">
      </div>
      <div class="row" style="margin:0">
        <button class="secondary sm" style="flex:1" onclick="setLight(25)">25%</button>
        <button class="secondary sm" style="flex:1" onclick="setLight(50)">50%</button>
        <button class="secondary sm" style="flex:1" onclick="setLight(75)">75%</button>
        <button class="secondary sm" style="flex:1" onclick="setLight(100)">100%</button>
      </div>
    </div>

    <!-- Audio Card -->
    <div class="card">
      <div class="card-header">
        <span>Âm lượng hệ thống</span>
        <span class="badge badge-online" id="au-val">50%</span>
      </div>
      <div class="slider-box">
        <input type="range" id="au-range" min="0" max="100" value="50" oninput="document.getElementById('au-val').innerText=this.value+'%'" onchange="setAu(this.value)">
      </div>
      <div class="row" style="margin:0">
        <button class="secondary sm" style="flex:1" onclick="setAu('mute')">Tắt tiếng</button>
        <button class="secondary sm" style="flex:1" onclick="setAu(30)">30%</button>
        <button class="secondary sm" style="flex:1" onclick="setAu(60)">60%</button>
        <button class="secondary sm" style="flex:1" onclick="setAu(100)">100%</button>
      </div>
    </div>

    <!-- Screenshot Card -->
    <div class="card">
      <div class="card-header">
        <span>Chụp ảnh màn hình</span>
      </div>
      <div class="row">
        <button style="flex:1" id="btn-sc" onclick="takeSc()">Chụp màn hình máy tính</button>
      </div>
      <img id="sc_img" style="width:100%;border-radius:8px;margin-top:10px;display:none;border:1px solid #334155;cursor:pointer" onclick="window.open(this.src,'_blank')" />
      <div id="sc_actions" style="display:none;margin-top:10px;" class="row">
        <a id="sc_dl" download="vic_screenshot.png" style="flex:1;text-decoration:none;">
          <button class="success" style="width:100%;">Tải ảnh về máy</button>
        </a>
      </div>
    </div>

    <!-- Power & Diagnostics Card -->
    <div class="card">
      <div class="card-header">Điều khiển hệ thống</div>
      <div class="btn-grid">
        <button class="danger" onclick="powerAction('lock')">Khóa máy</button>
        <button class="danger" onclick="powerAction('sleep')">Ngủ máy</button>
        <button class="secondary" onclick="runFast()">Fastfetch</button>
        <button class="secondary" onclick="runDoctor()">Doctor</button>
      </div>
      <pre id="sys-out" style="display:none"></pre>
    </div>
  </div>

  <!-- TAB 2: SHARE & CLIPBOARD -->
  <div id="tab-share" class="tab-pane">
    <!-- File Browser Card -->
    <div class="card">
      <div class="card-header">
        <span>Trình duyệt tệp tin</span>
        <div style="display:flex;gap:6px;">
          <button class="secondary sm" onclick="toggleRawFileList()" title="Xem danh sách dạng văn bản dòng lệnh">List Text</button>
          <button class="secondary sm" onclick="loadShareFiles(currentBrowsePath)">Làm mới</button>
        </div>
      </div>
      <pre id="raw-file-list" style="display:none;margin-bottom:10px;"></pre>

      <div class="row" style="margin-bottom:8px;flex-wrap:wrap;gap:6px;">
        <button class="secondary sm" onclick="loadShareFiles('~/.vic/share')">Share</button>
        <button class="secondary sm" onclick="loadShareFiles('~')">Home (~)</button>
        <button class="secondary sm" onclick="loadShareFiles('/tmp')">/tmp</button>
        <button class="secondary sm" id="btn-up-dir" onclick="goUpDir()" style="display:none">Lên thư mục cha</button>
      </div>

      <div class="row" style="margin-bottom:8px;gap:6px;">
        <input type="text" id="browser-path-input" placeholder="Đường dẫn thư mục..." style="font-family:monospace;font-size:12px;" onkeydown="if(event.key==='Enter')loadShareFiles(this.value)">
        <button class="sm" style="flex:0 0 auto;padding:8px 14px;" onclick="loadShareFiles(document.getElementById('browser-path-input').value)">Đến</button>
        <button class="secondary sm" style="flex:0 0 auto;padding:8px 14px;" title="Liệt kê danh sách tệp của thư mục" onclick="loadShareFiles(document.getElementById('browser-path-input').value || currentBrowsePath)">List</button>
      </div>

      <div class="row" style="margin-bottom:8px;">
        <input type="text" id="browser-filter-input" placeholder="Lọc tệp tin hoặc thư mục..." style="font-size:12px;" oninput="renderFileList()">
      </div>

      <div class="row" style="margin-bottom:10px;">
        <input type="file" id="share-file-input" style="display:none" onchange="uploadSelectedFile()">
        <button style="flex:1" onclick="document.getElementById('share-file-input').click()">Tải tệp lên thư mục này</button>
      </div>
      <div id="upload-status" style="font-size:12px;color:#94a3b8;margin-bottom:8px"></div>

      <div id="file-list-container">
        <div style="font-size:12px;color:#64748b;text-align:center;padding:12px">Đang tải danh sách tệp...</div>
      </div>
    </div>

    <!-- Clipboard Sync Card -->
    <div class="card">
      <div class="card-header">Đồng bộ Clipboard</div>
      <textarea id="clip-text" rows="3" placeholder="Nhập văn bản cần gửi hoặc nhấn Nhận từ PC..."></textarea>
      <div class="row" style="margin-top:8px">
        <button class="secondary" style="flex:1" onclick="pullClip()">Nhận từ PC</button>
        <button style="flex:1" onclick="pushClip()">Gửi đến PC</button>
        <button class="secondary" style="flex:0.6" onclick="copyLocalClip()">Sao chép</button>
      </div>
    </div>
  </div>

  <!-- TAB 3: TERMINAL -->
  <div id="tab-terminal" class="tab-pane">
    <div class="card">
      <div class="card-header" style="margin-bottom:8px;">
        <div style="display:flex;align-items:center;gap:8px;">
          <span style="font-weight:700;">Terminal Hệ Thống</span>
          <span class="badge badge-port" id="term-pwd">~</span>
        </div>
        <div style="display:flex;gap:6px;">
          <button class="secondary sm" onclick="clearTerminal()">Xóa màn hình</button>
        </div>
      </div>

      <div class="row" style="margin-bottom:8px;flex-wrap:wrap;gap:4px;">
        <button class="secondary sm" style="padding:3px 8px;font-size:11px;" onclick="runQuickTerm('ls -la')">ls -la</button>
        <button class="secondary sm" style="padding:3px 8px;font-size:11px;" onclick="runQuickTerm('pwd')">pwd</button>
        <button class="secondary sm" style="padding:3px 8px;font-size:11px;" onclick="runQuickTerm('whoami')">whoami</button>
        <button class="secondary sm" style="padding:3px 8px;font-size:11px;" onclick="runQuickTerm('uname -a')">uname -a</button>
        <button class="secondary sm" style="padding:3px 8px;font-size:11px;" onclick="runQuickTerm('free -h')">free -h</button>
        <button class="secondary sm" style="padding:3px 8px;font-size:11px;" onclick="runQuickTerm('df -h')">df -h</button>
        <button class="secondary sm" style="padding:3px 8px;font-size:11px;" onclick="runQuickTerm('uptime -p')">uptime</button>
        <button class="secondary sm" style="padding:3px 8px;font-size:11px;" onclick="runQuickTerm('vic doctor')">vic doctor</button>
        <button class="secondary sm" style="padding:3px 8px;font-size:11px;" onclick="runQuickTerm('vic setup list')">vic setup</button>
        <button class="secondary sm" style="padding:3px 8px;font-size:11px;" onclick="runQuickTerm('vic list')">vic list</button>
      </div>

      <div id="terminal-screen"></div>

      <div class="row" style="margin:0;gap:6px;">
        <div style="display:flex;align-items:center;flex:1;background:#0b1120;border:1px solid var(--card-border);border-radius:8px;padding:0 10px;">
          <span style="color:#10b981;font-family:monospace;font-weight:700;margin-right:6px;">$</span>
          <input type="text" id="term-input" placeholder="Nhập lệnh shell / bash và nhấn Enter..." style="border:none;background:transparent;padding:10px 0;font-family:monospace;font-size:13px;color:#38bdf8;outline:none;width:100%;" onkeydown="handleTermKey(event)">
        </div>
        <button class="sm" style="flex:0 0 auto;padding:10px 16px;" onclick="sendTermCommand()">Chạy</button>
      </div>
      <div style="font-size:11px;color:#64748b;margin-top:6px;display:flex;justify-content:space-between;">
        <span>Phím Mũi tên Lên/Xuống để xem lại lịch sử. Gõ 'cd &lt;dir&gt;' để đổi thư mục.</span>
        <span id="term-status" style="color:#38bdf8;"></span>
      </div>
    </div>
  </div>

  <!-- TAB 4: MODULES -->
  <div id="tab-modules" class="tab-pane">
    <div class="card">
      <div class="card-header">
        <span>Quản lý Modules</span>
        <button class="secondary sm" onclick="loadModules()">Làm mới</button>
      </div>
      <div class="row" style="margin-bottom:8px;">
        <input type="text" id="module-search" placeholder="Tìm kiếm module theo tên, mô tả..." style="font-size:12px;" oninput="filterModules()">
      </div>
      <div class="row" style="margin-bottom:8px;gap:4px;flex-wrap:wrap;">
        <button class="secondary sm" id="mod-filter-all" style="padding:3px 10px;font-size:11px;background:var(--primary);color:#fff;" onclick="setModuleFilter('all')">Tất cả</button>
        <button class="secondary sm" id="mod-filter-installed" style="padding:3px 10px;font-size:11px;" onclick="setModuleFilter('installed')">Đã cài</button>
        <button class="secondary sm" id="mod-filter-uninstalled" style="padding:3px 10px;font-size:11px;" onclick="setModuleFilter('uninstalled')">Chưa cài</button>
      </div>
      <div id="module-space">
        <div style="font-size:12px;color:#64748b;text-align:center;padding:12px">Đang tải danh sách module...</div>
      </div>
    </div>
  </div>


  <!-- TAB 4: CATALOG & GUIDE -->
  <div id="tab-catalog" class="tab-pane">
    <div class="card">
      <div class="card-header">
        <span>Danh mục lệnh</span>
        <button class="secondary sm" onclick="toggleRawMd()">Xem module.md</button>
      </div>
      <input type="text" id="catalog-search" placeholder="Tìm kiếm lệnh hoặc chức năng..." oninput="filterCatalog()">
      <pre id="raw-md" style="display:none"></pre>
      <div id="catalog-list" style="margin-top:12px">
        <div style="font-size:12px;color:#64748b;text-align:center;padding:12px">Đang tải danh mục...</div>
      </div>
    </div>
  </div>

<script>
  let currentScBlob = null;
  let allCatalogEntries = [];
  let currentBrowsePath = '';
  let parentBrowsePath = '';

  const curPort = window.location.port ? ':' + window.location.port : '';
  if (curPort) {
    const pEl = document.getElementById('port-badge');
    if (pEl) pEl.innerText = curPort;
  }

  const urlParams = new URLSearchParams(window.location.search);
  const qPin = urlParams.get('pin');
  if (qPin) {
    localStorage.vic_pin = qPin;
    document.cookie = 'vic_auth=' + encodeURIComponent(qPin) + '; path=/; SameSite=Strict';
    window.history.replaceState({}, document.title, window.location.pathname);
  }

  const pinInput = document.getElementById('pin');
  pinInput.value = localStorage.vic_pin || '';
  if (pinInput.value) {
    document.cookie = 'vic_auth=' + encodeURIComponent(pinInput.value) + '; path=/; SameSite=Strict';
  }

  function triggerAuthReload() {
    loadStatus();
    loadModules();
    loadShareFiles(currentBrowsePath);
  }

  function onPinInput() {
    const val = pinInput.value.trim();
    localStorage.vic_pin = val;
    document.cookie = 'vic_auth=' + encodeURIComponent(val) + '; path=/; SameSite=Strict';
    // Chi tu dong kiem tra khi dung 6 chu so de tranh loi gioi han rate-limit
    if (/^\d{6}$/.test(val)) {
      triggerAuthReload();
    }
  }

  function onPinKeyDown(e) {
    if (e.key === 'Enter') {
      const val = pinInput.value.trim();
      if (val.length >= 4) {
        triggerAuthReload();
        toast('Đang kiểm tra xác thực...');
      }
    }
  }

  function pinQ(sep = '?') {
    return sep + 'pin=' + encodeURIComponent(document.getElementById('pin').value);
  }

  function toast(msg) {
    const el = document.getElementById('toast');
    el.innerText = msg;
    el.classList.add('show');
    setTimeout(() => el.classList.remove('show'), 2200);
  }

  async function api(path, opts = {}) {
    opts.headers = opts.headers || {};
    const token = document.getElementById('pin').value;
    if (token) {
      opts.headers['X-VIC-AUTH'] = token;
      document.cookie = 'vic_auth=' + encodeURIComponent(token) + '; path=/; SameSite=Strict';
    }
    const sep = path.includes('?') ? '&' : '?';
    const res = await fetch(path + pinQ(sep), opts);
    if (res.status === 401 || res.status === 429) {
      const errText = await res.text();
      toast(errText || 'Lỗi xác thực (PIN hoặc Mật khẩu không đúng)');
    }
    return res;
  }

  async function setCustomPassword() {
    const current = prompt('Nhập mật khẩu riêng mới (tối thiểu 4 ký tự gồm chữ và số, hoặc gõ "clear" để xóa):');
    if (!current) return;
    const res = await api('/api/set-password?v=' + encodeURIComponent(current));
    const txt = await res.text();
    toast(txt);
  }

  function switchTabDirect(name) {
    document.querySelectorAll('.tab-btn').forEach(b => {
      if (b.getAttribute('data-tab') === name) b.classList.add('active');
      else b.classList.remove('active');
    });
    document.querySelectorAll('.tab-pane').forEach(p => p.classList.remove('active'));
    const pane = document.getElementById('tab-' + name);
    if (pane) pane.classList.add('active');
    triggerTabLoad(name);
  }

  function switchTab(name) {
    document.querySelectorAll('.tab-btn').forEach(b => {
      if (b.getAttribute('data-tab') === name) b.classList.add('active');
      else b.classList.remove('active');
    });
    document.querySelectorAll('.tab-pane').forEach(p => p.classList.remove('active'));
    const pane = document.getElementById('tab-' + name);
    if (pane) pane.classList.add('active');
    triggerTabLoad(name);
  }

  function triggerTabLoad(name) {
    if (name === 'controls') loadStatus();
    if (name === 'share') loadShareFiles(currentBrowsePath);
    if (name === 'terminal') initTerminal();
    if (name === 'modules') loadModules();
    if (name === 'catalog') loadCatalog();
  }

  async function loadStatus() {
    try {
      const res = await api('/api/status');
      if (!res.ok) return;
      const data = await res.json();
      if (data.light !== undefined) {
        document.getElementById('light-range').value = data.light;
        document.getElementById('light-val').innerText = data.light + '%';
      }
      if (data.volume !== undefined) {
        document.getElementById('au-range').value = data.volume;
        document.getElementById('au-val').innerText = (data.muted ? '[MUTE] ' : '') + data.volume + '%';
      }
    } catch (_) {}
  }

  async function setLight(val) {
    document.getElementById('light-range').value = val;
    document.getElementById('light-val').innerText = val + '%';
    await api('/api/light?v=' + val);
    toast('Đã chỉnh độ sáng: ' + val + '%');
  }

  async function setAu(val) {
    if (val !== 'mute') {
      document.getElementById('au-range').value = val;
      document.getElementById('au-val').innerText = val + '%';
    }
    await api('/api/au?v=' + val);
    toast(val === 'mute' ? 'Đã tắt/bật âm' : 'Đã chỉnh âm lượng: ' + val + '%');
  }

  async function takeSc() {
    const btn = document.getElementById('btn-sc');
    btn.innerText = 'Đang chụp...';
    btn.disabled = true;
    try {
      const res = await api('/api/sc');
      if (res.ok) {
        currentScBlob = await res.blob();
        const img = document.getElementById('sc_img');
        const blobUrl = URL.createObjectURL(currentScBlob);
        img.src = blobUrl;
        img.style.display = 'block';
        const dlLink = document.getElementById('sc_dl');
        if (dlLink) {
          dlLink.href = blobUrl;
          dlLink.download = 'vic_screen_' + Date.now() + '.png';
          document.getElementById('sc_actions').style.display = 'flex';
        }
        toast('Đã chụp ảnh màn hình thành công');
      } else {
        alert('Lỗi chụp màn hình hoặc sai mã PIN!');
      }
    } catch (e) {
      alert('Không thể kết nối máy tính: ' + e);
    } finally {
      btn.innerText = 'Chụp màn hình máy tính';
      btn.disabled = false;
    }
  }

  async function powerAction(action) {
    const title = action === 'lock' ? 'khóa màn hình máy tính' : 'đưa máy tính vào chế độ ngủ';
    if (!confirm('Bạn có chắc chắn muốn ' + title + ' không?')) return;
    await api('/api/power?v=' + action);
    toast('Đã gửi lệnh ' + action);
  }

  async function runFast() {
    const out = document.getElementById('sys-out');
    out.style.display = 'block';
    out.innerText = 'Đang tải thông tin phần cứng...';
    const res = await api('/api/fast');
    out.innerText = res.ok ? await res.text() : 'Lỗi lấy thông tin.';
  }

  async function runDoctor() {
    const out = document.getElementById('sys-out');
    out.style.display = 'block';
    out.innerText = 'Đang kiểm tra hệ thống...';
    const res = await api('/api/doctor');
    out.innerText = res.ok ? await res.text() : 'Lỗi kiểm tra hệ thống.';
  }

  // --- SHARE & FILE BROWSER ---
  let currentBrowseItems = [];
  let isDirsCollapsed = false;
  let isFilesCollapsed = false;

  function toggleDirsCollapse() {
    isDirsCollapsed = !isDirsCollapsed;
    renderFileList();
  }

  function toggleFilesCollapse() {
    isFilesCollapsed = !isFilesCollapsed;
    renderFileList();
  }

  function renderItemHtml(item) {
    const encPath = encodeURIComponent(item.path);
    const encName = encodeURIComponent(item.name);
    const safeJsPath = item.path.replace(/\\/g, '\\\\').replace(/'/g, "\\'");

    if (item.is_dir) {
      return `
        <div class="file-item">
          <div class="file-info" style="cursor:pointer" onclick="loadShareFiles('${safeJsPath}')">
            <span class="file-badge badge-dir">DIR</span>
            <div>
              <div class="file-name" title="${item.name}">${item.name}</div>
              <div class="file-meta">Thư mục</div>
            </div>
          </div>
          <div class="row" style="margin:0;gap:4px;">
            <button class="secondary sm" onclick="loadShareFiles('${safeJsPath}')">Mở</button>
            <button class="danger sm" onclick="deleteShareItem('${encPath}', '${encName}', true)">Xóa</button>
          </div>
        </div>
      `;
    }

    let actionBtns = '';
    if (item.is_image) {
      actionBtns += `<button class="success sm" onclick="previewImage('${encPath}', '${encName}')">Xem</button>`;
    }
    actionBtns += `
      <a href="/api/share/download?path=${encPath}${pinQ('&')}" download="${item.name}" style="text-decoration:none">
        <button class="secondary sm">Tải</button>
      </a>
      <button class="danger sm" onclick="deleteShareItem('${encPath}', '${encName}', false)">Xóa</button>
    `;

    const typeBadge = item.is_image ? 'badge-img' : 'badge-file';
    const typeText = item.is_image ? 'IMG' : 'FILE';

    return `
      <div class="file-item">
        <div class="file-info" ${item.is_image ? `style="cursor:pointer" onclick="previewImage('${encPath}', '${encName}')"` : ''}>
          <span class="file-badge ${typeBadge}">${typeText}</span>
          <div>
            <div class="file-name" title="${item.name}">${item.name}</div>
            <div class="file-meta">${item.size}</div>
          </div>
        </div>
        <div class="row" style="margin:0;gap:4px;">
          ${actionBtns}
        </div>
      </div>
    `;
  }

  function renderFileList() {
    const filterEl = document.getElementById('browser-filter-input');
    const q = (filterEl ? filterEl.value : '').toLowerCase().trim();
    const container = document.getElementById('file-list-container');

    const filtered = currentBrowseItems.filter(item => !q || item.name.toLowerCase().includes(q));
    const dirs = filtered.filter(i => i.is_dir);
    const files = filtered.filter(i => !i.is_dir);

    if (dirs.length === 0 && files.length === 0) {
      container.innerHTML = '<div style="font-size:12px;color:#64748b;text-align:center;padding:12px">Không tìm thấy tệp tin hoặc thư mục.</div>';
      return;
    }

    let html = '';

    if (dirs.length > 0) {
      html += `
        <div style="display:flex;justify-content:space-between;align-items:center;padding:6px 4px;margin-bottom:6px;border-bottom:1px solid #1e293b;cursor:pointer" onclick="toggleDirsCollapse()">
          <span style="font-size:11px;font-weight:700;color:var(--text-muted);font-family:monospace;letter-spacing:0.5px">THƯ MỤC (${dirs.length})</span>
          <button class="secondary sm" style="padding:2px 8px;font-size:11px" onclick="event.stopPropagation();toggleDirsCollapse()">${isDirsCollapsed ? 'Mở rộng' : 'Thu gọn'}</button>
        </div>
      `;
      if (!isDirsCollapsed) {
        html += dirs.map(renderItemHtml).join('');
      }
    }

    if (files.length > 0) {
      html += `
        <div style="display:flex;justify-content:space-between;align-items:center;padding:6px 4px;margin-top:10px;margin-bottom:6px;border-bottom:1px solid #1e293b;cursor:pointer" onclick="toggleFilesCollapse()">
          <span style="font-size:11px;font-weight:700;color:var(--text-muted);font-family:monospace;letter-spacing:0.5px">TỆP TIN (${files.length})</span>
          <button class="secondary sm" style="padding:2px 8px;font-size:11px" onclick="event.stopPropagation();toggleFilesCollapse()">${isFilesCollapsed ? 'Mở rộng' : 'Thu gọn'}</button>
        </div>
      `;
      if (!isFilesCollapsed) {
        html += files.map(renderItemHtml).join('');
      }
    }

    container.innerHTML = html;
  }

  function toggleRawFileList() {
    const pre = document.getElementById('raw-file-list');
    if (!pre) return;
    if (pre.style.display === 'block') {
      pre.style.display = 'none';
      return;
    }
    if (!currentBrowseItems || currentBrowseItems.length === 0) {
      pre.innerText = '[Thư mục trống hoặc chưa tải danh sách]';
    } else {
      let lines = ['=== DANH SÁCH TỆP TIN: ' + currentBrowsePath + ' ==='];
      currentBrowseItems.forEach(it => {
        if (it.is_dir) {
          lines.push('[THƯ MỤC]  ' + it.name + '/');
        } else {
          lines.push('[TỆP TIN]  ' + it.name + ' (' + it.size + ')');
        }
      });
      pre.innerText = lines.join('\n');
    }
    pre.style.display = 'block';
  }

  async function loadShareFiles(targetPath = '') {
    const container = document.getElementById('file-list-container');
    const pathQuery = targetPath ? '&dir=' + encodeURIComponent(targetPath) : '';
    const res = await api('/api/share/list' + (pathQuery ? '?' + pathQuery.slice(1) : ''));
    if (!res.ok) {
      container.innerHTML = '<div style="font-size:12px;color:#ef4444;text-align:center;padding:12px">Lỗi truy cập thư mục (Kiểm tra PIN hoặc đường dẫn)</div>';
      return;
    }
    const data = await res.json();
    currentBrowsePath = data.current;
    parentBrowsePath = data.parent;

    const pathInput = document.getElementById('browser-path-input');
    if (pathInput) pathInput.value = data.current;

    const upBtn = document.getElementById('btn-up-dir');
    if (upBtn) {
      upBtn.style.display = (data.parent && data.parent !== data.current) ? 'inline-flex' : 'none';
    }

    currentBrowseItems = data.items || [];
    renderFileList();

    const rawList = document.getElementById('raw-file-list');
    if (rawList && rawList.style.display === 'block') {
      rawList.style.display = 'none';
      toggleRawFileList();
    }
  }

  function goUpDir() {
    if (parentBrowsePath) {
      loadShareFiles(parentBrowsePath);
    }
  }

  function previewImage(encPath, encName) {
    const name = decodeURIComponent(encName);
    const url = '/api/share/preview?path=' + encPath + pinQ('&');
    const dlUrl = '/api/share/download?path=' + encPath + pinQ('&');

    document.getElementById('preview-title').innerText = name;
    document.getElementById('preview-img').src = url;
    const dl = document.getElementById('preview-dl');
    dl.href = dlUrl;
    dl.download = name;

    const modal = document.getElementById('preview-modal');
    modal.style.display = 'flex';
  }

  function closePreview() {
    const modal = document.getElementById('preview-modal');
    modal.style.display = 'none';
    document.getElementById('preview-img').src = '';
  }

  async function uploadSelectedFile() {
    const input = document.getElementById('share-file-input');
    if (!input.files || input.files.length === 0) return;
    const file = input.files[0];
    const status = document.getElementById('upload-status');
    status.innerText = 'Đang tải lên: ' + file.name + ' (' + (file.size / 1024).toFixed(1) + ' KB)...';

    try {
      const url = '/api/share/upload?name=' + encodeURIComponent(file.name) +
                  '&dir=' + encodeURIComponent(currentBrowsePath) +
                  pinQ('&');
      const res = await fetch(url, {
        method: 'POST',
        body: file
      });
      if (res.ok) {
        status.innerText = 'Đã tải lên thành công: ' + file.name;
        toast('Đã tải lên: ' + file.name);
        input.value = '';
        await loadShareFiles(currentBrowsePath);
      } else {
        status.innerText = 'Tải lên thất bại.';
      }
    } catch (e) {
      status.innerText = 'Lỗi: ' + e;
    }
  }

  async function deleteShareItem(encPath, encName, isDir) {
    const name = decodeURIComponent(encName);
    const typeLabel = isDir ? 'thư mục' : 'tệp';
    if (!confirm('Xóa ' + typeLabel + ' "' + name + '" trên máy tính?')) return;
    const res = await api('/api/share/delete?path=' + encPath);
    if (res.ok) {
      toast('Đã xóa: ' + name);
      loadShareFiles(currentBrowsePath);
    } else {
      alert('Xóa thất bại.');
    }
  }

  // --- CLIPBOARD ---
  async function pullClip() {
    const res = await api('/api/clip');
    if (res.ok) {
      document.getElementById('clip-text').value = await res.text();
      toast('Đã nhận clipboard từ máy tính');
    }
  }

  async function pushClip() {
    const txt = document.getElementById('clip-text').value;
    const res = await api('/api/clip', { method: 'POST', body: txt });
    if (res.ok) toast('Đã gửi clipboard sang máy tính');
  }

  function copyLocalClip() {
    const txt = document.getElementById('clip-text').value;
    navigator.clipboard.writeText(txt);
    toast('Đã sao chép vào bộ nhớ đệm');
  }

  // --- MODULES & MOUSE INTEGRATION ---
  let allLoadedModules = [];
  let currentModFilter = 'all';

  function setModuleFilter(f) {
    currentModFilter = f;
    ['all', 'installed', 'uninstalled'].forEach(k => {
      const btn = document.getElementById('mod-filter-' + k);
      if (btn) {
        if (k === f) {
          btn.style.background = 'var(--primary)';
          btn.style.color = '#fff';
        } else {
          btn.style.background = '';
          btn.style.color = '';
        }
      }
    });
    filterModules();
  }

  function filterModules() {
    const input = document.getElementById('module-search');
    const q = (input ? input.value : '').toLowerCase().trim();
    let filtered = allLoadedModules.slice();
    if (currentModFilter === 'installed') {
      filtered = filtered.filter(m => m.installed);
    } else if (currentModFilter === 'uninstalled') {
      filtered = filtered.filter(m => !m.installed);
    }
    if (q) {
      filtered = filtered.filter(m =>
        m.name.toLowerCase().includes(q) ||
        (m.title && m.title.toLowerCase().includes(q)) ||
        (m.desc && m.desc.toLowerCase().includes(q)) ||
        (m.usage && m.usage.toLowerCase().includes(q))
      );
    }
    renderModulesList(filtered);
  }

  async function loadModules() {
    const container = document.getElementById('module-space');
    const res = await api('/api/modules');
    if (!res.ok) {
      container.innerHTML = '<div style="font-size:12px;color:#ef4444;text-align:center;padding:12px">Lỗi tải modules (Kiểm tra PIN)</div>';
      return;
    }
    allLoadedModules = await res.json();
    allLoadedModules.sort((a, b) => (b.installed ? 1 : 0) - (a.installed ? 1 : 0));
    filterModules();
    checkMousePort();
  }

  function renderModulesList(mods) {
    const container = document.getElementById('module-space');
    if (!mods || mods.length === 0) {
      container.innerHTML = '<div style="font-size:12px;color:#64748b;text-align:center;padding:12px">Không tìm thấy module phù hợp.</div>';
      return;
    }

    container.innerHTML = mods.map(m => {
      let actionsHtml = '';
      if (!m.installed) {
        actionsHtml = `<button style="width:100%" onclick="installMod(${m.id})">Cài đặt Module (${m.name})</button>`;
      } else if (m.name === 'mouse') {
        actionsHtml = `
          <button class="primary" style="width:100%;padding:14px 10px;font-size:15px;font-weight:700;margin-bottom:8px;border-radius:10px;box-shadow:0 3px 12px rgba(59,130,246,0.35);letter-spacing:0.5px;" onclick="openMouseFullTab()">BẬT QUA TAB MOUSE</button>
          <div class="row" style="gap:8px;margin-bottom:8px;">
            <button class="success" style="flex:1" onclick="startMouseDaemon()">Khởi chạy (-d)</button>
            <button class="danger" style="flex:1" onclick="stopMouseDaemon()">Dừng (stop)</button>
          </div>
          <div class="row" style="gap:6px;">
            <input type="text" id="mouse-flags-input" placeholder="Nhập cờ (vd: --lan 3669)..." style="flex:1;font-family:monospace;font-size:12px;">
            <button class="secondary sm" style="flex:0 0 auto;padding:8px 12px;" onclick="startMouseWithFlags()">Chạy với flag</button>
          </div>
        `;
      } else if (m.actions) {
        const parts = m.actions.split('|').map(s => s.trim());
        actionsHtml += `<div class="row" style="flex-wrap:wrap;gap:8px;">`;
        for (const p of parts) {
          if (p.startsWith('btn:') || p.startsWith('view:')) {
            const [label, arg] = p.replace(/^(btn:|view:)/, '').split('=');
            actionsHtml += `<button onclick="execMod('${m.name}', '${arg || ''}', 'out_${m.name}')">${label}</button>`;
          } else if (p.startsWith('input:')) {
            const [label, prefix] = p.replace('input:', '').split('=');
            actionsHtml += `
              <div class="row" style="width:100%;margin-top:6px;gap:8px;">
                <input type="text" id="in_${m.name}" placeholder="Nhập tham số..." style="flex:1">
                <button style="flex:0.5" onclick="execMod('${m.name}', '${prefix ? prefix + ' ' : ''}' + document.getElementById('in_${m.name}').value, 'out_${m.name}')">${label}</button>
              </div>`;
          }
        }
        actionsHtml += `</div>`;
      } else {
        actionsHtml = `
          <div class="row" style="gap:8px;">
            <input type="text" id="in_${m.name}" placeholder="Tham số..." style="flex:1">
            <button style="flex:0.4" onclick="execMod('${m.name}', document.getElementById('in_${m.name}').value, 'out_${m.name}')">Run</button>
          </div>`;
      }

      return `
        <div class="card" style="margin-bottom:8px;">
          <div class="card-header">
            <span>[${m.id}] ${m.name}</span>
            ${m.installed ? '<span class="badge badge-online">[OK]</span>' : ''}
          </div>
          <div style="font-size:13px;color:#cbd5e1;margin-bottom:6px;">${m.title}</div>
          <div style="font-size:12px;color:#94a3b8;margin-bottom:10px;font-family:monospace;background:#090d16;padding:4px 8px;border-radius:6px;display:inline-block">${m.usage}</div>
          ${actionsHtml}
          <pre id="out_${m.name}" style="display:none;margin-top:8px;"></pre>
        </div>
      `;
    }).join('');
  }

  // --- MOUSE MODULE CONTROLLER ---
  let cachedMousePort = '3669';

  async function checkMousePort() {
    try {
      const res = await api('/api/mouse/info');
      if (res.ok) {
        const data = await res.json();
        if (data.port) cachedMousePort = data.port;
      }
    } catch (_) {}
  }

  async function startMouseDaemon() {
    toast('Đang khởi chạy Air Mouse ngầm...');
    const out = document.getElementById('out_mouse');
    if (out) {
      out.style.display = 'block';
      out.innerText = 'Đang khởi chạy: vic mouse -d...';
    }
    const res = await api('/api/exec?v=mouse|-d');
    const txt = await res.text();
    if (out) out.innerText = txt;
    toast('Đã khởi chạy Air Mouse');
    checkMousePort();
  }

  async function startMouseWithFlags() {
    const input = document.getElementById('mouse-flags-input');
    const flags = (input ? input.value : '').trim();
    const args = flags ? '-d ' + flags : '-d';
    toast('Đang khởi chạy: vic mouse ' + args);
    const out = document.getElementById('out_mouse');
    if (out) {
      out.style.display = 'block';
      out.innerText = 'Đang khởi chạy: vic mouse ' + args + '...';
    }
    const res = await api('/api/exec?v=mouse|' + encodeURIComponent(args));
    const txt = await res.text();
    if (out) out.innerText = txt;
    toast('Đã khởi chạy Air Mouse với cờ');
    checkMousePort();
  }

  async function stopMouseDaemon() {
    toast('Đang dừng Air Mouse...');
    const out = document.getElementById('out_mouse');
    if (out) {
      out.style.display = 'block';
      out.innerText = 'Đang dừng: vic mouse stop...';
    }
    const res = await api('/api/exec?v=mouse|stop');
    const txt = await res.text();
    if (out) out.innerText = txt;
    toast('Đã dừng Air Mouse');
  }

  function getMouseUrl() {
    const host = window.location.hostname || '127.0.0.1';
    const pin = document.getElementById('pin').value || '';
    return 'http://' + host + ':' + cachedMousePort + '/?pin=' + encodeURIComponent(pin);
  }

  function openMouseFullTab() {
    window.open(getMouseUrl(), '_blank');
  }

  async function installMod(id) {
    toast('Đang tải và kiểm tra module...');
    const res = await api('/api/install?v=' + id);
    const text = await res.text();
    if (text.includes('[-]')) {
      alert(text);
    } else {
      toast(text || 'Cài đặt hoàn tất');
    }
    await loadModules();
  }

  async function execMod(name, args, outId) {
    const out = document.getElementById(outId);
    out.style.display = 'block';
    out.innerText = 'Đang thực thi ' + name + '...';
    const res = await api('/api/exec?v=' + encodeURIComponent(name + '|' + args.trim()));
    out.innerText = await res.text();
  }

  // --- CATALOG & DOCS ---
  async function loadCatalog() {
    const res = await api('/api/catalog');
    if (!res.ok) return;
    allCatalogEntries = await res.json();
    renderCatalog(allCatalogEntries);
  }

  function renderCatalog(entries) {
    const container = document.getElementById('catalog-list');
    if (!entries || entries.length === 0) {
      container.innerHTML = '<div style="font-size:12px;color:#64748b;text-align:center;padding:12px">Không tìm thấy lệnh phù hợp.</div>';
      return;
    }
    container.innerHTML = entries.map(e => `
      <div class="cmd-item">
        <div class="cmd-title">
          <span>[${e.id}] ${e.name}</span>
          <span class="badge ${e.type === 'core' ? 'badge-port' : 'badge-online'}">${e.type.toUpperCase()}</span>
        </div>
        <div class="cmd-usage">${e.usage}</div>
        <div class="cmd-desc">${e.desc || e.title}</div>
      </div>
    `).join('');
  }

  function filterCatalog() {
    const q = document.getElementById('catalog-search').value.toLowerCase().trim();
    if (!q) {
      renderCatalog(allCatalogEntries);
      return;
    }
    const filtered = allCatalogEntries.filter(e =>
      e.name.toLowerCase().includes(q) ||
      (e.title && e.title.toLowerCase().includes(q)) ||
      (e.desc && e.desc.toLowerCase().includes(q)) ||
      (e.usage && e.usage.toLowerCase().includes(q))
    );
    renderCatalog(filtered);
  }

  async function toggleRawMd() {
    const pre = document.getElementById('raw-md');
    if (pre.style.display === 'block') {
      pre.style.display = 'none';
    } else {
      pre.style.display = 'block';
      pre.innerText = 'Đang tải module.md...';
      const res = await api('/api/md');
      pre.innerText = res.ok ? await res.text() : 'Lỗi tải module.md';
    }
  }

  // --- TERMINAL ---
  let termCwd = '~';
  let termHistory = [];
  let termHistoryIndex = -1;
  let termInitialized = false;

  function initTerminal() {
    if (termInitialized) return;
    termInitialized = true;
    const screen = document.getElementById('terminal-screen');
    if (!screen) return;
    screen.innerHTML = '<div style="color:#64748b;margin-bottom:6px;">=== VIC WEB REMOTE TERMINAL [v1.0.0] ===\nSẵn sàng nhận lệnh shell. Gõ lệnh và nhấn Enter hoặc dùng phím tắt phía trên.\n------------------------------------------------------------------------</div>';
    runTermCommandSilent('pwd');
  }

  async function runTermCommandSilent(cmd) {
    try {
      const res = await api('/api/terminal/exec?cmd=' + encodeURIComponent(cmd) + '&cwd=' + encodeURIComponent(termCwd));
      if (!res.ok) return;
      const data = await res.json();
      if (data.cwd) {
        termCwd = data.cwd;
        updateTermPwdDisplay();
      }
    } catch (_) {}
  }

  function updateTermPwdDisplay() {
    const el = document.getElementById('term-pwd');
    if (el) {
      let displayPath = termCwd;
      if (displayPath.length > 25) {
        displayPath = '...' + displayPath.slice(-22);
      }
      el.innerText = displayPath;
      el.title = termCwd;
    }
  }

  function clearTerminal() {
    const screen = document.getElementById('terminal-screen');
    if (screen) {
      screen.innerHTML = '<div style="color:#64748b;">=== VIC TERMINAL (Đã xóa màn hình) ===</div>';
    }
    const input = document.getElementById('term-input');
    if (input) input.focus();
  }

  function runQuickTerm(cmd) {
    const input = document.getElementById('term-input');
    if (input) input.value = cmd;
    sendTermCommand();
  }

  function handleTermKey(e) {
    if (e.key === 'Enter') {
      sendTermCommand();
    } else if (e.key === 'ArrowUp') {
      e.preventDefault();
      if (termHistory.length === 0) return;
      if (termHistoryIndex === -1) {
        termHistoryIndex = termHistory.length - 1;
      } else if (termHistoryIndex > 0) {
        termHistoryIndex--;
      }
      document.getElementById('term-input').value = termHistory[termHistoryIndex] || '';
    } else if (e.key === 'ArrowDown') {
      e.preventDefault();
      if (termHistory.length === 0) return;
      if (termHistoryIndex !== -1) {
        if (termHistoryIndex < termHistory.length - 1) {
          termHistoryIndex++;
          document.getElementById('term-input').value = termHistory[termHistoryIndex] || '';
        } else {
          termHistoryIndex = -1;
          document.getElementById('term-input').value = '';
        }
      }
    }
  }

  function escapeHtml(str) {
    return str.replace(/&/g, "&amp;").replace(/</g, "&lt;").replace(/>/g, "&gt;").replace(/"/g, "&quot;").replace(/'/g, "&#039;");
  }

  async function sendTermCommand() {
    const input = document.getElementById('term-input');
    const cmd = input.value.trim();
    if (!cmd) return;

    input.value = '';
    termHistory.push(cmd);
    termHistoryIndex = -1;

    const screen = document.getElementById('terminal-screen');
    const statusEl = document.getElementById('term-status');

    let shortCwd = termCwd;
    if (shortCwd.length > 20) shortCwd = '...' + shortCwd.slice(-17);

    const cmdEntry = document.createElement('div');
    cmdEntry.className = 'term-cmd-line';
    cmdEntry.innerHTML = `<span class="term-prompt">vic:${shortCwd}$</span> ${escapeHtml(cmd)}`;
    screen.appendChild(cmdEntry);
    screen.scrollTop = screen.scrollHeight;

    if (cmd === 'clear') {
      clearTerminal();
      return;
    }

    if (statusEl) statusEl.innerText = 'Đang chạy...';

    try {
      const res = await api('/api/terminal/exec?cmd=' + encodeURIComponent(cmd) + '&cwd=' + encodeURIComponent(termCwd));
      if (!res.ok) {
        const outEntry = document.createElement('div');
        outEntry.className = 'term-err-line';
        outEntry.innerText = 'Lỗi HTTP ' + res.status + ' (Kiểm tra xác thực)';
        screen.appendChild(outEntry);
        if (statusEl) statusEl.innerText = '';
        screen.scrollTop = screen.scrollHeight;
        return;
      }
      const data = await res.json();
      if (data.cwd) {
        termCwd = data.cwd;
        updateTermPwdDisplay();
      }
      const outEntry = document.createElement('div');
      outEntry.className = (data.code === 0) ? 'term-out-line' : 'term-err-line';
      outEntry.innerText = data.output ? data.output : (data.code === 0 ? '' : '[Thoát với mã ' + data.code + ']');
      if (outEntry.innerText) {
        screen.appendChild(outEntry);
      }
      screen.scrollTop = screen.scrollHeight;
      if (statusEl) statusEl.innerText = (data.code === 0) ? '' : '[Exit ' + data.code + ']';
    } catch (err) {
      const outEntry = document.createElement('div');
      outEntry.className = 'term-err-line';
      outEntry.innerText = 'Lỗi kết nối: ' + err.message;
      screen.appendChild(outEntry);
      screen.scrollTop = screen.scrollHeight;
      if (statusEl) statusEl.innerText = '';
    }
  }

  if (pinInput.value.length >= 4) {
    loadStatus();
    loadModules();
  }
</script>
</body>
</html>"##
}
