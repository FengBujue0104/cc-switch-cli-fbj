#Requires -Version 5.1
$ErrorActionPreference = "Stop"

$Repo = if ($env:CC_SWITCH_REPO) { $env:CC_SWITCH_REPO } else { "FengBujue0104/cc-switch-cli-fbj" }
$InstallDir = if ($env:CC_SWITCH_INSTALL_DIR) { $env:CC_SWITCH_INSTALL_DIR } else { Join-Path $env:LOCALAPPDATA "cc-switch" }
$BinName = "cc-switch.exe"
$Target = Join-Path $InstallDir $BinName
$Force = if ($env:CC_SWITCH_FORCE) { $env:CC_SWITCH_FORCE } else { "0" }
$Version = if ($args.Count -ge 1 -and $args[0]) { $args[0] } else { "latest" }
if ($Version -ne "latest" -and $Version -notmatch "^v") { $Version = "v$Version" }

function Info($msg) { Write-Host "  info: $msg" -ForegroundColor Green }
function Warn($msg) { Write-Host "  warn: $msg" -ForegroundColor Yellow }
function Err($msg) { Write-Host "  error: $msg" -ForegroundColor Red }

$ReleasesUrl = "https://github.com/$Repo/releases"
$ApiBase = "https://api.github.com/repos/$Repo/releases"
$Headers = @{ "User-Agent" = "cc-switch-install" }

if ((Test-Path -LiteralPath $Target) -and $Force -ne "1") {
    $reply = Read-Host "  Existing install: $Target. [U]pdate or [C]ancel? [U/c]"
    switch -Regex ($reply) {
        "^(c|C|cancel|CANCEL)$" { Info "Canceled."; exit 0 }
        "^(u|U|update|UPDATE)?$" { }
        default { Err "Unrecognized choice."; exit 1 }
    }
}

try {
    if ($Version -eq "latest") {
        $release = Invoke-RestMethod -Uri "$ApiBase/latest" -Headers $Headers
    } else {
        $release = Invoke-RestMethod -Uri "$ApiBase/tags/$Version" -Headers $Headers
    }
} catch {
    Err "Failed to query GitHub releases for $Repo ($Version)."
    Err "Download manually: $ReleasesUrl"
    throw
}

$tag = [string]$release.tag_name
if ([string]::IsNullOrWhiteSpace($tag) -or $tag.Contains("/") -or $tag.Contains("..")) {
    Err "Unsafe tag_name from GitHub API."
    exit 1
}

$assetName = "cc-switch-cli-$tag-windows-x64.zip"
$asset = $release.assets | Where-Object { $_.name -eq $assetName } | Select-Object -First 1
if (-not $asset) {
    Err "No $assetName on release $tag."
    throw "missing asset"
}
$checksumAsset = $release.assets | Where-Object { $_.name -eq "checksums.txt" } | Select-Object -First 1
if (-not $checksumAsset) {
    Err "Release $tag has no checksums.txt."
    throw "missing checksums"
}

$tmp = Join-Path ([System.IO.Path]::GetTempPath()) ("cc-switch-" + [guid]::NewGuid().ToString("N"))
New-Item -ItemType Directory -Path $tmp | Out-Null
try {
    $zip = Join-Path $tmp $assetName
    $checksums = Join-Path $tmp "checksums.txt"
    Info "Downloading $($asset.browser_download_url)"
    Invoke-WebRequest -Uri $asset.browser_download_url -OutFile $zip -UseBasicParsing
    Info "Downloading $($checksumAsset.browser_download_url)"
    Invoke-WebRequest -Uri $checksumAsset.browser_download_url -OutFile $checksums -UseBasicParsing

    $expected = $null
    Get-Content -LiteralPath $checksums | ForEach-Object {
        $parts = $_ -split "\s+", 2
        if ($parts.Count -ge 2) {
            $name = $parts[1].TrimStart("*")
            if ($name -eq $assetName) { $expected = $parts[0].ToLowerInvariant() }
        }
    }
    if (-not $expected) {
        Err "checksums.txt does not list $assetName."
        throw "checksum missing"
    }
    $actual = (Get-FileHash -LiteralPath $zip -Algorithm SHA256).Hash.ToLowerInvariant()
    if ($actual -ne $expected) {
        Err "Checksum mismatch for $assetName."
        Err "expected $expected"
        Err "got      $actual"
        throw "checksum mismatch"
    }
    Info "Checksum OK"

    Expand-Archive -Path $zip -DestinationPath $tmp -Force
    $exe = Join-Path $tmp $BinName
    if (-not (Test-Path -LiteralPath $exe)) {
        Err "Archive did not contain $BinName at the top level."
        throw "missing binary"
    }

    New-Item -ItemType Directory -Path $InstallDir -Force | Out-Null
    $staging = "$Target.new"
    $previous = "$Target.old"
    Copy-Item -LiteralPath $exe -Destination $staging -Force
    if (Test-Path -LiteralPath $previous) {
        Remove-Item -LiteralPath $previous -Force -ErrorAction SilentlyContinue
    }
    if (Test-Path -LiteralPath $Target) {
        # A running cc-switch.exe cannot be overwritten in place on Windows.
        # Rename it out of the way first; the old file can stay locked until the process exits.
        Rename-Item -LiteralPath $Target -NewName ([System.IO.Path]::GetFileName($previous))
    }
    Move-Item -LiteralPath $staging -Destination $Target -Force
    if (Test-Path -LiteralPath $previous) {
        Remove-Item -LiteralPath $previous -Force -ErrorAction SilentlyContinue
        if (Test-Path -LiteralPath $previous) {
            Warn "Left $previous in place because the previous binary is still running. Delete it after cc-switch exits."
        }
    }
    Info "Installed $Target ($tag)"

    $userPath = [Environment]::GetEnvironmentVariable("Path", "User")
    if (-not $userPath) { $userPath = "" }
    $parts = @($userPath -split ";" | Where-Object { $_ -and $_.Trim() -ne "" })
    $already = $parts | Where-Object { $_.TrimEnd("\") -ieq $InstallDir.TrimEnd("\") }
    if (-not $already) {
        $newPath = ($parts + $InstallDir) -join ";"
        [Environment]::SetEnvironmentVariable("Path", $newPath, "User")
        $env:Path = "$InstallDir;$env:Path"
        Info "Added $InstallDir to the user PATH. Open a new PowerShell window to use cc-switch."
    } else {
        Info "$InstallDir is already on PATH."
    }
    Info "Done. Run: cc-switch --version"
} finally {
    if (Test-Path -LiteralPath $tmp) {
        Remove-Item -Recurse -Force $tmp
    }
}
