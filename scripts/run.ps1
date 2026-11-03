#Requires -Version 5.1
<#
.SYNOPSIS
    Runs the application, with the unpacked GStreamer runtime when there is one.

.DESCRIPTION
    The application looks for GStreamer in its own data directory, beside the executable,
    and in the directory PARLOR_GSTREAMER_DIR names. This script points that variable at
    .tools/gstreamer/runtime so a development run uses the runtime it was built against
    rather than whatever else is on the machine.

.EXAMPLE
    powershell -ExecutionPolicy Bypass -File scripts/run.ps1
#>
[CmdletBinding()]
param()

$ErrorActionPreference = "Stop"
$root = Split-Path -Parent $PSScriptRoot
$runtime = Join-Path $root ".tools/gstreamer/runtime"
if (Test-Path (Join-Path $runtime "bin")) {
    $env:PARLOR_GSTREAMER_DIR = $runtime
    Write-Host "GStreamer runtime: $runtime"
}
Push-Location $root
try {
    & cargo run -p parlor-desktop
} finally {
    Pop-Location
}
