# Called from the project root by build.ps1.
param([string]$Framework)
$ErrorActionPreference='Stop'
Add-Type -AssemblyName System.IO.Compression.FileSystem
$dir=New-Item -ItemType Directory -Force cache/outils/fivem
$packages=@(
    ,@('codewalker.core','1.0.3','3DE63D8E6E0589E28B1C5D6CE269DDD02A92103C87D0EF3ECDB86C3BE8414660','lib/netstandard2.0/CodeWalker.Core.dll')
    ,@('sharpdx','4.2.0','117126040200FAAA118F1312AD5A5B1E7D067C90ABF6E726F18B6D1635216FC3','lib/net45/SharpDX.dll')
    ,@('sharpdx.mathematics','4.2.0','CB107F2B73C7982A31A7F9DD2E74C1C262F7B79E8B40344053A22F2AD8F1D594','lib/net45/SharpDX.Mathematics.dll')
)
$refs=@()
foreach($pkg in $packages){
    $id,$version,$hash,$entry=$pkg
    $file=Join-Path $dir "$id.nupkg"
    if(!(Test-Path $file) -or (Get-FileHash $file).Hash -ne $hash){
        Invoke-WebRequest "https://api.nuget.org/v3-flatcontainer/$id/$version/$id.$version.nupkg" -OutFile $file
    }
    if((Get-FileHash $file).Hash -ne $hash){throw "Paquet invalide : $id"}
    $archive=[IO.Compression.ZipFile]::OpenRead($file)
    try{
        $dll=Join-Path $dir ([IO.Path]::GetFileName($entry))
        [IO.Compression.ZipFileExtensions]::ExtractToFile($archive.GetEntry($entry),$dll,$true)
        $refs+="/reference:$dll"
        if($id -eq 'codewalker.core'){
            [IO.Compression.ZipFileExtensions]::ExtractToFile($archive.GetEntry('Notice.txt'),"$dir\CodeWalker-Notice.txt",$true)
        }
    }finally{$archive.Dispose()}
}
& "$Framework\csc.exe" /nologo /codepage:65001 /target:exe /platform:x64 /optimize+ "/out:$dir\FiveMExport.exe" '/win32manifest:desktop\app.manifest' @refs "/reference:$Framework\netstandard.dll" "/reference:$Framework\System.Xaml.dll" "/reference:$Framework\WPF\WindowsBase.dll" "/reference:$Framework\WPF\PresentationCore.dll" "/reference:$Framework\System.Web.Extensions.dll" desktop\FiveM.cs
if($LASTEXITCODE){throw 'Compilation du convertisseur FiveM en échec.'}


