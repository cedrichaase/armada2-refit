# Run the package's install.ps1 against a mock game directory.   test.ps1 <zip>
#
# test.sh's twin, for Windows; CI runs it on a Windows runner. The game is a few
# placeholder files, so this proves what the installer copies, backs up and puts back --
# not that anything loads. The model scripts are
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
$Steps = "verify prereqs chain display render3d ui qol models online cutscenes renderer bloom done"
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
# What earlier packages and installs left: the per-part plugins Render3D.asi replaced.
foreach ($n in "MSAA", "Planets", "Lighting", "Sky", "Fog", "Specials") {
    Set-Content (G "$n.asi") "old"; Set-Content (G "$n.ini") "old"
}
New-Item -ItemType Directory (G "Sky") | Out-Null
Set-Content (G "Sky\mbgaqu.ini") "old"; Set-Content (G "Sky\myown.ini") "[Sky]"

Write-Host "== every PE of ours carries its version resource, as Windows reads it (platform/versioninfo/)"
$Ver = (Split-Path $P -Leaf) -replace '^armada2-refit-', ''
foreach ($f in @(Get-ChildItem -LiteralPath (Join-Path $P "game"), (Join-Path $P "display") -File |
                 Where-Object { $_.Extension -in ".asi", ".dll" })) {
    $vi = $f.VersionInfo
    Check "$($f.Name): '$($vi.FileDescription)' $($vi.FileVersion), product $($vi.ProductVersion)" (
        ($vi.FileDescription -like "Armada II Refit: *") -and ($vi.ProductVersion -eq $Ver) -and
        ($vi.FileVersion -match '^\d+\.\d+\.\d+$') -and ($vi.OriginalFilename -eq $f.Name) -and
        ($vi.ProductName -eq "Armada II Refit"))
}

Write-Host "== install on a GOG game: prerequisites, no DXVK, with progress, the old plugins out"
$env:A2_PROGRESS = "1"
$out = Install @($G)
$env:A2_PROGRESS = $null
Check "::step ids in install.sh's order" ((@($out | Where-Object { $_ -like "::step *" } |
                                           ForEach-Object { ($_ -split " ")[1] }) -join " ") -eq $Steps)
$at = @{}; for ($i = 0; $i -lt $out.Count; $i++) { if (-not $at.ContainsKey($out[$i])) { $at[$out[$i]] = $i } }
Check "::step lines in order with the rest" (($at["::step ui the HUD and the menus"] -lt $at["  HUD.asi"]) -and
                                             ($at["  HUD.asi"] -lt $at["::step qol quality of life"]) -and
                                             ($at["::step done done"] -gt $at["  dxvk.conf"]))
Check "winmm.dll" ((Sha (G "winmm.dll")) -eq "baba99929487b005bb9b168acfd852550055f22e5f1059c9032765209bb185e5")
Check "STA2WidescreenPatch.asi" ((Sha (G "STA2WidescreenPatch.asi")) -eq "193828b15b8cdba84617dd9a359b555302132a35bbaa6fd3143fdb99442343c5")
Check "a d3d8.dll that is neither GOG's nor d3d8to9 left as it is" (((Text (G "d3d8.dll")) -eq "gogd3d8to9") -and
    -not (Test-Path (G "d3d8.dll.gog-backup")) -and -not (Test-Path (G "d3d8.dll.a2chain-backup")) -and
    (Has $out "*left as it is: d3d8.dll is neither GOG*") -and -not (Select-String -Quiet -SimpleMatch "d3d8.dll" (G "armada2-refit-prereqs.txt")))
Check "no d3d9.dll" (-not (Test-Path (G "d3d9.dll")))
Check "Display8 skipped: it goes in front of d3d8to9 only" ((Has $out "*Display8 skipped*") -and
    -not (Test-Path (G "d3d8_real.dll")) -and -not (Test-Path (G "Display8.ini")))
foreach ($n in "MSAA", "Planets", "Lighting", "Sky", "Fog", "Specials") {
    Check "$n.asi + $n.ini out" (-not (Test-Path (G "$n.asi")) -and -not (Test-Path (G "$n.ini")))
}
Check "the player's own recipe left, Sky.asi's gone" ((@(Get-ChildItem (G "Sky")).Name -join ",") -eq "myown.ini")
foreach ($n in "HUD", "Menus", "QOL", "QOLRules", "GridLayout", "Online") {
    Check "$n.asi + $n.ini" ((Test-Path (G "$n.asi")) -and (Test-Path (G "$n.ini")))
}
# No d3d8to9: no Render3D.asi or UI.asi; the older plugins above in the UI's place.
Check "no Render3D.asi, no UI.asi, said so" (-not (Test-Path (G "Render3D.asi")) -and -not (Test-Path (G "UI.asi")) -and
    -not (Test-Path (G "Surfaces")) -and (Has $out "*Render3D.asi skipped*") -and (Has $out "*UI.asi skipped*"))
