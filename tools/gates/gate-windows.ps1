<#
.SYNOPSIS
  Gate W: does the refit game run on native Windows, on Direct3D 12 and on Vulkan?

.DESCRIPTION
  A test tool, as the test bench is -- not an installer: it never installs, and the
  Refit screen stays the only writer of the player's settings in the game. It sets the
  backend for a run through RefitPlayer.ini (backing the file up first, and putting it
  back at the end, whatever happens), starts the game, and reads the game's logs.

  Five steps, one `GATE <name>: PASS|FAIL|SKIP <why>` line per check:
    1. probe-d3d12      d12probe.exe: Display8's Direct3D 12 passes, on this machine
    2. probe-vulkan     vkprobe.exe: Vulkan for Display8 (SKIP when there is no driver)
    3. the game on Direct3D 12   game-backend, game-mission, passthrough, device-removed, timing
    4. the game on Vulkan        the same, when probe-vulkan passed
    5. the zip          the logs, for the agents

  d12probe.exe and vkprobe.exe are looked for beside this script.

.PARAMETER Game
  The game directory (beside Armada2.exe).
.PARAMETER Out
  Where the zip goes (default: the folder of this script).
.PARAMETER ProbeArgs
  Extra arguments for d12probe.exe (CI passes --warp).
.PARAMETER SkipGame
  Run the probes only.
.PARAMETER WhatIf
  Say what would be written, started and zipped, and do none of it. The probes still
  run: they read nothing and write only their own report.
#>
[CmdletBinding(SupportsShouldProcess = $true)]
param(
    [Parameter(Mandatory = $true)][string]$Game,
    [string]$Out = $PSScriptRoot,
    [string]$ProbeArgs = '',
    [switch]$SkipGame
)
$ErrorActionPreference = 'Stop'
$results = [ordered]@{}
$failed = 0

function Gate($name, $state, $why) {
    $results[$name] = "$state $why".Trim()
    if ($state -eq 'FAIL') { $script:failed++ }
    Write-Host "GATE ${name}: $state $why".TrimEnd()
}

function Run-Probe($exe, $report, $probeArgs) {
    # A GUI program: -Wait for it, then read what it wrote beside itself.
    $path = Join-Path $PSScriptRoot $exe
    if (-not (Test-Path $path)) { return @{ ok = $false; text = "$exe is not beside this script" } }
    $txt = Join-Path $PSScriptRoot $report
    Remove-Item $txt -ErrorAction SilentlyContinue
    $argList = @($probeArgs | Where-Object { $_ })
    $p = if ($argList.Count) { Start-Process $path -ArgumentList $argList -PassThru -WorkingDirectory $PSScriptRoot }
         else { Start-Process $path -PassThru -WorkingDirectory $PSScriptRoot }
    if (-not $p.WaitForExit(120000)) { $p.Kill(); return @{ ok = $false; text = 'did not finish in 120 s' } }
    $text = if (Test-Path $txt) { (Get-Content $txt -TotalCount 1) } else { "no $report" }
    return @{ ok = ($p.ExitCode -eq 0); text = $text }
}

# -- 1, 2: the probes -----------------------------------------------------------------
$d12 = Run-Probe 'd12probe.exe' 'd12probe.txt' ($ProbeArgs -split '\s+')
Gate 'probe-d3d12' $(if ($d12.ok) { 'PASS' } else { 'FAIL' }) $d12.text
$vk = Run-Probe 'vkprobe.exe' 'vkprobe.txt' @()
$vkDriver = $vk.ok
if ($vk.ok) { Gate 'probe-vulkan' 'PASS' $vk.text }
elseif ((Test-Path (Join-Path $PSScriptRoot 'vkprobe.txt')) -and ((Get-Content (Join-Path $PSScriptRoot 'vkprobe.txt') -Raw) -match 'loader|vulkan-1|no Vulkan|ICD|driver')) {
    Gate 'probe-vulkan' 'SKIP' "no usable Vulkan driver here ($($vk.text))"
}
else { Gate 'probe-vulkan' 'FAIL' $vk.text }

# -- 3, 4: the game -------------------------------------------------------------------
$logs = Join-Path $env:LOCALAPPDATA 'armada2-refit\logs'
$ini = Join-Path $Game 'RefitPlayer.ini'
$exe = Join-Path $Game 'Armada2.exe'

# Sets [Display8.Display8] Backend=<value> in the overlay, keeping every other line.
function Set-Backend($value) {
    $lines = if (Test-Path $ini) { [System.Collections.Generic.List[string]](Get-Content $ini) } else { New-Object 'System.Collections.Generic.List[string]' }
    $in = $false; $done = $false
    for ($i = 0; $i -lt $lines.Count; $i++) {
        if ($lines[$i] -match '^\s*\[(.+)\]\s*$') {
            if ($in -and -not $done) { $lines.Insert($i, "Backend=$value"); $done = $true; break }
            $in = ($Matches[1] -eq 'Display8.Display8')
        }
        elseif ($in -and $lines[$i] -match '^\s*Backend\s*=') { $lines[$i] = "Backend=$value"; $done = $true; break }
    }
    if (-not $done) {
        if (-not $in) { $lines.Add('[Display8.Display8]') }
        $lines.Add("Backend=$value")
    }
    Set-Content -Path $ini -Value $lines -Encoding ASCII
}

function Read-Log($name) {
    $p = Join-Path $logs $name
    if (Test-Path $p) { Get-Content $p } else { @() }
}

