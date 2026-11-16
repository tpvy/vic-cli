param(
    [switch]$d,
    [switch]$V,
    [switch]$E
)

# Detect language and options
$isVi = ($E.IsPresent -ne $true -and $args -notcontains "-E" -and $env:VIC_LANG -ne "en")
$isDownloadOnly = ($d.IsPresent -or $args -contains "-d" -or $env:VIC_DOWNLOAD_ONLY -eq "1")
$LangCode = if ($isVi) { "vi" } else { "en" }

# Safe TLS configuration for Windows PowerShell (.NET Framework 4.5+)
try {
    [System.Net.ServicePointManager]::SecurityProtocol = [System.Net.ServicePointManager]::SecurityProtocol -bor [System.Net.SecurityProtocolType]::Tls12
} catch {}
try {
    [System.Net.ServicePointManager]::CheckCertificateRevocationList = $false
} catch {}

# Validate 64-bit OS
if (-not [Environment]::Is64BitOperatingSystem) {
    if ($isVi) {
        Write-Host "[-] LOI: vic-cli chi ho tro Windows 64-bit (x86_64)." -ForegroundColor Red
    } else {
        Write-Host "[-] ERROR: vic-cli currently only supports 64-bit Windows (x86_64)." -ForegroundColor Red
    }
    $global:LASTEXITCODE = 1
    return
}

# Resolve target directories
$UserDir = if ($env:USERPROFILE) { $env:USERPROFILE } else { $env:HOME }
$VicDir = "$UserDir\.vic"
$BinDir = "$VicDir\bin"
$ExePath = "$BinDir\vic.exe"

# Create directories
foreach ($sub in @("bin", "modules", "archive", "share")) {
    $dir = "$VicDir\$sub"
    if (-not (Test-Path $dir)) {
        New-Item -ItemType Directory -Force -Path $dir | Out-Null
    }
}

# Primary and Fallback URLs
$PrimaryBinaryUrl = "https://github.com/tpvy/vic-cli/releases/latest/download/vic-windows-x86_64.exe"
$TagBinaryUrl     = "https://github.com/tpvy/vic-cli/releases/download/v1.0.0/vic-windows-x86_64.exe"
$PrimaryShaUrl    = "https://github.com/tpvy/vic-cli/releases/latest/download/vic-windows-x86_64.exe.sha256"
$TagShaUrl        = "https://github.com/tpvy/vic-cli/releases/download/v1.0.0/vic-windows-x86_64.exe.sha256"

# Helper function to download binary file with multiple strategies
function Download-FileSafe($targetUrl, $destPath) {
    # Strategy 1: curl.exe (built-in on Windows 10 & 11)
    if (Get-Command curl.exe -ErrorAction SilentlyContinue) {
        try {
            & curl.exe -fsSL --ssl-revoke-best-effort --connect-timeout 15 -o "$destPath" "$targetUrl" 2>$null
            if ((Test-Path "$destPath") -and (Get-Item "$destPath").Length -gt 1000) {
                return $true
            }
        } catch {}
    }

    # Strategy 2: System.Net.WebClient
    try {
        $wc = New-Object System.Net.WebClient
        $wc.Headers.Add("User-Agent", "Mozilla/5.0 (Windows NT 10.0; Win64; x64) vic-installer")
        $wc.DownloadFile($targetUrl, $destPath)
        if ((Test-Path "$destPath") -and (Get-Item "$destPath").Length -gt 1000) {
            return $true
        }
    } catch {}

    # Strategy 3: Invoke-WebRequest with BasicParsing
    try {
        $oldPref = $ProgressPreference
        $ProgressPreference = 'SilentlyContinue'
        Invoke-WebRequest -Uri $targetUrl -OutFile $destPath -UseBasicParsing -ErrorAction Stop
        $ProgressPreference = $oldPref
        if ((Test-Path "$destPath") -and (Get-Item "$destPath").Length -gt 1000) {
            return $true
        }
    } catch {}

    return $false
}