$log = Get-Content -LiteralPath $PyLog
Check "the SOD rewrites, in order, with the game" (
    ($log[0] -like "*models\moon-sod.py --install") -and ($log[1] -like "game *Star Trek Armada II") -and
    ($log[2] -like "*models\select-sod.py --install") -and ($log[4] -like "*models\hull-sod.py --install") -and
    ($log[6] -like "*models\asteroid-sod.py --install"))
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

Write-Host "== DXVK's d3d8.dll: the player's own choice, left as it is; an old package's bloom preset out"
Set-Content (G "d3d8.dll") "dxvk"
Set-Content (G "ReShade.ini") "[GENERAL]"
Set-Content (G "A2Bloom.ini") "Techniques=MagicBloom"
Copy-Item (G "A2Bloom.ini") (G "ReShadePreset.ini")
Add-Content (G "HUD.ini") "; mine"
$out = Install @($G)
Check "prerequisites already installed" (Has $out "*STA2WidescreenPatch: already installed")
Check "DXVK's d3d8.dll left, nothing recorded" (((Text (G "d3d8.dll")) -eq "dxvk") -and
    -not (Test-Path (G "d3d8.dll.a2chain-backup")) -and (Has $out "*d3d8.dll is DXVK's*"))
Check "no Display8 in front of DXVK" ((Has $out "*Display8 skipped*") -and -not (Test-Path (G "d3d8_real.dll")))
Check "HUD.ini.bak kept" ((Get-Content (G "HUD.ini.bak") -Raw) -match "; mine")
Check "old bloom preset out, ReShade's own file left" (-not (Test-Path (G "A2Bloom.ini")) -and
    -not (Test-Path (G "ReShadePreset.ini")) -and (Test-Path (G "ReShade.ini")) -and (Has $out "*removed the old bloom preset*"))

Write-Host "== no d3d8.dll: d3d8to9 goes in with Display8 in front, and with them Render3D.asi and UI.asi"
Remove-Item -LiteralPath (G "d3d8.dll")
New-Item -ItemType Directory (G "Surfaces") | Out-Null
Set-Content (G "Surfaces\myown.ini") "[Surface]"
$out = Install @($G)
Check "d3d8.dll is Display8, d3d8_real.dll d3d8to9" (((Sha (G "d3d8.dll")) -eq (Sha (Join-Path $P "display\d3d8.dll"))) -and
    ((Sha (G "d3d8_real.dll")) -eq $D3d8to9Sha) -and -not (Test-Path (G "d3d8.dll.a2display-new")))
Check "Display8.ini as shipped" ((Sha (G "Display8.ini")) -eq (Sha (Join-Path $P "display\Display8.ini")))
Check "Render3D.asi and UI.asi" (((Sha (G "Render3D.asi")) -eq (Sha (Join-Path $P "game\Render3D.asi"))) -and
    ((Sha (G "UI.asi")) -eq (Sha (Join-Path $P "game\UI.asi"))) -and (Test-Path (G "Render3D.ini")) -and (Test-Path (G "UI.ini")))
Check "the recipes" ((@(Get-ChildItem (G "Nebulae")).Count -eq @(Get-ChildItem (Join-Path $P "game\Nebulae")).Count) -and
    (@(Get-ChildItem (G "Surfaces")).Count -eq @(Get-ChildItem (Join-Path $P "game\Surfaces")).Count + 1))
foreach ($n in "HUD", "Menus", "GridLayout") {
    Check "$n.asi + $n.ini out for the UI" (-not (Test-Path (G "$n.asi")) -and -not (Test-Path (G "$n.ini")))
}
Set-Content (G "d3d9.dll") "dxvk"
# Reinstalled over Display8: an older build replaced, d3d8to9 still behind it, and
# Display8.ini merged: the old defaults moved, the player's own values kept, every key
# the player's lacks added, each once.
[IO.File]::WriteAllText((G "d3d8.dll"), "MZ Display8 stopped the game: an older build")
[IO.File]::WriteAllText((G "Display8.ini"), "[Display8]`nMode=forward`nBackend=d3d9`nVSync=0`nLog=1`n`n[Drop]`nDrop2D=1`n")
$out = Install @($G)
Check "d3d8to9 already" (Has $out "*d3d8.dll: crosire's d3d8to9 already")
Check "Display8 replaced, d3d8to9 behind it" (((Sha (G "d3d8.dll")) -eq (Sha (Join-Path $P "display\d3d8.dll"))) -and
    ((Sha (G "d3d8_real.dll")) -eq $D3d8to9Sha) -and ((Sha (G "UI.asi")) -eq (Sha (Join-Path $P "game\UI.asi"))))
