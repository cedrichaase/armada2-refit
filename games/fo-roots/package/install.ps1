<#
Armada II Refit for Fleet Operations: Roots -- install into Roots on Windows.

  install.bat [<Roots folder>]               install
  install.bat -Uninstall [<Roots folder>]    take it all out again

The Roots folder is the one holding Data\ArmadaL.exe (its Data\ folder works too): the
argument, else the folder this package was unzipped into (Roots or its Data\), else an
entry for it in Programs and Features, else it asks with a folder picker.
Everything goes into Data\. A d3d8.dll (DxWrapper's) or winmm.dll there that is not ours is
kept as d3d8.dll.a2display-backup / winmm.dll.a2asi-backup and put back by -Uninstall.
An .ini already there is kept (the player's settings); delete it to get the shipped one.
PowerShell 5.1 or later; install.bat runs it.
#>
param(
    [switch]$Uninstall,
    [Parameter(ValueFromRemainingArguments = $true)][string[]]$Rest = @()
)
$ErrorActionPreference = "Stop"
trap { Write-Host ""; Write-Host "ERROR: $($_.Exception.Message)"; exit 1 }

$pkg = $PSScriptRoot
$dir = (($Rest | Where-Object { $_ -and $_ -ne "-Uninstall" }) -join " ").Trim().Trim('"')
if ($Rest -contains "-Uninstall") { $Uninstall = $true }

function Test-Roots([string]$d) { $d -and (Test-Path -LiteralPath (Join-Path $d "Data\ArmadaL.exe")) }
if ($dir -and (Test-Path -LiteralPath (Join-Path $dir "ArmadaL.exe"))) { $dir = Split-Path -Parent $dir }
if (-not $dir) {
    $up = Split-Path -Parent $pkg
    if (Test-Roots $up) { $dir = $up }
    elseif (Test-Path -LiteralPath (Join-Path $up "ArmadaL.exe")) { $dir = Split-Path -Parent $up }
}
# Programs and Features: an entry for Fleet Operations or Roots whose folder holds the game.
if (-not $dir) {
    $keys = "HKLM:\SOFTWARE\Microsoft\Windows\CurrentVersion\Uninstall\*",
            "HKLM:\SOFTWARE\WOW6432Node\Microsoft\Windows\CurrentVersion\Uninstall\*",
            "HKCU:\SOFTWARE\Microsoft\Windows\CurrentVersion\Uninstall\*"
    foreach ($p in Get-ItemProperty -Path $keys -ErrorAction SilentlyContinue) {
        if ($p.DisplayName -match "Fleet Op|Roots" -and $p.InstallLocation) {
            $d = $p.InstallLocation.Trim('"')
            if (Test-Roots $d) { $dir = $d; break }
            if (Test-Path -LiteralPath (Join-Path $d "ArmadaL.exe")) { $dir = Split-Path -Parent $d; break }
        }
    }
}
if (-not $dir) {
    Write-Host "Choose the Roots folder (the one with Data\ArmadaL.exe)."
    try {
        Add-Type -AssemblyName System.Windows.Forms
        $pick = New-Object System.Windows.Forms.FolderBrowserDialog
        $pick.Description = "The Fleet Operations: Roots folder (the one with Data\ArmadaL.exe)"
        if ($pick.ShowDialog() -eq [System.Windows.Forms.DialogResult]::OK) { $dir = $pick.SelectedPath }
    } catch { }
}
if (-not $dir) { $dir = (Read-Host "Roots folder (the one with Data\ArmadaL.exe)").Trim('"') }
if ($dir -and (Test-Path -LiteralPath (Join-Path $dir "ArmadaL.exe"))) { $dir = Split-Path -Parent $dir }
if (-not (Test-Roots $dir)) { throw "no Data\ArmadaL.exe in '$dir' -- pass the Roots folder as an argument" }
$data = (Resolve-Path -LiteralPath (Join-Path $dir "Data")).Path
if (Get-Process -Name ArmadaL -ErrorAction SilentlyContinue) { throw "ArmadaL.exe is running -- quit the game first" }

$loaderSha = "BABA99929487B005BB9B168ACFD852550055F22E5F1059C9032765209BB185E5"
$rootsSha  = "4300AB71731E323F5767D8EE05457E6444401B823253AEBEE86C5AC23C2B00DE"
function Sha([string]$f) { (Get-FileHash -Algorithm SHA256 -LiteralPath $f).Hash }
function Test-Display8([string]$f) {
    (Test-Path -LiteralPath $f) -and
        [Text.Encoding]::ASCII.GetString([IO.File]::ReadAllBytes($f)).Contains("Display8 stopped the game")
}
function Test-Loader([string]$f) { (Test-Path -LiteralPath $f) -and (Sha $f) -eq $loaderSha }

# The package's files other than the two DLLs, relative to Data\.
$src = Join-Path $pkg "Data"
$files = Get-ChildItem -LiteralPath $src -Recurse -File |
    Where-Object { $_.Name -ne "d3d8.dll" -and $_.Name -ne "winmm.dll" } |
    ForEach-Object { $_.FullName.Substring($src.Length + 1) }
$logs = "Display8.log", "Render3D.log", "UI.log", "QOL.log"

function Restore([string]$slot, [string]$backup) {
    Remove-Item -LiteralPath $slot -Force
    if (Test-Path -LiteralPath $backup) { Move-Item -LiteralPath $backup -Destination $slot }
}

if ($Uninstall) {
    $d3d8 = Join-Path $data "d3d8.dll"
    $winmm = Join-Path $data "winmm.dll"
    if (Test-Display8 $d3d8) { Restore $d3d8 "$d3d8.a2display-backup" }
    if (Test-Loader $winmm) { Restore $winmm "$winmm.a2asi-backup" }
    foreach ($f in @($files) + $logs) {
        $p = Join-Path $data $f
        if (Test-Path -LiteralPath $p) { Remove-Item -LiteralPath $p -Force }
    }
    foreach ($d in "Surfaces", "Nebulae") {
        $p = Join-Path $data $d
        if ((Test-Path -LiteralPath $p) -and -not (Get-ChildItem -LiteralPath $p -Force)) { Remove-Item -LiteralPath $p }
    }
    Write-Host "Refit taken out of $data; DxWrapper's d3d8.dll is back."
    exit 0
}

# The package itself first: a file an antivirus took or a broken download stops here.
foreach ($line in Get-Content -LiteralPath (Join-Path $pkg "SHA256SUMS")) {
    $hash, $name = $line -split "\s+\*?", 2
    $p = Join-Path $pkg ($name -replace "/", "\")
    if (-not (Test-Path -LiteralPath $p) -or (Sha $p) -ne $hash.ToUpper()) {
        throw "the package is damaged or incomplete: $name (an antivirus may have taken it; unzip again)"
    }
}
if ((Sha (Join-Path $data "ArmadaL.exe")) -ne $rootsSha) {
    Write-Host "WARNING: this ArmadaL.exe is not Roots 5.0Z11's. The plugins check the build and"
    Write-Host "  stand down where they cannot verify it, so parts (or all) of Refit may not run."
}

# A DLL into its slot, keeping one that is not ours as the backup.
function Put([string]$name, [string]$suffix, [scriptblock]$ours) {
    $slot = Join-Path $data $name
    $backup = "$slot$suffix"
    if ((Test-Path -LiteralPath $slot) -and -not (& $ours $slot)) {
        if (Test-Path -LiteralPath $backup) {
            throw "$slot is not ours and $backup already holds another one; move one of them away first"
        }
        Copy-Item -LiteralPath $slot -Destination $backup
    }
    Copy-Item -LiteralPath (Join-Path $src $name) -Destination $slot -Force
}
Put "d3d8.dll" ".a2display-backup" { param($f) Test-Display8 $f }
Put "winmm.dll" ".a2asi-backup" { param($f) Test-Loader $f }

$kept = @()
foreach ($f in $files) {
    $to = Join-Path $data $f
    New-Item -ItemType Directory -Force -Path (Split-Path -Parent $to) | Out-Null
    if ($f -like "*.ini" -and -not $f.Contains("\") -and (Test-Path -LiteralPath $to)) { $kept += $f; continue }
    Copy-Item -LiteralPath (Join-Path $src $f) -Destination $to -Force
}
foreach ($l in $logs) {
    $p = Join-Path $data $l
    if (Test-Path -LiteralPath $p) { Remove-Item -LiteralPath $p -Force }
}

Write-Host "Refit installed into $data ($((Get-Content -LiteralPath (Join-Path $pkg 'version.txt')).Trim()))."
if ($kept) { Write-Host "Kept your $($kept -join ', ') (delete one and re-run to get the shipped file)." }
Write-Host "Logs: $env:LOCALAPPDATA\armada2-refit\logs\"
