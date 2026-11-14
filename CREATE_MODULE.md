# Hướng Dẫn Phát Triển Module Cho VIC CLI

Tài liệu hướng dẫn kỹ thuật cách viết, thử nghiệm và tích hợp module mở rộng cho hệ thống `vic-cli`.

---

## 1. Cơ Chế Hoạt Động Của Module

Khi bạn chạy lệnh:
```bash
vic <tên_module> [tham_số_1] [tham_số_2] ...
```

Nếu `<tên_module>` không trùng với các lệnh cốt lõi (core commands như `ui`, `share`, `sc`, `light`...), `vic` sẽ tự động:
1. Tìm file thực thi tại:
   - **Linux / macOS**: `~/.vic/modules/<tên_module>`
   - **Windows**: `%USERPROFILE%\.vic\modules\<tên_module>.exe`
2. Chuyển giao toàn bộ danh sách tham số còn lại (`[tham_số_1]`, `[tham_số_2]`...) cho file thực thi đó.
3. Chờ tiến trình hoàn tất và trả mã thoát (exit code) cùng kết quả ra màn hình terminal hoặc Web UI.

---

## 2. Hỗ Trợ Ngôn Ngữ

Module có thể được viết bằng **bất kỳ ngôn ngữ lập trình nào**, miễn là hệ thống có trình thông dịch hoặc file đã được biên dịch thành file nhị phân (binary):
- **Bash / Shell script**: Phù hợp cho các tác vụ tự động hóa hệ thống nhanh.
- **Python**: Phù hợp cho xử lý dữ liệu, API, kịch bản nâng cao.
- **Rust / Go / C / C++**: Phù hợp cho các module cần tốc độ cao, độc lập không phụ thuộc môi trường cài đặt.
- **Node.js**: Phù hợp cho tương tác web, dịch vụ mạng.

---

## 3. Các Bước Tạo Module Mới

### Bước 1: Tạo file mã nguồn trong thư mục modules

Tạo file mới trực tiếp trong thư mục `~/.vic/modules/`:

```bash
mkdir -p ~/.vic/modules
```

### Bước 2: Viết mã nguồn

#### Ví dụ 1: Module bằng Shell Script (`battery`)

Tạo file `~/.vic/modules/battery`:
```bash
#!/usr/bin/env bash

case "$1" in
    status|"")
        if command -v upower >/dev/null 2>&1; then
            upower -i $(upower -e | grep battery | head -n1) | grep -E "state|percentage|time to"
        elif [ -d /sys/class/power_supply/BAT0 ]; then
            cat /sys/class/power_supply/BAT0/status
            cat /sys/class/power_supply/BAT0/capacity
        else
            echo "Khong tim thay thong tin pin tren thiet bi nay."
        fi
        ;;
    percent)
        if [ -f /sys/class/power_supply/BAT0/capacity ]; then
            cat /sys/class/power_supply/BAT0/capacity
        else
            echo "N/A"
        fi
        ;;
    *)
        echo "Cu phap: vic battery [status|percent]"
        exit 1
        ;;
esac
```

Cấp quyền thực thi:
```bash
chmod +x ~/.vic/modules/battery
```

Chạy thử nghiệm ngay:
```bash
vic battery
vic battery percent
```

---

#### Ví dụ 2: Module bằng Python (`weather`)

Tạo file `~/.vic/modules/weather`:
```python
#!/usr/bin/env python3
import sys
import urllib.request
import json

city = sys.argv[1] if len(sys.argv) > 1 else ""
url = f"https://wttr.in/{city}?format=3" if city else "https://wttr.in/?format=3"

try:
    req = urllib.request.Request(url, headers={"User-Agent": "curl/7.68.0"})
    with urllib.request.urlopen(req, timeout=5) as resp:
        print(resp.read().decode("utf-8").strip())
except Exception as e:
    print(f"Loi ket noi: {e}")
    sys.exit(1)
```

