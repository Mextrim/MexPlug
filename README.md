# MexPlug — punchy auto-mix + analog liveliness (VST3/CLAP)

Плагин для всех major DAW на Windows: авто-сведение и «аналоговая живость»
для стерильных ИИ-треков. Тёмная сторона не понадобится — только 14 ручек,
5 пресетов и A/B.

## Установка за 1 минуту (FL Studio, без сборки)

1. Скачай **MexPlug_Setup_vX.Y.Z.exe** со страницы
   [Releases](https://github.com/Mextrim/MexPlug/releases) и запусти
   (или `MexPlug_FL_vX.Y.Z.zip` → распаковать → `INSTALL.bat`).
2. В FL Studio: Options → Manage plugins → Find plugins → ищи **MexPlug**.

## Что внутри

- **Цепь:** input → DC-block → HP 28 Гц → mud-cut → Bass → Air → mono bass
  (LR4 @120) → сатурация Clean/Warm/Hard → M/S width → punch → wow/flutter →
  room → smooth (de-harsh) → плёночный шум → glue 2:1 → лимитер (−1 dBFS).
- **Интерфейс:** минимализм, 8 + 5 ручек, стерео-метр с клип-индикатором,
  пресеты (Gentle Polish, AI Rescue, Club Punch, Lo-Fi Warmth, Airy Clean),
  сравнение A/B.
- **Форматы:** VST3 + CLAP из одной кодовой базы
  ([nice-plug](https://codeberg.org/RustAudio/nice-plug)), без внешних DSP-зависимостей.

## Структура

- `daw-plugin/` — исходники плагина (Rust), сборка, тесты, доки для разработчиков.
- `installer/` — `INSTALL.bat` для ZIP-релизов и `MexPlug.nsi` (NSIS) для Setup.exe.

## Сборка из исходников

```
cd daw-plugin
cargo xtask bundle mex_plug --release
```

Бандлы появятся в `daw-plugin/target/bundled/`. Проверка:
`cargo test -p mex_plug`, валидация — [pluginval](https://github.com/Tracktion/pluginval).
