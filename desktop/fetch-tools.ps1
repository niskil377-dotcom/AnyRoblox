# Downloads the third-party tools that build.ps1 packages into cache/outils,
# from their official GitHub releases, and checks them against pinned SHA-256 hashes.
$ErrorActionPreference = 'Stop'
$ProgressPreference = 'SilentlyContinue'
[Net.ServicePointManager]::SecurityProtocol = [Net.SecurityProtocolType]::Tls12
$root = Split-Path $PSScriptRoot
$dir = New-Item -ItemType Directory -Force (Join-Path $root 'cache\outils')

function Test-Hash([string]$File, [string]$Hash) {
    (Test-Path $File) -and (Get-FileHash $File -Algorithm SHA256).Hash -eq $Hash
}
function Get-Pinned([string]$Url, [string]$File, [string]$Hash) {
    if (Test-Hash $File $Hash) { return }
    Invoke-WebRequest $Url -OutFile $File -UseBasicParsing
    if (!(Test-Hash $File $Hash)) {
        Remove-Item -Force $File
        throw "Checksum mismatch for $Url"
    }
}

# P3DDebin 1.2.1 (ODOL -> MLOD): https://github.com/Wesley-TB/P3DDebinarizer
Get-Pinned 'https://github.com/Wesley-TB/P3DDebinarizer/releases/download/v1.2.1/P3DDebin.exe' `
    (Join-Path $dir 'P3DDebin.exe') '3693CE7A073DDEEF8EEC15ED719F779D39A9A93C792813AE043685514F0211B4'

# HEMTT 1.23.1 (PAA -> PNG): https://github.com/BrettMayson/HEMTT
$hemtt = Join-Path $dir 'hemtt.exe'
$hemttHash = 'C372012B80A3D80D435C7F9A8DF0E003C3F6D1449E70D09E25130DA14DA96DBF'
if (!(Test-Hash $hemtt $hemttHash)) {
    $zip = Join-Path $dir 'hemtt-windows-x64.zip'
    Get-Pinned 'https://github.com/BrettMayson/HEMTT/releases/download/v1.23.1/windows-x64.zip' `
        $zip 'FE9ACB4D61D0ACC497C74BB63833DB0ADB77498F83692661AD2EC8A831773CEB'
    Add-Type -AssemblyName System.IO.Compression.FileSystem
    $archive = [IO.Compression.ZipFile]::OpenRead($zip)
    try {
        [IO.Compression.ZipFileExtensions]::ExtractToFile($archive.GetEntry('hemtt.exe'), $hemtt, $true)
    } finally { $archive.Dispose() }
    Remove-Item -Force $zip
    if (!(Test-Hash $hemtt $hemttHash)) { throw 'Checksum mismatch for hemtt.exe' }
}
Write-Host "Tools ready in $dir"