Cấp quyền thực thi:
```bash
chmod +x ~/.vic/modules/weather
```

Chạy thử nghiệm ngay:
```bash
vic weather
vic weather Hanoi
```

---

#### Ví dụ 3: Module bằng Rust (Biên dịch binary độc lập)

Nếu bạn phát triển module bằng Rust trong dự án `vic-cli`:
1. Tạo file mã nguồn trong `modules_src/<tên_module>.rs`.
2. Thêm khai báo binary vào `Cargo.toml`:
   ```toml
   [[bin]]
   name = "my_module"
   path = "modules_src/my_module.rs"
   ```
3. Biên dịch release:
   ```bash
   cargo build --release --bin my_module
   ```
4. Copy binary vào thư mục modules:
   ```bash
   cp target/release/my_module ~/.vic/modules/my_module
   chmod +x ~/.vic/modules/my_module
   ```

---

## 4. Tích Hợp Vào Bảng Điều Khiển Web UI & Danh Mục CLI

Để module của bạn xuất hiện trong `vic list`, `vic install list` và bảng điều khiển Web UI (`vic ui`), hãy khai báo thông tin vào file `~/.vic/module.md` (hoặc `module.md` trong mã nguồn repo khi đóng góp):

### Cấu trúc khai báo:
```markdown
---
## [15] battery
- type: module
- vi_short: Kiem tra trang thai pin
- en_short: Battery status checker
- vi_desc: Hien thi dung luong pin, trang thai sac va thoi gian con lai cua laptop.
- en_desc: Show laptop battery capacity, charging state, and remaining runtime.
- usage: vic battery [status|percent]
- ui_actions: btn:Kiem tra=status | btn:Xem phan tram=percent
```

### Quy tắc định dạng `ui_actions`:
Trường `ui_actions` dùng để tạo tự động các nút bấm và ô nhập liệu trên Web UI Dashboard:
- **Tạo nút bấm chạy lệnh trực tiếp (`btn`)**:
  - Cú pháp: `btn:<Nhãn nút>=<tham số>`
  - Ví dụ: `btn:Play/Pause=play | btn:Next=next`
- **Tạo nút xem kết quả (`view`)**:
  - Cú pháp: `view:<Nhãn nút>=<tham số>`
  - Ví dụ: `view:Xem Top 5=`
- **Tạo ô nhập dữ liệu kèm nút gửi (`input`)**:
  - Cú pháp: `input:<Nhãn nút>=<tiền tố tham số>`
  - Ví dụ: `input:Kill PID=kill` (khi người dùng nhập `1234` và bấm nút, hệ thống sẽ thực thi `vic <tên_module> kill 1234`).
- Các hành động được ngăn cách nhau bằng dấu gạch đứng `|`.

---

## 5. Quy Chuẩn Đóng Góp Module Lên Cộng Đồng (Pull Request)

Nếu muốn đưa module của bạn vào danh mục chính thức phát hành qua `https://vic-cli.phuongvy.xyz`:

1. **Vị trí mã nguồn**: Đặt file Rust trong thư mục `modules_src/<tên>.rs`.
2. **Khai báo Cargo.toml**: Thêm block `[[bin]]` cho module mới.
3. **Cập nhật CI/CD**: Kiểm tra `.github/workflows/release.yml` để đảm bảo binary được copy và tạo mã băm SHA256 cho các nền tảng (Linux, Windows, macOS).
4. **Cập nhật registry**: Thêm mục khai báo tương ứng vào `module.md` của repo với ID tiếp theo, bao gồm các URL tải binary và mã băm SHA256.
5. **Độc lập và gọn nhẹ**:
   - Module không nên làm tăng kích thước binary quá lớn.
   - Luôn xử lý lỗi ngoại lệ và trả thông báo rõ ràng cho cả Tiếng Việt và Tiếng Anh.
   - Tuân thủ quy chuẩn không dùng emoji trong mã nguồn và thông báo.
