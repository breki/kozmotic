#
# Fail when a kozmotic.exe imports the Visual C++ runtime.
#
# kozmotic.exe must start on Windows without the Visual C++
# Redistributable, which Windows does not ship; Windows Server
# Core, for one, lacks it, and there the program fails with
# 0xC0000135. The crt-static setting in .cargo/config.toml links
# the runtime in, and a RUSTFLAGS or CARGO_ENCODED_RUSTFLAGS
# variable would silently replace that setting. A machine that has
# the Redistributable, a CI runner included, still starts such a
# binary, so only reading its imports catches the regression.
#
# PowerShell rather than xtask because dumpbin, found through
# Visual Studio's vswhere, exists only on Windows.
#
# Run by .github/workflows/ci.yml on the debug build and by
# release.yml on the binary it ships, and runnable by hand on a
# Windows machine with Visual Studio's C++ tools:
#
#   scripts/check-no-vc-runtime.ps1 target\debug\kozmotic.exe
#
param(
    [Parameter(Mandatory)] [string] $Exe
)

$ErrorActionPreference = 'Stop'

if (-not (Test-Path -LiteralPath $Exe -PathType Leaf)) {
    throw "$Exe does not exist; build it first"
}

$vswhere = Join-Path ${env:ProgramFiles(x86)} `
    'Microsoft Visual Studio\Installer\vswhere.exe'
$dumpbin = & $vswhere -latest -products * `
    -find 'VC\Tools\MSVC\**\bin\Hostx64\x64\dumpbin.exe' |
    Select-Object -First 1
if (-not $dumpbin) {
    throw 'dumpbin.exe not found; install the Visual Studio C++ tools'
}

$imports = & $dumpbin /nologo /dependents $Exe
if ($LASTEXITCODE -ne 0) {
    throw "dumpbin exited with $LASTEXITCODE"
}
$imports

# VCRUNTIME140.dll and MSVCP140.dll are the C++ runtime proper;
# api-ms-win-crt-* and ucrtbase.dll are the Universal C runtime,
# which a crt-static build links in too. MSVCP needs a digit after
# it so msvcp_win.dll, a system DLL, does not match.
$runtime = $imports |
    Select-String -Pattern '^\s*(VCRUNTIME|MSVCP\d|api-ms-win-crt-|ucrtbase)'
if ($runtime) {
    $names = ($runtime | ForEach-Object { $_.Line.Trim() }) -join ', '
    throw "$Exe imports the Visual C++ runtime: $names"
}
Write-Output "$Exe imports no Visual C++ runtime"