# Helper function to download string content
function Download-StringSafe($targetUrl) {
    if (Get-Command curl.exe -ErrorAction SilentlyContinue) {
        try {
            $txt = (& curl.exe -fsSL --ssl-revoke-best-effort --connect-timeout 10 "$targetUrl" 2>$null)
            if ($txt -and $txt.Trim().Length -gt 0) {
                return $txt.Trim()
            }
        } catch {}
    }

    try {
        $wc = New-Object System.Net.WebClient
        $wc.Headers.Add("User-Agent", "Mozilla/5.0 (Windows NT 10.0; Win64; x64) vic-installer")
        $txt = $wc.DownloadString($targetUrl)
        if ($txt -and $txt.Trim().Length -gt 0) {
            return $txt.Trim()
        }
    } catch {}

    try {
        $oldPref = $ProgressPreference
        $ProgressPreference = 'SilentlyContinue'
        $resp = Invoke-WebRequest -Uri $targetUrl -UseBasicParsing -ErrorAction Stop
        $ProgressPreference = $oldPref
        if ($resp -and $resp.Content) {
            return $resp.Content.Trim()
        }
    } catch {}

    return $null
}

$KnownReleaseSha = "28b81ddb635b65db34a7e60753bb1084dfc1be2c10decba3a1ff8548d3345642"

# Fetch expected SHA256 checksum (with cache-busting timestamp to prevent stale cache on repeated runs)
$nowTs = [DateTimeOffset]::UtcNow.ToUnixTimeSeconds()
$shaContent = Download-StringSafe "${PrimaryShaUrl}?t=${nowTs}"
if (-not $shaContent) {
    $shaContent = Download-StringSafe "${TagShaUrl}?t=${nowTs}"
}

$expectedSha = ""
if ($shaContent -and ($shaContent -match '([a-fA-F0-9]{64})')) {
    $expectedSha = $Matches[1].ToLower()
} else {
    $expectedSha = $KnownReleaseSha
}

# 1. Idempotency Check: If vic.exe is already installed and matches the official checksum, skip re-downloading!
$skipDownload = $false
if ((Test-Path $ExePath) -and (Get-Item $ExePath).Length -gt 1000000) {
    $currentSha = (Get-FileHash -Path $ExePath -Algorithm SHA256).Hash.ToLower()
    if ($currentSha -eq $expectedSha) {
        if ($isVi) {
            Write-Host "[+] vic da duoc cai dat va toan ven (Khop ma SHA256 chinh thuc)." -ForegroundColor Green
        } else {
            Write-Host "[+] vic is already installed and verified (Matches official SHA256)." -ForegroundColor Green
        }
        $skipDownload = $true
    }
}

