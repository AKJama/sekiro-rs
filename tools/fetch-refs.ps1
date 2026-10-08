# Downloads public community reference files (archive keys, file name dictionary, param defs)
# into cache/refs. These are not vendored because some upstreams carry no licence.
[CmdletBinding()]
param()

$ErrorActionPreference = 'Stop'
$root = Split-Path -Parent $PSScriptRoot
$out = Join-Path $root 'cache/refs'
New-Item -ItemType Directory -Path $out -Force | Out-Null

$uxm = 'https://raw.githubusercontent.com/Nordgaren/UXM-Selective-Unpack/9501be87e272b6dae55e60a13c3f4753ca6fb3bb/UXM'
$paramdex = 'https://raw.githubusercontent.com/soulsmods/Paramdex/ff7245e524329bc3eab00036723d2bd53384cedf/SDT'

$files = @{
    'ArchiveKeys.cs'        = "$uxm/ArchiveKeys.cs"
    'SekiroDictionary.txt'  = "$uxm/res/SekiroDictionary.txt"
}
foreach ($name in $files.Keys) {
    Invoke-WebRequest -Uri $files[$name] -OutFile (Join-Path $out $name)
    Write-Host "fetched $name"
}

# Paramdex defs and row names for SDT: fetch the whole tree listing once.
$tree = Invoke-RestMethod 'https://api.github.com/repos/soulsmods/Paramdex/git/trees/ff7245e524329bc3eab00036723d2bd53384cedf?recursive=1'
foreach ($item in $tree.tree) {
    if ($item.type -ne 'blob') { continue }
    if ($item.path -notmatch '^SDT/(Defs|Names)/') { continue }
    $dest = Join-Path $out ('paramdex/' + $item.path.Substring(4))
    New-Item -ItemType Directory -Path (Split-Path -Parent $dest) -Force | Out-Null
    Invoke-WebRequest -Uri ("$paramdex/" + $item.path.Substring(4)) -OutFile $dest
}
Write-Host 'fetched Paramdex SDT defs and names'
