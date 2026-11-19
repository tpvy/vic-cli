# VIC Command & Module Registry

---
## [1] sc
- type: core
- vi_short: Chụp ảnh màn hình
- en_short: Take screenshot
- vi_desc: Chụp màn hình máy tính. Mặc định lưu tạm, dùng cờ -a để lưu vào ~/.vic/archive, cờ - [thiết_bị] để gửi thẳng qua Taildrop 1 chạm.
- en_desc: Capture PC screen. Use -a to save in ~/.vic/archive, - [peer] to send directly via Taildrop.
- usage: vic sc [-a] [- [peer]]

---
## [2] fast
- type: core
- vi_short: Thông tin máy & IP
- en_short: System info & IPs
- vi_desc: Hiển thị cấu hình phần cứng (fastfetch) kèm địa chỉ IP LAN và IP Tailscale.
- en_desc: Show hardware summary (fastfetch) with LAN and Tailscale IPs.
- usage: vic fast

---
## [3] share
- type: core
- vi_short: Chia sẻ file qua Web
- en_short: Web file sharing
- vi_desc: Mở máy chủ HTTP chia sẻ file (mặc định cổng 3668). Dùng -d để mở thư mục chung ~/.vic/share, thêm -u để cho phép điện thoại tải file lên PC.
- en_desc: Start HTTP file server (default port 3668). Use -d for ~/.vic/share, add -u to allow file uploads from phone to PC.
- usage: vic share [port] [dir] [-d] [-u]

---
## [4] light
- type: core
- vi_short: Chỉnh độ sáng màn hình
- en_short: Screen brightness
- vi_desc: Điều chỉnh độ sáng màn hình từ 0% đến 100%.
- en_desc: Set screen brightness from 0% to 100%.
- usage: vic light <0-100>

---
## [5] au
- type: core
- vi_short: Chỉnh âm lượng hệ thống
- en_short: System audio volume
- vi_desc: Điều chỉnh âm lượng từ 0% đến 100% hoặc bật/tắt tiếng (mute).
- en_desc: Set volume from 0% to 100% or toggle mute.
- usage: vic au <0-100 | mute>

---
## [6] clip
- type: core
- vi_short: Đồng bộ Clipboard
- en_short: Clipboard sync
- vi_desc: Không truyền tham số để đọc Clipboard của PC, hoặc truyền chữ vào để copy sang PC.
- en_desc: Run without args to read PC clipboard, or pass text to copy to PC.
- usage: vic clip [text]

---
## [7] lock
- type: core
- vi_short: Khóa máy / Ngủ (sleep)
- en_short: Lock / Sleep PC
- vi_desc: Khóa màn hình ngay lập tức hoặc đưa máy vào chế độ ngủ (vic sleep).
- en_desc: Lock workstation immediately or suspend system (vic sleep).
- usage: vic lock | vic sleep

---
## [8] ui
- type: core
- vi_short: Giao diện điều khiển Web
- en_short: Web Remote Dashboard
- vi_desc: Mở bảng điều khiển trực quan trên trình duyệt điện thoại qua mạng Tailscale (mặc định cổng 3667). Tích hợp chia sẻ tệp, đồng bộ clipboard, Web Terminal tương tác trực tiếp và điều khiển modules. Hỗ trợ chạy ngầm (-d), dừng tiến trình (vic ui stop), kiểm tra trạng thái (vic ui status) và khởi động lại (vic ui restart).
- en_desc: Launch mobile web dashboard over Tailscale (default port 3667) with integrated file sharing, clipboard sync, interactive Web Terminal, and module controls. Supports background daemon (-d), stop daemon (vic ui stop), check status (vic ui status), and restart (vic ui restart).
- usage: vic ui [port] [-d] [--lan] | vic ui stop | vic ui status | vic ui restart

