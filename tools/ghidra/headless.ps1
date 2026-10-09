# Runs Ghidra headless scripts against the local sekiro project in re/ghidra.
# Example: powershell -NoProfile -File tools/ghidra/headless.ps1 ExportIndex.java re/exports
#          powershell -NoProfile -File tools/ghidra/headless.ps1 Decompile.java re/exports/decomp bd6710
# Ghidra is found via GHIDRA_INSTALL_DIR (default: ~/tools/ghidra); Java via JAVA_HOME
# (default: the newest Temurin JDK 21 under Program Files).
param(
    [Parameter(Mandatory)][string]$Script,
    [Parameter(ValueFromRemainingArguments)][string[]]$ScriptArgs
)
$ErrorActionPreference = 'Stop'
$root = Split-Path -Parent (Split-Path -Parent $PSScriptRoot)
$ghidra = if ($env:GHIDRA_INSTALL_DIR) { $env:GHIDRA_INSTALL_DIR } else { Join-Path $HOME 'tools\ghidra' }
if (-not $env:JAVA_HOME) {
    $jdk = Get-ChildItem 'C:\Program Files\Eclipse Adoptium' -Directory -Filter 'jdk-21*' -ErrorAction SilentlyContinue |
        Sort-Object Name -Descending | Select-Object -First 1
    if ($jdk) { $env:JAVA_HOME = $jdk.FullName }
}
if (-not $env:GHIDRA_MAXMEM) { $env:GHIDRA_MAXMEM = '20G' }
Set-Location $root
& (Join-Path $ghidra 'support\analyzeHeadless.bat') (Join-Path $root 're\ghidra') sekiro `
    -process -noanalysis -scriptPath (Join-Path $root 'tools\ghidra') -postScript $Script @ScriptArgs