if (-not $skipDownload) {
    if ($isVi) {
        Write-Host "[*] Dang tai vic.exe cho Windows x86_64..." -ForegroundColor Cyan
    } else {
        Write-Host "[*] Downloading vic.exe for Windows x86_64..." -ForegroundColor Cyan
    }

    # 2. Download to a unique temporary file first (prevents corrupting existing binary or failing on locked files)
    $TempExe = "$BinDir\vic.exe.tmp.$PID"
    Remove-Item $TempExe -Force -ErrorAction SilentlyContinue

    $downloadOk = Download-FileSafe $PrimaryBinaryUrl $TempExe
    if (-not $downloadOk) {
        $downloadOk = Download-FileSafe $TagBinaryUrl $TempExe
    }

    if (-not $downloadOk -or -not (Test-Path $TempExe) -or (Get-Item $TempExe).Length -lt 1000000) {
        if ($isVi) {
            Write-Host "[-] LOI: Khong the tai ban cai dat vic.exe tu GitHub Release." -ForegroundColor Red
            Write-Host "    Vui long kiem tra ket noi Internet hoac thu lai sau." -ForegroundColor Red
        } else {
            Write-Host "[-] ERROR: Failed to download vic.exe from GitHub Release." -ForegroundColor Red
            Write-Host "    Please check your Internet connection and try again." -ForegroundColor Red
        }
        Remove-Item $TempExe -Force -ErrorAction SilentlyContinue
        $global:LASTEXITCODE = 1
        return
    }

    # 3. Verify SHA256 checksum on the TEMPORARY file
    $actualSha = (Get-FileHash -Path $TempExe -Algorithm SHA256).Hash.ToLower()

    if ($expectedSha.Length -ne 64 -or $expectedSha -ne $actualSha) {
        if ($isVi) {
            Write-Host "[-] LOI BAO MAT: Ma SHA256 khong khop voi ban phat hanh chinh thuc!" -ForegroundColor Red
            Write-Host "    Mong doi: $expectedSha" -ForegroundColor Red
            Write-Host "    Thuc te : $actualSha" -ForegroundColor Red
        } else {
            Write-Host "[-] SECURITY ERROR: SHA256 checksum mismatch for vic.exe!" -ForegroundColor Red
            Write-Host "    Expected: $expectedSha" -ForegroundColor Red
            Write-Host "    Actual  : $actualSha" -ForegroundColor Red
        }
        # Only remove the temporary file, existing working binary is unharmed!
        Remove-Item $TempExe -Force -ErrorAction SilentlyContinue
        $global:LASTEXITCODE = 1
        return
    }

    if ($isVi) {
        Write-Host "[+] Da xac minh toan ven SHA256 checksum thanh cong." -ForegroundColor Green
    } else {
        Write-Host "[+] SHA256 checksum verified successfully." -ForegroundColor Green
    }

    # 4. Safe file swap: Stop running instances if any, then move
    Get-Process -Name vic -ErrorAction SilentlyContinue | Stop-Process -Force -ErrorAction SilentlyContinue
    Start-Sleep -Milliseconds 100

    $OldExe = "$BinDir\vic.exe.old"
    Remove-Item $OldExe -Force -ErrorAction SilentlyContinue

    if (Test-Path $ExePath) {
        try {
            Move-Item -Path $ExePath -Destination $OldExe -Force -ErrorAction Stop
        } catch {
            Remove-Item $ExePath -Force -ErrorAction SilentlyContinue
        }
    }

    Move-Item -Path $TempExe -Destination $ExePath -Force
    Remove-Item $OldExe -Force -ErrorAction SilentlyContinue
}

# Unblock file to remove Mark of the Web
Unblock-File -Path $ExePath -ErrorAction SilentlyContinue

# Download module.md
Download-FileSafe "https://vic-cli.phuongvy.xyz/module.md" "$VicDir\module.md" | Out-Null

if ($isDownloadOnly) {
    if ($isVi) {
        Write-Host "[+] Da tai ve $ExePath (che do -d)." -ForegroundColor Green
    } else {
        Write-Host "[+] Downloaded to $ExePath (-d mode)." -ForegroundColor Green
    }
    $global:LASTEXITCODE = 0
    return
}

# Generate 6-digit random PIN
$Pin = (Get-Random -Minimum 100000 -Maximum 1000000).ToString()
$CfgFile = "$VicDir\config.toml"
if (-not (Test-Path $CfgFile)) {
    $cfgContent = "lang = `"$LangCode`"`r`npin = `"$Pin`"`r`n"
    [System.IO.File]::WriteAllText($CfgFile, $cfgContent, [System.Text.Encoding]::UTF8)
}

# Update PATH in current PowerShell session
if ($env:Path -notlike "*$BinDir*") {
    $env:Path = "$BinDir;$env:Path"
}

# Update User PATH in Registry for future sessions
try {
    $UserPath = [Environment]::GetEnvironmentVariable("Path", "User")
    if ($UserPath -notlike "*$BinDir*") {
        $newUserPath = if ([string]::IsNullOrEmpty($UserPath)) { $BinDir } else { "$UserPath;$BinDir" }
        [Environment]::SetEnvironmentVariable("Path", $newUserPath, "User")
    }
} catch {}

if ($isVi) {
    Write-Host "[+] Cai dat hoan tat! Ban co the su dung ngay 'vic' trong cua so nay." -ForegroundColor Green
    Write-Host " -> Kiem tra phien ban : vic version" -ForegroundColor Yellow
    Write-Host " -> Danh muc lenh       : vic list" -ForegroundColor Yellow
} else {
    Write-Host "[+] Installation complete! You can run 'vic' right away in this session." -ForegroundColor Green
    Write-Host " -> Check version : vic version" -ForegroundColor Yellow
    Write-Host " -> Command list  : vic list" -ForegroundColor Yellow
}
$global:LASTEXITCODE = 0
