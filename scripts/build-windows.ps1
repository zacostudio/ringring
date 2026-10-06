# Windows 릴리스 빌드 — 실행 파일과 NSIS 설치 파일을 만든다. 서명하지 않는다
#
#   .\scripts\build-windows.ps1                  릴리스 빌드와 설치 파일
#   .\scripts\build-windows.ps1 --no-bundle      실행 파일만
#
# 인자는 `tauri build` 로 그대로 넘어간다.
#
# Rust 는 panic 이 난 자리를 알리려고 소스 파일의 경로를 실행 파일에 넣는다. 그대로 두면 빌드한 사람의
# 홈 폴더(`C:\Users\<계정>`)가 배포하는 파일에 남는다. 그 앞부분을 `~` 로 바꿔 넣고, 빌드 뒤에 실행 파일을
# 훑어 남은 것이 있으면 실패로 끝낸다.

$ErrorActionPreference = 'Stop'
Set-Location (Split-Path $PSScriptRoot -Parent)

$userHome = $env:USERPROFILE
# 경로에 빈칸이 있어도 한 인자로 넘어가게 CARGO_ENCODED_RUSTFLAGS 를 쓴다 (인자 사이는 0x1f).
$flags = @("--remap-path-prefix=$userHome=~")
if ($env:CARGO_ENCODED_RUSTFLAGS) { $flags += $env:CARGO_ENCODED_RUSTFLAGS -split [char]0x1f }
$env:CARGO_ENCODED_RUSTFLAGS = $flags -join [char]0x1f

bunx '@tauri-apps/cli@2' build @args
if ($LASTEXITCODE -ne 0) { exit $LASTEXITCODE }

$targetDir = if ($env:CARGO_TARGET_DIR) { $env:CARGO_TARGET_DIR } else { 'src-tauri\target' }
$exe = Join-Path $targetDir 'release\ringring.exe'
# 바이트를 그대로 글자로 읽는다. 경로는 ASCII 로도 UTF-16 으로도 들어갈 수 있어 둘 다 찾는다.
$bytes = [IO.File]::ReadAllBytes($exe)
$leaks = 0
foreach ($encoding in [Text.Encoding]::GetEncoding(28591), [Text.Encoding]::Unicode) {
	$text = $encoding.GetString($bytes)
	foreach ($needle in $userHome, $userHome.Replace('\', '/')) {
		$leaks += [regex]::Matches($text, [regex]::Escape($needle), 'IgnoreCase').Count
	}
}
if ($leaks -gt 0) {
	Write-Error "$exe still holds the home folder path in $leaks places. Do not ship this file."
	exit 1
}
Write-Host "OK: no home folder path in $exe"
