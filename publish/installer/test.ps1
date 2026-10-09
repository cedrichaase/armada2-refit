# Run the package's install.ps1 against a mock game directory.   test.ps1 <zip>
#
# test.sh's twin, for Windows; CI runs it on a Windows runner. The game is a few
# placeholder files, so this proves what the installer copies, backs up and puts back --
# not that anything loads. Needs network, for the bloom shaders.
param([Parameter(Mandatory)][string]$Zip)
$ErrorActionPreference = "Stop"
$T = Join-Path ([IO.Path]::GetTempPath()) ([Guid]::NewGuid())
Expand-Archive -LiteralPath $Zip -DestinationPath $T
$P = (Get-ChildItem -LiteralPath $T -Directory -Filter "armada2-refit-*").FullName
# crosire's d3d8to9 as the repository vendors it: the installers know it by hash.
$D3d8to9 = Join-Path $PSScriptRoot "..\..\platform\vendor\d3d8to9-1.16.0\d3d8.dll"
function Check([string]$what, $ok) {   # $ok untyped: a [bool] parameter refuses strings
    if ($ok) { Write-Host "ok    $what" } else { throw "FAIL  $what" }
}
function Sha([string]$f) { (Get-FileHash -Algorithm SHA256 -LiteralPath $f).Hash.ToLower() }
function Text([string]$f) { (Get-Content -LiteralPath $f -Raw).Trim() }
function Install([string[]]$a) {
    $out = & powershell -NoProfile -ExecutionPolicy Bypass -File (Join-Path $P "install.ps1") @a
    if ($LASTEXITCODE) { $out | Out-Host; throw "install.ps1 $a exited $LASTEXITCODE" }
    return $out
}
function Mock([string]$d) {   # a GOG install: the exe, the stock Bink DLL, GOG's d3d8to9
    New-Item -ItemType Directory $d | Out-Null
    Set-Content (Join-Path $d "Armada2.exe") "exe"
    Set-Content (Join-Path $d "binkw32.dll") "stockbink"
    Set-Content (Join-Path $d "d3d8.dll") "gogd3d8to9"
}

$G = Join-Path $T "Star Trek Armada II"; Mock $G
function G([string]$n) { Join-Path $G $n }

Write-Host "== install on a GOG game: prerequisites, no DXVK, no ReShade"
$out = Install @($G)
Check "winmm.dll" ((Sha (G "winmm.dll")) -eq "baba99929487b005bb9b168acfd852550055f22e5f1059c9032765209bb185e5")
Check "STA2WidescreenPatch.asi" ((Sha (G "STA2WidescreenPatch.asi")) -eq "193828b15b8cdba84617dd9a359b555302132a35bbaa6fd3143fdb99442343c5")
Check "GOG's d3d8.dll untouched" (((Text (G "d3d8.dll")) -eq "gogd3d8to9") -and -not (Test-Path (G "d3d8.dll.gog-backup")))
Check "no MSAA.asi" (-not (Test-Path (G "MSAA.asi")))
Check "Lighting, per vertex here" ((Test-Path (G "Lighting.asi")) -and (Test-Path (G "Lighting.ini")) -and
                                  ($out | Where-Object { $_ -like "*Lighting: per vertex*" }))
Check "Sky, the stock sky here" ((Test-Path (G "Sky.asi")) -and (Test-Path (G "Sky\mbgaqu.ini")) -and
                                ($out | Where-Object { $_ -like "*Sky: the stock sky here*" }))
Check "no bloom" (-not (Test-Path (G "A2Bloom.ini")))
Check "binkw32.dll is the proxy" ((Get-Content (G "binkw32.dll") -Raw) -match "BinkProxy")
Check "binkw32_orig.dll is stock" ((Text (G "binkw32_orig.dll")) -eq "stockbink")
Check "dxvk.conf installed" ((Sha (G "dxvk.conf")) -eq (Sha (Join-Path $P "game\dxvk.conf")))

Write-Host "== DXVK takes the d3d8 slot; reinstall with ReShade and an edited .ini"
Set-Content (G "d3d8.dll") "dxvk"
Set-Content (G "ReShade.ini") "[GENERAL]"
Add-Content (G "HUD.ini") "; mine"
$out = Install @($G)
Check "prerequisites already installed" ($out | Where-Object { $_ -like "*STA2WidescreenPatch: already installed" })
Check "MSAA.asi" (Test-Path (G "MSAA.asi"))
Check "HUD.ini.bak kept" ((Get-Content (G "HUD.ini.bak") -Raw) -match "; mine")
Check "binkw32_orig.dll still stock" ((Text (G "binkw32_orig.dll")) -eq "stockbink")
foreach ($f in "MagicBloom.fx", "ReShade.fxh", "ReShadeUI.fxh") {
    Check "shader $f" ((Get-Item (G "reshade-shaders\Shaders\$f")).Length -gt 0)
}
Check "A2Bloom.ini" (Test-Path (G "A2Bloom.ini"))
Check "and as ReShadePreset.ini" ((Sha (G "ReShadePreset.ini")) -eq (Sha (G "A2Bloom.ini")))

