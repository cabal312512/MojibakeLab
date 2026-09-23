# Dot-source: . .\scripts\env.ps1
# All paths and caches are project-local. No registry or persistent PATH changes.
$MojibakeRoot = Split-Path -Parent $PSScriptRoot
$env:CARGO_HOME = Join-Path $MojibakeRoot '.cache\cargo'
$env:RUSTUP_HOME = Join-Path $MojibakeRoot '.tools\rustup'
$env:CARGO_TARGET_DIR = Join-Path $MojibakeRoot 'target'
$env:TEMP = Join-Path $MojibakeRoot '.tmp'
$env:TMP = $env:TEMP
$env:npm_config_cache = Join-Path $MojibakeRoot '.cache\npm'
$env:PYTHONDONTWRITEBYTECODE = '1'
$env:VSCMD_SKIP_SENDTELEMETRY = '1'
$env:DOTNET_CLI_TELEMETRY_OPTOUT = '1'
$env:PLAYWRIGHT_BROWSERS_PATH = Join-Path $MojibakeRoot '.cache\playwright'
$env:TAURI_BUNDLER_CACHE_DIR = Join-Path $MojibakeRoot '.cache\tauri'
New-Item -ItemType Directory -Force $env:CARGO_HOME,$env:TEMP,$env:npm_config_cache,$env:PLAYWRIGHT_BROWSERS_PATH,$env:TAURI_BUNDLER_CACHE_DIR | Out-Null
$MojibakeRustBin = Join-Path $MojibakeRoot '.tools\rust\bin'
$MojibakeNodeBin = Join-Path $MojibakeRoot '.tools\node'
$env:PATH = "$MojibakeRustBin;$MojibakeNodeBin;$env:PATH"
$MojibakeMsvcBase = Join-Path $MojibakeRoot '.tools\msvc'
$MojibakeMsvc = Get-ChildItem (Join-Path $MojibakeMsvcBase 'VC\Tools\MSVC') -Directory -ErrorAction SilentlyContinue | Sort-Object Name -Descending | Select-Object -First 1
$MojibakeSdk = Get-ChildItem (Join-Path $MojibakeMsvcBase 'Windows Kits\10\Lib') -Directory -ErrorAction SilentlyContinue | Sort-Object Name -Descending | Select-Object -First 1
if ($MojibakeMsvc -and $MojibakeSdk) {
    $MojibakeSdkRoot = Join-Path $MojibakeMsvcBase 'Windows Kits\10'
    $MojibakeCompilerBin = Join-Path $MojibakeMsvc.FullName 'bin\Hostx64\x64'
    $MojibakeSdkBin = Join-Path $MojibakeSdkRoot "bin\$($MojibakeSdk.Name)\x64"
    $env:PATH = "$MojibakeCompilerBin;$MojibakeSdkBin;$env:PATH"
    $env:INCLUDE = "$($MojibakeMsvc.FullName)\include;$MojibakeSdkRoot\Include\$($MojibakeSdk.Name)\ucrt;$MojibakeSdkRoot\Include\$($MojibakeSdk.Name)\shared;$MojibakeSdkRoot\Include\$($MojibakeSdk.Name)\um;$MojibakeSdkRoot\Include\$($MojibakeSdk.Name)\winrt"
    $env:LIB = "$($MojibakeMsvc.FullName)\lib\x64;$($MojibakeSdk.FullName)\ucrt\x64;$($MojibakeSdk.FullName)\um\x64"
    $env:VCToolsInstallDir = "$($MojibakeMsvc.FullName)\"
    $env:VCToolsVersion = $MojibakeMsvc.Name
    $env:WindowsSdkDir = "$MojibakeSdkRoot\"
    $env:WindowsSDKVersion = "$($MojibakeSdk.Name)\"
    $env:WindowsSdkBinPath = "$MojibakeSdkRoot\bin\"
    $env:VSCMD_ARG_HOST_ARCH = 'x64'
    $env:VSCMD_ARG_TGT_ARCH = 'x64'
    $env:CARGO_TARGET_X86_64_PC_WINDOWS_MSVC_LINKER = Join-Path $MojibakeCompilerBin 'link.exe'
    $env:CC = Join-Path $MojibakeCompilerBin 'cl.exe'
    $env:CXX = $env:CC
    $env:AR = Join-Path $MojibakeCompilerBin 'lib.exe'
    $env:RC = Join-Path $MojibakeSdkBin 'rc.exe'
}
