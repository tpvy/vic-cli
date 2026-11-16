#!/usr/bin/env bash
set -e

MODE="exec"
LANG_CODE="vi"
TARGET_OS=""
VIC_DIR="$HOME/.vic"
REPO_URL="https://github.com/tpvy/vic-cli.git"
RELEASE_BASE="https://github.com/tpvy/vic-cli/releases/latest/download"

for arg in "$@"; do
    case $arg in
        -d) MODE="download" ;;
        -e) MODE="exec" ;;
        -V) LANG_CODE="vi" ;;
        -E) LANG_CODE="en" ;;
        --l|--linux) TARGET_OS="linux" ;;
        --m|--mac) TARGET_OS="macos" ;;
    esac
done

if [ -z "$TARGET_OS" ]; then
    case "$(uname -s)" in
        Linux*)  TARGET_OS="linux" ;;
        Darwin*) TARGET_OS="macos" ;;
        *)
            if [ "$LANG_CODE" = "en" ]; then
                echo "[-] Unsupported OS"; exit 1
            else
                echo "[-] Hệ điều hành không được hỗ trợ"; exit 1
            fi
            ;;
    esac
fi

ARCH="$(uname -m)"
if [ "$ARCH" = "aarch64" ] || [ "$ARCH" = "arm64" ]; then
    ARCH_SUFFIX="aarch64"
else
    ARCH_SUFFIX="x86_64"
fi

BIN_NAME="vic-${TARGET_OS}-${ARCH_SUFFIX}"
BIN_URL="$RELEASE_BASE/$BIN_NAME"

mkdir -p "$VIC_DIR"/{bin,modules,archive,share}

KNOWN_LINUX_X86_64_SHA="daa4d4b637596e81a710d8b7f05110a93cb8d4198b1a8d85f98d02b7b4d8e639"
KNOWN_LINUX_AARCH64_SHA="8308deb38622a54bfe079678f1bebdf20cef9c3c28389140a0a8c2374f367a39"
KNOWN_MACOS_AARCH64_SHA="fcc16cedd8cb3d31e7ddc62d0f68c8fc0c53a8dab7ada56d3d7907a39cee5c59"

# Get expected SHA with cache-busting timestamp
EXPECTED_SHA=""
SHA_TMP="$VIC_DIR/bin/vic.sha256.tmp.$$"
if curl -fsSL "${BIN_URL}.sha256?t=$(date +%s)" -o "$SHA_TMP" 2>/dev/null && [ -s "$SHA_TMP" ]; then
    EXPECTED_SHA=$(awk '{print $1}' "$SHA_TMP" | tr '[:upper:]' '[:lower:]')
    rm -f "$SHA_TMP"
fi

