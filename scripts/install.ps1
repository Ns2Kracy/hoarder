param(
    [string]$Repo = $env:HOARDER_REPO,
    [string]$Version = $env:HOARDER_VERSION,
    [string]$InstallDir = $env:HOARDER_INSTALL_DIR
)

$ErrorActionPreference = "Stop"

if ([string]::IsNullOrWhiteSpace($Repo)) {
    $Repo = "Ns2Kracy/hoarder"
}

if ([string]::IsNullOrWhiteSpace($Version)) {
    $Version = "latest"
}

if ([string]::IsNullOrWhiteSpace($InstallDir)) {
    $InstallDir = Join-Path $env:USERPROFILE ".local\bin"
}

$archive = "hoarder-windows-x86_64.zip"
if ($Version -eq "latest") {
    $url = "https://github.com/$Repo/releases/latest/download/$archive"
} else {
    $url = "https://github.com/$Repo/releases/download/$Version/$archive"
}

$tempRoot = Join-Path ([System.IO.Path]::GetTempPath()) ("hoarder-install-" + [System.Guid]::NewGuid())
$zipPath = Join-Path $tempRoot $archive

try {
    New-Item -ItemType Directory -Path $tempRoot, $InstallDir -Force | Out-Null
    Write-Host "Downloading $url"
    Invoke-WebRequest -Uri $url -OutFile $zipPath
    Expand-Archive -Path $zipPath -DestinationPath $tempRoot -Force
    Copy-Item -Path (Join-Path $tempRoot "hoarder.exe") -Destination (Join-Path $InstallDir "hoarder.exe") -Force
    Write-Host "Installed hoarder to $(Join-Path $InstallDir 'hoarder.exe')"

    $pathEntries = ($env:PATH -split ';') | Where-Object { $_ -ne '' }
    if ($pathEntries -notcontains $InstallDir) {
        Write-Host "Add $InstallDir to PATH to run hoarder from any shell."
    }
} finally {
    Remove-Item -Path $tempRoot -Recurse -Force -ErrorAction SilentlyContinue
}