function Check-Run($label, $backend, $want) {
    $log = @(Read-Log 'Display8.log')
    if ($log.Count -eq 0) { Gate "game-backend ($label)" 'FAIL' 'no Display8.log: the game did not start, or Display8 is not installed'; return }
    if ($log -match $want) { Gate "game-backend ($label)" 'PASS' ($log -match $want | Select-Object -First 1) }
    else { Gate "game-backend ($label)" 'FAIL' "Display8.log has no line matching '$want'" }
    if ($log -match 'frame \d+ presented') { Gate "game-mission ($label)" 'PASS' 'frames were presented' }
    else { Gate "game-mission ($label)" 'FAIL' 'no frame was presented (the match was not reached?)' }
    if ($backend -eq 'd3d12') {
        if ($log -match 'passthrough: on Direct3D 12') { Gate "passthrough ($label)" 'PASS' 'the engine draws went through Display8' }
        else { Gate "passthrough ($label)" 'FAIL' 'no "passthrough: on Direct3D 12" line' }
    }
    else {
        if ($log -match 'passthrough: on Vulkan' -or $log -match 'passthrough: ') { Gate "passthrough ($label)" 'PASS' 'the engine draws went through Display8' }
        else { Gate "passthrough ($label)" 'FAIL' 'no passthrough line' }
    }
    $bad = $log | Where-Object { $_ -match 'device is gone|device removed|DEVICE_LOST|Vulkan only' }
    if ($bad) { Gate "device-removed ($label)" 'FAIL' ($bad | Select-Object -First 1) }
    else { Gate "device-removed ($label)" 'PASS' 'no lost device' }
    $t = @($log | Where-Object { $_ -match '(?i)timing|frame time|ms/frame' })
    Gate "timing ($label)" 'INFO' $(if ($t.Count) { ($t | Select-Object -Last 1) } else { 'not reported (Timing=0)' })
}

function Play($label, $backend, $want) {
    Write-Host ''
    Write-Host "== The game on $label ==" -ForegroundColor Cyan
    Write-Host '  main menu, then Options, then Refit; an Instant Action match for 2 minutes;'
    Write-Host '  Refit, Graphics, Ships off (stock ships), then on; then quit the game.'
    if ($PSCmdlet.ShouldProcess($ini, "set Backend=$backend and start $exe")) {
        Set-Backend $backend
        Remove-Item (Join-Path $logs '*.log') -ErrorAction SilentlyContinue
        $p = Start-Process $exe -WorkingDirectory $Game -PassThru
        $p.WaitForExit()
        Check-Run $label $backend $want
        # Keep this run's logs apart from the next one's.
        $keep = Join-Path $script:work "logs-$backend"
        New-Item -ItemType Directory -Force $keep | Out-Null
        Copy-Item (Join-Path $logs '*') $keep -ErrorAction SilentlyContinue
    }
}

$work = Join-Path ([System.IO.Path]::GetTempPath()) ("a2gates-" + [guid]::NewGuid().ToString('N').Substring(0, 8))
New-Item -ItemType Directory -Force $work | Out-Null
if (-not $SkipGame) {
    if (-not (Test-Path $exe)) { Gate 'game' 'FAIL' "no Armada2.exe in $Game"; $SkipGame = $true }
}
if (-not $SkipGame) {
    $backup = Join-Path $work 'RefitPlayer.ini.backup'
    $had = Test-Path $ini
    if ($had -and $PSCmdlet.ShouldProcess($ini, 'back up')) { Copy-Item $ini $backup }
    try {
        Play 'Direct3D 12' 'd3d12' 'the backend is Direct3D 12'
        if ($vkDriver) { Play 'Vulkan' 'vulkan' 'the backend is Vulkan' }
        else { Gate 'game on Vulkan' 'SKIP' 'probe-vulkan did not pass' }
    }
    finally {
        if ($PSCmdlet.ShouldProcess($ini, 'restore')) {
            if ($had) { Copy-Item $backup $ini -Force } else { Remove-Item $ini -ErrorAction SilentlyContinue }
        }
    }
}

# -- 5: the zip -----------------------------------------------------------------------
$zip = Join-Path $Out ("a2gates-" + (Get-Date -Format 'yyyyMMdd-HHmmss') + ".zip")
$summary = Join-Path $work 'GATES.result.txt'
$results.GetEnumerator() | ForEach-Object { "GATE $($_.Key): $($_.Value)" } | Set-Content $summary
foreach ($f in 'd12probe.txt', 'vkprobe.txt') { if (Test-Path (Join-Path $PSScriptRoot $f)) { Copy-Item (Join-Path $PSScriptRoot $f) $work } }
if (Test-Path $logs) { New-Item -ItemType Directory -Force (Join-Path $work 'logs-last') | Out-Null; Copy-Item (Join-Path $logs '*') (Join-Path $work 'logs-last') -ErrorAction SilentlyContinue }
if ($PSCmdlet.ShouldProcess($zip, 'write the zip of reports and logs')) {
    Compress-Archive -Path (Join-Path $work '*') -DestinationPath $zip -Force
    Gate 'zip' 'PASS' $zip
}
else { Gate 'zip' 'SKIP' "would write $zip" }
Remove-Item $work -Recurse -Force -ErrorAction SilentlyContinue
if ($failed) { Write-Host "$failed check(s) failed." -ForegroundColor Red; exit 1 }
Write-Host 'No check failed.' -ForegroundColor Green
