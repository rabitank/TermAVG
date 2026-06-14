param(
    [Parameter(Mandatory)]
    [string]$Target,

    [Parameter(Mandatory)]
    [string]$Version
)

$ErrorActionPreference = "Stop"

$Staging = "staging/$Target"

cargo build --locked --profile release-windows --target $Target -p tmj_terminal -p tmj_wgpu

New-Item -ItemType Directory -Force -Path $Staging | Out-Null
Copy-Item "target/$Target/release-windows/tmj_terminal.exe" "$Staging/"
Copy-Item "target/$Target/release-windows/tmj_wgpu.exe" "$Staging/"
Copy-Item README.md "$Staging/"
Copy-Item LICENSE "$Staging/"

New-Item -ItemType Directory -Force -Path target/artifacts | Out-Null

$zipDir = "target/zips"
New-Item -ItemType Directory -Force -Path "$zipDir/tmj", "$zipDir/tmj-wgpu" | Out-Null

Copy-Item "$Staging/tmj_terminal.exe" "$zipDir/tmj/"
Copy-Item "$Staging/README.md" "$zipDir/tmj/"
Copy-Item "$Staging/LICENSE" "$zipDir/tmj/"
$ArtifactName = "tmj-${Target}-v${Version}"
Compress-Archive -Path "$zipDir/tmj/*" -DestinationPath "target/artifacts/${ArtifactName}.zip" -Force
Write-Host "Built target/artifacts/${ArtifactName}.zip"

Copy-Item "$Staging/tmj_wgpu.exe" "$zipDir/tmj-wgpu/"
Copy-Item "$Staging/README.md" "$zipDir/tmj-wgpu/"
Copy-Item "$Staging/LICENSE" "$zipDir/tmj-wgpu/"
$ArtifactNameWgpu = "tmj-wgpu-${Target}-v${Version}"
Compress-Archive -Path "$zipDir/tmj-wgpu/*" -DestinationPath "target/artifacts/${ArtifactNameWgpu}.zip" -Force
Write-Host "Built target/artifacts/${ArtifactNameWgpu}.zip"

Remove-Item -Recurse -Force $zipDir
