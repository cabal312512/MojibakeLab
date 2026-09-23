# Portable Windows x64 development tools, extracted into this checkout.
# No Windows Installer, registry edits, system PATH changes, or global npm installs.
# The Microsoft downloads are governed by the Visual Studio Build Tools license:
# https://visualstudio.microsoft.com/license-terms/vs2022-ga-diagnosticbuildtools/
# Usage: powershell -ExecutionPolicy Bypass -File .\scripts\toolchain.ps1
[CmdletBinding()]
param()
$ErrorActionPreference = 'Stop'
. (Join-Path $PSScriptRoot 'env.ps1')
$MojibakeDownloads = Join-Path $MojibakeRoot '.cache\downloads'
$MojibakeUnpack = Join-Path $MojibakeRoot '.tmp\rust-components'
New-Item -ItemType Directory -Force $MojibakeDownloads,$MojibakeUnpack | Out-Null

function Get-ProjectDownload([string]$Url, [string]$Destination) {
    if (-not (Test-Path -LiteralPath $Destination)) {
        & curl.exe -fL --retry 3 --silent --show-error $Url -o "$Destination.partial"
        if ($LASTEXITCODE -ne 0) { throw "Download failed: $Url" }
        Move-Item -LiteralPath "$Destination.partial" -Destination $Destination -Force
    }
}

function Confirm-ProjectHash([string]$Path, [string]$Expected) {
    if ((Get-FileHash -LiteralPath $Path -Algorithm SHA256).Hash -ne $Expected) {
        throw "SHA256 verification failed: $Path. Remove this local download and retry."
    }
}

$MojibakeRustVersion = '1.90.0'
$MojibakeRust = Join-Path $MojibakeRoot '.tools\rust'
if (-not (Test-Path (Join-Path $MojibakeRust 'bin\cargo.exe')) -or -not (Test-Path (Join-Path $MojibakeRust 'bin\rustfmt.exe'))) {
    New-Item -ItemType Directory -Force $MojibakeRust | Out-Null
    foreach ($MojibakeComponent in @('rustc','rust-std','cargo','rustfmt')) {
        $MojibakeName = "$MojibakeComponent-$MojibakeRustVersion-x86_64-pc-windows-msvc"
        $MojibakeArchive = Join-Path $MojibakeDownloads "$MojibakeName.tar.xz"
        $MojibakeUrl = "https://static.rust-lang.org/dist/$MojibakeName.tar.xz"
        Get-ProjectDownload $MojibakeUrl $MojibakeArchive
        Get-ProjectDownload "$MojibakeUrl.sha256" "$MojibakeArchive.sha256"
        $MojibakeExpected = ((Get-Content -LiteralPath "$MojibakeArchive.sha256" -Raw).Trim() -split '\s+')[0]
        Confirm-ProjectHash $MojibakeArchive $MojibakeExpected
        & tar.exe -xf $MojibakeArchive -C $MojibakeUnpack
        if ($LASTEXITCODE -ne 0) { throw "Extraction failed: $MojibakeName" }
        $MojibakeExtracted = Join-Path $MojibakeUnpack $MojibakeName
        foreach ($MojibakeEntry in (Get-Content (Join-Path $MojibakeExtracted 'components'))) {
            Get-ChildItem (Join-Path $MojibakeExtracted $MojibakeEntry) -Force | Where-Object { $_.Name -ne 'manifest.in' } | Copy-Item -Destination $MojibakeRust -Recurse -Force
        }
    }
}

$MojibakeNodeVersion = '24.14.1'
$MojibakeNode = Join-Path $MojibakeRoot '.tools\node'
if (-not (Test-Path (Join-Path $MojibakeNode 'node.exe'))) {
    $MojibakeNodeName = "node-v$MojibakeNodeVersion-win-x64"
    $MojibakeNodeArchive = Join-Path $MojibakeDownloads "$MojibakeNodeName.zip"
    $MojibakeNodeSums = Join-Path $MojibakeDownloads "node-v$MojibakeNodeVersion-SHASUMS256.txt"
    Get-ProjectDownload "https://nodejs.org/dist/v$MojibakeNodeVersion/$MojibakeNodeName.zip" $MojibakeNodeArchive
    Get-ProjectDownload "https://nodejs.org/dist/v$MojibakeNodeVersion/SHASUMS256.txt" $MojibakeNodeSums
    $MojibakeNodeSum = Get-Content $MojibakeNodeSums | Where-Object { $_.EndsWith("$MojibakeNodeName.zip") } | Select-Object -First 1
    if (-not $MojibakeNodeSum) { throw 'Node.js checksum was not listed.' }
    Confirm-ProjectHash $MojibakeNodeArchive (($MojibakeNodeSum -split '\s+')[0])
    Expand-Archive -LiteralPath $MojibakeNodeArchive -DestinationPath $env:TEMP -Force
    New-Item -ItemType Directory -Force $MojibakeNode | Out-Null
    Get-ChildItem (Join-Path $env:TEMP $MojibakeNodeName) -Force | Copy-Item -Destination $MojibakeNode -Recurse -Force
}

