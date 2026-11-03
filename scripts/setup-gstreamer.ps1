#Requires -Version 5.1
<#
.SYNOPSIS
    Downloads the GStreamer runtime and development packages and unpacks them under .tools.

.DESCRIPTION
    The application loads the GStreamer backend from a plugin, and the plugin needs the
    GStreamer development files to build. This script fetches the official MSVC packages
    and unpacks them without installing anything: an administrative install writes the
    same files into a directory and leaves no uninstall entry behind.

    The packages are the MSVC ones on purpose. GStreamer from MSYS2 is built with MinGW,
    and an MSVC build cannot link its import libraries.

.PARAMETER Version
    The GStreamer release to fetch. The plugin is built against one release at a time.

.PARAMETER Architecture
    The MSVC architecture package; x86_64 is what the application targets.

.PARAMETER Destination
    Where the two packages are unpacked. The default is .tools/gstreamer in this
    repository, which .gitignore already excludes.

.EXAMPLE
    powershell -ExecutionPolicy Bypass -File scripts/setup-gstreamer.ps1
#>
[CmdletBinding()]
param(
    [string]$Version = "1.26.11",
    [string]$Architecture = "x86_64",
    [string]$Destination = ""
)

$ErrorActionPreference = "Stop"

if (-not $Destination) {
    $Destination = Join-Path (Split-Path -Parent $PSScriptRoot) ".tools/gstreamer"
}

# A path an argument list can hold without its spaces splitting the argument.
function Quoted([string]$value) { return '"' + $value + '"' }

$base = "https://gstreamer.freedesktop.org/data/pkg/windows/$Version/msvc"
$packages = @(
    @{ Name = "runtime"; File = "gstreamer-1.0-msvc-$Architecture-$Version.msi" },
    @{ Name = "devel"; File = "gstreamer-1.0-devel-msvc-$Architecture-$Version.msi" }
)

New-Item -ItemType Directory -Force -Path $Destination | Out-Null

foreach ($package in $packages) {
    $target = Join-Path $Destination $package.Name
    if (Test-Path (Join-Path $target "bin")) {
        Write-Host "$($package.Name) is already unpacked in $target"
        continue
    }
    $archive = Join-Path $Destination ($package.Name + ".msi")
    if (-not (Test-Path $archive)) {
        Write-Host "downloading $($package.File)"
        Invoke-WebRequest -Uri "$base/$($package.File)" -OutFile $archive
    }
    $staging = Join-Path $Destination ($package.Name + "-staging")
    Remove-Item -Recurse -Force $staging -ErrorAction SilentlyContinue
    New-Item -ItemType Directory -Force -Path $staging | Out-Null
    $log = Join-Path $Destination ($package.Name + ".log")
    Write-Host "unpacking $($package.Name)"
    $arguments = @(
        "/a", (Quoted $archive), "/qn", ("TARGETDIR=" + (Quoted $staging)), "/L*v", (Quoted $log)
    )
    $started = Start-Process msiexec.exe -Wait -PassThru -ArgumentList $arguments
    if ($started.ExitCode -ne 0) {
        throw "msiexec exited with $($started.ExitCode); its log is $log"
    }
    # The package decides its own layout, so the payload is found rather than assumed.
    $payload = Get-ChildItem -Path $staging -Recurse -Directory -Filter "msvc_*" |
        Select-Object -First 1
    if (-not $payload) {
        throw "the package has no msvc payload; its log is $log"
    }
    Move-Item -Path $payload.FullName -Destination $target
    Remove-Item -Recurse -Force $staging
    Remove-Item -Force $archive
}

$development = Join-Path $Destination "devel"
$runtime = Join-Path $Destination "runtime"
Write-Host ""
Write-Host "GStreamer is unpacked in $Destination"
Write-Host "For the current shell:"
Write-Host ("  " + [char]36 + "env:PKG_CONFIG_PATH = " + [char]34 + (Join-Path $development "lib/pkgconfig") + [char]34)
Write-Host ("  " + [char]36 + "env:PARLOR_GSTREAMER_DIR = " + [char]34 + $runtime + [char]34)
Write-Host ""
Write-Host "scripts/build.ps1 sets both itself."
