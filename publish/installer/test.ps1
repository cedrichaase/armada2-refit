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
# install.sh's ::step ids, in order (manifest.json's "steps").
$Steps = "verify prereqs display retire render3d ui qol rules models online cutscenes bloom done"
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
function Mock([string]$d, [switch]$NoD3d8) {   # a GOG install: the exe, the stock Bink DLL, GOG's d3d8.dll
    New-Item -ItemType Directory $d | Out-Null
    Set-Content (Join-Path $d "Armada2.exe") "exe"
    Set-Content (Join-Path $d "binkw32.dll") "stockbink"
    # Not GOG's real d3d8to9 (that is the game's): the player's own, until the package is
    # tweaked to take this content for GOG's.
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
# The player's settings file: no installer writes, merges, ships or removes it, so it is
# byte for byte what the player left through install, upgrade and uninstall.
[IO.File]::WriteAllText((G "RefitPlayer.ini"), "; mine`r`n[Render3D.Modules]`r`nhulls=0`r`n[UI.HUD]`r`nBarHidden=1`r`n")
$OverlaySha = Sha (G "RefitPlayer.ini")
function OverlayKept {
    ((Sha (G "RefitPlayer.ini")) -eq $OverlaySha) -and -not (Test-Path (Join-Path $P "layers\qol\RefitPlayer.ini")) -and
        (@(Get-ChildItem -LiteralPath $G -Filter "RefitPlayer.ini*").Count -eq 1)
}
# What earlier packages and installs left: the per-part plugins Render3D.asi replaced.
foreach ($n in "MSAA", "Planets", "Lighting", "Sky", "Fog", "Specials") {
    Set-Content (G "$n.asi") "old"; Set-Content (G "$n.ini") "old"
}
New-Item -ItemType Directory (G "Sky") | Out-Null
# ...and Sky.asi's recipes: a shipped copy goes, one the player edited stays (retire.txt's
# glob-hash knows the shipped ones by hash, so the copies are the repository's own bytes).
$skyd = Join-Path $PSScriptRoot "..\..\render3d\content\sky"
Copy-Item (Join-Path $skyd "mbgaqu.ini") (G "Sky\mbgaqu.ini"); Set-Content (G "Sky\myown.ini") "[Sky]"
[IO.File]::WriteAllBytes((G "Sky\mbgred.ini"), [byte[]]((Get-Content -LiteralPath (Join-Path $skyd "mbgred.ini") -Encoding Byte) + [Text.Encoding]::ASCII.GetBytes("`n; mine`n")))

# ...and the legacy plugins UI.asi replaced: retired.
foreach ($n in "HUD", "Menus", "GridLayout", "MenuScale") {
    Set-Content (G "$n.asi") "old"; Set-Content (G "$n.ini") "old"
}

Write-Host "== every PE of ours carries its version resource, as Windows reads it (platform/versioninfo/)"
$Ver = (Split-Path $P -Leaf) -replace '^armada2-refit-', ''
foreach ($f in @(Get-ChildItem -LiteralPath (Join-Path $P "layers") -Recurse -File |
                 Where-Object { $_.Extension -in ".asi", ".dll" })) {
    $vi = $f.VersionInfo
    Check "$($f.Name): '$($vi.FileDescription)' $($vi.FileVersion), product $($vi.ProductVersion)" (
        ($vi.FileDescription -like "Armada II Refit: *") -and ($vi.ProductVersion -eq $Ver) -and
        ($vi.FileVersion -match '^\d+\.\d+\.\d+$') -and ($vi.OriginalFilename -eq $f.Name) -and
        ($vi.ProductName -eq "Armada II Refit"))
}

Write-Host "== install on a game whose d3d8.dll is the player's own: prerequisites, the old plugins and bloom out, with progress"
$env:A2_PROGRESS = "1"
$out = Install @($G)
$env:A2_PROGRESS = $null
Check "::step ids in install.sh's order" ((@($out | Where-Object { $_ -like "::step *" } |
                                           ForEach-Object { ($_ -split " ")[1] }) -join " ") -eq $Steps)
$at = @{}; for ($i = 0; $i -lt $out.Count; $i++) { if (-not $at.ContainsKey($out[$i])) { $at[$out[$i]] = $i } }
Check "::step lines in order with the rest" (($at["::step display Display8"] -lt $at["retired:"]) -and
                                             ($at["retired:"] -lt $at["::step render3d the 3D view"]) -and
                                             ($at["::step qol quality of life"] -lt $at["::step done done"]))
Check "winmm.dll" ((Sha (G "winmm.dll")) -eq "baba99929487b005bb9b168acfd852550055f22e5f1059c9032765209bb185e5")
Check "STA2WidescreenPatch.asi" ((Sha (G "STA2WidescreenPatch.asi")) -eq "193828b15b8cdba84617dd9a359b555302132a35bbaa6fd3143fdb99442343c5")
Check "the player's own d3d8.dll left as it is" (((Text (G "d3d8.dll")) -eq "gogd3d8to9") -and
    -not (Test-Path (G "d3d8.dll.gog-backup")) -and -not (Test-Path (G "d3d8.dll.a2chain-backup")) -and
    -not (Select-String -Quiet -SimpleMatch "d3d8.dll" (G "armada2-refit-prereqs.txt")))
Check "nothing of the chain: no d3d9.dll, no dxvk.conf, no chain\ or layers\renderer in the package" (
    -not (Test-Path (G "d3d9.dll")) -and -not (Test-Path (G "dxvk.conf")) -and
    -not (Test-Path (Join-Path $P "chain")) -and -not (Test-Path (Join-Path $P "layers\renderer")))
Check "Display8 skipped: a d3d8.dll that is none of GOG's, the old chain's or ours stays" ((Has $out "*Display8 skipped: d3d8.dll is none of GOG's, the old chain's or ours*") -and
    -not (Test-Path (G "d3d8_real.dll")) -and -not (Test-Path (G "Display8.ini")))
foreach ($n in "MSAA", "Planets", "Lighting", "Sky", "Fog", "Specials") {
    Check "$n.asi + $n.ini out" (-not (Test-Path (G "$n.asi")) -and -not (Test-Path (G "$n.ini")))
}
foreach ($n in "HUD", "Menus", "GridLayout", "MenuScale") {
    Check "$n.asi + $n.ini retired" (-not (Test-Path (G "$n.asi")) -and -not (Test-Path (G "$n.ini")))
}
Check "retired said so" ((Has $out "  retired HUD.asi") -and (Has $out "  retired Sky\mbgaqu.ini"))
Check "the player's own recipe left, Sky.asi's gone" (((@(Get-ChildItem (G "Sky")).Name -join ",") -eq "mbgred.ini,myown.ini") -and
    (Select-String -Quiet -SimpleMatch "; mine" (G "Sky\mbgred.ini")))
Remove-Item -LiteralPath (G "Sky\mbgred.ini")
foreach ($n in "QOL", "QOLRules", "Online") {
    Check "$n.asi + $n.ini" ((Test-Path (G "$n.asi")) -and (Test-Path (G "$n.ini")))
}
# Display8 stays out, and so do Render3D.asi and UI.asi: the game keeps its own look.
Check "no Render3D.asi, no UI.asi, said so" (-not (Test-Path (G "Render3D.asi")) -and -not (Test-Path (G "UI.asi")) -and
    -not (Test-Path (G "Surfaces")) -and
    (Has $out "  Render3D.asi skipped: it needs Display8 as d3d8.dll -- the 3D view stays the engine's") -and
    (Has $out "  UI.asi skipped: it needs Display8 as d3d8.dll -- the HUD and menus stay the engine's"))
$log = @(Get-Content -LiteralPath $PyLog)
Check "retire\hud-files.py run with the game" ($log[0] -like "*retire\hud-files.py *")
$k = 0; while ($k -lt $log.Count -and $log[$k] -notlike "*moon-sod.py*") { $k++ }
$log = @($log[$k..($log.Count - 1)])
Check "the SOD rewrites, in order, with the game" (
    ($log[0] -like "*models\moon-sod.py --install") -and ($log[1] -like "game *Star Trek Armada II") -and
    ($log[2] -like "*models\select-sod.py --install") -and ($log[4] -like "*models\hull-sod.py --install") -and
    ($log[6] -like "*models\asteroid-sod.py --install"))
Remove-Item -LiteralPath $PyLog
Check "no bloom" (-not (Test-Path (G "A2Bloom.ini")))
Check "binkw32.dll is the proxy" ((Get-Content (G "binkw32.dll") -Raw) -match "BinkProxy")
Check "binkw32_orig.dll is stock" ((Text (G "binkw32_orig.dll")) -eq "stockbink")
Check "no WINEDLLOVERRIDES" (-not (Has $out "*WINEDLLOVERRIDES*"))
Check "RefitPlayer.ini untouched by the install" (OverlayKept)

# From here on the package is a copy whose hashes are the mock files': the mock GOG d3d8.dll,
# a stand-in for crosire's d3d8to9 and for DXVK's d3d9. The scripts know the real ones by
# hash, and the repository no longer carries them.
$GogDll = Join-Path $T "gog.dll"; Set-Content $GogDll "gogd3d8to9"
$To9Dll = Join-Path $T "d3d8to9.dll"; Set-Content $To9Dll "faked3d8to9"
$Dxvk9Dll = Join-Path $T "dxvk9.dll"; Set-Content $Dxvk9Dll "fakedxvkd3d9"
$Q = Join-Path $T "tweaked"; Copy-Item -Recurse $P $Q
function Rewrite([string]$file, [hashtable]$map) {
    $txt = [IO.File]::ReadAllText($file)
    foreach ($k in $map.Keys) { $txt = $txt.Replace($k, $map[$k]) }
    [IO.File]::WriteAllText($file, $txt)
}
$Rehash = @{ "735dbb81a5fa0368c6436349fc5bf97a9ae15daeb6829dfe95a17704294e6719" = (Sha $GogDll)
             "122928cfe225c25d30decf7184a5d37e490cecf3b58256ba3206c7e1853f8ab8" = (Sha $To9Dll)
             "265888c31ca78dffa290c39cb7e50bfb02762590e41927906e46fb32f01497fa" = (Sha $Dxvk9Dll) }
Rewrite (Join-Path $Q "install.ps1") $Rehash
Rewrite (Join-Path $Q "retire.txt") $Rehash
$sums = @()
foreach ($l in Get-Content -LiteralPath (Join-Path $Q "SHA256SUMS")) {
    $sum, $file = $l -split "\s+\*?", 2
    if ($file -in "install.ps1", "retire.txt") { $l = "$(Sha (Join-Path $Q $file))  $file" }
    $sums += $l
}
[IO.File]::WriteAllText((Join-Path $Q "SHA256SUMS"), (($sums -join "`n") + "`n"))
$P = $Q

Write-Host "== GOG's d3d8.dll: Display8 goes in, GOG's kept; Render3D.asi and UI.asi with it; reinstall with an edited .ini"
$out = Install @($G)   # (the mock's d3d8.dll is GOG's by hash now)
Check "d3d8.dll is Display8, GOG's kept as d3d8.dll.gog-backup" (((Sha (G "d3d8.dll")) -eq (Sha (Join-Path $P "layers\display\d3d8.dll"))) -and
    ((Text (G "d3d8.dll.gog-backup")) -eq "gogd3d8to9"))
Check "no d3d8_real.dll, no d3d9.dll, no dxvk.conf" (-not (Test-Path (G "d3d8_real.dll")) -and -not (Test-Path (G "d3d9.dll")) -and
    -not (Test-Path (G "dxvk.conf")))
Check "Display8.ini as shipped" ((Sha (G "Display8.ini")) -eq (Sha (Join-Path $P "layers\display\Display8.ini")))
Check "no half-copied files" (-not (Test-Path (G "d3d8.dll.a2display-new")))
Check "Render3D.asi and UI.asi" (((Sha (G "Render3D.asi")) -eq (Sha (Join-Path $P "layers\render3d\Render3D.asi"))) -and
    ((Sha (G "UI.asi")) -eq (Sha (Join-Path $P "layers\ui\UI.asi"))) -and (Test-Path (G "Render3D.ini")) -and (Test-Path (G "UI.ini")))
Check "the recipes" ((@(Get-ChildItem (G "Nebulae")).Count -eq @(Get-ChildItem (Join-Path $P "layers\render3d\Nebulae")).Count) -and
    (@(Get-ChildItem (G "Surfaces")).Count -eq @(Get-ChildItem (Join-Path $P "layers\render3d\Surfaces")).Count))
Check "no WINEDLLOVERRIDES" (-not (Has $out "*WINEDLLOVERRIDES*"))
Check "RefitPlayer.ini untouched by the Display8 install" (OverlayKept)

Write-Host "== no progress lines unasked; a failing model script is not fatal"
$env:A2_PYTHON = $BadPy
$out = Install @($G)
$env:A2_PYTHON = $FakePy
Check "no ::step lines" (-not (Has $out "::step*"))
Check "model script failures said, install goes on" ((Has $out "    boom*") -and (Has $out "  hull-sod.py --install failed -- the rest goes on") -and
                                                     (Has $out "Uninstall: *"))
Check "nothing left to retire on a re-run" (Has $out "  nothing to take out")

Write-Host "== A2_SKIP=cutscenes: the proxy out, the stock binkw32.dll back (and remembered)"
$env:A2_SKIP = "cutscenes"
$out = Install @($G)
$env:A2_SKIP = $null
Check "said so" (Has $out "  cutscene player skipped (left out)")
Check "binkw32.dll stock again" ((Text (G "binkw32.dll")) -eq "stockbink")
Check "no proxy files" (-not (Test-Path (G "binkw32_orig.dll")) -and -not (Test-Path (G "binkw32.dll.a2neb-backup")) -and
                        -not (Test-Path (G "BinkProxy.ini")))
Check "the choice is remembered" ((Get-Content -LiteralPath (G "armada2-refit-layers.txt")) -contains "without cutscenes")
$out = Install @($G)
Check "a plain re-run keeps it" ((Has $out "keeping your earlier choice: without cutscenes*") -and ((Text (G "binkw32.dll")) -eq "stockbink"))
$out = Install @("-All", $G)
Check "the proxy back with -All, the choice forgotten" (((Get-Content (G "binkw32.dll") -Raw) -match "BinkProxy") -and
    ((Text (G "binkw32_orig.dll")) -eq "stockbink") -and -not (Test-Path (G "armada2-refit-layers.txt")))

Write-Host "== -Without rules,qol: taken out, kept by a plain re-run, an unknown or core layer refused, -All brings them back"
Set-Content (G "QOLRules.log") "log"
$out = Install @("-Without", "rules,qol", $G)
Check "no QOLRules.*, no QOL.*" (-not (@(Get-ChildItem -LiteralPath $G -Filter "QOLRules*").Count) -and -not (Test-Path (G "QOL.asi")) -and
    -not (Test-Path (G "QOL.ini")))
Check "the other layers stay" ((Test-Path (G "Online.asi")) -and (Test-Path (G "UI.asi")))
$out = Install @($G)
Check "a plain re-run keeps the choice" ((Has $out "keeping your earlier choice: without rules qol*") -and -not (Test-Path (G "QOL.asi")))
$r = Fail (Join-Path $P "install.ps1") @("-Without", "display", $G)
Check "a core layer cannot be left out" ($r.Last -like "layer display is not a choice*")
$r = Fail (Join-Path $P "install.ps1") @("-Without", "nonesuch", $G)
Check "an unknown layer is refused" ($r.Last -like "unknown layer: nonesuch*")
$out = Install @("-All", $G)
Check "-All brings them back" ((Test-Path (G "QOL.asi")) -and (Test-Path (G "QOLRules.asi")) -and -not (Test-Path (G "armada2-refit-layers.txt")))
Check "RefitPlayer.ini untouched by the selection" (OverlayKept)

Write-Host "== a DXVK d3d8.dll is the old chain's: replaced by Display8, GOG's copy kept; an old package's bloom preset out"
Set-Content (G "d3d8.dll") "dxvk"
Set-Content (G "ReShade.ini") "[GENERAL]"
Set-Content (G "A2Bloom.ini") "Techniques=MagicBloom"
Copy-Item (G "A2Bloom.ini") (G "ReShadePreset.ini")
Add-Content (G "QOL.ini") "; mine"
New-Item -ItemType Directory (G "Surfaces") -Force | Out-Null
Set-Content (G "Surfaces\myown.ini") "[Surface]"
$env:A2_PROGRESS = "1"
$out = Install @($G)
$env:A2_PROGRESS = $null
Check "prerequisites already installed" (Has $out "*STA2WidescreenPatch: already installed")
Check "DXVK's d3d8.dll replaced by Display8, GOG's copy kept" (((Sha (G "d3d8.dll")) -eq (Sha (Join-Path $P "layers\display\d3d8.dll"))) -and
    ((Text (G "d3d8.dll.gog-backup")) -eq "gogd3d8to9") -and (Has $out "  d3d8.dll: not one we know, replaced*") -and
    ((Text (G "d3d8.dll.a2chain-backup")) -eq "dxvk"))
Check "the display step, then the retire step, then the plugins'" ((@($out | Where-Object { $_ -like "::step *" } |
    ForEach-Object { ($_ -split " ")[1] } | Select-Object -Skip 2 -First 3) -join " ") -eq "display retire render3d")
Check "no chain or renderer step" (-not (Has $out "::step chain*") -and -not (Has $out "::step renderer*"))
Check "QOL.ini merged, not set aside as .bak" (((Get-Content (G "QOL.ini") -Raw) -match "; mine") -and -not (Test-Path (G "QOL.ini.bak")))
Check "old bloom preset out, ReShade's own file left" (-not (Test-Path (G "A2Bloom.ini")) -and
    -not (Test-Path (G "ReShadePreset.ini")) -and (Test-Path (G "ReShade.ini")) -and (Has $out "*removed the old bloom preset*"))
Check "the recipes" (@(Get-ChildItem (G "Surfaces")).Count -eq @(Get-ChildItem (Join-Path $P "layers\render3d\Surfaces")).Count + 1)

Write-Host "== reinstalled over an older Display8: replaced, Display8.ini merged (old defaults moved, the player's values kept, keys added once)"
[IO.File]::WriteAllText((G "d3d8.dll"), "MZ Display8 stopped the game: an older build")
[IO.File]::WriteAllText((G "Display8.ini"), "[Display8]`nDefaults=1`nMode=forward`nBackend=d3d9`nVSync=0`nLog=1`n`n[Drop]`nDrop2D=1`n")
$out = Install @($G)
Check "Display8 replaced" (((Sha (G "d3d8.dll")) -eq (Sha (Join-Path $P "layers\display\d3d8.dll"))) -and
    ((Sha (G "UI.asi")) -eq (Sha (Join-Path $P "layers\ui\UI.asi"))))
$ini = @(Get-Content -LiteralPath (G "Display8.ini"))
Check "Display8.ini: Defaults moved to the shipped one, Mode gone, d3d9 gone from Backend" (($ini -ccontains "Defaults=3") -and
    -not ($ini -like "Mode=*") -and -not ($ini -like "Backend=*d3d9*") -and ($out -ccontains "  Display8.ini: [display8] Mode dropped (defaults 3)"))
Check "Display8.ini: the player's values kept, the missing keys added once" (($ini -ccontains "VSync=0") -and
    ($ini -ccontains "Drop2D=1") -and (@($ini | Where-Object { $_ -like "Backend=*" }).Count -eq 1))
$merged = Sha (G "Display8.ini")
Install @($G) | Out-Null
Check "Display8.ini merged again: nothing left to change" ((Sha (G "Display8.ini")) -eq $merged)
Check "RefitPlayer.ini untouched by the upgrade" (OverlayKept)

function SectionOf($lines, [string]$line) {   # the [section] header above a line
    $s = ""
    foreach ($l in $lines) { if ($l.StartsWith("[")) { $s = $l } elseif ($l -ceq $line) { return $s } }
}
Write-Host "== a 14.x-shaped game: the old chain, a marked dxvk.conf with an edited LodBias, the legacy plugins"
$O = Join-Path $T "old"; Mock $O -NoD3d8
Copy-Item (G "d3d8.dll.gog-backup") (Join-Path $O "d3d8.dll.gog-backup")
[IO.File]::WriteAllText((Join-Path $O "d3d8.dll"), "MZ Display8 stopped the game: a 14.x build")
Copy-Item $To9Dll (Join-Path $O "d3d8_real.dll"); Copy-Item $Dxvk9Dll (Join-Path $O "d3d9.dll")
[IO.File]::WriteAllText((Join-Path $O "armada2-refit-prereqs.txt"), "d3d8.dll $(Sha $To9Dll)`nd3d9.dll $(Sha $Dxvk9Dll)`n")
[IO.File]::WriteAllText((Join-Path $O "dxvk.conf"), "# generated by tools/renderer-config.sh`nd3d9.samplerAnisotropy = 16`nd3d9.samplerLodBias = -1.25`nd3d9.clampNegativeLodBias = False`n")
[IO.File]::WriteAllText((Join-Path $O "Display8.ini"), "[Display8]`nDefaults=2`nMode=forward`nBackend=d3d9,vulkan`nVSync=0`n")
foreach ($n in "HUD", "Menus", "GridLayout", "MSAA") { Set-Content (Join-Path $O "$n.asi") "old"; Set-Content (Join-Path $O "$n.ini") "old" }
Copy-Item (G "RefitPlayer.ini") (Join-Path $O "RefitPlayer.ini")
$env:A2_PROGRESS = "1"
$out = Install @($O)
$env:A2_PROGRESS = $null
function OF([string]$n) { Join-Path $O $n }
Check "d3d8_real.dll, d3d9.dll and dxvk.conf retired" (-not (Test-Path (OF "d3d8_real.dll")) -and -not (Test-Path (OF "d3d9.dll")) -and
    -not (Test-Path (OF "dxvk.conf")) -and (Has $out "  retired dxvk.conf") -and (Has $out "  retired d3d9.dll") -and (Has $out "  retired d3d8_real.dll"))
Check "Display8 in, GOG's kept" (((Sha (OF "d3d8.dll")) -eq (Sha (Join-Path $P "layers\display\d3d8.dll"))) -and
    ((Text (OF "d3d8.dll.gog-backup")) -eq "gogd3d8to9"))
Check "the legacy plugins retired" (-not (@(Get-ChildItem -LiteralPath $O | Where-Object { $_.Name -match '^(HUD|Menus|GridLayout|MSAA)\.(asi|ini)$' }).Count))
Check "the d3d records gone from the prerequisites manifest" (-not (Select-String -Quiet -Pattern "^d3d" (OF "armada2-refit-prereqs.txt")))
$ini = @(Get-Content -LiteralPath (OF "Display8.ini"))
Check "Display8.ini: VSync kept, Defaults=3, Mode gone" (($ini -ccontains "VSync=0") -and ($ini -ccontains "Defaults=3") -and -not ($ini -like "Mode=*"))
Check "Display8.ini: Backend without d3d9" (($ini -ccontains "Backend=vulkan") -or ($ini -ccontains "Backend=auto"))
Check "Display8.ini: the edited LodBias, in [Samplers]" (($ini -ccontains "LodBias=-1.25") -and
    ((SectionOf $ini "LodBias=-1.25") -ieq "[Samplers]"))
Check "RefitPlayer.ini byte for byte" ((Sha (OF "RefitPlayer.ini")) -eq $OverlaySha)
Check "the overrides carry no d3d9" (-not (Has $out "*d3d9=*") -and -not (Has $out "*WINEDLLOVERRIDES*d3d9*"))
Check "the display step, the retire step" ((@($out | Where-Object { $_ -like "::step *" } | ForEach-Object { ($_ -split " ")[1] } |
    Select-Object -Skip 2 -First 2) -join " ") -eq "display retire")
$kept = Sha (OF "Display8.ini")
$out = Install @($O)
Check "again: nothing left to change" (((Sha (OF "Display8.ini")) -eq $kept) -and (Has $out "  nothing to take out"))
Write-Host "== ...uninstall from it: GOG's d3d8.dll is back, nothing of the chain left"
Install @("-Uninstall", $O) | Out-Null
Check "GOG's d3d8.dll back" (((Text (OF "d3d8.dll")) -eq "gogd3d8to9") -and -not (Test-Path (OF "d3d8.dll.gog-backup")))
Check "nothing of the chain or Display8 left" (-not (@(Get-ChildItem -LiteralPath $O | Where-Object { $_.Name -match '^(d3d9\.dll|d3d8_real\.dll|Display8\..*|dxvk\.conf)$' }).Count))
Check "RefitPlayer.ini byte for byte after the uninstall" ((Sha (OF "RefitPlayer.ini")) -eq $OverlaySha)

Write-Host "== a 14.x game without Display8: d3d8to9 in the slot, no Display8.ini, a marked dxvk.conf"
$O = Join-Path $T "old2"; Mock $O -NoD3d8
Copy-Item $To9Dll (Join-Path $O "d3d8.dll"); Copy-Item $Dxvk9Dll (Join-Path $O "d3d9.dll")
Copy-Item (G "d3d8.dll.gog-backup") (Join-Path $O "d3d8.dll.gog-backup")
[IO.File]::WriteAllText((Join-Path $O "dxvk.conf"), "# generated by tools/renderer-config.sh`nd3d9.samplerLodBias = -0.75`n")
Install @($O) | Out-Null
Check "Display8 in the slot, the d3d9.dll and dxvk.conf retired" (((Sha (OF "d3d8.dll")) -eq (Sha (Join-Path $P "layers\display\d3d8.dll"))) -and
    -not (Test-Path (OF "d3d9.dll")) -and -not (Test-Path (OF "dxvk.conf")))
$ini = @(Get-Content -LiteralPath (OF "Display8.ini"))
Check "Display8.ini carries the dxvk.conf's LodBias" (($ini -ccontains "LodBias=-0.75") -and ($ini -ccontains "Defaults=3"))

Write-Host "== the slot was empty: Display8 in, nothing kept, uninstall leaves the game as it was"
$N = Join-Path $T "nod3d8"; Mock $N -NoD3d8
$before = Snap $N
$out = Install @($N)
Check "Display8 in, nothing kept, UI.asi with it" (((Sha (Join-Path $N "d3d8.dll")) -eq (Sha (Join-Path $P "layers\display\d3d8.dll"))) -and
    -not (Test-Path (Join-Path $N "d3d8.dll.gog-backup")) -and (Test-Path (Join-Path $N "UI.asi")))
Check "no d3d9.dll, no backup, no d3d8_real.dll" (-not (Test-Path (Join-Path $N "d3d9.dll")) -and -not (Test-Path (Join-Path $N "d3d8.dll.a2chain-backup")) -and
    -not (Test-Path (Join-Path $N "d3d8_real.dll")))
Install @("-Uninstall", $N) | Out-Null
Check "the game as it was" ((Snap $N) -eq $before)
Check "d3d8.dll gone (the system's is the game's)" (-not (Test-Path (Join-Path $N "d3d8.dll")))

Write-Host "== a dxvk.conf of the player's own making stays: no marker, no removal"
$O = Join-Path $T "own"; Mock $O
Set-Content (OF "dxvk.conf") "d3d9.foo = 1"; Set-Content (OF "d3d9.dll") "faked3d8to9"
Install @($O) | Out-Null
Check "dxvk.conf and d3d9.dll stay" (((Text (OF "dxvk.conf")) -eq "d3d9.foo = 1") -and ((Text (OF "d3d9.dll")) -eq "faked3d8to9"))

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
               "armada2-refit-prereqs.txt", "d3d8.dll.a2chain-backup", "d3d8_real.dll", "d3d8.dll.gog-backup", "Display8.ini",
               "Display8.log", "d3d8.dll.a2display-backup") {
    Check "$f removed" (-not (Test-Path (G $f)))
}
Check "changed file left" (((Text (G "UltimateASILoader-license.txt")) -eq "mine") -and (Has $out "*left UltimateASILoader-license.txt*"))
Check "Display8 out, GOG's d3d8.dll back, its backup gone" (((Text (G "d3d8.dll")) -eq "gogd3d8to9") -and
    -not (Test-Path (G "d3d8.dll.gog-backup")))
