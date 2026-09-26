# MexPlug 🎛️ — punchy auto-mix + analog liveliness

<img width="1280" height="688" alt="MexPlug in FL Studio" src="https://github.com/user-attachments/assets/3c5bed82-ab0c-4a85-9899-7bbdafb6b825" />
<img width="1280" height="688" alt="MexPlug themes" src="https://github.com/user-attachments/assets/d0a49e15-80b4-4c1b-a3ee-734381d88d5c" />
<img width="1280" height="688" alt="MexPlug mastering" src="https://github.com/user-attachments/assets/e12948a3-0ee2-4418-9ac7-7d41f33c5eac" />

[![GitHub release](https://img.shields.io/github/v/release/Mextrim/MexPlug?style=flat-square)](https://github.com/Mextrim/MexPlug/releases)
[![Windows](https://img.shields.io/badge/Windows-x64-blue?style=flat-square)](https://github.com/Mextrim/MexPlug/releases)
[![VST3](https://img.shields.io/badge/VST3-ready-green?style=flat-square)](https://github.com/Mextrim/MexPlug/releases)
[![CLAP](https://img.shields.io/badge/CLAP-ready-green?style=flat-square)](https://github.com/Mextrim/MexPlug/releases)
[![Tests](https://img.shields.io/badge/tests-20%2F20-brightgreen?style=flat-square)](https://github.com/Mextrim/MexPlug)

Плагин для всех major DAW на Windows, который превращает стерильные,
плоские треки в живые: авто-сведение, ламповое тепло, tape-характер
и мастеринговая дисциплина — в одном окне.

> **Честно:** никакая обработка не делает трек «100% человеческим»
> и не гарантирует обход детекторов ИИ-контента. MexPlug убирает
> слышимые маркеры стерильности, а не подделывает происхождение.
> Соблюдай правила площадок и дистрибьюторов по раскрытию ИИ.

---

## ⚡ Установка за 1 минуту (FL Studio, без сборки)

| # | Шаг |
|---|-----|
| 1 | Скачай **MexPlug_Setup_vX.Y.Z.exe** со страницы [Releases](https://github.com/Mextrim/MexPlug/releases) и запусти |
| 2 | В FL Studio: **Options → Manage plugins → Find plugins** |
| 3 | Ищи **MexPlug** — вешай на мастер или любой трек микшера |

Нет установщика под рукой? Вариант B: `MexPlug_FL_vX.Y.Z.zip` → распаковать → запустить `INSTALL.bat` (сам запросит права администратора).

---

## ✨ Что внутри

### 🔗 Цепь обработки (по порядку)

```
input → DC-block → HP 28 Гц → mud-cut → Bass → Air → mono bass (LR4 @120)
→ sat Clean/Warm/Hard → tube → M/S width → Haas → punch → wow/flutter
→ room → smooth → tape noise → glue 2:1 (SC HP) → output → limiter
→ dirt → mix → balance → monitor
```

### 🎚️ Интерфейс

- **19 ручек** — drag крутит, double-click сбрасывает, shift — точно; на каждой тултип
- **8 пресетов**: Gentle Polish · AI Rescue · Club Punch · Lo-Fi Warmth · Airy Clean · Streaming Loud · Vinyl Dust · Analog Ghost
- **DICE** — музыкальный рандомайзер настроек · **A/B** — сравнение в один клик
- **Стерео-метр + GR-метр** с peak-hold и залипающим клипом (клик — сбросить)
- **5 тем оформления** — точками в футере: Hardware · White · Black · Bootstrap · Flat (выбор хранится в проекте)

<details>
<summary><b>📋 Все параметры (нажми чтобы раскрыть)</b></summary>

| Параметр | Диапазон | Умолч. | Что делает |
|---|---|---|---|
| Drive | 1.0–4.0 | 2.0 | глубина сатурации |
| Width | 1.0–1.5 | 1.18 | стерео-ширина M/S |
| Room | 0.0–0.25 | 0.07 | малая комната |
| Human | 0.0–1.0 | 0.6 | wow/flutter + плёночный шум |
| Punch | 0.0–1.0 | 0.3 | атака транзиентов |
| Smooth | 0.0–1.0 | 0.25 | душит жёсткие верхи |
| Output | −12…+12 dB | 0 | трим перед лимитером |
| Ceiling | −3…−0.1 dB | −1.0 | потолок лимитера |
| Tube | 0.0–1.0 | 0.3 | чётные гармоники без постоянки |
| Dirt | 0.0–1.0 | 0.0 | биткрашер 16→6 бит |
| Input | −12…+12 dB | 0 | трим входа |
| Bass | −6…+6 dB | 0 | полка @100 Гц |
| Air | 0…+3 dB | 1.6 | полка @8.2 кГц |
| Glue | 0.0–1.0 | 1.0 | количество glue-компрессии |
| Style | Clean/Warm/Hard | Warm | характер сатурации |
| Mix | 0–100 % | 100 % | параллельный dry/wet |
| Haas | 0.0–1.0 | 0.0 | микро-задержка правого канала |
| SC HP | 20–500 Hz | 20 | сайдчейн glue: кик не качает микс |
| Balance | −1…+1 | 0 | баланс L/R на выходе |
| Monitor | Stereo/Mid/Side | Stereo | соло для проверки моно |
| Mono Bass | on/off | on | низ ниже 120 Гц в моно |

</details>

### 🖥️ Совместимость

| DAW | VST3 | CLAP |
|---|---|---|
| FL Studio, Ableton, Cubase, Studio One, Reaper | ✅ | ✅ |
| Bitwig | — | ✅ |
| Logic (AU), Pro Tools (AAX) | нужны macOS / Avid SDK | — |

---

## 🛠️ Для разработчиков

```
daw-plugin/     исходники (Rust, nice-plug), тесты, доки
installer/      INSTALL.bat + MexPlug.nsi (NSIS Setup.exe)
```

```sh
cd daw-plugin
cargo xtask bundle mex_plug --release   # бандлы в target/bundled/
cargo test -p mex_plug                  # 20 тестов DSP-ядра
cargo clippy -p mex_plug --all-targets  # ноль варнингов
```

Проверено: DSP ≈0.33 мкс/стерео-фрейм @48 кГц, ноль аллокаций на аудио-потоке,
валидация [pluginval](https://github.com/Tracktion/pluginval) strictness 5 — SUCCESS.
Фреймворк: [nice-plug](https://codeberg.org/RustAudio/nice-plug) (Rust, без внешних DSP-зависимостей).