---
## [9] setup
- type: core
- vi_short: Cấu hình & Kiểm tra
- en_short: Setup & Doctor
- vi_desc: Cấu hình và quản lý môi trường hệ thống: xem danh sách (list), cài tất cả (all), cài/gỡ từng mục riêng lẻ (udev, symlink, package...), dọn dẹp file tạm (clean), kiểm tra công cụ (vic doctor) hoặc tự nâng cấp (vic upgrade).
- en_desc: Configure and manage system environment: view list, install all, install/remove single item (udev, symlink, package...), clean temp files (clean), check tools (vic doctor), or self-upgrade (vic upgrade).
- usage: vic setup [list|all|<item>|rm <item>|clean] | vic doctor | vic upgrade | vic lang <vi|en> | vic sync

---
## [10] media
- type: module
- vi_short: Điều khiển nhạc & video
- en_short: Media playback control
- vi_desc: Tạm dừng, phát tiếp hoặc chuyển bài hát/video đang phát trên máy tính.
- en_desc: Play, pause, or skip active media tracks on PC.
- usage: vic media <play|pause|next|prev>
- ui_actions: btn:Prev=prev | btn:Play/Pause=play | btn:Next=next
- linux_url: https://vic-cli.phuongvy.xyz/bin/linux/media
- windows_url: https://vic-cli.phuongvy.xyz/bin/windows/media.exe
- mac_url: https://vic-cli.phuongvy.xyz/bin/mac/media
- sha256_linux: 2b779806b26df91181d31c18f02d70b5da44c41028ea62619d01bdcc5fe36725
- sha256_windows: fc52ba05143f8efe0af94db09a2bbb7b88577006d24ae176ef53400bc5434832
- sha256_mac: 88ac1d7e8744b97e8230a9634a5dd70ce3c649e8c462f27ec3e06b018677a12c

---
## [11] top
- type: module
- vi_short: Xem & tắt ứng dụng nặng
- en_short: View & kill heavy processes
- vi_desc: Hiển thị 5 tiến trình ngốn CPU/RAM nhất và tắt nhanh theo số thứ tự.
- en_desc: Show top 5 resource-heavy processes and kill by index.
- usage: vic top [kill <id>]
- ui_actions: view:Xem Top 5= | input:Kill ID=kill
- linux_url: https://vic-cli.phuongvy.xyz/bin/linux/top
- windows_url: https://vic-cli.phuongvy.xyz/bin/windows/top.exe
- mac_url: https://vic-cli.phuongvy.xyz/bin/mac/top
- sha256_linux: 8d172fa2a480965a65a6be484b9f9e2df182b61b6833d430c28c0f608d1e43f1
- sha256_windows: 38a697c0f6ef849fdb46ddc78ac258d808cd45d8e2aaa7dce2169edf7b24c26b
- sha256_mac: 4317f965c5863e41b7ff625fabf1681bfea3468577e51fa97056acee2262ef14

---
## [12] mouse
- type: module
- vi_short: Chuột bay & Bàn di Trackpad
- en_short: Air Mouse & Virtual Trackpad
- vi_desc: Biến điện thoại thành chuột bay (cảm biến con quay) và bàn di chuột ảo qua WiFi/Tailscale.
- en_desc: Control mouse using mobile gyro sensor and virtual trackpad over WiFi/Tailscale.
- usage: vic mouse [-d] [--lan] [port] | vic mouse stop | vic mouse status
- ui_actions: btn:Chạy nền=-d | btn:Dừng=stop | btn:Trạng thái=status
- linux_url: https://vic-cli.phuongvy.xyz/bin/linux/mouse
- windows_url: https://vic-cli.phuongvy.xyz/bin/windows/mouse.exe
- mac_url: https://vic-cli.phuongvy.xyz/bin/mac/mouse
- sha256_linux: 15a5d71f25243ebd89c32382f47d30b50fac650a522eb6d7051ff836123e2f7c
- sha256_windows: 11045ce787839e0ec534fc2d8a405a00370413523bd7c9f03b6a612155f80b43
- sha256_mac: 9ee0320629264168fd6115502e701189ee2821e7329215ecbc406cf07a77c060


