# Frame-time runs of the Owl fight: replays the same raw input through the live input path with
# PLAY_LOG_PERF=1 and keeps one CSV per run in cache/perf/<tag>-<run>.csv.
# Runs: passive (the Owl stands, Wolf walks up and mashes attack), ai (the same against his real
# AI) and control (the Owl moved 1000 m away, Wolf attacks the air), each cold (first run) and
# warm (second run). Summarise with: uv run tools/perf/summary.py cache/perf/<tag>-*.csv
# Example: powershell -NoProfile -File tools/perf/owl-replays.ps1 -Exe target/release/sekiro-rs.exe -Tag after
param(
    [string]$Exe = 'target/release/sekiro-rs.exe',
    [string]$Tag = 'run',
    [string[]]$Only = @()
)
$ErrorActionPreference = 'Stop'
Set-Location (Split-Path -Parent (Split-Path -Parent $PSScriptRoot))
New-Item -ItemType Directory -Force cache/perf | Out-Null

# Walk (or wait) 300 frames, lock on, then 72 left clicks five frames apart.
function Write-Replay([string]$path, [string]$walk) {
    $lines = @('20 -', '1 Click', '10 -', "300 $walk", '1 Q', '20 -')
    foreach ($i in 1..72) { $lines += '1 Click'; $lines += '4 -' }
    $lines += '30 -'
    Set-Content -Path $path -Value $lines -Encoding ascii
}
Write-Replay 'cache/perf/contact.input' 'W'
Write-Replay 'cache/perf/control.input' '-'

$owl = @('--arena', 'owl', '--god')
$far = @('--spawn', '120.337,54.04422,234.226,121.8,0', '--enemy-distance', '1000')
$runs = [ordered]@{
    passive_cold = $owl + @('--passive', '--input', 'cache/perf/contact.input')
    passive_warm = $owl + @('--passive', '--input', 'cache/perf/contact.input')
    ai_cold      = $owl + @('--input', 'cache/perf/contact.input')
    ai_warm      = $owl + @('--input', 'cache/perf/contact.input')
    control_cold = $owl + @('--passive') + $far + @('--input', 'cache/perf/control.input')
    control_warm = $owl + @('--passive') + $far + @('--input', 'cache/perf/control.input')
}
$env:PLAY_LOG_PERF = '1'
foreach ($name in $runs.Keys) {
    if ($Only.Count -gt 0 -and $name -notin $Only) { continue }
    $before = @(Get-ChildItem cache/perf/*.csv | ForEach-Object Name)
    $a = $runs[$name] + @('--exit')
    & $Exe @a 1> "cache/perf/$Tag-$name.stdout.log" 2> "cache/perf/$Tag-$name.stderr.log"
    $new = Get-ChildItem cache/perf/*.csv | Where-Object { $_.Name -notin $before }
    if ($new) { Move-Item $new.FullName "cache/perf/$Tag-$name.csv" -Force }
    Write-Output "$Tag-${name}: exit $LASTEXITCODE"
}