$ini = @(Get-Content -LiteralPath (G "Display8.ini"))
Check "Display8.ini: the old defaults moved, said so" (($ini -ccontains "Mode=stub") -and ($ini -ccontains "Backend=auto") -and
    ($ini -ccontains "Defaults=2") -and ($out -ccontains "  Display8.ini: [display8] Mode forward -> stub (defaults 2)"))
Check "Display8.ini: the player's values kept, the missing keys added once" (($ini -ccontains "VSync=0") -and
    ($ini -ccontains "Drop2D=1") -and ($ini -ccontains "Real=auto") -and ($ini -ccontains "Frames=3") -and
    (@($ini | Where-Object { $_ -like "Backend=*" }).Count -eq 1))
$merged = Sha (G "Display8.ini")
Install @($G) | Out-Null
Check "Display8.ini merged again: nothing left to change" ((Sha (G "Display8.ini")) -eq $merged)
Set-Content (G "d3d9.dll") "dxvk"

Write-Host "== unzipped into the game directory, run with no argument"
Copy-Item -Recurse $P (G "pkg")
$out = & powershell -NoProfile -ExecutionPolicy Bypass -File (G "pkg\install.ps1")
Check "found the game" (Has $out "installing into *Star Trek Armada II")
Remove-Item -Recurse (G "pkg")

Write-Host "== uninstall: what it added goes, what changed since stays"
Set-Content (G "UltimateASILoader-license.txt") "mine"
Set-Content (G "Sky\myown.ini") "[Sky]"   # a recipe of the player's own stays
Set-Content (G "Display8.log") "log"
$env:A2_PROGRESS = "1"
$out = Install @("-Uninstall", $G)
$env:A2_PROGRESS = $null
Check "::step uninstall, and only that" ((@($out | Where-Object { $_ -like "::step *" }) -join "|") -eq "::step uninstall taking the mod out")
Check "the SOD rewrites reverted" ((@(Get-Content -LiteralPath $PyLog | Where-Object { $_ -like "*--revert" }).Count -eq 4))
Remove-Item -LiteralPath $PyLog
Check "the player's own recipe left" ((@(Get-ChildItem (G "Sky")).Name -join ",") -eq "myown.ini")
Check "Render3D's recipes out, the player's own left" (((@(Get-ChildItem (G "Surfaces")).Name -join ",") -eq "myown.ini") -and
    -not (Test-Path (G "Nebulae")))
Check "binkw32.dll is stock again" ((Text (G "binkw32.dll")) -eq "stockbink")
Check "no binkw32 copies" (-not (Test-Path (G "binkw32_orig.dll")) -and -not (Test-Path (G "binkw32.dll.a2neb-backup")))
foreach ($n in "Render3D", "UI", "HUD", "Menus", "QOL", "QOLRules", "GridLayout", "Online") {
    foreach ($e in ".asi", ".ini", ".log") { Check "$n$e removed" (-not (Test-Path (G "$n$e"))) }
}
foreach ($f in "BinkProxy.ini", "dxvk.conf", "A2Bloom.ini", "ReShadePreset.ini", "winmm.dll", "STA2WidescreenPatch.asi",
               "armada2-refit-prereqs.txt", "d3d8.dll.a2chain-backup", "d3d8_real.dll", "Display8.ini",
               "Display8.log", "d3d8.dll.a2display-backup") {
    Check "$f removed" (-not (Test-Path (G $f)))
}
Check "changed file left" (((Text (G "UltimateASILoader-license.txt")) -eq "mine") -and (Has $out "*left UltimateASILoader-license.txt*"))
Check "d3d8to9 taken out again" (-not (Test-Path (G "d3d8.dll")))
Check "the player's d3d9.dll left" ((Text (G "d3d9.dll")) -eq "dxvk")

