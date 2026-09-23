param([string]$Executable = 'target\release\mojibake-lab.exe')
$ErrorActionPreference = 'Stop'
. (Join-Path $PSScriptRoot 'env.ps1')
Set-Location $MojibakeRoot
$MojibakeExecutable = (Resolve-Path -LiteralPath $Executable).Path
if (-not $MojibakeExecutable.StartsWith($MojibakeRoot + '\', [System.StringComparison]::OrdinalIgnoreCase)) {
    throw 'The test executable must be inside this project.'
}
$env:MOJIBAKE_DATA_DIR = Join-Path $MojibakeRoot ('.tmp\desktop-test-' + [Guid]::NewGuid().ToString('N'))
$env:WEBVIEW2_ADDITIONAL_BROWSER_ARGUMENTS = '--remote-debugging-port=9223'
$MojibakeAppProcess = Start-Process -FilePath $MojibakeExecutable -WorkingDirectory $MojibakeRoot -WindowStyle Hidden -PassThru
try {
    $MojibakeReady = $false
    for ($MojibakeAttempt = 0; $MojibakeAttempt -lt 50; $MojibakeAttempt++) {
        try {
            $null = Invoke-WebRequest -Uri 'http://127.0.0.1:9223/json/version' -UseBasicParsing -TimeoutSec 1
            $MojibakeReady = $true
            break
        } catch { Start-Sleep -Milliseconds 200 }
    }
    if (-not $MojibakeReady) { throw 'Desktop webview did not start.' }
    & node (Join-Path $PSScriptRoot 'test-desktop.mjs')
    if ($LASTEXITCODE -ne 0) { throw "Desktop checks failed ($LASTEXITCODE)." }
} finally {
    if (-not $MojibakeAppProcess.HasExited) { Stop-Process -Id $MojibakeAppProcess.Id }
}
