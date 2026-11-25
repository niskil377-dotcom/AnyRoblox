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
    $refs = @('PresentationCore','PresentationFramework','WindowsBase') | ForEach-Object { "/reference:$framework\WPF\$_.dll" }
    foreach ($name in @('System.Xaml','System.Windows.Forms','System.Web.Extensions')) { $refs += "/reference:$framework\$name.dll" }
    $resources = @('/resource:desktop\Main.xaml,Main.xaml','/resource:target\release\arma2obj.exe,arma2obj.exe')
    foreach($name in @('FiveMExport.exe','CodeWalker.Core.dll','SharpDX.dll','SharpDX.Mathematics.dll','CodeWalker-Notice.txt')){
        $resources += "/resource:cache\outils\fivem\$name,$name"
    }
    foreach ($name in @('P3DDebin.exe','hemtt.exe')) {
        if (!(Test-Path "cache\outils\$name")) { throw "Placer $name dans cache/outils avant la compilation graphique." }
        $resources += "/resource:cache\outils\$name,$name"
    }
    & "$framework\csc.exe" /nologo /codepage:65001 /target:winexe /platform:x64 /optimize+ '/out:AnyRoblox.exe' @refs @resources desktop\Studio.cs desktop\Viewer.cs desktop\ModCatalog.cs
    if ($LASTEXITCODE) { throw 'Compilation graphique en échec.' }
    Get-Item 'AnyRoblox.exe' | Select-Object FullName,Length
} finally { Pop-Location }


