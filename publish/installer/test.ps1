# Run the package's install.ps1 against a mock game directory.   test.ps1 <zip>
#
# test.sh's twin, for Windows; CI runs it on a Windows runner. The game is a few
# placeholder files, so this proves what the installer copies, backs up and puts back --
# not that anything loads. Needs network, for the bloom shaders. The model scripts are
# never run for real: A2_PYTHON points at a stand-in that logs how it was called.
param([Parameter(Mandatory)][string]$Zip)
$ErrorActionPreference = "Stop"
$T = Join-Path ([IO.Path]::GetTempPath()) ([Guid]::NewGuid())
Expand-Archive -LiteralPath $Zip -DestinationPath $T
$P = (Get-ChildItem -LiteralPath $T -Directory -Filter "armada2-refit-*").FullName
# crosire's d3d8to9 as the repository vendors it: the installers know it by hash.
$D3d8to9 = Join-Path $PSScriptRoot "..\..\platform\vendor\d3d8to9-1.16.0\d3d8.dll"
$D3d8to9Sha = "122928cfe225c25d30decf7184a5d37e490cecf3b58256ba3206c7e1853f8ab8"
# install.sh's ::step ids, in order (manifest.json's "steps").
$Steps = "verify prereqs chain hud menus qol grid models lighting sky online msaa cutscenes renderer bloom done"
function Check([string]$what, $ok) {   # $ok untyped: a [bool] parameter refuses strings
    if ($ok) { Write-Host "ok    $what" } else { throw "FAIL  $what" }
}
function Sha([string]$f) { (Get-FileHash -Algorithm SHA256 -LiteralPath $f).Hash.ToLower() }
function Text([string]$f) { (Get-Content -LiteralPath $f -Raw).Trim() }
function Has($out, [string]$pattern) { [bool]($out | Where-Object { $_ -like $pattern }) }
function Install([string[]]$a) {
    $out = & powershell -NoProfile -ExecutionPolicy Bypass -File (Join-Path $P "install.ps1") @a
    if ($LASTEXITCODE) { $out | Out-Host; throw "install.ps1 $a exited $LASTEXITCODE" }
    return $out
}
# A run expected to fail: stdout and stderr together, as the GUI reads them, and the code.
function Fail([string]$ps1, [string[]]$a) {
    $ErrorActionPreference = "Continue"   # the child's stderr arrives as error records
    $out = @(& powershell -NoProfile -ExecutionPolicy Bypass -File $ps1 @a 2>&1 | ForEach-Object { "$_" })
    return @{ Code = $LASTEXITCODE; Last = @($out | Where-Object { $_.Trim() })[-1]; Out = $out }
}
function Mock([string]$d, [switch]$NoD3d8) {   # a GOG install: the exe, the stock Bink DLL, a d3d8.dll
    New-Item -ItemType Directory $d | Out-Null
    Set-Content (Join-Path $d "Armada2.exe") "exe"
    Set-Content (Join-Path $d "binkw32.dll") "stockbink"
    # Not GOG's real d3d8to9 (that is the game's), so the installer must leave it be.
    if (-not $NoD3d8) { Set-Content (Join-Path $d "d3d8.dll") "gogd3d8to9" }
}
# Every file and folder under a directory, with each file's hash.
function Snap([string]$d) {
    (Get-ChildItem -LiteralPath $d -Recurse -Force | ForEach-Object {
        $rel = $_.FullName.Substring($d.Length)
        if ($_.PSIsContainer) { "$rel\" } else { "$rel $(Sha $_.FullName)" }
    } | Sort-Object) -join "`n"
}

# The model scripts' stand-ins: one that logs and succeeds, one that fails loudly.
$PyLog = Join-Path $T "python.log"
$FakePy = Join-Path $T "python.cmd"
Set-Content -LiteralPath $FakePy -Value @('@echo off', 'echo %*>>"%~dp0python.log"',
                                          'echo game %A2_GAME%>>"%~dp0python.log"', 'exit /b 0')
$BadPy = Join-Path $T "badpython.cmd"
Set-Content -LiteralPath $BadPy -Value @('@echo off', 'echo boom 1>&2', 'exit /b 1')
$env:A2_PYTHON = $FakePy

$G = Join-Path $T "Star Trek Armada II"; Mock $G
function G([string]$n) { Join-Path $G $n }

Write-Host "== install on a GOG game: prerequisites, no DXVK, no ReShade, with progress"
$env:A2_PROGRESS = "1"
$out = Install @($G)
$env:A2_PROGRESS = $null
Check "::step ids in install.sh's order" ((@($out | Where-Object { $_ -like "::step *" } |
                                           ForEach-Object { ($_ -split " ")[1] }) -join " ") -eq $Steps)
$at = @{}; for ($i = 0; $i -lt $out.Count; $i++) { if (-not $at.ContainsKey($out[$i])) { $at[$out[$i]] = $i } }
Check "::step lines in order with the rest" (($at["::step hud HUD"] -lt $at["  HUD.asi"]) -and
                                             ($at["  HUD.asi"] -lt $at["::step menus menus"]) -and
                                             ($at["::step done done"] -gt $at["  dxvk.conf"]))
Check "winmm.dll" ((Sha (G "winmm.dll")) -eq "baba99929487b005bb9b168acfd852550055f22e5f1059c9032765209bb185e5")
Check "STA2WidescreenPatch.asi" ((Sha (G "STA2WidescreenPatch.asi")) -eq "193828b15b8cdba84617dd9a359b555302132a35bbaa6fd3143fdb99442343c5")
Check "a d3d8.dll that is neither GOG's nor d3d8to9 left as it is" (((Text (G "d3d8.dll")) -eq "gogd3d8to9") -and
    -not (Test-Path (G "d3d8.dll.gog-backup")) -and -not (Test-Path (G "d3d8.dll.a2chain-backup")) -and
    (Has $out "*left as it is: d3d8.dll is neither GOG*") -and -not (Select-String -Quiet -SimpleMatch "d3d8.dll" (G "armada2-refit-prereqs.txt")))
Check "no d3d9.dll" (-not (Test-Path (G "d3d9.dll")))
Check "no MSAA.asi" (-not (Test-Path (G "MSAA.asi")))
foreach ($n in "HUD", "Menus", "QOL", "QOLRules", "GridLayout", "Planets", "Online") {
    Check "$n.asi + $n.ini" ((Test-Path (G "$n.asi")) -and (Test-Path (G "$n.ini")))
}
Check "Lighting, per vertex here" ((Test-Path (G "Lighting.asi")) -and (Test-Path (G "Lighting.ini")) -and (Has $out "*Lighting: per vertex*"))
Check "Sky, the stock sky here" ((Test-Path (G "Sky.asi")) -and (Test-Path (G "Sky\mbgaqu.ini")) -and (Has $out "*Sky: the stock sky here*"))
Check "every sky recipe" (@(Get-ChildItem (G "Sky")).Count -eq @(Get-ChildItem (Join-Path $P "sky")).Count)
$log = Get-Content -LiteralPath $PyLog
Check "the SOD rewrites, in order, with the game" (
    ($log[0] -like "*models\moon-sod.py --install") -and ($log[1] -like "game *Star Trek Armada II") -and
    ($log[2] -like "*models\select-sod.py --install") -and ($log[4] -like "*models\hull-sod.py --install"))
Remove-Item -LiteralPath $PyLog
Check "no bloom" (-not (Test-Path (G "A2Bloom.ini")))
Check "binkw32.dll is the proxy" ((Get-Content (G "binkw32.dll") -Raw) -match "BinkProxy")
Check "binkw32_orig.dll is stock" ((Text (G "binkw32_orig.dll")) -eq "stockbink")
Check "dxvk.conf installed" ((Sha (G "dxvk.conf")) -eq (Sha (Join-Path $P "game\dxvk.conf")))
Check "no WINEDLLOVERRIDES" (-not (Has $out "*WINEDLLOVERRIDES*"))

Write-Host "== no progress lines unasked; a failing model script is not fatal; the player's GridLayout.ini kept"
Set-Content (G "GridLayout.ini") "[Cells]"
$env:A2_PYTHON = $BadPy
$out = Install @($G)
$env:A2_PYTHON = $FakePy
Check "no ::step lines" (-not (Has $out "::step*"))
Check "model script failures said, install goes on" ((Has $out "    boom*") -and (Has $out "  hull-sod.py --install failed -- the rest goes on") -and
                                                     (Has $out "Uninstall: *"))
Check "GridLayout.ini kept" (((Text (G "GridLayout.ini")) -eq "[Cells]") -and (Has $out "*GridLayout.ini (yours, kept)*") -and
                             -not (Test-Path (G "GridLayout.ini.bak")))

Write-Host "== A2_SKIP=cutscenes: the proxy out, the stock binkw32.dll back"
$env:A2_SKIP = "cutscenes"
$out = Install @($G)
$env:A2_SKIP = $null
Check "said so" (Has $out "  cutscene player skipped (A2_SKIP)")
Check "binkw32.dll stock again" ((Text (G "binkw32.dll")) -eq "stockbink")
Check "no proxy files" (-not (Test-Path (G "binkw32_orig.dll")) -and -not (Test-Path (G "binkw32.dll.a2neb-backup")) -and
                        -not (Test-Path (G "BinkProxy.ini")))
$out = Install @($G)
Check "the proxy back without A2_SKIP" (((Get-Content (G "binkw32.dll") -Raw) -match "BinkProxy") -and ((Text (G "binkw32_orig.dll")) -eq "stockbink"))

Write-Host "== DXVK's d3d8.dll: the player's own choice, left as it is; MSAA on it"
Set-Content (G "d3d8.dll") "dxvk"
Set-Content (G "ReShade.ini") "[GENERAL]"
Add-Content (G "HUD.ini") "; mine"
$out = Install @($G)
Check "prerequisites already installed" (Has $out "*STA2WidescreenPatch: already installed")
Check "DXVK's d3d8.dll left, nothing recorded" (((Text (G "d3d8.dll")) -eq "dxvk") -and
    -not (Test-Path (G "d3d8.dll.a2chain-backup")) -and (Has $out "*d3d8.dll is DXVK's*"))
Check "Lighting per vertex there" (Has $out "*Lighting: per vertex*")
Check "MSAA.asi on DXVK's d3d8" (Test-Path (G "MSAA.asi"))
Check "HUD.ini.bak kept" ((Get-Content (G "HUD.ini.bak") -Raw) -match "; mine")
foreach ($f in "MagicBloom.fx", "ReShade.fxh", "ReShadeUI.fxh") {
    Check "shader $f" ((Get-Item (G "reshade-shaders\Shaders\$f")).Length -gt 0)
}
Check "A2Bloom.ini" (Test-Path (G "A2Bloom.ini"))
Check "and as ReShadePreset.ini" ((Sha (G "ReShadePreset.ini")) -eq (Sha (G "A2Bloom.ini")))

Write-Host "== no d3d8.dll: d3d8to9 goes in; MSAA only with DXVK's d3d9 behind it"
Remove-Item -LiteralPath (G "d3d8.dll")
$out = Install @($G)
Check "d3d8.dll is d3d8to9" ((Sha (G "d3d8.dll")) -eq $D3d8to9Sha)
Check "Lighting and Sky in shaders (no notes)" (-not (Has $out "*Lighting: per vertex*") -and -not (Has $out "*Sky: the stock sky*"))
Check "no MSAA.asi on Windows' own d3d9" ((-not (Test-Path (G "MSAA.asi"))) -and (Has $out "*MSAA.asi skipped*"))
Set-Content (G "d3d9.dll") "dxvk"
$out = Install @($G)
Check "d3d8to9 already" (Has $out "*d3d8.dll: crosire's d3d8to9 already")
Check "MSAA.asi on the d3d8to9 chain" (Test-Path (G "MSAA.asi"))
Set-Content (G "d3d9.dll") "gogd3d8to9"
$out = Install @($G)
Check "no MSAA.asi without DXVK's d3d9" ((-not (Test-Path (G "MSAA.asi"))) -and (Has $out "*MSAA.asi skipped*"))
Set-Content (G "d3d9.dll") "dxvk"

Write-Host "== unzipped into the game directory, run with no argument"
Copy-Item -Recurse $P (G "pkg")
$out = & powershell -NoProfile -ExecutionPolicy Bypass -File (G "pkg\install.ps1")
Check "found the game" (Has $out "installing into *Star Trek Armada II")
Remove-Item -Recurse (G "pkg")

Write-Host "== uninstall: what it added goes, what changed since stays"
Set-Content (G "UltimateASILoader-license.txt") "mine"
Set-Content (G "Sky\myown.ini") "[Sky]"   # a recipe of the player's own stays
$env:A2_PROGRESS = "1"
$out = Install @("-Uninstall", $G)
$env:A2_PROGRESS = $null
Check "::step uninstall, and only that" ((@($out | Where-Object { $_ -like "::step *" }) -join "|") -eq "::step uninstall taking the mod out")
Check "the SOD rewrites reverted" ((@(Get-Content -LiteralPath $PyLog | Where-Object { $_ -like "*--revert" }).Count -eq 3))
Remove-Item -LiteralPath $PyLog
Check "the player's own recipe left" ((@(Get-ChildItem (G "Sky")).Name -join ",") -eq "myown.ini")
Check "binkw32.dll is stock again" ((Text (G "binkw32.dll")) -eq "stockbink")
Check "no binkw32 copies" (-not (Test-Path (G "binkw32_orig.dll")) -and -not (Test-Path (G "binkw32.dll.a2neb-backup")))
foreach ($n in "HUD", "Menus", "MSAA", "QOL", "QOLRules", "GridLayout", "Planets", "Lighting", "Sky", "Online") {
    foreach ($e in ".asi", ".ini", ".log") { Check "$n$e removed" (-not (Test-Path (G "$n$e"))) }
}
foreach ($f in "BinkProxy.ini", "dxvk.conf", "A2Bloom.ini", "ReShadePreset.ini", "winmm.dll", "STA2WidescreenPatch.asi",
               "armada2-refit-prereqs.txt", "d3d8.dll.a2chain-backup") {
    Check "$f removed" (-not (Test-Path (G $f)))
}
Check "changed file left" (((Text (G "UltimateASILoader-license.txt")) -eq "mine") -and (Has $out "*left UltimateASILoader-license.txt*"))
Check "d3d8to9 taken out again" (-not (Test-Path (G "d3d8.dll")))
Check "the player's d3d9.dll left" ((Text (G "d3d9.dll")) -eq "dxvk")

Write-Host "== no d3d8.dll at all: d3d8to9 goes in, and out again; the game as it was"
$N = Join-Path $T "nod3d8"; Mock $N -NoD3d8
$before = Snap $N
$out = Install @($N)
Check "d3d8.dll is d3d8to9" ((Sha (Join-Path $N "d3d8.dll")) -eq $D3d8to9Sha)
Check "recorded" (Select-String -Quiet -SimpleMatch "d3d8.dll $D3d8to9Sha" (Join-Path $N "armada2-refit-prereqs.txt"))
Check "no d3d9.dll, no backup" (-not (Test-Path (Join-Path $N "d3d9.dll")) -and -not (Test-Path (Join-Path $N "d3d8.dll.a2chain-backup")))
Install @("-Uninstall", $N) | Out-Null
Check "the game as it was" ((Snap $N) -eq $before)
Check "d3d8.dll gone" (-not (Test-Path (Join-Path $N "d3d8.dll")))

Write-Host "== install and uninstall on a fresh GOG-like game: the game as it was"
$M = Join-Path $T "roundtrip"; Mock $M
$before = Snap $M
Install @($M) | Out-Null
Install @("-Uninstall", $M) | Out-Null
Check "the game as it was" ((Snap $M) -eq $before)

Write-Host "== errors: exit 1, the message as the last line"
$r = Fail (Join-Path $P "install.ps1") @((Join-Path $T "nowhere"))
Check "no game: exit 1" ($r.Code -eq 1)
Check "no game: said last" ($r.Last -like "no Armada2.exe in *nowhere*")
$D = Join-Path $T "damaged"; Copy-Item -Recurse $P $D
Add-Content (Join-Path $D "game\HUD.ini") "; tampered"
$r = Fail (Join-Path $D "install.ps1") @($M)
Check "damaged package: exit 1" ($r.Code -eq 1)
Check "damaged package: said last" ($r.Last -eq "the package is damaged: game/HUD.ini does not match SHA256SUMS")
Check "damaged package: nothing installed" (-not (Test-Path (Join-Path $M "HUD.asi")))

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
Check "the d3d8.dll left as it was" ((Text (O "d3d8.dll")) -eq "gogd3d8to9")
$env:A2_PYTHON = $null
Remove-Item -Recurse -Force $T
Write-Host "all passed"
