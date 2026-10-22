# The frozen Windows installer against a mock game.   test-windows.ps1 <zip> <exe>
#
# test.sh's twin for armada2-refit-installer-windows.exe, the file that ships: its command
# line (the same Job the window runs) with LOCALAPPDATA, APPDATA and USERPROFILE in a
# scratch folder, a mock GOG game of placeholder files, and the release zip in the
# installer's cache. The exe is windowed, so it is started with its standard handles
# redirected to files (Python then has a stdout, as it does under a pipe); a hung run
# -- say, a traceback dialog -- is killed after a timeout. Proves the version resource,
# the frozen program standing in for Python (A2_PYTHON), install.ps1 run with progress
# and the cutscene skip, the state file, and uninstall leaving the game as it was -- not
# that anything loads. CI runs it on a Windows runner after building the exe.
param([Parameter(Mandatory)][string]$Zip, [Parameter(Mandatory)][string]$Exe)
$ErrorActionPreference = "Stop"
$Zip = (Resolve-Path -LiteralPath $Zip).Path
$Exe = (Resolve-Path -LiteralPath $Exe).Path
$T = Join-Path ([IO.Path]::GetTempPath()) ([Guid]::NewGuid())
New-Item -ItemType Directory $T | Out-Null
foreach ($v in "A2_GAME", "A2_GAME_DIR", "A2_DIR", "A2_CONF", "A2_PYTHON", "A2_SKIP", "A2_PROGRESS") {
    Remove-Item -LiteralPath "env:$v" -ErrorAction SilentlyContinue
}
$env:LOCALAPPDATA = Join-Path $T "Local"
$env:APPDATA = Join-Path $T "Roaming"
$env:USERPROFILE = Join-Path $T "Home"
foreach ($d in $env:LOCALAPPDATA, $env:APPDATA, $env:USERPROFILE) { New-Item -ItemType Directory $d | Out-Null }

