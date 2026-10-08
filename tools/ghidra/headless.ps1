# Runs Ghidra headless scripts against the local sekiro project in re/ghidra.
# Example: powershell -NoProfile -File tools/ghidra/headless.ps1 ExportIndex.java re/exports
#          powershell -NoProfile -File tools/ghidra/headless.ps1 Decompile.java re/exports/decomp bd6710
param(
    [Parameter(Mandatory)][string]$Script,
    [Parameter(ValueFromRemainingArguments)][string[]]$ScriptArgs
)
$ErrorActionPreference = 'Stop'
$root = Split-Path -Parent (Split-Path -Parent $PSScriptRoot)
$env:JAVA_HOME = 'C:\Program Files\Eclipse Adoptium\jdk-21.0.12.101-hotspot'
$env:GHIDRA_MAXMEM = '20G'
Set-Location $root
& '~\tools\ghidra\support\analyzeHeadless.bat' (Join-Path $root 're\ghidra') sekiro `
    -process -noanalysis -scriptPath (Join-Path $root 'tools\ghidra') -postScript $Script @ScriptArgs
