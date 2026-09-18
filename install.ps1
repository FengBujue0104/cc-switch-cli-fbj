#Requires -Version 5.1
$ErrorActionPreference = "Stop"

$Repo = if ($env:CC_SWITCH_REPO) { $env:CC_SWITCH_REPO } else { "FengBujue0104/cc-switch-cli-fbj" }
$InstallDir = if ($env:CC_SWITCH_INSTALL_DIR) { $env:CC_SWITCH_INSTALL_DIR } else { Join-Path $env:LOCALAPPDATA "cc-switch" }
$BinName = "cc-switch.exe"
$Target = Join-Path $InstallDir $BinName
$Version = if ($args.Count -ge 1 -and $args[0]) { $args[0] } else { "latest" }
if ($Version -ne "latest" -and $Version -notmatch "^v") { $Version = "v$Version" }

function Info($msg) { Write-Host "  info: $msg" -ForegroundColor Green }
function Warn($msg) { Write-Host "  warn: $msg" -ForegroundColor Yellow }
function Err($msg) { Write-Host "  error: $msg" -ForegroundColor Red }

$ReleasesUrl = "https://github.com/$Repo/releases"
$ApiBase = "https://api.github.com/repos/$Repo/releases"

try {
    if ($Version -eq "latest") {
        $release = Invoke-RestMethod -Uri "$ApiBase/latest" -Headers @{ "User-Agent" = "cc-switch-install" }
    } else {
        $release = Invoke-RestMethod -Uri "$ApiBase/tags/$Version" -Headers @{ "User-Agent" = "cc-switch-install" }
    }
} catch {
    Err "Failed to query GitHub releases for $Repo ($Version)."
    Err "Download manually: $ReleasesUrl"
    throw
}

$asset = $release.assets | Where-Object { $_.name -match "windows-x64\.zip$" } | Select-Object -First 1
if (-not $asset) {
    Err "No windows-x64.zip asset found on release $($release.tag_name)."
    throw "missing asset"
}

$tmp = Join-Path ([System.IO.Path]::GetTempPath()) ("cc-switch-" + [guid]::NewGuid().ToString("N"))
New-Item -ItemType Directory -Path $tmp | Out-Null
$zip = Join-Path $tmp "cc-switch.zip"
Info "Downloading $($asset.browser_download_url)"
Invoke-WebRequest -Uri $asset.browser_download_url -OutFile $zip -UseBasicParsing
Expand-Archive -Path $zip -DestinationPath $tmp -Force

$exe = Get-ChildItem -Path $tmp -Filter $BinName -Recurse | Select-Object -First 1
if (-not $exe) {
    Err "Archive did not contain $BinName"
    throw "missing binary"
}

New-Item -ItemType Directory -Path $InstallDir -Force | Out-Null
Copy-Item -Path $exe.FullName -Destination $Target -Force
Info "Installed $Target ($($release.tag_name))"

$userPath = [Environment]::GetEnvironmentVariable("Path", "User")
if (-not $userPath) { $userPath = "" }
$parts = $userPath -split ";" | Where-Object { $_ -and $_.Trim() -ne "" }
if ($parts -notcontains $InstallDir) {
    $newPath = ($parts + $InstallDir) -join ";"
    [Environment]::SetEnvironmentVariable("Path", $newPath, "User")
    $env:Path = "$InstallDir;$env:Path"
    Info "Added $InstallDir to the user PATH. Open a new PowerShell window to use cc-switch."
} else {
    Info "$InstallDir is already on PATH."
}

Remove-Item -Recurse -Force $tmp
Info "Done. Run: cc-switch --version"
