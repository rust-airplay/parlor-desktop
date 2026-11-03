#Requires -Version 5.1
<#
.SYNOPSIS
    Builds the application, and the GStreamer plugin when its development files are present.

.DESCRIPTION
    The application does not link GStreamer: the backend is a plugin the application loads
    at run time. That is what lets the application start on a machine with no GStreamer and
    offer to install one. The plugin is a separate build, and this script builds it only
    when scripts/setup-gstreamer.ps1 has run, because otherwise the GStreamer development
    files are simply not on this machine.

.PARAMETER Release
    Build an optimized application.

.PARAMETER SkipPlugin
    Build only the application, even when the development files are present.

.EXAMPLE
    powershell -ExecutionPolicy Bypass -File scripts/build.ps1 -Release
#>
[CmdletBinding()]
param(
    [switch]$Release,
    [switch]$SkipPlugin
)

$ErrorActionPreference = "Stop"
$root = Split-Path -Parent $PSScriptRoot
$gstreamer = Join-Path $root ".tools/gstreamer"
$development = Join-Path $gstreamer "devel"

if (Test-Path (Join-Path $development "lib/pkgconfig")) {
    $env:PKG_CONFIG_PATH = (Resolve-Path (Join-Path $development "lib/pkgconfig")).Path
    Write-Host "GStreamer development files: $development"
} else {
    Write-Host "No GStreamer development files; the application is built without the plugin."
    Write-Host "Run scripts/setup-gstreamer.ps1 to add them."
}

Push-Location $root
try {
    Write-Host "building the application"
    if ($Release) {
        & cargo build -p parlor-desktop --release
    } else {
        & cargo build -p parlor-desktop
    }
    if ($LASTEXITCODE -ne 0) { throw "the application did not build" }
    if ($SkipPlugin) { return }
    if (-not $env:PKG_CONFIG_PATH) { return }
    Write-Host "building the GStreamer plugin"
    if ($Release) {
        & cargo build -p parlor-gst-backend --release
    } else {
        & cargo build -p parlor-gst-backend
    }
    if ($LASTEXITCODE -ne 0) { throw "the plugin did not build" }
} finally {
    Pop-Location
}