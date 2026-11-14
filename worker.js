const GITHUB_RAW_BASE = "https://raw.githubusercontent.com/tpvy/vic-cli/main";
const OFFICIAL_REPO = "https://github.com/tpvy/vic-cli";
const AUTHOR_GITHUB = "https://github.com/tpvy";
const MAIN_SITE = "https://phuongvy.xyz";
const TICKET_URL = "https://phuongvy.xyz/question/";
const GITHUB_ISSUES = "https://github.com/tpvy/vic-cli/issues";
const CREATE_MODULE_DOC = "https://github.com/tpvy/vic-cli/blob/main/CREATE_MODULE.md";

async function fetchFromGithub(file, contentType) {
  const res = await fetch(`${GITHUB_RAW_BASE}/${file}?t=${Math.floor(Date.now() / 60000)}`, {
    cf: { cacheTtl: 60 },
  });
  if (!res.ok) {
    return new Response(`Error fetching ${file}: ${res.statusText}`, {
      status: res.status,
      headers: { "access-control-allow-origin": "*" },
    });
  }
  const body = await res.text();
  return new Response(body, {
    headers: {
      "content-type": contentType,
      "access-control-allow-origin": "*",
      "cache-control": "public, max-age=60",
    },
  });
}

function parseModuleMdText(text) {
  const modules = [];
  if (!text) return modules;
  const sections = text.split(/^---$/m);
  for (const sec of sections) {
    const headerMatch = sec.match(/##\s*\[(\d+)\]\s*([a-zA-Z0-9_\-]+)/);
    if (!headerMatch) continue;
    const id = parseInt(headerMatch[1], 10);
    const name = headerMatch[2].trim();
    const item = { id, name, type: "module" };

    const lines = sec.split("\n");
    for (const line of lines) {
      const kv = line.match(/^-\s*([a-zA-Z0-9_]+):\s*(.*)$/);
      if (kv) {
        const key = kv[1].trim();
        const val = kv[2].trim();
        item[key] = val;
      }
    }
    modules.push(item);
  }
  return modules;
}

function escapeHtml(str) {
  if (!str) return "";
  return String(str)
    .replace(/&/g, "&amp;")
    .replace(/</g, "&lt;")
    .replace(/>/g, "&gt;")
    .replace(/"/g, "&quot;")
    .replace(/'/g, "&#39;");
}

function renderLandingHtml() {
  const html = `<!DOCTYPE html>
<html lang="vi">
<head>
<meta charset="utf-8">
<meta name="viewport" content="width=device-width,initial-scale=1">
<title>vic-cli — Remote Workspace Assistant</title>
<link rel="preconnect" href="https://fonts.googleapis.com">
<link rel="preconnect" href="https://fonts.gstatic.com" crossorigin>
<link href="https://fonts.googleapis.com/css2?family=JetBrains+Mono:wght@400;500;600;700&display=swap" rel="stylesheet">
<script src="https://cdnjs.cloudflare.com/ajax/libs/three.js/r128/three.min.js"></script>
<style>
  :root {
    --bg: #090d16;
    --card: #0f1523;
    --card-hover: #141c2e;
    --border: #1e293b;
    --border-subtle: #172033;
    --primary: #3b82f6;
    --primary-hover: #2563eb;
    --accent: #10b981;
    --text: #f1f5f9;
    --muted: #94a3b8;
    --dim: #64748b;
    --code-bg: #050811;
    --font-mono: 'JetBrains Mono', ui-monospace, monospace;
  }
  * { box-sizing: border-box; margin: 0; padding: 0; }
  html { scroll-behavior: smooth; }
  body {
    background: var(--bg);
    color: var(--text);
    font-family: -apple-system, BlinkMacSystemFont, "Segoe UI", Roboto, sans-serif;
    line-height: 1.55;
    padding: 16px 18px 90px;
    max-width: 860px;
    margin: auto;
  }
  a { color: var(--primary); text-decoration: none; }
  a:hover { text-decoration: underline; }

  /* Navbar */
  .nav {
    display: flex;
    justify-content: space-between;
    align-items: center;
    padding: 12px 0 20px;
    border-bottom: 1px solid var(--border);
    margin-bottom: 28px;
    font-size: 13.5px;
    flex-wrap: wrap;
    gap: 12px;
  }
  .brand {
    font-family: var(--font-mono);
    font-weight: 700;
    font-size: 15.5px;
    display: flex;
    align-items: center;
    gap: 8px;
    color: var(--text);
  }
  .brand-dot {
    width: 7px;
    height: 7px;
    border-radius: 50%;
    background: var(--accent);
    box-shadow: 0 0 6px var(--accent);
  }
  .badge-ver {
    font-size: 11px;
    padding: 2px 7px;
    background: #172554;
    color: #93c5fd;
    border-radius: 5px;
    font-family: var(--font-mono);
  }
  .nav-links { display: flex; gap: 14px; align-items: center; flex-wrap: wrap; }
  .nav-links a { color: var(--muted); }
  .nav-links a:hover { color: var(--text); text-decoration: none; }
  .btn-nav-home {
    color: #60a5fa !important;
    font-weight: 600;
  }
  .btn-nav-hub {
    background: #1e3a8a;
    color: #93c5fd !important;
    padding: 4px 10px;
    border-radius: 6px;
    font-size: 12px;
    font-weight: 600;
    border: 1px solid #2563eb;
  }
  .btn-nav-hub:hover { background: #1d4ed8; }
  .gh-link {
    font-size: 12px;
    background: #1e293b;
    color: #e2e8f0 !important;
    padding: 4px 10px;
    border-radius: 6px;
    border: 1px solid var(--border);
  }
  .gh-link:hover { background: #334155; }

  /* 3D Hub Container */
  .hero-3d-box {
    position: relative;
    border-radius: 14px;
    background: radial-gradient(circle at 50% 50%, #0d1527 0%, #090d16 100%);
    border: 1px solid var(--border);
    overflow: hidden;
    margin-bottom: 32px;
    padding: 24px 20px 28px;
    text-align: center;
    box-shadow: 0 10px 30px rgba(0,0,0,0.5);
  }
  #hub-canvas {
    width: 100%;
    height: 220px;
    display: block;
    margin: -10px auto 10px;
    cursor: grab;
  }
  #hub-canvas:active { cursor: grabbing; }

  /* Hero Content */
  .hero-title {
    font-family: var(--font-mono);
    font-size: clamp(32px, 5.5vw, 46px);
    font-weight: 800;
    letter-spacing: -1px;
    color: #ffffff;
    margin-bottom: 8px;
    line-height: 1.1;
  }
  .hero-title span { color: var(--primary); }
  .hero-desc {
    font-size: 15px;
    color: var(--muted);
    max-width: 600px;
    margin: 0 auto 16px;
    line-height: 1.6;
  }

  /* Tags */
  .tags-row {
    display: flex;
    justify-content: center;
    gap: 8px;
    flex-wrap: wrap;
    margin-bottom: 22px;
  }
  .tag-item {
    font-size: 12px;
    background: #0d131f;
    border: 1px solid var(--border);
    color: #cbd5e1;
    padding: 3px 10px;
    border-radius: 6px;
    font-family: var(--font-mono);
  }

  /* Hero Buttons */
  .hero-actions {
    display: flex;
    justify-content: center;
    gap: 12px;
    flex-wrap: wrap;
  }
  .btn-primary {
    background: var(--primary);
    color: white !important;
    padding: 9px 18px;
    border-radius: 7px;
    font-size: 13px;
    font-weight: 600;
    transition: background 0.15s;
  }
  .btn-primary:hover { background: var(--primary-hover); text-decoration: none; }
  .btn-secondary {
    background: #1e293b;
    border: 1px solid var(--border);
    color: #f1f5f9 !important;
    padding: 9px 16px;
    border-radius: 7px;
    font-size: 13px;
    font-weight: 600;
  }
  .btn-secondary:hover { background: #334155; text-decoration: none; }
  .btn-ticket {
    background: rgba(16, 185, 129, 0.12);
    border: 1px solid rgba(16, 185, 129, 0.3);
    color: #34d399 !important;
    padding: 9px 16px;
    border-radius: 7px;
    font-size: 13px;
    font-weight: 600;
  }
  .btn-ticket:hover { background: rgba(16, 185, 129, 0.2); text-decoration: none; }

  /* Install Bar */
  .install-box {
    background: var(--card);
    border: 1px solid var(--border);
    border-radius: 10px;
    padding: 16px;
    margin-bottom: 32px;
  }
  .install-title { font-size: 13px; font-weight: 700; color: #f8fafc; margin-bottom: 10px; }
  .install-tabs { display: flex; gap: 6px; margin-bottom: 8px; }
  .tab-btn {
    background: transparent;
    border: none;
    color: var(--muted);
    padding: 4px 10px;
    border-radius: 5px;
    font-size: 12px;
    font-weight: 600;
    cursor: pointer;
  }
  .tab-btn.active { background: #1e293b; color: #60a5fa; }
  .cmd-bar {
    display: flex;
    align-items: center;
    justify-content: space-between;
    background: var(--code-bg);
    border: 1px solid var(--border);
    border-radius: 7px;
    padding: 9px 12px;
    font-family: var(--font-mono);
    font-size: 13px;
    gap: 10px;
  }
  .cmd-prompt { color: var(--dim); user-select: none; }
  .cmd-text { color: #93c5fd; overflow-x: auto; white-space: nowrap; flex: 1; }
  .copy-btn {
    background: var(--primary);
    color: white;
    border: none;
    padding: 6px 12px;
    border-radius: 5px;
    font-size: 12px;
    font-weight: 600;
    cursor: pointer;
    white-space: nowrap;
    transition: background 0.15s;
  }
  .copy-btn:hover { background: var(--primary-hover); }
  .copy-btn.copied { background: var(--accent); }

  /* Dropdown Steps Accordion */
  .section-title {
    font-size: 17px;
    font-weight: 700;
    margin: 32px 0 8px;
    color: #f8fafc;
  }
  .section-desc { font-size: 13px; color: var(--muted); margin-bottom: 14px; }

  .steps-accordion { display: flex; flex-direction: column; gap: 8px; margin-bottom: 32px; }
  .step-item {
    background: var(--card);
    border: 1px solid var(--border);
    border-radius: 8px;
    overflow: hidden;
  }
  .step-header {
    padding: 12px 16px;
    display: flex;
    justify-content: space-between;
    align-items: center;
    cursor: pointer;
    user-select: none;
    font-size: 13.5px;
    font-weight: 600;
    color: #f1f5f9;
  }
  .step-header:hover { background: var(--card-hover); }
  .step-arrow { font-size: 11px; color: var(--dim); transition: transform 0.2s; }
  .step-item.open .step-arrow { transform: rotate(180deg); color: #60a5fa; }
  .step-body {
    padding: 0 16px 14px;
    font-size: 13px;
    color: var(--muted);
    line-height: 1.6;
    display: none;
    border-top: 1px solid #141d2e;
  }
  .step-item.open .step-body { display: block; }
  .step-link-btn {
    display: inline-flex;
    align-items: center;
    gap: 4px;
    background: #1e293b;
    border: 1px solid var(--border);
    color: #60a5fa !important;
    padding: 3px 9px;
    border-radius: 4px;
    font-size: 11.5px;
    margin-top: 8px;
  }
  .step-link-btn:hover { background: #334155; }

  /* Ticket Callout */
  .ticket-callout {
    background: #0d1527;
    border: 1px solid rgba(59, 130, 246, 0.4);
    border-radius: 10px;
    padding: 16px 18px;
    margin-bottom: 32px;
    display: flex;
    justify-content: space-between;
    align-items: center;
    gap: 16px;
    flex-wrap: wrap;
  }
  .ticket-text h3 { font-size: 14.5px; font-weight: 700; color: #f8fafc; margin-bottom: 4px; }
  .ticket-text p { font-size: 12.5px; color: #94a3b8; }
  .ticket-actions { display: flex; gap: 8px; }

  /* Collapsible Cheat Sheet */
  details.cheat-details {
    background: var(--card);
    border: 1px solid var(--border);
    border-radius: 8px;
    overflow: hidden;
    margin-bottom: 32px;
  }
  details.cheat-details summary {
    padding: 12px 16px;
    font-size: 13.5px;
    font-weight: 600;
    color: #f1f5f9;
    cursor: pointer;
    user-select: none;
    display: flex;
    justify-content: space-between;
    align-items: center;
  }
  details.cheat-details summary:hover { background: var(--card-hover); }
  .summary-badge {
    font-family: var(--font-mono);
    font-size: 11px;
    background: #1e293b;
    color: #93c5fd;
    padding: 2px 7px;
    border-radius: 4px;
  }
  .table-box {
    width: 100%;
    border-collapse: collapse;
    font-size: 12.5px;
    border-top: 1px solid var(--border);
  }
  .table-box th {
    text-align: left;
    padding: 9px 12px;
    background: #0d131f;
    border-bottom: 1px solid var(--border);
    color: var(--muted);
    font-size: 11px;
    font-weight: 600;
  }
  .table-box td {
    padding: 9px 12px;
    border-bottom: 1px solid var(--border);
    color: #cbd5e1;
  }
  .table-box tr:last-child td { border-bottom: none; }
  .cmd-name { font-family: var(--font-mono); color: #60a5fa; font-weight: 600; }

  /* Footer */
  footer {
    margin-top: 48px;
    padding-top: 18px;
    border-top: 1px solid var(--border);
    display: flex;
    justify-content: space-between;
    align-items: center;
    font-size: 12px;
    color: var(--dim);
    flex-wrap: wrap;
    gap: 8px;
  }
  .footer-links { display: flex; gap: 14px; flex-wrap: wrap; }
  .footer-links a { color: var(--dim); }
  .footer-links a:hover { color: var(--text); }
</style>
</head>
<body>

  <!-- Navbar -->
  <div class="nav">
    <div class="brand">
      <span class="brand-dot"></span>
      <span>vic-cli</span>
      <span class="badge-ver">v1.0.0</span>
    </div>
    <div class="nav-links">
      <a href="${MAIN_SITE}" class="btn-nav-home">← phuongvy.xyz</a>
      <a href="/modules" class="btn-nav-hub">Modules Hub ↗</a>
      <a href="${OFFICIAL_REPO}" target="_blank" class="gh-link">GitHub ↗</a>
    </div>
  </div>

  <!-- Hero & 3D Interactive Hub -->
  <div class="hero-3d-box">
    <canvas id="hub-canvas"></canvas>
    
    <h1 class="hero-title">vic<span>-cli</span></h1>
    <p class="hero-desc">Trợ lý điều khiển máy trạm từ xa qua Tailscale WireGuard Mesh. Biến chiếc điện thoại thành bảng điều khiển và trao đổi dữ liệu tức thời: Gọn — Nhẹ — Nhanh — Bảo mật.</p>

    <div class="tags-row">
      <span class="tag-item">&lt;5ms Boot</span>
      <span class="tag-item">Zero Open Ports</span>
      <span class="tag-item">Pure Rust Core</span>
      <span class="tag-item">WireGuard Mesh</span>
      <span class="tag-item">Mobile Web C2</span>
      <span class="tag-item">&lt;8MB RAM</span>
    </div>

    <div class="hero-actions">
      <a href="/modules" class="btn-primary">Kho Module (/modules) →</a>
      <a href="${OFFICIAL_REPO}" target="_blank" class="btn-secondary">GitHub Repository ↗</a>
      <a href="${TICKET_URL}" class="btn-ticket">Gửi Ticket / Báo Lỗi ↗</a>
    </div>
  </div>

  <!-- Quick Install Bar -->
  <div class="install-box">
    <div class="install-title">Cài đặt nhanh 1-chạm:</div>
    <div class="install-tabs">
      <button class="tab-btn active" id="tab-bash" onclick="setInstallTab('bash')">Linux / macOS (Bash)</button>
      <button class="tab-btn" id="tab-ps1" onclick="setInstallTab('ps1')">Windows (PowerShell)</button>
    </div>
    <div class="cmd-bar">
      <span class="cmd-prompt" id="cmd-prompt">$</span>
      <span class="cmd-text" id="install-cmd">curl -fsSL https://vic-cli.phuongvy.xyz/install.sh | bash</span>
      <button class="copy-btn" id="copy-btn" onclick="copyInstall()">Sao chép</button>
    </div>
  </div>

  <!-- Dropdown Steps (Các Bước Cần Làm) -->
  <h2 class="section-title">Các Bước Cần Làm (Hướng Dẫn Nhanh)</h2>
  <p class="section-desc">Nhấn vào từng bước bên dưới để xem hướng dẫn chi tiết kèm liên kết tải ứng dụng.</p>

  <div class="steps-accordion">
    <!-- Step 1 -->
    <div class="step-item open">
      <div class="step-header" onclick="toggleStep(this)">
        <span>Bước 1: Cài đặt mạng riêng ảo Tailscale WireGuard</span>
        <span class="step-arrow">▼</span>
      </div>
      <div class="step-body">
        Cài đặt ứng dụng Tailscale trên cả máy tính và điện thoại. Đăng nhập cùng một tài khoản để hai thiết bị tự động liên kết thành mạng P2P Mesh WireGuard mã hóa đầu cuối. Bạn hoàn toàn không cần mở bất kỳ cổng nào trên router/modem (Zero Open Ports).
        <div>
          <a href="https://tailscale.com/download" target="_blank" class="step-link-btn">Tải Tailscale ↗</a>
        </div>
      </div>
    </div>

    <!-- Step 2 -->
    <div class="step-item">
      <div class="step-header" onclick="toggleStep(this)">
        <span>Bước 2: Mở dịch vụ SSH trên máy tính</span>
        <span class="step-arrow">▼</span>
      </div>
      <div class="step-body">
        Kích hoạt dịch vụ SSH server trên máy trạm để sẵn sàng nhận lệnh điều khiển.
        <br>&bull; Trên Linux: <code>sudo systemctl enable --now sshd</code>
        <br>&bull; Trên Windows: Cài đặt OpenSSH Server qua <em>Settings &gt; Optional Features</em> và khởi chạy service.
      </div>
    </div>

    <!-- Step 3 -->
    <div class="step-item">
      <div class="step-header" onclick="toggleStep(this)">
        <span>Bước 3: Kết nối SSH qua ứng dụng Termius trên điện thoại</span>
        <span class="step-arrow">▼</span>
      </div>
      <div class="step-body">
        Cài đặt app Termius (iOS / Android) trên smartphone. Tạo Host mới với địa chỉ IP là IP Tailscale (dạng <code>100.x.y.z</code>) của máy tính. Giờ đây bạn có thể mở terminal máy trạm chỉ với 1-chạm mọi lúc mọi nơi.
        <div>
          <a href="https://termius.com" target="_blank" class="step-link-btn">Tải Termius ↗</a>
        </div>
      </div>
    </div>

    <!-- Step 4 -->
    <div class="step-item">
      <div class="step-header" onclick="toggleStep(this)">
        <span>Bước 4: Mở bảng điều khiển Web Remote C2 di động</span>
        <span class="step-arrow">▼</span>
      </div>
      <div class="step-body">
        Trong terminal Termius, chạy lệnh: <code>vic ui -d</code>
        <br>Lệnh sẽ khởi chạy máy chủ điều khiển ngầm cổng 3667 và sinh một mã PIN ngẫu nhiên bảo mật. Chỉ cần click vào đường link (dạng <code>http://100.x.y.z:3667?pin=...</code>) để mở bảng điều khiển trên trình duyệt điện thoại.
      </div>
    </div>
  </div>

  <!-- Ticket & Bug Report Callout -->
  <div class="ticket-callout">
    <div class="ticket-text">
      <h3>Gặp sự cố hoặc muốn đóng góp ý kiến?</h3>
      <p>Bạn có thể gửi ticket trực tiếp qua hệ thống ticket của phuongvy.xyz hoặc tạo Issue trên GitHub.</p>
    </div>
    <div class="ticket-actions">
      <a href="${TICKET_URL}" class="btn-ticket">Gửi Ticket ↗</a>
      <a href="${GITHUB_ISSUES}" target="_blank" class="btn-secondary">GitHub Issues ↗</a>
    </div>
  </div>

  <!-- Collapsible Cheat Sheet -->
  <details class="cheat-details">
    <summary>
      <span>▼ Bảng tra cứu lệnh nhanh (Core Cheat Sheet)</span>
      <span class="summary-badge">10 lệnh core</span>
    </summary>
    <table class="table-box">
      <thead>
        <tr>
          <th style="width: 80px;">Lệnh</th>
          <th>Cú pháp câu lệnh</th>
          <th>Chức năng</th>
        </tr>
      </thead>
      <tbody>
        <tr>
          <td><span class="cmd-name">ui</span></td>
          <td><code>vic ui [-d] [--lan] [cổng]</code></td>
          <td>Mở Web C2 Dashboard (3667). Dùng <code>vic ui stop</code> để tắt, <code>status</code> để kiểm tra.</td>
        </tr>
        <tr>
          <td><span class="cmd-name">share</span></td>
          <td><code>vic share [dir] [cổng] [-d] [-u]</code></td>
          <td>Máy chủ chia sẻ file HTTP độc lập (3668). Thêm <code>-u</code> để upload file.</td>
        </tr>
        <tr>
          <td><span class="cmd-name">sc</span></td>
          <td><code>vic sc [-a] [- [peer]]</code></td>
          <td>Chụp ảnh màn hình và gửi thẳng 1-chạm qua Taildrop tới smartphone.</td>
        </tr>
        <tr>
          <td><span class="cmd-name">fast</span></td>
          <td><code>vic fast</code></td>
          <td>Hiển thị nhanh cấu hình phần cứng (fastfetch), RAM, IP LAN và IP Tailscale.</td>
        </tr>
        <tr>
          <td><span class="cmd-name">light / au</span></td>
          <td><code>vic light &lt;0-100&gt;</code> / <code>vic au &lt;0-100&gt;</code></td>
          <td>Chỉnh độ sáng màn hình (brightnessctl) và âm lượng loa từ xa.</td>
        </tr>
        <tr>
          <td><span class="cmd-name">clip</span></td>
          <td><code>vic clip [text]</code></td>
          <td>Đọc nội dung clipboard máy trạm hoặc gửi nội dung từ điện thoại sang PC.</td>
        </tr>
        <tr>
          <td><span class="cmd-name">install</span></td>
          <td><code>vic install &lt;tên | id | all&gt;</code></td>
          <td>Cài đặt thêm các module mở rộng từ registry Cloudflare Edge.</td>
        </tr>
        <tr>
          <td><span class="cmd-name">setup</span></td>
          <td><code>vic setup</code></td>
          <td>Cấu hình quyền udev, group video, tạo symlink và cài gói thiếu qua server.</td>
        </tr>
        <tr>
          <td><span class="cmd-name">update</span></td>
          <td><code>vic update</code></td>
          <td>Nâng cấp binary tự động trong 1 giây qua GitHub Releases API.</td>
        </tr>
      </tbody>
    </table>
  </details>

  <!-- Footer -->
  <footer>
    <div>Tác giả: <a href="${AUTHOR_GITHUB}" target="_blank"><strong>tpvy</strong></a> &bull; Giấy phép MIT</div>
    <div class="footer-links">
      <a href="${MAIN_SITE}">phuongvy.xyz</a>
      <a href="${OFFICIAL_REPO}" target="_blank">GitHub vic-cli</a>
      <a href="${TICKET_URL}">Gửi Ticket</a>
      <a href="/modules">Kho Module</a>
    </div>
  </footer>

<script>
  function setInstallTab(type) {
    document.querySelectorAll('.tab-btn').forEach(function(b) { b.classList.remove('active'); });
    var promptEl = document.getElementById('cmd-prompt');
    var cmdEl = document.getElementById('install-cmd');
    if (type === 'bash') {
      document.getElementById('tab-bash').classList.add('active');
      promptEl.innerText = '$';
      cmdEl.innerText = 'curl -fsSL https://vic-cli.phuongvy.xyz/install.sh | bash';
    } else {
      document.getElementById('tab-ps1').classList.add('active');
      promptEl.innerText = 'PS >';
      cmdEl.innerText = 'irm https://vic-cli.phuongvy.xyz/install.ps1 | iex';
    }
  }

  function copyInstall() {
    var text = document.getElementById('install-cmd').innerText;
    navigator.clipboard.writeText(text);
    var btn = document.getElementById('copy-btn');
    btn.innerText = '[OK] Đã sao chép';
    btn.classList.add('copied');
    setTimeout(function() {
      btn.innerText = 'Sao chép';
      btn.classList.remove('copied');
    }, 2000);
  }

  function toggleStep(header) {
    var item = header.parentElement;
    item.classList.toggle('open');
  }

  /* 3D Hub Canvas with Three.js */
  (function init3D() {
    var canvas = document.getElementById('hub-canvas');
    if (!canvas || typeof THREE === 'undefined') return;

    var scene = new THREE.Scene();
    var camera = new THREE.PerspectiveCamera(50, canvas.clientWidth / canvas.clientHeight, 0.1, 1000);
    camera.position.z = 4.2;

    var renderer = new THREE.WebGLRenderer({ canvas: canvas, alpha: true, antialias: true });
    renderer.setSize(canvas.clientWidth, canvas.clientHeight);
    renderer.setPixelRatio(Math.min(window.devicePixelRatio, 2));

    // Outer wireframe icosahedron
    var geom = new THREE.IcosahedronGeometry(1.8, 1);
    var wireMat = new THREE.MeshBasicMaterial({ color: 0x2563eb, wireframe: true, transparent: true, opacity: 0.35 });
    var mesh = new THREE.Mesh(geom, wireMat);
    scene.add(mesh);

    // Inner glowing nodes
    var pointsGeom = new THREE.BufferGeometry();
    var pos = geom.attributes.position.array;
    pointsGeom.setAttribute('position', new THREE.BufferAttribute(pos, 3));
    var pointsMat = new THREE.PointsMaterial({ color: 0x38bdf8, size: 0.08, transparent: true, opacity: 0.85 });
    var points = new THREE.Points(pointsGeom, pointsMat);
    mesh.add(points);

    // Central pulsing node (P2P Mesh Hub)
    var centerGeom = new THREE.SphereGeometry(0.35, 16, 16);
    var centerMat = new THREE.MeshBasicMaterial({ color: 0x10b981, wireframe: true });
    var center = new THREE.Mesh(centerGeom, centerMat);
    scene.add(center);

    var mouseX = 0, mouseY = 0;
    var targetX = 0, targetY = 0;

    window.addEventListener('mousemove', function(e) {
      var rect = canvas.getBoundingClientRect();
      var x = e.clientX - rect.left - (rect.width / 2);
      var y = e.clientY - rect.top - (rect.height / 2);
      mouseX = (x / rect.width) * 1.5;
      mouseY = (y / rect.height) * 1.5;
    });

    function animate() {
      requestAnimationFrame(animate);
      targetX += (mouseX - targetX) * 0.05;
      targetY += (mouseY - targetY) * 0.05;

      mesh.rotation.y += 0.005;
      mesh.rotation.x += 0.002;
      mesh.rotation.x = targetY * 0.5 + 0.1;
      mesh.rotation.y += targetX * 0.02;

      center.rotation.y -= 0.008;
      renderer.render(scene, camera);
    }
    animate();

    window.addEventListener('resize', function() {
      if (!canvas) return;
      var w = canvas.clientWidth;
      var h = canvas.clientHeight;
      camera.aspect = w / h;
      camera.updateProjectionMatrix();
      renderer.setSize(w, h);
    });
  })();
</script>
</body>
</html>`;

  return new Response(html, {
    headers: {
      "content-type": "text/html; charset=utf-8",
      "access-control-allow-origin": "*",
      "cache-control": "public, max-age=60",
    },
  });
}

function renderModulesHubHtml(modules) {
  const coreList = modules.filter(m => m.type === "core");
  const extList = modules.filter(m => m.type !== "core");

  // Render Core items as compact expandable accordions
  const coreHtml = coreList.map(m => `
    <div class="accordion-item" id="core-item-${m.id}">
      <div class="accordion-header" onclick="toggleAccordion('core-item-${m.id}')">
        <div class="acc-title-left">
          <span class="acc-id">[${m.id}]</span>
          <span class="acc-name">${escapeHtml(m.name)}</span>
          <span class="acc-short">— ${escapeHtml(m.vi_short || '')}</span>
        </div>
        <div class="acc-title-right">
          <span class="badge-core">Built-in Core</span>
          <span class="acc-arrow">▼</span>
        </div>
      </div>
      <div class="accordion-body">
        <div class="acc-desc-vi">
          <strong>${escapeHtml(m.vi_short || '')}</strong>
          <p>${escapeHtml(m.vi_desc || '')}</p>
        </div>
        ${m.en_desc ? `
          <div class="acc-desc-en">
            <span class="en-tag">EN:</span> ${escapeHtml(m.en_short || '')} — ${escapeHtml(m.en_desc || '')}
          </div>
        ` : ''}
        <div class="acc-usage">
          <span class="usage-label">Cú pháp:</span>
          <code>${escapeHtml(m.usage || 'vic ' + m.name)}</code>
        </div>
        <div class="acc-note">
          * Lệnh tích hợp sẵn trong vic-cli binary (không cần cài thêm).
        </div>
      </div>
    </div>
  `).join("\n");

  // Render Extension Module items with [+] selection and collapsible details
  const extHtml = extList.map(m => `
    <div class="accordion-item ext-item" id="mod-item-${escapeHtml(m.name)}" data-name="${escapeHtml(m.name)}">
      <div class="accordion-header">
        <div class="acc-title-left" onclick="toggleAccordion('mod-item-${escapeHtml(m.name)}')">
          <span class="acc-id">[${m.id}]</span>
          <span class="acc-name">${escapeHtml(m.name)}</span>
          <span class="acc-short">— ${escapeHtml(m.vi_short || '')}</span>
        </div>
        <div class="acc-title-right">
          <button class="btn-plus-mod" id="btn-plus-${escapeHtml(m.name)}" onclick="toggleSelectMod('${escapeHtml(m.name)}', this)" title="Chọn vào lệnh cài đặt">+</button>
          <span class="acc-arrow" onclick="toggleAccordion('mod-item-${escapeHtml(m.name)}')">▼</span>
        </div>
      </div>
      <div class="accordion-body">
        <div class="acc-desc-vi">
          <strong>${escapeHtml(m.vi_short || '')}</strong>
          <p>${escapeHtml(m.vi_desc || '')}</p>
        </div>
        ${m.en_desc ? `
          <div class="acc-desc-en">
            <span class="en-tag">EN:</span> ${escapeHtml(m.en_short || '')} — ${escapeHtml(m.en_desc || '')}
          </div>
        ` : ''}
        <div class="acc-usage">
          <span class="usage-label">Cú pháp:</span>
          <code>${escapeHtml(m.usage || 'vic ' + m.name)}</code>
        </div>

        <div class="ext-install-bar">
          <span class="ext-cmd">vic install ${escapeHtml(m.name)}</span>
          <button class="btn-copy-sm" onclick="copySnippet('vic install ${escapeHtml(m.name)}', this)">Sao chép</button>
        </div>

        <div class="ext-bin-row">
          <span class="bin-label">Tải binary trực tiếp:</span>
          ${m.linux_url ? `<a href="${escapeHtml(m.linux_url)}" class="bin-link" target="_blank">Linux</a>` : ''}
          ${m.windows_url ? `<a href="${escapeHtml(m.windows_url)}" class="bin-link" target="_blank">Windows (.exe)</a>` : ''}
          ${m.mac_url ? `<a href="${escapeHtml(m.mac_url)}" class="bin-link" target="_blank">macOS</a>` : ''}
        </div>

        ${m.sha256_linux ? `
          <div class="sha-row">
            <span class="sha-title">SHA256 (Linux):</span>
            <span class="sha-code">${escapeHtml(m.sha256_linux)}</span>
            <button class="btn-copy-sha" onclick="copySnippet('${escapeHtml(m.sha256_linux)}', this)">Copy</button>
          </div>
        ` : ''}
      </div>
    </div>
  `).join("\n");

  const html = `<!DOCTYPE html>
<html lang="vi">
<head>
<meta charset="utf-8">
<meta name="viewport" content="width=device-width,initial-scale=1">
<title>Kho Module Lệnh Mở Rộng — vic-cli</title>
<link rel="preconnect" href="https://fonts.googleapis.com">
<link rel="preconnect" href="https://fonts.gstatic.com" crossorigin>
<link href="https://fonts.googleapis.com/css2?family=JetBrains+Mono:wght@400;500;600;700&display=swap" rel="stylesheet">
<style>
  :root {
    --bg: #090d16;
    --card: #0f1523;
    --card-hover: #141c2e;
    --border: #1e293b;
    --border-subtle: #172033;
    --primary: #3b82f6;
    --primary-hover: #2563eb;
    --accent: #10b981;
    --text: #f1f5f9;
    --muted: #94a3b8;
    --dim: #64748b;
    --code-bg: #050811;
    --font-mono: 'JetBrains Mono', ui-monospace, monospace;
  }
  * { box-sizing: border-box; margin: 0; padding: 0; }
  html { scroll-behavior: smooth; }
  body {
    background: var(--bg);
    color: var(--text);
    font-family: -apple-system, BlinkMacSystemFont, "Segoe UI", Roboto, sans-serif;
    line-height: 1.55;
    padding: 16px 18px 90px;
    max-width: 860px;
    margin: auto;
  }
  a { color: var(--primary); text-decoration: none; }
  a:hover { text-decoration: underline; }

  /* Navbar */
  .nav {
    display: flex;
    justify-content: space-between;
    align-items: center;
    padding: 12px 0 20px;
    border-bottom: 1px solid var(--border);
    margin-bottom: 28px;
    font-size: 13.5px;
    flex-wrap: wrap;
    gap: 12px;
  }
  .brand {
    font-family: var(--font-mono);
    font-weight: 700;
    font-size: 15.5px;
    display: flex;
    align-items: center;
    gap: 8px;
    color: var(--text);
  }
  .brand-dot {
    width: 7px;
    height: 7px;
    border-radius: 50%;
    background: var(--accent);
    box-shadow: 0 0 6px var(--accent);
  }
  .badge-hub {
    font-size: 11px;
    padding: 2px 7px;
    background: #172554;
    color: #93c5fd;
    border-radius: 5px;
    font-family: var(--font-mono);
  }
  .nav-links { display: flex; gap: 14px; align-items: center; flex-wrap: wrap; }
  .nav-links a { color: var(--muted); }
  .nav-links a:hover { color: var(--text); text-decoration: none; }
  .btn-nav-home { color: #60a5fa !important; font-weight: 600; }
  .gh-link {
    font-size: 12px;
    background: #1e293b;
    color: #e2e8f0 !important;
    padding: 4px 10px;
    border-radius: 6px;
    border: 1px solid var(--border);
  }

  /* Header */
  .hub-header { margin-bottom: 24px; text-align: left; }
  .hub-badge {
    display: inline-block;
    font-size: 11px;
    font-weight: 600;
    letter-spacing: 0.5px;
    color: #60a5fa;
    background: rgba(59, 130, 246, 0.1);
    border: 1px solid rgba(59, 130, 246, 0.25);
    padding: 3px 10px;
    border-radius: 12px;
    margin-bottom: 8px;
  }
  .hub-header h1 { font-size: clamp(22px, 3.5vw, 32px); font-weight: 700; color: #f8fafc; margin-bottom: 8px; }
  .hub-header p { font-size: 14px; color: var(--muted); max-width: 680px; line-height: 1.6; }

  /* Terminal Command Reference Box */
  .cmd-guide-box {
    background: #0d1424;
    border: 1px solid var(--border);
    border-radius: 10px;
    padding: 16px;
    margin-bottom: 30px;
  }
  .cmd-guide-title { font-size: 13px; font-weight: 700; color: #93c5fd; margin-bottom: 10px; font-family: var(--font-mono); }
  .cmd-guide-grid {
    display: grid;
    grid-template-columns: repeat(auto-fit, minmax(240px, 1fr));
    gap: 10px;
  }
  .cmd-guide-item {
    background: #060911;
    border: 1px solid #1a2436;
    border-radius: 6px;
    padding: 8px 12px;
    font-size: 12px;
  }
  .cmd-guide-syntax { font-family: var(--font-mono); color: #60a5fa; font-weight: 600; margin-bottom: 3px; display: block; }
  .cmd-guide-desc { color: var(--muted); font-size: 11.5px; }

  /* Category Titles */
  .category-title {
    font-size: 16px;
    font-weight: 700;
    color: #f8fafc;
    margin: 28px 0 12px;
    display: flex;
    align-items: center;
    justify-content: space-between;
  }
  .cat-count {
    font-family: var(--font-mono);
    font-size: 11px;
    color: var(--dim);
    background: #0d131f;
    padding: 2px 7px;
    border-radius: 4px;
    border: 1px solid var(--border);
  }

  /* Accordion Item Styling */
  .accordion-list { display: flex; flex-direction: column; gap: 8px; margin-bottom: 24px; }
  .accordion-item {
    background: var(--card);
    border: 1px solid var(--border);
    border-radius: 8px;
    overflow: hidden;
    transition: border-color 0.15s, background 0.15s;
  }
  .accordion-item.selected { border-color: #3b82f6; background: #0f182c; }
  .accordion-header {
    padding: 11px 14px;
    display: flex;
    justify-content: space-between;
    align-items: center;
    cursor: pointer;
    user-select: none;
    font-size: 13px;
    gap: 8px;
  }
  .accordion-header:hover { background: var(--card-hover); }
  .acc-title-left { display: flex; align-items: center; gap: 8px; flex: 1; flex-wrap: wrap; }
  .acc-id { font-family: var(--font-mono); font-size: 11px; color: #60a5fa; font-weight: 700; }
  .acc-name { font-weight: 700; color: #f8fafc; font-size: 13.5px; }
  .acc-short { color: var(--muted); font-size: 12.5px; }
  .acc-title-right { display: flex; align-items: center; gap: 10px; }
  .badge-core { font-size: 10px; background: #1e293b; color: #94a3b8; padding: 2px 6px; border-radius: 4px; font-weight: 600; }
  .acc-arrow { font-size: 10px; color: var(--dim); transition: transform 0.2s; }
  .accordion-item.open .acc-arrow { transform: rotate(180deg); color: #60a5fa; }

  .btn-plus-mod {
    background: #1e293b;
    border: 1px solid #334155;
    color: #e2e8f0;
    width: 24px;
    height: 24px;
    border-radius: 5px;
    font-size: 14px;
    font-weight: 700;
    cursor: pointer;
    display: flex;
    align-items: center;
    justify-content: center;
    transition: all 0.15s;
  }
  .btn-plus-mod:hover { background: #334155; }
  .accordion-item.selected .btn-plus-mod { background: var(--primary); border-color: var(--primary); color: white; }

  /* Accordion Body */
  .accordion-body {
    padding: 0 14px 14px;
    font-size: 12.5px;
    color: var(--muted);
    line-height: 1.55;
    display: none;
    border-top: 1px solid #141d2e;
  }
  .accordion-item.open .accordion-body { display: block; padding-top: 12px; }

  .acc-desc-vi strong { font-size: 13px; color: #f1f5f9; display: block; margin-bottom: 3px; }
  .acc-desc-vi p { font-size: 12.5px; color: var(--muted); margin-bottom: 8px; }
  .acc-desc-en { font-size: 11.5px; color: var(--dim); margin-bottom: 8px; }
  .en-tag { font-size: 10px; font-weight: 700; color: #64748b; }
  .acc-usage {
    background: #060911;
    border: 1px solid var(--border);
    border-radius: 5px;
    padding: 5px 8px;
    font-family: var(--font-mono);
    font-size: 11.5px;
    color: #93c5fd;
    margin-bottom: 8px;
    overflow-x: auto;
    white-space: nowrap;
  }
  .usage-label { color: var(--dim); margin-right: 4px; }
  .acc-note { font-size: 11.5px; color: var(--dim); font-style: italic; }

  .ext-install-bar {
    display: flex;
    align-items: center;
    justify-content: space-between;
    background: #060911;
    border: 1px solid var(--border);
    border-radius: 6px;
    padding: 6px 10px;
    font-family: var(--font-mono);
    font-size: 12px;
    margin-bottom: 8px;
    gap: 8px;
  }
  .ext-cmd { color: #60a5fa; overflow-x: auto; white-space: nowrap; flex: 1; }
  .btn-copy-sm {
    background: #1e293b;
    border: none;
    color: #cbd5e1;
    padding: 3px 8px;
    border-radius: 4px;
    font-size: 11px;
    font-weight: 600;
    cursor: pointer;
  }
  .btn-copy-sm:hover { background: #334155; }

  .ext-bin-row { display: flex; gap: 6px; align-items: center; font-size: 11px; flex-wrap: wrap; margin-bottom: 6px; }
  .bin-label { color: var(--dim); }
  .bin-link {
    background: #172033;
    border: 1px solid var(--border);
    color: #cbd5e1;
    padding: 2px 7px;
    border-radius: 4px;
    font-size: 11px;
  }
  .bin-link:hover { background: #1e293b; color: var(--text); text-decoration: none; }

  .sha-row {
    display: flex;
    align-items: center;
    gap: 6px;
    font-size: 10.5px;
    color: var(--dim);
    font-family: var(--font-mono);
  }
  .sha-title { font-size: 10px; }
  .sha-code { color: #94a3b8; overflow: hidden; text-overflow: ellipsis; white-space: nowrap; max-width: 320px; }
  .btn-copy-sha { background: transparent; border: none; color: var(--primary); cursor: pointer; font-size: 10.5px; }

  /* Floating Module Cart / Dock */
  .mod-dock {
    position: fixed;
    bottom: 20px;
    left: 50%;
    transform: translateX(-50%) translateY(110px);
    width: calc(100% - 36px);
    max-width: 600px;
    background: #0d1424;
    border: 1px solid #3b82f6;
    border-radius: 10px;
    padding: 10px 16px;
    box-shadow: 0 12px 30px rgba(0,0,0,0.7);
    display: flex;
    align-items: center;
    justify-content: space-between;
    gap: 12px;
    z-index: 1000;
    transition: transform 0.22s cubic-bezier(0.16, 1, 0.3, 1);
  }
  .mod-dock.active { transform: translateX(-50%) translateY(0); }
  .dock-left { display: flex; flex-direction: column; gap: 2px; overflow: hidden; }
  .dock-title { font-size: 11px; color: var(--muted); }
  .dock-cmd { font-family: var(--font-mono); font-size: 13px; color: #60a5fa; white-space: nowrap; overflow-x: auto; }
  .dock-actions { display: flex; gap: 6px; align-items: center; }
  .dock-copy-btn {
    background: var(--primary);
    color: white;
    border: none;
    padding: 6px 14px;
    border-radius: 5px;
    font-size: 12px;
    font-weight: 600;
    cursor: pointer;
    white-space: nowrap;
  }
  .dock-copy-btn:hover { background: var(--primary-hover); }
  .dock-clear-btn {
    background: transparent;
    border: 1px solid var(--border);
    color: var(--muted);
    padding: 5px 8px;
    border-radius: 5px;
    font-size: 11.5px;
    cursor: pointer;
  }
  .dock-clear-btn:hover { background: #1e293b; color: var(--text); }

  /* Dev Guide Banner */
  .dev-banner {
    background: var(--card);
    border: 1px solid var(--border);
    border-radius: 10px;
    padding: 18px;
    margin-top: 36px;
    display: flex;
    justify-content: space-between;
    align-items: center;
    gap: 16px;
    flex-wrap: wrap;
  }
  .dev-banner-text h3 { font-size: 14.5px; font-weight: 700; color: #f8fafc; margin-bottom: 4px; }
  .dev-banner-text p { font-size: 12.5px; color: var(--muted); line-height: 1.5; }
  .btn-dev {
    background: #1e293b;
    border: 1px solid #334155;
    color: #e2e8f0 !important;
    padding: 8px 14px;
    border-radius: 6px;
    font-size: 12px;
    font-weight: 600;
    white-space: nowrap;
  }
  .btn-dev:hover { background: #334155; text-decoration: none; }

  /* Footer */
  footer {
    margin-top: 48px;
    padding-top: 18px;
    border-top: 1px solid var(--border);
    display: flex;
    justify-content: space-between;
    align-items: center;
    font-size: 12px;
    color: var(--dim);
    flex-wrap: wrap;
    gap: 8px;
  }
  .footer-links { display: flex; gap: 14px; flex-wrap: wrap; }
  .footer-links a { color: var(--dim); }
  .footer-links a:hover { color: var(--text); }
</style>
</head>
<body>

  <!-- Navbar -->
  <div class="nav">
    <div class="brand">
      <span class="brand-dot"></span>
      <span>vic-cli / modules</span>
      <span class="badge-hub">Kho Module</span>
    </div>
    <div class="nav-links">
      <a href="/" class="btn-nav-home">← vic-cli</a>
      <a href="${MAIN_SITE}">phuongvy.xyz</a>
      <a href="/modules?json=1" target="_blank">JSON API</a>
      <a href="${OFFICIAL_REPO}" target="_blank" class="gh-link">GitHub ↗</a>
    </div>
  </div>

  <!-- Header -->
  <div class="hub-header">
    <span class="hub-badge">Modules Lệnh Mở Rộng</span>
    <h1>Kho Module Mở Rộng</h1>
    <p>Hệ sinh thái lệnh mở rộng micro-kernel cho vic-cli. Tự động đồng bộ thời gian thực từ <code>module.md</code> trên GitHub và đối soát mã băm SHA256.</p>
  </div>

  <!-- Command Guide Box -->
  <div class="cmd-guide-box">
    <div class="cmd-guide-title">Lệnh quản lý module trên Terminal:</div>
    <div class="cmd-guide-grid">
      <div class="cmd-guide-item">
        <span class="cmd-guide-syntax">vic install &lt;tên | id | all&gt;</span>
        <span class="cmd-guide-desc">Cài đặt module từ registry (VD: <code>vic install media</code> hoặc <code>vic install all</code>).</span>
      </div>
      <div class="cmd-guide-item">
        <span class="cmd-guide-syntax">vic list</span>
        <span class="cmd-guide-desc">Xem toàn bộ danh mục lệnh &amp; module kèm trạng thái đã cài đặt.</span>
      </div>
      <div class="cmd-guide-item">
        <span class="cmd-guide-syntax">vic update / upgrade</span>
        <span class="cmd-guide-desc">Tự động so sánh semver và nâng cấp binary vic-cli mới nhất.</span>
      </div>
      <div class="cmd-guide-item">
        <span class="cmd-guide-syntax">vic sync</span>
        <span class="cmd-guide-desc">Đồng bộ lại danh mục registry mới nhất từ server GitHub Raw.</span>
      </div>
    </div>
  </div>

  <!-- Category 1: Core Commands Accordion -->
  <div class="category-title">
    <span>1. Danh Mục Lệnh Core (Tích hợp sẵn)</span>
    <span class="cat-count">${coreList.length} lệnh</span>
  </div>
  <div class="accordion-list">
    ${coreHtml}
  </div>

  <!-- Category 2: Extension Modules Accordion -->
  <div class="category-title">
    <span>2. Danh Mục Modules Mở Rộng</span>
    <span class="cat-count">${extList.length} module</span>
  </div>
  <div class="accordion-list">
    ${extHtml}
  </div>

  <!-- Bottom Dev Guide -->
  <div class="dev-banner">
    <div class="dev-banner-text">
      <h3>Hướng dẫn tự tạo module riêng cho vic-cli</h3>
      <p>vic-cli hỗ trợ thực thi bất kỳ tệp tin nào đặt tại <code>~/.vic/modules/&lt;tên&gt;</code> (Bash, Python, Rust, Go, PowerShell...). Tự do mở rộng tính năng điều khiển máy tính của bạn.</p>
    </div>
    <a href="${CREATE_MODULE_DOC}" target="_blank" class="btn-dev">Xem Hướng Dẫn CREATE_MODULE.md ↗</a>
  </div>

  <!-- Floating Module Cart / Dock -->
  <div class="mod-dock" id="mod-dock">
    <div class="dock-left">
      <span class="dock-title" id="dock-title">1 module đã chọn</span>
      <span class="dock-cmd" id="dock-cmd">vic install media</span>
    </div>
    <div class="dock-actions">
      <button class="dock-copy-btn" id="dock-copy-btn" onclick="copyDockCmd()">Sao chép lệnh cài</button>
      <button class="dock-clear-btn" onclick="clearSelectedMods()" title="Bỏ chọn">&times;</button>
    </div>
  </div>

  <!-- Footer -->
  <footer>
    <div>Tác giả: <a href="${AUTHOR_GITHUB}" target="_blank"><strong>tpvy</strong></a> &bull; Giấy phép MIT</div>
    <div class="footer-links">
      <a href="${MAIN_SITE}">phuongvy.xyz</a>
      <a href="${OFFICIAL_REPO}" target="_blank">GitHub vic-cli</a>
      <a href="${TICKET_URL}">Gửi Ticket</a>
      <a href="/">Trang Chủ CLI</a>
    </div>
  </footer>

<script>
  function toggleAccordion(id) {
    var item = document.getElementById(id);
    if (item) item.classList.toggle('open');
  }

  function copySnippet(text, btn) {
    navigator.clipboard.writeText(text);
    var old = btn.innerText;
    btn.innerText = '[OK] Đã chép';
    setTimeout(function() { btn.innerText = old; }, 2000);
  }

  var selectedMods = [];

  function toggleSelectMod(name, btn) {
    var idx = selectedMods.indexOf(name);
    var item = document.getElementById('mod-item-' + name);

    if (idx > -1) {
      selectedMods.splice(idx, 1);
      if (item) item.classList.remove('selected');
      btn.innerText = '+';
    } else {
      selectedMods.push(name);
      if (item) item.classList.add('selected');
      btn.innerText = '[x]';
    }
    updateModDock();
  }

  function updateModDock() {
    var dock = document.getElementById('mod-dock');
    var dockCmd = document.getElementById('dock-cmd');
    var dockTitle = document.getElementById('dock-title');

    if (selectedMods.length > 0) {
      var cmdStr = 'vic install ' + selectedMods.join(' ');
      dockCmd.innerText = cmdStr;
      dockTitle.innerText = selectedMods.length + ' module đã chọn:';
      dock.classList.add('active');
    } else {
      dock.classList.remove('active');
    }
  }

  function copyDockCmd() {
    var text = document.getElementById('dock-cmd').innerText;
    navigator.clipboard.writeText(text);
    var btn = document.getElementById('dock-copy-btn');
    btn.innerText = '[OK] Đã sao chép!';
    setTimeout(function() { btn.innerText = 'Sao chép lệnh cài'; }, 2000);
  }

  function clearSelectedMods() {
    selectedMods.forEach(function(name) {
      var item = document.getElementById('mod-item-' + name);
      var btn = document.getElementById('btn-plus-' + name);
      if (item) item.classList.remove('selected');
      if (btn) btn.innerText = '+';
    });
    selectedMods = [];
    updateModDock();
  }
</script>
</body>
</html>`;

  return new Response(html, {
    headers: {
      "content-type": "text/html; charset=utf-8",
      "access-control-allow-origin": "*",
      "cache-control": "public, max-age=60",
    },
  });
}

export default {
  async fetch(request) {
    const url = new URL(request.url);
    const userAgent = (request.headers.get("user-agent") || "").toLowerCase();
    const accept = (request.headers.get("accept") || "").toLowerCase();

    if (url.pathname === "/version") {
      return new Response("1.0.0", {
        headers: {
          "content-type": "text/plain; charset=utf-8",
          "access-control-allow-origin": "*",
        },
      });
    }

    if (url.pathname === "/modules" || url.pathname === "/module") {
      let md = "";
      try {
        const res = await fetch(`${GITHUB_RAW_BASE}/module.md?t=${Math.floor(Date.now() / 60000)}`, {
          cf: { cacheTtl: 60 },
        });
        if (res.ok) {
          md = await res.text();
        }
      } catch (err) {
        md = "";
      }
      
      const modules = parseModuleMdText(md);

      if (url.searchParams.get("json") === "1" || accept.includes("application/json")) {
        return new Response(JSON.stringify(modules, null, 2), {
          headers: {
            "content-type": "application/json; charset=utf-8",
            "access-control-allow-origin": "*",
            "cache-control": "public, max-age=60",
          },
        });
      }

      return renderModulesHubHtml(modules);
    }

    if (url.pathname === "/install.ps1" || url.pathname === "/ps1") {
      return fetchFromGithub("install.ps1", "text/plain; charset=utf-8");
    }

    if (url.pathname === "/install.sh" || url.pathname === "/sh") {
      return fetchFromGithub("install.sh", "text/plain; charset=utf-8");
    }

    if (url.pathname === "/") {
      // Windows PowerShell clients get install.ps1 directly
      if (userAgent.includes("powershell") || userAgent.includes("pwsh")) {
        return fetchFromGithub("install.ps1", "text/plain; charset=utf-8");
      }
      // Unix/Linux terminal clients get install.sh
      if (
        userAgent.includes("curl") ||
        userAgent.includes("wget") ||
        userAgent.includes("httpie") ||
        (!accept.includes("text/html") && !userAgent.includes("mozilla"))
      ) {
        return fetchFromGithub("install.sh", "text/plain; charset=utf-8");
      }
      // Browsers get landing page
      return renderLandingHtml();
    }

    if (url.pathname === "/module.md") {
      return fetchFromGithub("module.md", "text/markdown; charset=utf-8");
    }

    const binMatch = url.pathname.match(/^\/bin\/(.+)$/);
    if (binMatch) {
      let rawPath = binMatch[1];
      let ghFile = rawPath;
      if (rawPath === "windows/vic.exe" || rawPath === "vic.exe") {
        ghFile = "vic-windows-x86_64.exe";
      } else if (rawPath === "windows/vic.exe.sha256" || rawPath === "vic.exe.sha256") {
        ghFile = "vic-windows-x86_64.exe.sha256";
      } else if (rawPath === "linux/vic" || rawPath === "vic-linux") {
        ghFile = "vic-linux-x86_64";
      } else if (rawPath === "linux/vic.sha256" || rawPath === "vic-linux.sha256") {
        ghFile = "vic-linux-x86_64.sha256";
      } else if (rawPath === "mac/vic" || rawPath === "vic-mac") {
        ghFile = "vic-macos-aarch64";
      } else if (rawPath === "mac/vic.sha256" || rawPath === "vic-mac.sha256") {
        ghFile = "vic-macos-aarch64.sha256";
      } else {
        const parts = rawPath.split("/");
        if (parts.length === 2) {
          ghFile = `${parts[0]}-${parts[1]}`;
        }
      }

      const ghUrl = `https://github.com/tpvy/vic-cli/releases/latest/download/${ghFile}`;
      try {
        const ghRes = await fetch(ghUrl, {
          redirect: "follow",
          headers: { "user-agent": "vic-cdn-worker" },
          cf: { cacheTtl: 300 },
        });
        if (ghRes.ok) {
          const respHeaders = new Headers(ghRes.headers);
          respHeaders.set("access-control-allow-origin", "*");
          respHeaders.set("cache-control", "public, max-age=300");
          return new Response(ghRes.body, {
            status: 200,
            headers: respHeaders,
          });
        }
      } catch (err) {}
      return Response.redirect(ghUrl, 302);
    }

    if (url.pathname === "/resolve") {
      const distro = url.searchParams.get("distro") || "unknown";
      const arch = url.searchParams.get("arch") || "x86_64";
      const session = url.searchParams.get("session") || "headless";
      const de = url.searchParams.get("de") || "none";
      const audio = url.searchParams.get("audio") || "none";

      let pkgs = ["tailscale", "fastfetch", "playerctl"];

      if (session === "wayland") {
        pkgs.push("wl-clipboard", "brightnessctl");
        if (de.includes("gnome")) {
          pkgs.push("gnome-screenshot");
        } else if (de.includes("kde") || de.includes("plasma")) {
          pkgs.push("spectacle");
        } else {
          pkgs.push("grim");
        }
      } else if (session === "x11") {
        pkgs.push("scrot", "xclip", "brightnessctl");
      }

      if (audio === "pipewire") {
        pkgs.push(distro === "arch" ? "wireplumber" : "pipewire-bin");
      } else if (audio === "pulseaudio") {
        pkgs.push(distro === "arch" ? "libpulse" : "pulseaudio-utils");
      }

      let install_cmd = "";
      if (distro === "arch") {
        install_cmd = `sudo pacman -S --needed --noconfirm ${pkgs.join(" ")}`;
      } else if (distro === "ubuntu" || distro === "debian") {
        install_cmd = `sudo apt-get update && sudo apt-get install -y ${pkgs.join(" ")}`;
      } else if (distro === "fedora") {
        install_cmd = `sudo dnf install -y ${pkgs.join(" ")}`;
      }

      const bin_name = `vic-linux-${arch === "aarch64" ? "aarch64" : "x86_64"}`;
      const bin_url = `https://github.com/tpvy/vic-cli/releases/latest/download/${bin_name}`;

      const responseScript = `echo "[*] Server resolved environment: distro=${distro}, session=${session}, de=${de}, audio=${audio}"
echo "[*] Required packages: ${pkgs.join(", ")}"
BIN_URL="${bin_url}"
PKG_CMD="${install_cmd}"
`;
      return new Response(responseScript.trim(), {
        headers: {
          "content-type": "text/plain; charset=utf-8",
          "access-control-allow-origin": "*",
        },
      });
    }

    return new Response(
      JSON.stringify({ status: "online", service: "vic-cli", version: "1.0.0" }),
      {
        status: 200,
        headers: {
          "content-type": "application/json; charset=utf-8",
          "access-control-allow-origin": "*",
        },
      }
    );
  },
};
