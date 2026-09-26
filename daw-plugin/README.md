# MexPlug — VST3/CLAP плагин (v0.2.0)

Панчевое авто-сведение + аналоговая живость. Реалтайм-версия офлайн-обрабатывалки
(`Program.cs` в корне проекта). Фреймворк: [nice-plug](https://codeberg.org/RustAudio/nice-plug) 0.4.
Один код → два формата из одной сборки.

## Готовые бандлы

После `cargo xtask bundle mex_plug --release` лежат в `target/bundled/`:

- `mex_plug.vst3/` — VST3 (FL Studio, Ableton, Cubase, Studio One, Reaper, ...).
- `mex_plug.clap` — CLAP (Bitwig, FL Studio 21+, Reaper, ...).

## Установка (Windows x64)

Проще всего: запусти `BUILD_PLUGIN.bat` в корне проекта —
соберёт release, удалит старый `fl_human_mix` и разложит новые бандлы
по системным папкам (запросит админа):

- `%ProgramFiles%\Common Files\VST3\mex_plug.vst3`
- `%ProgramFiles%\Common Files\CLAP\mex_plug.clap`

Затем рескан плагинов в DAW, ищи **MexPlug**.

## Параметры

| Параметр | Диапазон | Умолч. | Что делает |
|---|---|---|---|
| Drive | 1.0–4.0 | 2.0 | tanh-сатурация |
| Width | 1.0–1.5 | 1.18 | стерео-ширина M/S |
| Room | 0.0–0.25 | 0.07 | малая комната (wet) |
| Human | 0.0–1.0 | 0.6 | wow/flutter + плёночный шум |
| Punch | 0.0–1.0 | 0.3 | transient shaper — атака и щелчок |
| Smooth | 0.0–1.0 | 0.25 | динамический tamer жёстких верхов |
| Mono Bass | on/off | on | низ ниже ~120 Гц в моно |
| Output | −12…+12 dB | 0 | трим перед лимитером |
| Ceiling | −3…−0.1 dB | −1.0 | потолок лимитера |

Цепь: DC-block → HP 28 Гц → −1.2 дБ @320 → +1.6 дБ @8.2к → tanh →
mono bass → M/S → punch → wow/flutter → room → smooth → шум →
glue 2:1 → лимитер. Все ручки сглажены (30 мс), автоматизация пишется хостом.
Интерфейс — generic UI хоста (свой GUI позже).

ID старых параметров (`drive/width/room/human/output`) не менялись.

## Проверено

- `cargo test -p mex_plug`: 7 тестов (finite+потолок лимитера mono/stereo,
  эффект меняет сигнал, тишина стабильна, mono bass складывает противофазный
  низ, punch бустит атаку, smooth давит жесткие верхи, ceiling держится).
- pluginval, strictness 5: SUCCESS.

## Что НЕ покрыто (и почему)

- **AU / Logic** — собирается только на macOS.
- **AAX / Pro Tools** — нужен Avid SDK и подпись.
- VST3+CLAP закрывают все остальные major DAW на Windows.

## Пересобрать вручную

```
cd daw-plugin
cargo xtask bundle mex_plug --release
```

Требуется Rust (gnu target, MinGW уже стоит через winget: WinLibs).
