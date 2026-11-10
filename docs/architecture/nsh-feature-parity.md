# NSH Feature Parity и границы совместимости

- Статус: scope и parity specification.
- Референс: longbridge/nsh, текущая документация NSH проверена 2026-11-10.
- Назначение: понимать, что значит «полноценная замена», не смешивая архитектурную схожесть, совместимость API и зрелость продукта.

## 1. Определение замены

SolidJS Native является функциональной альтернативой NSH, когда пользователь может:

- создать TSX/Solid application;
- запускать её как самостоятельное native application на целевых ОС;
- встраивать UI в существующие native приложения;
- использовать нативный ввод, текст, focus, scrolling и accessibility;
- создавать UI с layout, overlays, navigation, images и common controls;
- применять единый API и общую архитектуру на всех поддерживаемых платформах;
- собирать и тестировать приложение через единый CLI/toolchain;
- обновлять UI при разработке через HMR;
- поставлять динамические UI modules с проверяемой целостностью и ограниченными capabilities;
- иметь headless тестовый режим и platform integration tests.

Замена означает функциональную и workflow-эквивалентность по целевым сценариям. Это не означает бинарную, source-level или API-level совместимость с NSH.

## 2. Стратегия совместимости

Не копировать NSH API имена вслепую. Сначала определить:
- семантику компонента;
- типы и default values;
- accessibility behavior;
- focus and keyboard model;
- event ordering;
- child/slot behavior;
- platform-specific variations;
- cleanup and lifecycle;
- security requirements.

После этого проектировать SolidJS-friendly API с нормальным TypeScript типизированием.

Не стремиться поддержать весь NSH API в одном релизе. Использовать feature matrix и maturity levels по каждой платформе.

## 3. Feature mapping

| Возможность NSH | Целевой эквивалент SolidJS Native | Граница/отличие |
|---|---|---|
| TSX app modules | SolidJS TSX modules | Используем настоящий SolidJS Universal |
| Rust apps | Native Rust extension modules | Rust-only UI authoring не является обязательным v1 |
| Shared reactive engine | SolidJS runtime per native Surface | Нет второй Rust reactive graph |
| Native renderer | Rust retained tree + platform backends | Typed batched bridge |
| Layout | sn-layout over Taffy | Публичный API не привязан к Taffy types |
| Native text | Text subsystem + OS engines | Native input first; custom editor later |
| Native views | PlatformHost/MountSink | Logical nodes не обязаны быть 1:1 OS views |
| Input/focus/gestures | Typed EngineEvent + platform input adapters | Main thread never enters JS |
| Native scrolling | Platform-owned scrolling and engine state | Native frame-rate handling |
| Accessibility | Semantic tree + OS accessibility adapters | Tested independently from drawing |
| Animations | Engine descriptions + native animation driver | No JS per-frame for ordinary transitions |
| Images/resources | Resource pipeline | Background decode, IDs and caching |
| Network/fetch | Capability-backed runtime services | Explicit API compatibility table |
| Navigation/overlays | Engine/base component primitives | Must preserve per-platform back/escape semantics |
| Unstyled base components | sn-base | Behavior only; app owns visual design |
| Styled component library | Optional community/app package | Not coupled to engine |
| Web output | Browser DOM renderer | Browser JS engine, no QuickJS in browser |
| CLI | sn-cli | Gradual evolution; not all commands in first milestone |
| Dev server/HMR | sn-dev | Only after lifecycle/cleanup semantics are proven |
| Package tooling | sn-package | Versioned manifests and hashes |
| Signed runtime modules | Capability-sandboxed packages | Signature does not replace permission enforcement |
| Embedding | AppView/Surface host APIs | Versioned C ABI plus Swift/Kotlin wrappers |
| Headless tests | sn-testing/headless engine | Same runtime/core, no fake browser DOM |
| GPUI backend | Optional platform adapter | Not required for initial parity |
| Terminal UI | Optional TUI adapter | Useful for diagnostics, not core release requirement |

## 4. Core platform parity

Target platforms:

- Windows desktop.
- macOS desktop.
- Linux desktop.
- iOS.
- Android.
- Web.

A platform is not considered supported merely because the Rust crate compiles. Each platform must pass maturity levels and the acceptance checklist in platform-backends.md.

Minimum parity required before stable release:

- same logical app model and TSX component API;
- layout semantics within documented support subset;
- text rendering and native editing;
- pointer/touch/keyboard input appropriate to platform;
- focus and accessibility;
- images/resources;
- scroll containers and virtualizable lists;
- dialogs/overlays/navigation;
- environment values such as scale, safe area, locale and color scheme;
- packaging and lifecycle;
- diagnostics and error reporting.

The exact look may differ according to platform and app styles. The system behavior must feel appropriate to the platform.

## 5. Scope tiers

### Tier 1 — Framework foundation

- runtime and bridge;
- core tree;
- scheduler;
- layout;
- native surface;
- text and input;
- events and cleanup;
- headless testing.

Without Tier 1 there is no framework.

### Tier 2 — App development essentials

- View, Text, Image, Button, TextInput, ScrollView;
- focus, keyboard, accessibility;
- overlays and navigation;
- styles, theme tokens and environment;
- resource loading and async network primitives;
- application lifecycle;
- CLI dev/build/run/test.

This is the first usable app-development toolkit.

### Tier 3 — Full Native Shell alternative

- all target platform backends;
- embedding APIs;
- HMR and diagnostics;
- robust package/bundle format;
- signed dynamic modules;
- capability security and runtime quotas;
- native components extension API;
- broad component behavior foundation;
- platform parity and regression suite.

### Tier 4 — Ecosystem and advanced capabilities

- community component libraries;
- custom native extensions;
- advanced virtualization;
- native rich text/editor components;
- media, maps and platform-specific services;
- optional GPUI/TUI backends;
- advanced developer inspector/profiling;
- optional Rust-authored native components and modules.

## 6. Explicit non-goals versus NSH

The first stable generation does not promise:

- drop-in replacement for NSH source files;
- identical Rust signals/computed/effect API;
- source-compatible NSH compiler transforms;
- Rust-only application authoring;
- every native feature NSH currently exposes;
- every platform backend at once;
- superior runtime performance without benchmark evidence.

These limitations do not prevent a full functional alternative. They prevent the team from hiding costly architectural differences behind vague parity claims.

## 7. License and reuse

The NSH repository is an architectural reference. Do not copy its implementation code or port large code sections until its applicable license and provenance are independently verified. Repository metadata reported a non-standard/NOASSERTION license marker and no root LICENSE file was retrieved in the checked paths.

Reusing concepts such as engine/UI thread separation, typed operations, retained tree, Taffy, platform adapters and test boundaries is different from copying source code. For any future code reuse, record source URL, commit, license, copyright notice and modifications in a third-party notices file.

## 8. Release definition

A public release may claim:

- experimental: feature exists but API/platform semantics may change;
- preview: documented feature works on named targets with known limitations;
- stable: public contract versioned, automated tests pass, upgrade policy exists, platform acceptance checklist completed.

A global "stable" badge is not allowed while a target platform is only experimental. Publish target-specific status.
