param([string]$SigningThumbprint)
$ErrorActionPreference = 'Stop'
$root = Split-Path $PSScriptRoot
Push-Location $root
try {
    if (Test-Path 'cache\outils\rust-gnu') {
        $env:Path = "$(Resolve-Path 'cache\outils\rust-gnu');$env:Path"
    }
    $cargo = Get-Command cargo.exe -ErrorAction SilentlyContinue
    if ($cargo) { $cargoPath = $cargo.Source } else { $cargoPath = "$env:USERPROFILE\.cargo\bin\cargo.exe" }
    & $cargoPath build --release --locked
    if ($LASTEXITCODE) { throw 'Compilation Rust en échec.' }
    $framework = "$env:WINDIR\Microsoft.NET\Framework64\v4.0.30319"
    & "$PSScriptRoot\build-fivem.ps1" $framework
    foreach ($name in @('P3DDebin.exe','hemtt.exe')) {
        if (!(Test-Path "cache\outils\$name")) { throw "Placer $name dans cache/outils avant la compilation graphique." }
    }

    # The release is a folder: AnyRoblox.exe plus its helper programs as plain files in tools\.
    # Helpers are not embedded in the executable, so nothing is unpacked or written to disk at run time.
    $dist = 'dist\AnyRoblox'
    if (Test-Path $dist) { Remove-Item -Recurse -Force $dist }
    $tools = New-Item -ItemType Directory -Force "$dist\tools"
    Copy-Item 'target\release\arma2obj.exe','cache\outils\P3DDebin.exe','cache\outils\hemtt.exe' $tools
    foreach ($name in @('FiveMExport.exe','CodeWalker.Core.dll','SharpDX.dll','SharpDX.Mathematics.dll','CodeWalker-Notice.txt')) {
        Copy-Item "cache\outils\fivem\$name" $tools
    }
    Copy-Item 'LICENSE','README.md' $dist -ErrorAction SilentlyContinue

    $refs = @('PresentationCore','PresentationFramework','WindowsBase') | ForEach-Object { "/reference:$framework\WPF\$_.dll" }
    foreach ($name in @('System.Xaml','System.Windows.Forms','System.Web.Extensions')) { $refs += "/reference:$framework\$name.dll" }
    & "$framework\csc.exe" /nologo /codepage:65001 /target:winexe /platform:x64 /optimize+ "/out:$dist\AnyRoblox.exe" '/win32manifest:desktop\app.manifest' @refs '/resource:desktop\Main.xaml,Main.xaml' desktop\Studio.cs desktop\Viewer.cs desktop\ModCatalog.cs
    if ($LASTEXITCODE) { throw 'Compilation graphique en échec.' }

    if ($PSBoundParameters.ContainsKey('SigningThumbprint')) {
        if ($SigningThumbprint -notmatch '^[0-9a-fA-F]{40}$') {
            throw 'Provide the 40-character thumbprint of a code-signing certificate.'
        }
        $cert = Get-ChildItem Cert:\CurrentUser\My,Cert:\LocalMachine\My -CodeSigningCert |
            Where-Object { $_.Thumbprint -eq $SigningThumbprint -and $_.HasPrivateKey } |
            Select-Object -First 1
        if (!$cert) { throw 'Code-signing certificate with an accessible private key not found.' }
        # Sign the executables this project builds. Third-party tools keep their own publisher's signature.
        foreach ($file in @("$dist\AnyRoblox.exe","$dist\tools\arma2obj.exe","$dist\tools\FiveMExport.exe")) {
            Set-AuthenticodeSignature -FilePath $file -Certificate $cert -HashAlgorithm SHA256 -TimestampServer 'http://timestamp.digicert.com' | Out-Null
            $signature = Get-AuthenticodeSignature -FilePath $file
            if ($signature.Status -ne 'Valid' -or !$signature.TimeStamperCertificate -or $signature.SignerCertificate.Thumbprint -ne $SigningThumbprint) {
                throw "Signature or timestamp verification failed for $file ($($signature.Status))."
            }
        }
    }

    $zip = 'dist\AnyRoblox-win-x64.zip'
    if (Test-Path $zip) { Remove-Item -Force $zip }
    Compress-Archive -Path $dist -DestinationPath $zip
    Get-Item "$dist\AnyRoblox.exe",$zip | Select-Object FullName,Length
} finally { Pop-Location }