function Check([string]$what, $ok) {   # $ok untyped: a [bool] parameter refuses strings
    if ($ok) { Write-Host "ok    $what" } else { throw "FAIL  $what" }
}
function Sha([string]$f) { (Get-FileHash -Algorithm SHA256 -LiteralPath $f).Hash.ToLower() }
function Text([string]$f) { (Get-Content -LiteralPath $f -Raw).Trim() }
function Has($lines, [string]$pattern) { [bool]($lines | Where-Object { $_ -like $pattern }) }
# Every file and folder under a directory, with each file's hash.
function Snap([string]$d) {
    (Get-ChildItem -LiteralPath $d -Recurse -Force | ForEach-Object {
        $rel = $_.FullName.Substring($d.Length)
        if ($_.PSIsContainer) { "$rel\" } else { "$rel $(Sha $_.FullName)" }
    } | Sort-Object) -join "`n"
}
# Run the exe; its exit code and what it wrote to stdout and stderr (UTF-8: it runs in
# Python's UTF-8 mode).
function Run([string[]]$a, [int]$seconds = 600) {
    $o = Join-Path $T "stdout.txt"; $e = Join-Path $T "stderr.txt"
    $line = ($a | ForEach-Object { if ($_ -match '[\s"]') { '"' + ($_ -replace '"', '\"') + '"' } else { $_ } }) -join " "
    $p = Start-Process -FilePath $Exe -ArgumentList $line -NoNewWindow -PassThru `
                       -RedirectStandardOutput $o -RedirectStandardError $e
    $null = $p.Handle   # without it, ExitCode reads empty once the process is gone
    if (-not $p.WaitForExit($seconds * 1000)) {
        Stop-Process -Id $p.Id -Force -ErrorAction SilentlyContinue
        throw "$a did not finish in $seconds s"
    }
    $r = @{ Code = $p.ExitCode; Out = @(Get-Content -Encoding UTF8 -LiteralPath $o)
            Err = @(Get-Content -Encoding UTF8 -LiteralPath $e) }
    if ($r.Code -ne 0) { $r.Out + $r.Err | Select-Object -Last 30 | Out-Host }
    return $r
}

Write-Host "== the exe: its version resource, --check-version, the self-test"
$pkg = Join-Path $T "pkg"
Expand-Archive -LiteralPath $Zip -DestinationPath $pkg
$manifest = Get-Content -Raw -LiteralPath (Get-ChildItem -LiteralPath $pkg -Recurse -Filter manifest.json | Select-Object -First 1).FullName | ConvertFrom-Json
$ver = $manifest.installer.version
$info = (Get-Item -LiteralPath $Exe).VersionInfo
Check "version resource $($info.ProductVersion) is the package's installer $ver" (($info.ProductVersion -eq $ver) -and ($info.FileVersion -eq $ver))
$r = Run @("--check-version", $ver)
Check "--check-version prints $ver through redirected stdout" (($r.Code -eq 0) -and (($r.Out -join "") -eq $ver))
$r = Run @("--selftest")
Check "--selftest" (($r.Code -eq 0) -and (Has $r.Out "ok    *") -and -not (Has $r.Out "FAIL*"))

Write-Host "== the exe stands in for Python (A2_PYTHON)"
$probe = Join-Path $T "probe"
New-Item -ItemType Directory $probe | Out-Null
Set-Content -LiteralPath (Join-Path $probe "helper.py") -Value "X = 'helper-ok'"
Set-Content -LiteralPath (Join-Path $probe "probe.py") -Value @(
    'import argparse, glob, hashlib, math, os, re, shutil, struct, sys',
    'import helper   # beside the script, as the SOD scripts import a2env',
    'ap = argparse.ArgumentParser(); ap.add_argument("out"); ap.add_argument("--code", type=int, default=0)',
    'a = ap.parse_args()',
    'with open(a.out, "w") as f:',
    '    f.write("%s %d %.4f %s utf8=%d\n" % (helper.X, struct.calcsize("<If"), math.pi,',
    '            hashlib.sha256(b"x").hexdigest()[:8], sys.flags.utf8_mode))',
    'print("probe ran")',
    'sys.exit(a.code)')
$pout = Join-Path $probe "out.txt"
$r = Run @((Join-Path $probe "probe.py"), $pout)
Check "<exe> probe.py runs it, with the standard library and its own folder on the path" (($r.Code -eq 0) -and
    ((Text $pout) -eq "helper-ok 8 3.1416 2d711642 utf8=1") -and (Has $r.Out "probe ran"))
$r = Run @((Join-Path $probe "probe.py"), $pout, "--code", "3")
Check "... and returns the script's exit code" ($r.Code -eq 3)

Write-Host "== install from the package"
$G = Join-Path $T "GOG Games\Star Trek Armada II"
New-Item -ItemType Directory $G | Out-Null
function G([string]$n) { Join-Path $G $n }
Set-Content (G "Armada2.exe") "exe"
Set-Content (G "binkw32.dll") "stockbink"
Set-Content (G "d3d8.dll") "gogd3d8to9"     # none of GOG's, DXVK's or d3d8to9: left as it is
$before = Snap $G
$cache = Join-Path $env:LOCALAPPDATA "armada2-refit\installer\cache\releases"
New-Item -ItemType Directory $cache | Out-Null
Copy-Item -LiteralPath $Zip -Destination $cache
$pver = $manifest.version
$r = Run @("--list")
Check "--list sees release $pver in the cache" (($r.Code -eq 0) -and (Has $r.Out "  $pver *"))
$r = Run @("--package", $Zip, "--game", $G)
Check "--package exits 0" ($r.Code -eq 0)
foreach ($n in "HUD.asi", "Menus.asi", "QOL.asi", "Online.asi", "winmm.dll") {
    Check $n (Test-Path -LiteralPath (G $n))
}
Check "the cutscene player left out (A2_SKIP=cutscenes)" (((Text (G "binkw32.dll")) -eq "stockbink") -and
    -not (Test-Path -LiteralPath (G "binkw32_orig.dll")) -and -not (Test-Path -LiteralPath (G "BinkProxy.ini")))
Check "progress from install.ps1's ::step lines" ((Has $r.Err "*Installing the HUD*") -and (Has $r.Err "*Installing online play*"))
Check "the SOD scripts ran under the exe without a traceback" (-not (Has ($r.Out + $r.Err) "*Traceback*") -and
    -not (Has ($r.Out + $r.Err) "*ModuleNotFoundError*"))
$state = Get-Content -Raw -LiteralPath (G "armada2-refit-installed.json") | ConvertFrom-Json
Check "state file: version $pver, installer $ver" (($state.version -eq $pver) -and ($state.installer -eq $ver))
Check "no launch settings on Windows" (-not $state.heroic -and -not $state.prefix)

Write-Host "== install again while an antivirus holds a file of the first unpack: it goes beside it"
$unpacked = Join-Path $env:LOCALAPPDATA "armada2-refit\installer\cache\unpacked"
$first = Join-Path $unpacked "$pver\armada2-refit-$pver\game\QOL.asi"
Check "the first unpack is there" (Test-Path -LiteralPath $first)
$hold = [IO.File]::Open($first, "Open", "Read", "Read")   # no delete sharing: rmtree cannot take it
try {
    $r = Run @("--package", $Zip, "--game", $G)
} finally { $hold.Dispose() }
Check "--package exits 0 with the old unpack held" ($r.Code -eq 0)
Check "unpacked beside it, into $pver-1" (Test-Path -LiteralPath (Join-Path $unpacked "$pver-1\armada2-refit-$pver\game\QOL.asi"))
Check "installed" ((Sha (G "QOL.asi")) -eq (Sha (Join-Path $pkg "armada2-refit-$pver\game\QOL.asi")))

Write-Host "== a package an antivirus took a plugin from: named, as an antivirus, with ::error"
# The zip without game/QOL.asi, written by the exe as Python (Compress-Archive in
# PowerShell 5.1 can write backslashes into the names).
$badzip = Join-Path $T "armada2-refit-$pver.zip"
Set-Content -LiteralPath (Join-Path $probe "strip.py") -Value @(
    'import sys, zipfile',
    'with zipfile.ZipFile(sys.argv[1]) as a, zipfile.ZipFile(sys.argv[2], "w") as b:',
    '    for i in a.infolist():',
    '        if not i.filename.endswith("/game/QOL.asi"):',
    '            b.writestr(i, a.read(i))')
$r = Run @((Join-Path $probe "strip.py"), $Zip, $badzip)
Check "a zip without QOL.asi" ($r.Code -eq 0)
$r = Run @("--package", $badzip, "--game", $G)
Check "--package exits 1" ($r.Code -eq 1)
Check "the window's words: the file, the antivirus, the README" ((Has $r.Err "error: game\QOL.asi is missing from the package: an antivirus*") -and
    (Has $r.Err "*More: https://github.com/cedrichaase/armada2-refit#antivirus-false-positives"))
Check "the script's ::error line in the log" (Has $r.Out "::error av-missing game\QOL.asi")

Write-Host "== uninstall: the game as it was"
$r = Run @("--uninstall", "--game", $G)
Check "--uninstall exits 0" ($r.Code -eq 0)
Check "the game as it was" ((Snap $G) -eq $before)
Check "the spare unpack swept" (-not (Test-Path -LiteralPath (Join-Path $unpacked "$pver-1")))
Remove-Item -LiteralPath $T -Recurse -Force -ErrorAction SilentlyContinue
Write-Host "all passed"
