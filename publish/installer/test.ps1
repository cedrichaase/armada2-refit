# Run the package's install.ps1 against a mock game directory.   test.ps1 <zip>
#
# test.sh's twin, for Windows; CI runs it on a Windows runner. The game is three
# placeholder files, so this proves what the installer copies, backs up and puts
# back -- not that anything loads.
param([Parameter(Mandatory)][string]$Zip)
$ErrorActionPreference = "Stop"
$T = Join-Path ([IO.Path]::GetTempPath()) ([Guid]::NewGuid())
Expand-Archive -LiteralPath $Zip -DestinationPath $T
$P = (Get-ChildItem -LiteralPath $T -Directory -Filter "armada2-refit-*").FullName
$G = Join-Path $T "Star Trek Armada II"
New-Item -ItemType Directory $G | Out-Null
Set-Content (Join-Path $G "Armada2.exe") "exe"
Set-Content (Join-Path $G "binkw32.dll") "stockbink"
Set-Content (Join-Path $G "winmm.dll") "loader"
function G([string]$n) { Join-Path $G $n }
function Check([string]$what, $ok) {   # $ok untyped: a [bool] parameter refuses strings
    if ($ok) { Write-Host "ok    $what" } else { throw "FAIL  $what" }
}
function Install([string[]]$a) {
    & powershell -NoProfile -ExecutionPolicy Bypass -File (Join-Path $P "install.ps1") @a | Out-Host
    if ($LASTEXITCODE) { throw "install.ps1 $a exited $LASTEXITCODE" }
}

Write-Host "== install, no DXVK"
Install @($G)
Check "no MSAA.asi" (-not (Test-Path (G "MSAA.asi")))
Check "binkw32.dll is the proxy" ((Get-Content (G "binkw32.dll") -Raw) -match "BinkProxy")
Check "binkw32_orig.dll is stock" ((Get-Content (G "binkw32_orig.dll")) -eq "stockbink")
Check "dxvk.conf installed" ((Get-FileHash (G "dxvk.conf")).Hash -eq (Get-FileHash (Join-Path $P "game\dxvk.conf")).Hash)

Write-Host "== reinstall with DXVK, an edited .ini, and bloom"
Set-Content (G "d3d8.dll") "dxvk"
Add-Content (G "HUD.ini") "; mine"
Install @("-Bloom", $G)
Check "MSAA.asi" (Test-Path (G "MSAA.asi"))
Check "HUD.ini.bak kept" ((Get-Content (G "HUD.ini.bak") -Raw) -match "; mine")
Check "binkw32_orig.dll still stock" ((Get-Content (G "binkw32_orig.dll")) -eq "stockbink")
foreach ($f in "MagicBloom.fx", "ReShade.fxh", "ReShadeUI.fxh") {
    Check "shader $f" ((Get-Item (G "reshade-shaders\Shaders\$f")).Length -gt 0)
}
Check "A2Bloom.ini" (Test-Path (G "A2Bloom.ini"))

Write-Host "== unzipped into the game directory, run with no argument"
Copy-Item -Recurse $P (G "pkg")
$out = & powershell -NoProfile -ExecutionPolicy Bypass -File (G "pkg\install.ps1")
Check "found the game" ($out | Where-Object { $_ -like "installing into *Star Trek Armada II" })
Remove-Item -Recurse (G "pkg")

Write-Host "== uninstall"
Install @("-Uninstall", $G)
Check "binkw32.dll is stock again" ((Get-Content (G "binkw32.dll")) -eq "stockbink")
Check "no binkw32 copies" (-not (Test-Path (G "binkw32_orig.dll")) -and -not (Test-Path (G "binkw32.dll.a2neb-backup")))
foreach ($f in "HUD.asi", "Menus.asi", "MSAA.asi", "HUD.ini", "Menus.ini", "MSAA.ini", "BinkProxy.ini", "dxvk.conf", "A2Bloom.ini") {
    Check "$f removed" (-not (Test-Path (G $f)))
}

Write-Host "== a dxvk.conf it did not write is left alone"
Set-Content (G "dxvk.conf") "d3d9.foo = 1"
Install @($G)
Install @("-Uninstall", $G)
Check "foreign dxvk.conf kept" ((Get-Content (G "dxvk.conf")) -eq "d3d9.foo = 1")
Remove-Item -Recurse -Force $T
Write-Host "all passed"
