# Postwire PowerShell Installer for Windows
# Usage: iwr -useb https://raw.githubusercontent.com/yoosuf/postwire/main/install.ps1 | iex
$ErrorActionPreference = 'Stop'
[Net.ServicePointManager]::SecurityProtocol = [Net.SecurityProtocolType]::Tls12

$Repo = "yoosuf/postwire"
$DefaultInstallDir = Join-Path $ENV:USERPROFILE ".postwire\bin"
$InstallDir = if ($ENV:POSTWIRE_INSTALL_DIR) { $ENV:POSTWIRE_INSTALL_DIR } elseif ($ENV:POSTWIRE_INSTALL_DIR) { $ENV:POSTWIRE_INSTALL_DIR } else { $DefaultInstallDir }

if (-not (Test-Path -Path $InstallDir)) {
    New-Item -ItemType Directory -Force -Path $InstallDir | Out-Null
}

$Tag = if ($ENV:POSTWIRE_VERSION) { $ENV:POSTWIRE_VERSION } else { $ENV:POSTWIRE_VERSION }
if (-not $Tag) {
    try {
        $Release = Invoke-RestMethod -Uri "https://api.github.com/repos/$Repo/releases/latest" -Headers @{ "User-Agent" = "Postwire-Installer" }
        $Tag = $Release.tag_name
    } catch {
        $Tag = "v0.1.0"
    }
}

$Target = "x86_64-pc-windows-msvc"
$Url = "https://github.com/$Repo/releases/download/$Tag/postwire-$Tag-$Target.zip"
$ZipPath = Join-Path $ENV:TEMP "postwire-$Tag.zip"

Write-Host "Postwire Official Windows Installer" -ForegroundColor Cyan
Write-Host "Downloading Postwire $Tag for Windows ($Target)..." -ForegroundColor White

try {
    Invoke-WebRequest -Uri $Url -OutFile $ZipPath -UseBasicParsing
} catch {
    $Url = "https://github.com/$Repo/releases/download/$Tag/postwire-$Tag-$Target.zip"
    Write-Host "Postwire archive unavailable; trying legacy Pine Mail release..." -ForegroundColor Yellow
    Invoke-WebRequest -Uri $Url -OutFile $ZipPath -UseBasicParsing
}

Write-Host "Extracting binaries to $InstallDir..." -ForegroundColor White
Expand-Archive -Path $ZipPath -DestinationPath $InstallDir -Force
Remove-Item -Path $ZipPath -Force -ErrorAction SilentlyContinue

# Older tagged releases only contain the legacy command names.
foreach ($Alias in @(@("postwire.exe", "postwire.exe"), @("postwire-mcp.exe", "postwire-mcp.exe"))) {
    $LegacyPath = Join-Path $InstallDir $Alias[0]
    $PostwirePath = Join-Path $InstallDir $Alias[1]
    if ($Url -like "*postwire-*.zip" -and (Test-Path $LegacyPath)) {
        Copy-Item -Path $LegacyPath -Destination $PostwirePath -Force
    }
}

# Update User PATH environment variable
$UserPath = [Environment]::GetEnvironmentVariable("PATH", "User")
if ($UserPath -notlike "*$InstallDir*") {
    $NewPath = "$UserPath;$InstallDir"
    [Environment]::SetEnvironmentVariable("PATH", $NewPath, "User")
    $ENV:PATH = "$ENV:PATH;$InstallDir"
    Write-Host "Added $InstallDir to User PATH." -ForegroundColor Yellow
}

Write-Host "✅ Postwire $Tag successfully installed to $InstallDir" -ForegroundColor Green
Write-Host ""
Write-Host "To start Postwire:" -ForegroundColor White
Write-Host "  postwire          # SMTP on :1025, Web UI & REST API on :8025" -ForegroundColor Gray
Write-Host "  postwire-mcp      # MCP stdio server for AI agents" -ForegroundColor Gray
Write-Host "  postwire / postwire-mcp remain available as compatibility commands" -ForegroundColor Gray
