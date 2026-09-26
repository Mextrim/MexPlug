@echo off
chcp 65001 >nul
setlocal
title MexPlug - сборка и установка

:: Чтобы cargo находился даже в "старой" консоли:
set "PATH=%PATH%;%USERPROFILE%\.cargo\bin"

if "%1"=="install-only" goto :install

where cargo >nul 2>&1
if errorlevel 1 (
  echo [x] cargo не найден. Поставь Rust: winget install Rustlang.Rustup
  pause
  exit /b 1
)

echo [*] Сборка VST3 + CLAP (release, ~2-5 минут)...
pushd "%~dp0daw-plugin"
cargo xtask bundle mex_plug --release
set "ERR=%ERRORLEVEL%"
popd
if "%ERR%" neq "0" (
  echo [x] Сборка не удалась, код %ERR%.
  pause
  exit /b 1
)

:install
net session >nul 2>&1
if errorlevel 1 (
  echo [*] Нужны права администратора для копирования в Common Files...
  powershell -NoProfile -Command "Start-Process '%~f0' -Verb RunAs -ArgumentList 'install-only'"
  exit /b
)

set "SRC=%~dp0daw-plugin\target\bundled"
if not exist "%SRC%\mex_plug.vst3" (
  echo [x] Нет бандлов в %SRC%. Сначала сборка без install-only.
  pause
  exit /b 1
)

echo [*] Удаляю старый FLHumanMix (если был)...
rmdir /S /Q "%ProgramFiles%\Common Files\VST3\fl_human_mix.vst3" 2>nul
del /Q "%ProgramFiles%\Common Files\CLAP\fl_human_mix.clap" 2>nul

echo [*] Установка VST3...
if not exist "%ProgramFiles%\Common Files\VST3" mkdir "%ProgramFiles%\Common Files\VST3"
xcopy /E /I /Y "%SRC%\mex_plug.vst3" "%ProgramFiles%\Common Files\VST3\mex_plug.vst3" >nul

echo [*] Установка CLAP...
if not exist "%ProgramFiles%\Common Files\CLAP" mkdir "%ProgramFiles%\Common Files\CLAP"
copy /Y "%SRC%\mex_plug.clap" "%ProgramFiles%\Common Files\CLAP\mex_plug.clap" >nul

echo.
echo [OK] Установлено:
echo   %ProgramFiles%\Common Files\VST3\mex_plug.vst3
echo   %ProgramFiles%\Common Files\CLAP\mex_plug.clap
echo.
echo Дальше в DAW сделай рескан плагинов:
echo   FL Studio:  Options -^> Manage plugins -^> Find plugins
echo   Ableton:    Settings -^> Plug-ins -^> Rescan
echo   Reaper:     Options -^> Preferences -^> Plug-ins -^> VST -^> Re-scan
echo   Ищи: MexPlug
echo.
pause
