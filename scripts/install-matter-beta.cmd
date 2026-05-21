@echo off
setlocal

set "HERE=%~dp0"
set "ZIP=%HERE%matter-core-windows-x64.zip"
set "INSTALLER=%HERE%install-release-zip.ps1"
set "CHECKSUM_JSON=%HERE%release-checksums.json"
set "SHA256=%HERE%SHA256SUMS.txt"

echo Matter Core beta installer
echo.

if not exist "%ZIP%" (
  echo Missing file: %ZIP%
  goto fail
)

if not exist "%INSTALLER%" (
  echo Missing file: %INSTALLER%
  goto fail
)

if not exist "%CHECKSUM_JSON%" (
  echo Missing file: %CHECKSUM_JSON%
  goto fail
)

if not exist "%SHA256%" (
  echo Missing file: %SHA256%
  goto fail
)

powershell -NoProfile -ExecutionPolicy Bypass -File "%INSTALLER%" -ZipPath "%ZIP%" -ChecksumJsonPath "%CHECKSUM_JSON%" -Sha256Path "%SHA256%" %*
if errorlevel 1 goto fail

echo.
echo Matter Core beta installed.
echo Open a new terminal and run:
echo   matter run examples\agent_policy_demo.matter
echo.
if /i not "%CI%"=="true" pause
exit /b 0

:fail
echo.
echo Install failed. Keep all beta download files in the same folder and try again.
echo Required files:
echo   matter-core-windows-x64.zip
echo   install-release-zip.ps1
echo   release-checksums.json
echo   SHA256SUMS.txt
echo.
if /i not "%CI%"=="true" pause
exit /b 1
