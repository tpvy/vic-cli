# VIC CLI — Personal Remote Workspace Assistant

> Giải pháp tối giản giúp bạn điều khiển, giám sát và trao đổi dữ liệu với máy tính trực tiếp từ điện thoại: **Gọn — Nhẹ — Nhanh — Bảo mật**.

Hệ sinh thái công cụ dòng lệnh (CLI) bằng Rust và bảng điều khiển Web di động (Mobile Web C2) độc lập, kết nối an toàn qua mạng riêng ảo Tailscale Mesh mà không cần mở bất kỳ cổng nào ra Internet (Zero Open Ports).

---

## Cài Đặt Nhanh

### 1. Dành Cho Người Dùng (Thủ công)

* **Linux & macOS**:
  ```bash
  curl -fsSL https://vic-cli.phuongvy.xyz/install.sh | bash
  ```
  *Tùy chọn: Thêm cờ `-E` để dùng tiếng Anh, `-d` để chỉ tải binary mà không sửa `PATH`.*

* **Windows (PowerShell)**:
  ```powershell
  irm https://vic-cli.phuongvy.xyz/install.ps1 | iex
  ```
  *Tùy chọn tiếng Anh: `$env:VIC_LANG='en'; irm https://vic-cli.phuongvy.xyz/install.ps1 | iex`*

> [!NOTE]
> Mọi bản binary phát hành chính thức đều được đối soát mã băm toàn vẹn SHA256 tự động khi tải và nâng cấp (`vic upgrade`). Bạn có thể tải script về kiểm tra trước khi chạy nếu muốn: `curl -fsSL https://vic-cli.phuongvy.xyz/install.sh > install.sh && less install.sh`.

### 2. Dành Cho AI Coding Agent (Antigravity, Claude Code, Cursor, Codex, Devin...)

Nếu bạn đang làm việc với một AI Coding Agent trong terminal hoặc IDE, chỉ cần copy & paste câu prompt dưới đây để Agent tự động thực hiện toàn bộ quy trình:

> "Hãy cài đặt và thiết lập vic-cli trên hệ thống của tôi:
> 1. Chạy script cài đặt (Linux: `https://vic-cli.phuongvy.xyz/install.sh`, Windows: `https://vic-cli.phuongvy.xyz/install.ps1`).
> 2. Chạy `vic setup` (trên Linux) để tự động cấp quyền udev, tạo symlink và cài các gói phụ thuộc còn thiếu.
> 3. Chạy `vic fast` để kiểm tra kết quả và in IP Tailscale."

**Lệnh 1 dòng chạy tự động trong sandbox / terminal của Agent:**
* **Linux / macOS**:
  ```bash
  curl -fsSL https://vic-cli.phuongvy.xyz/install.sh | bash && vic setup && vic fast
  ```
* **Windows (PowerShell)**:
  ```powershell
  powershell -NoExit -ExecutionPolicy Bypass -Command "irm https://vic-cli.phuongvy.xyz/install.ps1 | iex; vic fast"
  ```

---

## Hướng Dẫn Điều Khiển Máy Tính Từ Điện Thoại (Tailscale + Termius)

Mục tiêu: Ngồi sofa, nằm trên giường hoặc ở quán cà phê chỉ cần mở điện thoại là điều khiển trọn vẹn máy tính mà không cần ngồi trước màn hình.

