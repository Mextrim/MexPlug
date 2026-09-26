; MexPlug NSIS installer: VST3 + CLAP into shared Common Files locations.
; Build from the repo root:  makensis installer\MexPlug.nsi
; (expects release bundles in daw-plugin\target\bundled\).
; NOTE: this file must stay CRLF (like all .bat in this repo).

!include "MUI2.nsh"

Name "MexPlug v0.7.0"
OutFile "MexPlug_Setup_v0.7.0.exe"
RequestExecutionLevel admin
Unicode True

!define MUI_WELCOMEPAGE_TITLE "Установка MexPlug v0.7.0"
!define MUI_WELCOMEPAGE_TEXT "Этот мастер установит плагин MexPlug:$\r$\n$\r$\n- VST3 в Common Files\VST3$\r$\n- CLAP в Common Files\CLAP$\r$\n$\r$\nПосле установки сделай рескан плагинов в DAW."
!define MUI_FINISHPAGE_TEXT "Готово! MexPlug установлен.$\r$\n$\r$\nFL Studio: Options -> Manage plugins -> Find plugins -> ищи MexPlug.$\r$\n$\r$\nНажмите «Готово», чтобы закрыть мастер."
!define MUI_FINISHPAGE_NOAUTOCLOSE

!insertmacro MUI_PAGE_WELCOME
!insertmacro MUI_PAGE_INSTFILES
!insertmacro MUI_PAGE_FINISH
!insertmacro MUI_LANGUAGE "Russian"

Section "MexPlug VST3 + CLAP" SEC_MAIN
  SectionIn RO

  DetailPrint "VST3 -> Common Files\VST3\mex_plug.vst3"
  RMDir /r "$PROGRAMFILES64\Common Files\VST3\mex_plug.vst3"
  RMDir /r "$PROGRAMFILES64\Common Files\VST3\fl_human_mix.vst3"
  SetOutPath "$PROGRAMFILES64\Common Files\VST3\mex_plug.vst3"
  File /r "..\daw-plugin\target\bundled\mex_plug.vst3\*.*"

  DetailPrint "CLAP -> Common Files\CLAP\mex_plug.clap"
  Delete "$PROGRAMFILES64\Common Files\CLAP\mex_plug.clap"
  Delete "$PROGRAMFILES64\Common Files\CLAP\fl_human_mix.clap"
  SetOutPath "$PROGRAMFILES64\Common Files\CLAP"
  File "..\daw-plugin\target\bundled\mex_plug.clap"

  DetailPrint "Готово"
SectionEnd
