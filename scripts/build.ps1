param([switch]$SkipTests)
$ErrorActionPreference='Stop'
. (Join-Path $PSScriptRoot 'env.ps1')
Set-Location $MojibakeRoot
function Invoke-Checked([scriptblock]$Command) {
    & $Command
    if ($LASTEXITCODE -ne 0) { throw "Command failed with exit code $LASTEXITCODE" }
}
if (!(Test-Path 'node_modules')) { Invoke-Checked { npm.cmd ci } }
Invoke-Checked { npm.cmd run build }
if (!$SkipTests) {
    Invoke-Checked { npm.cmd run typecheck }
    Invoke-Checked { npm.cmd test }
    Invoke-Checked { cargo fmt --all --check }
    Invoke-Checked { cargo test --workspace --locked }
    Invoke-Checked { cargo check --workspace --locked }
}
Invoke-Checked { npm.cmd run desktop:build }
$MojibakeRelease=Join-Path $MojibakeRoot 'release\MojibakeLab'
New-Item -ItemType Directory -Force $MojibakeRelease | Out-Null
Copy-Item -LiteralPath 'target\release\mojibake-lab.exe' -Destination (Join-Path $MojibakeRelease 'MojibakeLab.exe') -Force
Copy-Item -LiteralPath 'README.md','README.en.md','LICENSE','THIRD_PARTY_NOTICES.md' -Destination $MojibakeRelease -Force
Copy-Item -LiteralPath 'docs' -Destination $MojibakeRelease -Recurse -Force
Copy-Item -LiteralPath 'docs\PORTABLE.txt' -Destination (Join-Path $MojibakeRelease '使用说明.txt') -Force
Invoke-Checked { node scripts/collect-licenses.mjs }
Copy-Item -LiteralPath 'THIRD_PARTY_LICENSES.txt' -Destination $MojibakeRelease -Force
Write-Host "Ready: $MojibakeRelease\MojibakeLab.exe"