Write-Host "== d3d8to9 in front of DXVK's d3d9: MSAA, and Lighting's shaders (no note)"
Copy-Item (G "d3d8.dll") (Join-Path $T "dxvk8")
Copy-Item $D3d8to9 (G "d3d8.dll")
Set-Content (G "d3d9.dll") "dxvk"
Remove-Item (G "MSAA.asi")
$out = Install @($G)
Check "MSAA.asi on the d3d8to9 chain" ((Test-Path (G "MSAA.asi")) -and -not ($out | Where-Object { $_ -like "*Lighting: per vertex*" }))
Set-Content (G "d3d9.dll") "gogd3d8to9"
$out = Install @($G)
Check "no MSAA.asi without DXVK's d3d9" ((-not (Test-Path (G "MSAA.asi"))) -and ($out | Where-Object { $_ -like "*MSAA.asi skipped*" }))
Copy-Item (Join-Path $T "dxvk8") (G "d3d8.dll")
Remove-Item (G "d3d9.dll")

Write-Host "== unzipped into the game directory, run with no argument"
Copy-Item -Recurse $P (G "pkg")
$out = & powershell -NoProfile -ExecutionPolicy Bypass -File (G "pkg\install.ps1")
Check "found the game" ($out | Where-Object { $_ -like "installing into *Star Trek Armada II" })
Remove-Item -Recurse (G "pkg")

Write-Host "== uninstall: what it added goes, what changed since stays"
Set-Content (G "UltimateASILoader-license.txt") "mine"
Set-Content (G "Sky\myown.ini") "[Sky]"   # a recipe of the player's own stays
$out = Install @("-Uninstall", $G)
Check "the player's own recipe left" ((@(Get-ChildItem (G "Sky")).Name -join ",") -eq "myown.ini")
Check "binkw32.dll is stock again" ((Text (G "binkw32.dll")) -eq "stockbink")
Check "no binkw32 copies" (-not (Test-Path (G "binkw32_orig.dll")) -and -not (Test-Path (G "binkw32.dll.a2neb-backup")))
foreach ($f in "HUD.asi", "Menus.asi", "MSAA.asi", "QOL.asi", "Lighting.asi", "Sky.asi", "Online.asi", "HUD.ini", "Menus.ini", "MSAA.ini", "QOL.ini", "Lighting.ini", "Sky.ini", "Online.ini", "BinkProxy.ini", "dxvk.conf",
               "A2Bloom.ini", "ReShadePreset.ini", "winmm.dll", "STA2WidescreenPatch.asi", "armada2-refit-prereqs.txt") {
    Check "$f removed" (-not (Test-Path (G $f)))
}
Check "changed file left" (((Text (G "UltimateASILoader-license.txt")) -eq "mine") -and ($out | Where-Object { $_ -like "*left UltimateASILoader-license.txt*" }))
Check "DXVK's d3d8.dll left" ((Text (G "d3d8.dll")) -eq "dxvk")

Write-Host "== what it did not write is left alone: a loader, a dxvk.conf, a ReShade preset"
$H = Join-Path $T "other"; Mock $H
function O([string]$n) { Join-Path $H $n }
Set-Content (O "winmm.dll") "myloader"
Set-Content (O "dxvk.conf") "d3d9.foo = 1"
Set-Content (O "ReShade.ini") "[GENERAL]"
Set-Content (O "ReShadePreset.ini") "Techniques=Mine"
Install @($H) | Out-Null
Check "own loader kept, patch added" (((Text (O "winmm.dll")) -eq "myloader") -and (Test-Path (O "STA2WidescreenPatch.asi")))
Check "own preset kept on install" ((Text (O "ReShadePreset.ini")) -eq "Techniques=Mine")
Check "A2Bloom.ini beside it" (Test-Path (O "A2Bloom.ini"))
Install @("-Uninstall", $H) | Out-Null
Check "own loader kept, patch removed" (((Text (O "winmm.dll")) -eq "myloader") -and -not (Test-Path (O "STA2WidescreenPatch.asi")))
Check "foreign dxvk.conf kept" ((Text (O "dxvk.conf")) -eq "d3d9.foo = 1")
Check "own preset kept on uninstall" ((Text (O "ReShadePreset.ini")) -eq "Techniques=Mine")
Check "GOG's d3d8.dll back" ((Text (O "d3d8.dll")) -eq "gogd3d8to9")
Remove-Item -Recurse -Force $T
Write-Host "all passed"