if [ -z "$EXPECTED_SHA" ] || [ ${#EXPECTED_SHA} -ne 64 ]; then
    if [ "$TARGET_OS" = "linux" ] && [ "$ARCH_SUFFIX" = "x86_64" ]; then
        EXPECTED_SHA="$KNOWN_LINUX_X86_64_SHA"
    elif [ "$TARGET_OS" = "linux" ]; then
        EXPECTED_SHA="$KNOWN_LINUX_AARCH64_SHA"
    elif [ "$TARGET_OS" = "macos" ]; then
        EXPECTED_SHA="$KNOWN_MACOS_AARCH64_SHA"
    fi
fi

# 1. Idempotency Check: If already installed and checksum matches, skip re-downloading!
SKIP_DOWNLOAD=false
if [ -f "$VIC_DIR/bin/vic" ] && [ -n "$EXPECTED_SHA" ]; then
    CURRENT_SHA=""
    if command -v sha256sum >/dev/null 2>&1; then
        CURRENT_SHA=$(sha256sum "$VIC_DIR/bin/vic" | awk '{print $1}' | tr '[:upper:]' '[:lower:]')
    elif command -v shasum >/dev/null 2>&1; then
        CURRENT_SHA=$(shasum -a 256 "$VIC_DIR/bin/vic" | awk '{print $1}' | tr '[:upper:]' '[:lower:]')
    fi
    if [ "$CURRENT_SHA" = "$EXPECTED_SHA" ]; then
        if [ "$LANG_CODE" = "en" ]; then
            echo "[+] vic is already installed and verified (Matches official SHA256)."
        else
            echo "[+] vic đã được cài đặt và toàn vẹn (Khớp mã SHA256 chính thức)."
        fi
        SKIP_DOWNLOAD=true
    fi
fi

if [ "$SKIP_DOWNLOAD" = false ]; then
    if [ "$LANG_CODE" = "en" ]; then
        echo "[*] Trying pre-built binary ($BIN_NAME)..."
    else
        echo "[*] Đang thử tải bản build sẵn ($BIN_NAME)..."
    fi

    TEMP_VIC="$VIC_DIR/bin/vic.tmp.$$"
    rm -f "$TEMP_VIC"

    if curl -fsSL "$BIN_URL" -o "$TEMP_VIC" 2>/dev/null; then
        ACTUAL_SHA=""
        if command -v sha256sum >/dev/null 2>&1; then
            ACTUAL_SHA=$(sha256sum "$TEMP_VIC" | awk '{print $1}' | tr '[:upper:]' '[:lower:]')
        elif command -v shasum >/dev/null 2>&1; then
            ACTUAL_SHA=$(shasum -a 256 "$TEMP_VIC" | awk '{print $1}' | tr '[:upper:]' '[:lower:]')
        else
            if [ "$LANG_CODE" = "en" ]; then
                echo "[-] SECURITY ERROR: Neither sha256sum nor shasum tool found to verify checksum! Installation aborted."
            else
                echo "[-] LỖI BẢO MẬT: Không tìm thấy lệnh sha256sum hoặc shasum để đối soát mã băm! Hủy cài đặt."
            fi
            rm -f "$TEMP_VIC"
            exit 1
        fi

        if [ -z "$EXPECTED_SHA" ] || [ ${#EXPECTED_SHA} -ne 64 ] || [ "$EXPECTED_SHA" != "$ACTUAL_SHA" ]; then
            if [ "$LANG_CODE" = "en" ]; then
                echo "[-] SECURITY ERROR: SHA256 checksum mismatch! Expected: $EXPECTED_SHA, Got: $ACTUAL_SHA"
            else
                echo "[-] LỖI BẢO MẬT: Mã băm SHA256 không khớp! Mong đợi: $EXPECTED_SHA, Thực tế: $ACTUAL_SHA"
            fi
            rm -f "$TEMP_VIC"
            exit 1
        fi

        if [ "$LANG_CODE" = "en" ]; then
            echo "[+] SHA256 checksum verified."
        else
            echo "[+] Đã xác minh toàn vẹn SHA256 checksum."
        fi

        chmod +x "$TEMP_VIC"
        mv -f "$TEMP_VIC" "$VIC_DIR/bin/vic"
        curl -fsSL "https://vic-cli.phuongvy.xyz/module.md" -o "$VIC_DIR/module.md" 2>/dev/null || true
        if [ "$LANG_CODE" = "en" ]; then
            echo "[+] Pre-built binary downloaded and verified."
        else
            echo "[+] Đã tải và xác minh bản build sẵn thành công."
        fi
    else
        if [ "$LANG_CODE" = "en" ]; then
            echo "[!] Pre-built release not found, falling back to building from source..."
        else
            echo "[!] Không tìm thấy bản phát hành có sẵn, chuyển sang build từ mã nguồn..."
        fi

    if ! command -v cargo >/dev/null 2>&1; then
        if [ "$LANG_CODE" = "en" ]; then
            echo "[*] Installing Rust toolchain..."
        else
            echo "[*] Đang cài đặt Rust toolchain..."
        fi
        curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh -s -- -y
        source "$HOME/.cargo/env"
    fi

    TMP_DIR=$(mktemp -d)
    git clone --depth 1 "$REPO_URL" "$TMP_DIR/vic"
    cd "$TMP_DIR/vic"
    cargo build --release
    cp target/release/vic "$VIC_DIR/bin/vic"
    cp module.md "$VIC_DIR/module.md"
    chmod +x "$VIC_DIR/bin/vic"
    rm -rf "$TMP_DIR"
    fi
fi

if [ "$MODE" = "download" ]; then
    if [ "$LANG_CODE" = "en" ]; then
        echo "[+] Downloaded to $VIC_DIR/bin/vic (-d mode)."
    else
        echo "[+] Đã tải về $VIC_DIR/bin/vic (chế độ -d)."
    fi
    exit 0
fi

if [ -c /dev/urandom ] && command -v od >/dev/null 2>&1; then
    PIN=$(od -An -N4 -tu4 /dev/urandom | awk '{print ($1 % 900000) + 100000}')
elif [ -n "$RANDOM" ]; then
    PIN=$(( ((RANDOM * 32768 + RANDOM) % 900000) + 100000 ))
else
    PIN="683921"
fi
if [ ! -f "$VIC_DIR/config.toml" ]; then
    printf "lang = \"%s\"\npin = \"%s\"\n" "$LANG_CODE" "$PIN" > "$VIC_DIR/config.toml"
fi

for rc in "$HOME/.bashrc" "$HOME/.zshrc" "$HOME/.zprofile"; do
    if [ -f "$rc" ]; then
        if ! grep -q '\.vic/bin' "$rc"; then
            echo 'export PATH="$HOME/.vic/bin:$PATH"' >> "$rc"
        fi
    fi
done

if [ -t 0 ]; then
    "$VIC_DIR/bin/vic" setup
elif [ -e /dev/tty ]; then
    "$VIC_DIR/bin/vic" setup < /dev/tty
else
    "$VIC_DIR/bin/vic" setup
fi

if [ "$LANG_CODE" = "en" ]; then
    echo "[+] Setup complete! Run 'vic list' or 'vic ui'."
else
    echo "[+] Cài đặt hoàn tất! Hãy chạy 'vic list' hoặc 'vic ui'."
fi
