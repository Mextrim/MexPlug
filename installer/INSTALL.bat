@echo off
chcp 65001 >nul
setlocal
title MexPlug — автоустановка в FL Studio

:: Лог для диагностики
set "LOG=%TEMP%\mexplug_install.log"
echo [%DATE% %TIME%] start %~f0 >> "%LOG%"

:: Папка, откуда запущен этот файл (там же лежат mex_plug.vst3 и mex_plug.clap)
set "HERE=%~dp0"

if not exist "%HERE%mex_plug.vst3" (
  echo [x] Рядом нет папки mex_plug.vst3. Распакуй ZIP целиком и запусти INSTALL.bat из него.
  pause
  exit /b 1
)
if not exist "%HERE%mex_plug.clap" (
  echo [x] Рядом нет файла mex_plug.clap. Распакуй ZIP целиком и запусти INSTALL.bat из него.
  pause
  exit /b 1
)

net session >nul 2>&1
if errorlevel 1 (
  echo [*] Нужны права администратора — перезапускаю с запросом...
  echo [%DATE% %TIME%] relaunch elevated >> "%LOG%"
  powershell -NoProfile -Command "Start-Process '%~f0' -Verb RunAs"
  exit /b
)
echo [%DATE% %TIME%] elevated ok >> "%LOG%"

echo [*] Удаляю старую копию, если была...
rmdir /S /Q "%ProgramFiles%\Common Files\VST3\mex_plug.vst3" 2>nul
rmdir /S /Q "%ProgramFiles%\Common Files\VST3\fl_human_mix.vst3" 2>nul
del /Q "%ProgramFiles%\Common Files\CLAP\mex_plug.clap" 2>nul
del /Q "%ProgramFiles%\Common Files\CLAP\fl_human_mix.clap" 2>nul

echo [*] Ставлю VST3...
xcopy /E /I /Y "%HERE%mex_plug.vst3" "%ProgramFiles%\Common Files\VST3\mex_plug.vst3" >nul
if errorlevel 1 (
  echo [x] Не вышло скопировать VST3. Подробности в %LOG%.
  echo [%DATE% %TIME%] VST3 copy FAILED >> "%LOG%"
  pause
  exit /b 1
)
echo [%DATE% %TIME%] VST3 ok >> "%LOG%"

echo [*] Ставлю CLAP...
copy /Y "%HERE%mex_plug.clap" "%ProgramFiles%\Common Files\CLAP\mex_plug.clap" >nul
echo [%DATE% %TIME%] CLAP ok >> "%LOG%"

echo.
echo [OK] MexPlug установлен:
echo   %ProgramFiles%\Common Files\VST3\mex_plug.vst3
echo   %ProgramFiles%\Common Files\CLAP\mex_plug.clap
echo.
echo Последний шаг в FL Studio:
echo   1. Options -^> Manage plugins
echo   2. Find plugins (дождись конца скана)
echo   3. Найди MexPlug в списке
echo.
pause