### Bước 1: Kết nối mạng an toàn qua Tailscale (Không cần mở cổng modem)
1. Tải và cài đặt ứng dụng Tailscale trên máy tính & điện thoại: [tailscale.com/download](https://tailscale.com/download).
2. Đăng nhập cùng một tài khoản (Google, GitHub, Microsoft, Apple...) trên cả hai thiết bị.
3. Kích hoạt kết nối Tailscale:
   * **Linux**: `sudo tailscale up`
   * **Windows / macOS**: Bật switch **Connect** trong ứng dụng Tailscale.
   * **Điện thoại (iOS / Android)**: Mở app Tailscale và gạt nút bật VPN.
4. Lấy địa chỉ IP Tailscale của máy tính (dạng `100.x.y.z`) bằng lệnh:
   ```bash
   vic fast
   ```

### Bước 2: Bật dịch vụ SSH trên máy tính (Chỉ làm 1 lần duy nhất)
* **Arch Linux**:
  ```bash
  sudo pacman -S --needed openssh && sudo systemctl enable --now sshd
  ```
* **Ubuntu / Debian**:
  ```bash
  sudo apt update && sudo apt install -y openssh-server && sudo systemctl enable --now ssh
  ```
* **Fedora**:
  ```bash
  sudo dnf install -y openssh-server && sudo systemctl enable --now sshd
  ```
* **Windows (Mở PowerShell với quyền Administrator)**:
  ```powershell
  Add-WindowsCapability -Online -Name OpenSSH.Server~~~~0.0.1.0
  Start-Service sshd
  Set-Service -Name sshd -StartupType 'Automatic'
  ```

### Bước 3: Kết nối và gõ lệnh qua Termius trên điện thoại
1. Tải ứng dụng **Termius** (miễn phí trên iOS App Store hoặc Android Google Play).
2. Mở Termius -> Chọn tab **Hosts** -> Bấm **New Host**:
   * **Hostname / IP**: Nhập IP Tailscale của máy tính (ví dụ: `100.82.66.29`).
   * **Port**: `22`.
   * **Username**: Tên tài khoản đăng nhập máy tính của bạn (kiểm tra bằng lệnh `whoami` trên PC).
   * **Password**: Mật khẩu đăng nhập máy tính của bạn.
3. Bấm **Save** rồi nhấn vào Host vừa tạo để kết nối. Bạn đã có một màn hình dòng lệnh mượt mà trên điện thoại để chạy mọi lệnh `vic`:
   ```bash
   vic light 50         # Chỉnh độ sáng màn hình 50%
   vic au 80            # Chỉnh âm lượng loa 80%
   vic sc - iphone      # Chụp màn hình máy tính gửi thẳng qua iPhone
   vic lock             # Khóa màn hình máy tính
   ```

> [!TIP]
> **Khuyến nghị bảo mật SSH**: Để bảo mật tối đa và tránh nguy cơ dò mật khẩu, bạn nên sử dụng **Tailscale SSH** (`sudo tailscale up --ssh`) để xác thực an toàn qua tài khoản Tailscale mà không cần gõ mật khẩu, hoặc sử dụng SSH Key trong Termius thay vì Password.

### Bước 4: Mở bảng điều khiển Web cảm ứng trên điện thoại (`vic ui`)
Nếu không muốn gõ lệnh terminal, bạn có thể mở giao diện Web điều khiển trực quan:
1. Trong Termius trên điện thoại, gõ:
   ```bash
   vic ui -d
   ```
2. Màn hình sẽ in ra đường dẫn Web kèm mã PIN bảo mật, ví dụ:
   ```text
   -> Tailscale URL : http://100.82.66.29:3667?pin=101595
   ```
3. Bấm mở link đó trên trình duyệt Safari hoặc Chrome của điện thoại.
4. Bạn sẽ có ngay một bảng điều khiển di động: kéo thanh chỉnh độ sáng, âm lượng loa, điều khiển bài hát đang phát, duyệt và tải file từ máy tính về máy điện thoại chỉ bằng 1 chạm!

> [!WARNING]
> **Khuyến nghị an toàn mạng**: Web UI được thiết kế để truy cập an toàn qua mạng mã hóa đầu cuối WireGuard của Tailscale. Tránh sử dụng cờ `--lan` khi đang kết nối vào Wi-Fi công cộng (quán cà phê, sân bay) vì Web UI truyền dữ liệu qua giao thức HTTP không mã hóa.

---

## Bảng Tra Cứu Lệnh (Cheat Sheet)

### Lệnh Hệ Thống (Core Commands)

| Lệnh | Cú pháp | Chức năng |
| :--- | :--- | :--- |
| **ui** | `vic ui [-d] [--lan]`<br>`vic ui stop`<br>`vic ui status`<br>`vic ui restart` | Mở bảng điều khiển Web Mobile trên điện thoại (mặc định cổng 3667, tự chuyển cổng nếu bận). Mặc định chỉ lắng nghe trên giao diện Tailscale (khóa mạng LAN/Wi-Fi, dùng `--lan` nếu muốn mở). Sinh mã PIN ngẫu nhiên bảo mật mỗi phiên, hỗ trợ đặt mật khẩu riêng cố định (`vic pass`), tích hợp Anti-Brute-Force không chặn luồng server (khóa IP 5 phút nếu sai quá 5 lần). Chạy nền (`-d`), dừng (`vic ui stop`), xem trạng thái (`vic ui status`). |
| **pass** | `vic pass [mật_khẩu]`<br>`vic ui pass [mật_khẩu]` | Đặt hoặc đổi mật khẩu riêng cố định (chữ và số) để truy cập Web UI mọi lúc thay cho mã PIN ngẫu nhiên. Dùng `vic pass clear` để xóa. |
| **share** | `vic share [cổng] [thư_mục] [-d] [-u]` | Mở máy chủ chia sẻ tệp HTTP (mặc định cổng 3668, không đụng cổng UI). Thêm `-d` để mở `~/.vic/share`, thêm `-u` để bật upload từ điện thoại lên PC. |
| **sc** | `vic sc [-a] [- [thiết_bị]]` | Chụp ảnh màn hình máy tính. Thêm `-a` để lưu kho lưu trữ, thêm `- [thiết_bị]` (vd: `vic sc - iphone`) để gửi thẳng qua Taildrop 1 chạm. |
| **fast** | `vic fast` | Hiển thị thông số phần cứng (`fastfetch`) kèm IP LAN và IP Tailscale. |
| **light** | `vic light <0-100>` | Điều chỉnh độ sáng màn hình máy tính từ xa. |
| **au** | `vic au <0-100 \| mute>` | Điều chỉnh âm lượng loa hệ thống hoặc bật/tắt tiếng (mute). |
| **clip** | `vic clip [nội_dung]` | Đọc nội dung clipboard của máy tính hoặc gửi văn bản từ điện thoại sang PC. |
| **lock** | `vic lock` / `vic sleep` | Khóa màn hình làm việc hoặc đưa máy vào chế độ ngủ ngay lập tức. |
| **mouse** | `vic mouse [-d] [--lan] [cổng]`<br>`vic mouse stop`<br>`vic mouse status`<br>`vic mouse click [l\|r\|m]`<br>`vic mouse move <dx> <dy>`<br>`vic mouse scroll <dy>` | Biến điện thoại (iPhone/Android) thành bàn di chuột cảm ứng Trackpad không dây siêu mượt qua Safari/Chrome (mặc định cổng 3669). Hỗ trợ chạm 1 ngón di chuột, 1 ngón chạm nhẹ click trái, 2 ngón chạm nhẹ click phải, 2 ngón vuốt cuộn trang, thanh cuộn riêng biệt, nút bấm chuột trái/phải to và các phím tắt nhanh Esc, Space, Enter. Chạy nền (`-d`), dừng (`vic mouse stop`), xem trạng thái (`vic mouse status`). |
| **setup** | `vic setup` / `vic doctor` | Tự động phân quyền udev độ sáng, symlink toàn cục, cài gói phụ thuộc còn thiếu (`vic setup`) hoặc kiểm tra công cụ (`vic doctor`). |
| **upgrade** | `vic upgrade [-f] [-y]` | Kiểm tra GitHub Releases và hỏi xác nhận trước khi nâng cấp binary (thêm `-y` để tự động xác nhận). |

### Quản Lý Cấu Hình & Registry

| Lệnh | Cú pháp | Chức năng |
| :--- | :--- | :--- |
| **list** | `vic list` | Xem danh mục các lệnh Core và Modules mở rộng. |
| **sync** | `vic sync` | Đồng bộ danh mục module mới nhất từ máy chủ. |
| **lang** | `vic lang <vi\|en>` | Chuyển đổi ngôn ngữ hiển thị CLI giữa Tiếng Việt và Tiếng Anh. |
| **install** | `vic install list`<br>`vic install <tên \| id \| all> [-f] [-y]` | Xem danh mục module (`vic install list`), cài đặt module (hiển thị SHA256 và hỏi xác nhận `[y/N]` trước khi tải; thêm `-f` để cài đè, `-y` để bỏ qua hỏi). |

---

## Kho Module Mở Rộng (Extension Modules)

Toàn bộ các gói tính năng mở rộng chính thức được cập nhật liên tục tại:
* **[https://vic-cli.phuongvy.xyz/modules](https://vic-cli.phuongvy.xyz/modules)**

| Module | Cú pháp | Chức năng |
| :--- | :--- | :--- |
| **[10] media** | `vic media <play\|pause\|next\|prev>` | Điều khiển nhạc / video đang phát trên PC (Spotify, YouTube, VLC...). |
| **[11] top** | `vic top`<br>`vic top kill <1-5 \| PID>` | Xem top 5 ứng dụng ngốn nhiều CPU/RAM nhất và tắt nhanh theo số thứ tự hoặc PID. |
| **[12] mouse** | `vic mouse [-d] [--lan] [cổng]`<br>`vic mouse stop`<br>`vic mouse status` | Biến điện thoại thành chuột bay (cảm biến con quay) và bàn di trackpad ảo không dây. |

Xem danh sách và trạng thái cài đặt ngay trong terminal:
```bash
vic install list
```

---

## Tự Viết Module Tùy Biến (Custom Modules)

`vic-cli` được thiết kế theo kiến trúc plugin mở, cho phép bạn tự viết module mở rộng bằng **bất kỳ ngôn ngữ nào** (Bash, Python, Rust, Go, Node.js, C...) và thực thi trực tiếp qua cú pháp `vic <tên_module>`:

### Cách tạo nhanh trong 1 phút:
1. Tạo file thực thi trong thư mục `~/.vic/modules/<tên_module>` (hoặc `%USERPROFILE%\.vic\modules\<tên_module>.exe` trên Windows).
2. Cấp quyền thực thi: `chmod +x ~/.vic/modules/<tên_module>`.
3. Chạy ngay: `vic <tên_module> [tham_số]`.

Ví dụ tạo module `hello` bằng Bash script:
```bash
echo -e '#!/usr/bin/env bash\necho "Xin chào $1 từ module VIC!"' > ~/.vic/modules/hello
chmod +x ~/.vic/modules/hello
vic hello World
```

Xem toàn bộ tài liệu hướng dẫn kỹ thuật chi tiết, cách tích hợp nút bấm vào Web UI di động và quy chuẩn đóng góp module tại: **[CREATE_MODULE.md](CREATE_MODULE.md)**.

---

## Kiến Trúc Hạ Tầng (Cloudflare & GitHub)

* **Domain Chính Thức**: [https://vic-cli.phuongvy.xyz](https://vic-cli.phuongvy.xyz)
* **Kho Module**: [https://vic-cli.phuongvy.xyz/modules](https://vic-cli.phuongvy.xyz/modules)
* **Cloudflare Edge Worker**: Tự động phân luồng: Trình duyệt xem trang Landing & Modules Hub, Terminal (`curl`/`irm`) tải script cài đặt, tự động chuyển hướng tải binary siêu tốc.
* **CI/CD**: GitHub Actions tự động biên dịch đa nền tảng (Linux musl x86_64/aarch64, Windows x64, macOS Apple Silicon) ngay khi phát hành phiên bản.