Write-Host "== no d3d8.dll at all: d3d8to9 goes in, and out again; the game as it was"
$N = Join-Path $T "nod3d8"; Mock $N -NoD3d8
$before = Snap $N
$out = Install @($N)
Check "d3d8to9 behind Display8" ((Sha (Join-Path $N "d3d8_real.dll")) -eq $D3d8to9Sha)
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
Check "damaged package: said last" ($r.Last -like "the package is damaged: game\HUD.ini does not match SHA256SUMS*")
Check "damaged package: nothing installed" (-not (Test-Path (Join-Path $M "HUD.asi")))
Check "no ::error lines unasked" (-not (Has $r.Out "::error*"))
$env:A2_PROGRESS = "1"
$r = Fail (Join-Path $D "install.ps1") @($M)
Check "damaged package: ::error damaged game\HUD.ini" (Has $r.Out "::error damaged game\HUD.ini")
$r = Fail (Join-Path $P "install.ps1") @((Join-Path $T "nowhere"))
Check "no game: ::error no-game" (Has $r.Out "::error no-game *nowhere")

Write-Host "== an antivirus holding package files open: every one named, as an antivirus"
$A = Join-Path $T "locked"; Copy-Item -Recurse $P $A
$held = foreach ($n in "Menus.asi", "QOL.asi") { [IO.File]::Open((Join-Path $A "game\$n"), "Open", "Read", "None") }
try {
    $r = Fail (Join-Path $A "install.ps1") @($M)
} finally { foreach ($h in $held) { $h.Dispose() } }
Check "locked: exit 1" ($r.Code -eq 1)
Check "locked: ::error av-locked for each" ((Has $r.Out "::error av-locked game\Menus.asi") -and (Has $r.Out "::error av-locked game\QOL.asi"))
Check "locked: both named, as an antivirus, last" (($r.Last -like "*game\Menus.asi, game\QOL.asi from the package could not be read. An antivirus*") -and
    ($r.Last -like "*exclusions for *locked and *roundtrip*"))
Check "locked: nothing installed" (-not (Test-Path (Join-Path $M "HUD.asi")))

Write-Host "== an antivirus quarantined a package file: named, as an antivirus"
Remove-Item -LiteralPath (Join-Path $A "game\Online.asi")
$r = Fail (Join-Path $A "install.ps1") @($M)
Check "missing: exit 1" ($r.Code -eq 1)
Check "missing: ::error av-missing" (Has $r.Out "::error av-missing game\Online.asi")
Check "missing: named, as an antivirus, last" ($r.Last -like "game\Online.asi from the package is missing. An antivirus has most likely quarantined*")

Write-Host "== a file the game folder refuses is named"
$R = Join-Path $T "readonly"; Mock $R
Set-Content (Join-Path $R "QOL.asi") "locked"
$hold = [IO.File]::Open((Join-Path $R "QOL.asi"), "Open", "Read", "None")
try {
    $r = Fail (Join-Path $P "install.ps1") @($R)
} finally { $hold.Dispose() }
Check "write refused: ::error write-denied" (Has $r.Out "::error write-denied *readonly\QOL.asi")
Check "write refused: named last" ($r.Last -like "QOL.asi could not be written into *readonly*")
$env:A2_PROGRESS = $null

Write-Host "== what it did not write is left alone: a loader, a dxvk.conf, a ReShade preset of the player's"
$H = Join-Path $T "other"; Mock $H
function O([string]$n) { Join-Path $H $n }
Set-Content (O "winmm.dll") "myloader"
Set-Content (O "dxvk.conf") "d3d9.foo = 1"
Set-Content (O "ReShade.ini") "[GENERAL]"
Set-Content (O "ReShadePreset.ini") "Techniques=Mine"
Set-Content (O "A2Bloom.ini") "Techniques=MagicBloom"
Install @($H) | Out-Null
Check "own loader kept, patch added" (((Text (O "winmm.dll")) -eq "myloader") -and (Test-Path (O "STA2WidescreenPatch.asi")))
Check "own preset kept on install" ((Text (O "ReShadePreset.ini")) -eq "Techniques=Mine")
Check "the old A2Bloom.ini beside it out" (-not (Test-Path (O "A2Bloom.ini")))
Install @("-Uninstall", $H) | Out-Null
Check "own loader kept, patch removed" (((Text (O "winmm.dll")) -eq "myloader") -and -not (Test-Path (O "STA2WidescreenPatch.asi")))
Check "foreign dxvk.conf kept" ((Text (O "dxvk.conf")) -eq "d3d9.foo = 1")
Check "own preset kept on uninstall" ((Text (O "ReShadePreset.ini")) -eq "Techniques=Mine")
Check "the d3d8.dll left as it was" ((Text (O "d3d8.dll")) -eq "gogd3d8to9")
$env:A2_PYTHON = $null
Remove-Item -Recurse -Force $T
Write-Host "all passed"