Check "no d3d9.dll, no d3d8_real.dll" (-not (Test-Path (G "d3d9.dll")) -and -not (Test-Path (G "d3d8_real.dll")))
Check "RefitPlayer.ini left by the uninstall" (OverlayKept)

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
Add-Content (Join-Path $D "layers\ui\UI.ini") "; tampered"
$r = Fail (Join-Path $D "install.ps1") @($M)
Check "damaged package: exit 1" ($r.Code -eq 1)
Check "damaged package: said last" ($r.Last -like "the package is damaged: layers\ui\UI.ini does not match SHA256SUMS*")
Check "damaged package: nothing installed" (-not (Test-Path (Join-Path $M "QOL.asi")))
Check "no ::error lines unasked" (-not (Has $r.Out "::error*"))
$env:A2_PROGRESS = "1"
$r = Fail (Join-Path $D "install.ps1") @($M)
Check "damaged package: ::error damaged layers\ui\UI.ini" (Has $r.Out "::error damaged layers\ui\UI.ini")
$r = Fail (Join-Path $P "install.ps1") @((Join-Path $T "nowhere"))
Check "no game: ::error no-game" (Has $r.Out "::error no-game *nowhere")

Write-Host "== an antivirus holding package files open: every one named, as an antivirus"
$A = Join-Path $T "locked"; Copy-Item -Recurse $P $A
$held = foreach ($n in "qol\QOL.asi", "ui\UI.asi") { [IO.File]::Open((Join-Path $A "layers\$n"), "Open", "Read", "None") }
try {
    $r = Fail (Join-Path $A "install.ps1") @($M)
} finally { foreach ($h in $held) { $h.Dispose() } }
Check "locked: exit 1" ($r.Code -eq 1)
Check "locked: ::error av-locked for each" ((Has $r.Out "::error av-locked layers\ui\UI.asi") -and (Has $r.Out "::error av-locked layers\qol\QOL.asi"))
Check "locked: both named, as an antivirus, last" (($r.Last -like "*layers\qol\QOL.asi, layers\ui\UI.asi from the package could not be read. An antivirus*") -and
    ($r.Last -like "*exclusions for *locked and *roundtrip*"))
Check "locked: nothing installed" (-not (Test-Path (Join-Path $M "QOL.asi")))

Write-Host "== an antivirus quarantined a package file: named, as an antivirus"
Remove-Item -LiteralPath (Join-Path $A "layers\online\Online.asi")
$r = Fail (Join-Path $A "install.ps1") @($M)
Check "missing: exit 1" ($r.Code -eq 1)
Check "missing: ::error av-missing" (Has $r.Out "::error av-missing layers\online\Online.asi")
Check "missing: named, as an antivirus, last" ($r.Last -like "layers\online\Online.asi from the package is missing. An antivirus has most likely quarantined*")

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
Check "GOG's d3d8.dll back" ((Text (O "d3d8.dll")) -eq "gogd3d8to9")
$env:A2_PYTHON = $null
Remove-Item -Recurse -Force $T
Write-Host "all passed"
