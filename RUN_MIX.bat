@echo off
chcp 65001 >nul
setlocal EnableDelayedExpansion
title FLHumanMix - запускатор

set "EXE=%~dp0bin\FLHumanMix.exe"
if not exist "%EXE%" (
  echo [x] Не найден: %EXE%
  echo [*] Пробую собрать: dotnet build -c Release -o bin
  pushd "%~dp0"
  dotnet build -c Release -o bin
  popd
)
if not exist "%EXE%" (
  echo [x] Собрать не удалось. Поставь .NET 10 SDK: https://dotnet.microsoft.com/download
  pause
  exit /b 1
)

:: --- 1. Входной файл: drag-n-drop (%1) или диалог ---
set "INPUT=%~1"
if defined INPUT (
  if not exist "%INPUT%" (
    echo [x] Файл не найден: %INPUT%
    set "INPUT="
  )
)
if not defined INPUT (
  echo.
  echo  Перетащи WAV на этот .bat ИЛИ выбери файл в окне...
  echo.
  for /f "delims=" %%F in ('powershell -noprofile -command "Add-Type -AssemblyName System.Windows.Forms; $d=New-Object System.Windows.Forms.OpenFileDialog; $d.Filter='WAV (*.wav)|*.wav|Все файлы (*.*)|*.*'; $d.Title='Выбери входной WAV'; if($d.ShowDialog() -eq 'OK'){ $d.FileName }"') do set "INPUT=%%F"
)
if not defined INPUT (
  echo [x] Файл не выбран.
  pause
  exit /b 1
)
echo [OK] Вход: "%INPUT%"

:: --- 2. Пресет ---
echo.
echo  === Пресет ===
echo  [1] Стандарт (drive 2.0, width 1.18, room 0.07, human 0.6)
echo  [2] Мягкий   (drive 1.5, width 1.10, room 0.05, human 0.4)
echo  [3] Жирный   (drive 2.6, width 1.25, room 0.09, human 0.75)
echo  [4] Живость максимум (drive 2.2, width 1.20, room 0.10, human 1.0)
echo  [5] Свой вариант (ввести вручную)
echo.
set "CHOICE="
set /p "CHOICE=Выбери [1-5, по умолч. 1]: "
if "%CHOICE%"=="" set "CHOICE=1"

if "%CHOICE%"=="1" (
  set "DRIVE=2.0" & set "WIDTH=1.18" & set "ROOM=0.07" & set "HUMAN=0.6"
) else if "%CHOICE%"=="2" (
  set "DRIVE=1.5" & set "WIDTH=1.10" & set "ROOM=0.05" & set "HUMAN=0.4"
) else if "%CHOICE%"=="3" (
  set "DRIVE=2.6" & set "WIDTH=1.25" & set "ROOM=0.09" & set "HUMAN=0.75"
) else if "%CHOICE%"=="4" (
  set "DRIVE=2.2" & set "WIDTH=1.20" & set "ROOM=0.10" & set "HUMAN=1.0"
) else (
  echo.
  echo  Оставь пустым = значение по умолчанию.
  set /p "DRIVE=  --drive 1.0..4.0 (умолч. 2.0): "
  set /p "WIDTH=  --width 1.0..1.5 (умолч. 1.18): "
  set /p "ROOM=  --room  0.0..0.25 (умолч. 0.07): "
  set /p "HUMAN=  --human 0.0..1.0 (умолч. 0.6): "
  if "!DRIVE!"=="" set "DRIVE=2.0"
  if "!WIDTH!"=="" set "WIDTH=1.18"
  if "!ROOM!"=="" set "ROOM=0.07"
  if "!HUMAN!"=="" set "HUMAN=0.6"
)

:: --- 3. Выходной файл (из INPUT, работает и для drag-n-drop, и для диалога) ---
for %%I in ("!INPUT!") do set "OUT=%%~dpnI_MIXED.wav"
echo.
echo  Выход: "!OUT!"
echo  (нажми Enter чтобы оставить, или введи другой путь)
set "CUSTOM_OUT="
set /p "CUSTOM_OUT=  -o: "
if defined CUSTOM_OUT set "OUT=!CUSTOM_OUT:"=!"

:: --- 4. Запуск ---
echo.
echo  --------------------------------------------------
echo  "%EXE%" "%INPUT%" -o "%OUT%" --drive %DRIVE% --width %WIDTH% --room %ROOM% --human %HUMAN%
echo  --------------------------------------------------
"%EXE%" "%INPUT%" -o "%OUT%" --drive %DRIVE% --width %WIDTH% --room %ROOM% --human %HUMAN%
set "ERR=%ERRORLEVEL%"
echo.
if "%ERR%"=="0" (
  echo  [OK] Готово: "%OUT%"
  echo  Тяни его обратно в Playlist FL Studio.
) else (
  echo  [x] Ошибка, код %ERR%.
)
echo.
pause
