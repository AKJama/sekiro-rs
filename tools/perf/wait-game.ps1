# Renamed benchmark binaries under target must also block a new game window.
function Wait-QuietGame {
    $targetRoot = [IO.Path]::GetFullPath((Join-Path $PSScriptRoot '../../target')) + '\'
    $limit = (Get-Date).AddMinutes(15)
    while ($true) {
        $running = @(Get-Process | Where-Object {
            if ($_.ProcessName -in @('sekiro-rs', 'sekiro')) { return $true }
            if (-not $_.Path) { return $false }
            if ($_.Path.StartsWith($targetRoot, [StringComparison]::OrdinalIgnoreCase)) { return $true }
            $version = [Diagnostics.FileVersionInfo]::GetVersionInfo($_.Path)
            $version.ProductName -eq 'sekiro-rs' -or $version.FileDescription -eq 'sekiro-rs'
        })
        if ($running.Count -eq 0) { return }
        $running | Select-Object Id,ProcessName,Path
        if ((Get-Date) -ge $limit) { throw 'Game still running after 15 minutes; no launch.' }
        Write-Output 'Waiting 60 seconds for the existing game to exit.'
        Start-Sleep -Seconds 60
    }
}