$MojibakePython = Join-Path $MojibakeRoot '.tools\python'
if (-not (Test-Path (Join-Path $MojibakePython 'python.exe'))) {
    $MojibakePythonArchive = Join-Path $MojibakeDownloads 'python-3.12.10-embed-amd64.zip'
    Get-ProjectDownload 'https://www.python.org/ftp/python/3.12.10/python-3.12.10-embed-amd64.zip' $MojibakePythonArchive
    Confirm-ProjectHash $MojibakePythonArchive '4acbed6dd1c744b0376e3b1cf57ce906f9dc9e95e68824584c8099a63025a3c3'
    New-Item -ItemType Directory -Force $MojibakePython | Out-Null
    Expand-Archive -LiteralPath $MojibakePythonArchive -DestinationPath $MojibakePython -Force
}

$MojibakeMsvcMarker = Join-Path $MojibakeRoot '.tools\msvc\setup_x64.bat'
if (-not (Test-Path -LiteralPath $MojibakeMsvcMarker)) {
    $MojibakeMsvcUpstream = Join-Path $MojibakeRoot '.tools\portable-msvc-upstream.py'
    $MojibakeMsvcScript = Join-Path $MojibakeRoot '.tools\portable-msvc.py'
    # Pinned upstream downloader by mmozeiko. All vendor payloads are hash checked.
    Get-ProjectDownload 'https://gist.githubusercontent.com/mmozeiko/7f3162ec2988e81e56d5c4e22cde9977/raw/49b58850aa5e5c1e65f090d4651ee76607883e11/portable-msvc.py' $MojibakeMsvcUpstream
    Confirm-ProjectHash $MojibakeMsvcUpstream '1791cccca594ff083ce8d3e0a139e6156d2c312e24332a909e608010f0230a51'
    $MojibakeMsvcCode = (Get-Content -LiteralPath $MojibakeMsvcUpstream -Raw).Replace("`r`n", "`n")
    $MojibakeMsvcCode = $MojibakeMsvcCode.Replace('OUTPUT = Path("msvc")', 'OUTPUT = Path(__file__).resolve().parent / "msvc"')
    $MojibakeMsvcCode = $MojibakeMsvcCode.Replace('DOWNLOADS = Path("downloads")', 'DOWNLOADS = Path(__file__).resolve().parent.parent / ".cache" / "downloads" / "msvc"')
    $MojibakeOldExtract = '      subprocess.check_call(f''msiexec /a "{m}" /quiet /qn TARGETDIR="{OUTPUT.resolve()}"'')' + "`n" + '      (OUTPUT / m.name).unlink()'
    $MojibakeNewExtract = '      import runpy' + "`n" + '      helper = runpy.run_path(str(Path(__file__).with_name("extract-sdk-msi.py")))' + "`n" + '      helper["extract_msi"](m, OUTPUT)'
    if (-not $MojibakeMsvcCode.Contains($MojibakeOldExtract)) { throw 'Upstream extraction block changed; refusing to run an installer.' }
    $MojibakeMsvcCode = $MojibakeMsvcCode.Replace($MojibakeOldExtract, $MojibakeNewExtract)
    # Keep vendor files intact. Telemetry opt-out is set for the current process in env.ps1.
    $MojibakeTelemetryStart = $MojibakeMsvcCode.IndexOf('# executable that is collecting')
    $MojibakeTelemetryEnd = $MojibakeMsvcCode.IndexOf('# extra files for nvcc')
    if ($MojibakeTelemetryStart -lt 0 -or $MojibakeTelemetryEnd -le $MojibakeTelemetryStart) { throw 'Upstream cleanup block changed.' }
    $MojibakeMsvcCode = $MojibakeMsvcCode.Remove($MojibakeTelemetryStart, $MojibakeTelemetryEnd - $MojibakeTelemetryStart)
    [IO.File]::WriteAllText($MojibakeMsvcScript, $MojibakeMsvcCode, [Text.UTF8Encoding]::new($false))
    Copy-Item (Join-Path $PSScriptRoot 'extract-sdk-msi.py') (Join-Path $MojibakeRoot '.tools\extract-sdk-msi.py') -Force
    & (Join-Path $MojibakePython 'python.exe') -B -u $MojibakeMsvcScript --vs 2022 --msvc-version 14.44 --sdk-version 22621 --accept-license
    if ($LASTEXITCODE -ne 0) { throw 'Portable MSVC preparation failed.' }
}
. (Join-Path $PSScriptRoot 'env.ps1')
& rustc.exe --version
& cargo.exe --version
& node.exe --version
Write-Host 'Tools are ready. For a new shell: . .\scripts\env.ps1'
