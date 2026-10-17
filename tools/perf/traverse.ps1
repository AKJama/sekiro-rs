param(
    [string]$Exe = 'target/agent/release/sekiro-rs.exe',
    [string]$Name = 'traversal',
    [switch]$WalkOnly
)
$ErrorActionPreference = 'Stop'
$folder = Join-Path 'target/perf' $Name
New-Item -ItemType Directory -Force $folder | Out-Null
$encoding = New-Object System.Text.UTF8Encoding($false)
$script = Join-Path $folder 'walk.txt'
# Wait five seconds, then walk, including the LOD boundaries around each stop.
[IO.File]::WriteAllText($script, "300 0 0 -`n1140 0 0.6 -`n", $encoding)
$tour = Join-Path $folder 'tour.txt'
$rows = @()
if (-not $WalkOnly) {
    $outskirts = Get-Content cache/maps/m11_00_00_00/layout.json -Raw | ConvertFrom-Json
    $castle = Get-Content cache/maps/m11_01_00_00/layout.json -Raw | ConvertFrom-Json
    $stops = @($outskirts.players[4], $outskirts.players[5], $outskirts.players[6], $castle.players[0])
    $step = 480
    foreach ($stop in $stops) {
        $pose = @($stop.translation) + @($stop.yaw_degrees)
        $rows += "$step $($pose -join ' ')"
        $step += 240
    }
}
[IO.File]::WriteAllText($tour, ($rows -join "`n") + "`n", $encoding)
$limit = (Get-Date).AddMinutes(15)
while ($true) {
    $running = @(Get-Process sekiro-rs -ErrorAction SilentlyContinue)
    if ($running.Count -eq 0) { break }
    $running | Select-Object Id,Path
    if ((Get-Date) -ge $limit) { throw 'Game still running after 15 minutes; no launch.' }
    Write-Output 'Waiting 60 seconds for the existing game to exit.'
    Start-Sleep -Seconds 60
}
$saveRoot = Join-Path $env:APPDATA 'Sekiro'
if (Test-Path -LiteralPath $saveRoot) {
    Copy-Item -LiteralPath $saveRoot -Destination (Join-Path $folder ('saves-' + (Get-Date -Format yyyyMMdd-HHmmss))) -Recurse
}
$env:PLAY_LOG_PERF = '1'
$env:PLAY_PERF_DIR = $folder
$arguments = @('--play', '--map', 'm11_00_00_00', '--start', '3', '--no-enemy', '--brain-memory', 'off', '--uncapped', '--profile-systems', '--script', $script, '--exit')
if (-not $WalkOnly) { $arguments += @('--perf-tour', $tour) }
$ErrorActionPreference = 'Continue'
& $Exe @arguments *> (Join-Path $folder 'run.log')
$result = $LASTEXITCODE
Write-Output "Traversal exit code: $result"
exit $result
