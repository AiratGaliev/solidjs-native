# NSH Implementation Reference and Reuse Map

- Статус: engineering reference; не является спецификацией совместимости API.
- Upstream: https://github.com/longbridge/nsh
- Pinned source snapshot: [c16a7009674797eda8781076145c696e22144184](https://github.com/longbridge/nsh/commit/c16a7009674797eda8781076145c696e22144184), observed as the `main` branch head on 2026-11-10.
- Проверка upstream-документации и исходников: 2026-11-10.
- Product direction: [ADR-0008](../decisions/0008-tauri-class-application-framework.md).
- Tauri-class application scope: [Tauri replacement scope](tauri-replacement-scope.md).
- Link catalog and provenance rules: [Upstream references](../reference/upstream-references.md).
- Consolidated dependency decisions: [Architecture Reuse and Dependency Adoption Map](dependency-adoption.md).

## 1. Роль NSH в проекте

NSH — основной implementation reference для native UI subsystems SolidJS Native. Он помогает изучать проверенные решения в следующих областях:

- retained UI tree, commit pipeline и mount batches;
- Taffy integration, layout invalidation, measurement и caching;
- platform rendering и native resource lifecycle;
- text shaping, measurement, editable text и IME;
- pointer/keyboard/touch input, hit testing, focus, scrolling и gestures;
- accessibility semantic tree и системные accessibility providers;
- application/window integration, embedding, runtime packages, tooling и capability enforcement.

NSH **не является продуктовой целью SolidJS Native**. Продуктовая цель — самостоятельный Tauri-class application framework, способный заменить Tauri 2 в выбранных сценариях Codogma Engine и использоваться сторонними SolidJS/TypeScript приложениями. См. [Tauri replacement scope](tauri-replacement-scope.md).

NSH is not an obligatory runtime dependency and does not define SolidJS Native's public API names, package format or version baseline.

### Relationship to GPUI Kit / GPUI Shell

NSH and GPUI Kit are related, not independent reference projects. NSH's current project documentation identifies GPUI Shell as the starting point for its original mobile plugin layer. NSH then evolved into a multi-platform framework for whole applications. It chose platform-owned UI backends for standalone apps, while keeping an optional GPUI backend for embedding NSH `AppView` inside GPUI and GPUI Kit applications.

- [NSH project history and positioning](https://github.com/longbridge/nsh/blob/c16a7009674797eda8781076145c696e22144184/website/docs/index.md)
- [NSH platform/backend matrix](https://github.com/longbridge/nsh/blob/c16a7009674797eda8781076145c696e22144184/website/docs/platforms.md)
- [NSH GPUI embedding backend](https://github.com/longbridge/nsh/blob/c16a7009674797eda8781076145c696e22144184/website/docs/gpui.md)
- [GPUI Kit comparison](gpui-kit-comparison.md)

Do not attribute the GPUI renderer to NSH's standalone AppKit/Win32/GTK backends. The GPUI backend is a separate host/embedding option.

## 2. Архитектурные границы, которые нельзя переносить автоматически

NSH и SolidJS Native имеют разные application runtime contracts:

| Область | NSH | SolidJS Native |
|---|---|---|
| Reactivity | Rust-native reactive graph с Solid-like semantics | Настоящий upstream SolidJS runtime |
| JSX/TSX | Собственный JSX compilation pipeline с вызовами native renderer | Официальный Solid compiler, Universal generation |
| Renderer | NSH native renderer и собственная модель операций | `solid-js/universal` adapter и собственный versioned host protocol |
| Tree ownership | NSH core shadow/retained tree | Rust `sn-core` authoritative tree после атомарного commit |
| VM | QuickJS для TSX apps в NSH | QuickJS-NG через rquickjs для native targets |
| Platform commit | NSH MountBatch и platform sink contracts | Собственный immutable MountBatch и Platform API |
| UI thread policy | В основном отдельный UI thread; отдельные платформенные исключения документированы NSH | UI thread не исполняет пользовательский JS и не ждёт engine work синхронно |
| Application APIs | NSH runtime modules и platform APIs | Собственный публичный Application Bridge и plugin contract |

Не переносить NSH Rust reactive graph, JSX compiler, renderer callbacks или внутренние types в SolidJS Native. Можно перенимать наблюдаемое поведение и алгоритмы после адаптации к настоящему SolidJS и нашим ownership/threading invariants.

NSH documentation описывает ограниченные ожидания layout на некоторых resize paths. Это не отменяет более строгий контракт SolidJS Native: platform UI thread не должен синхронно ждать QuickJS или выполнения engine work.

## 3. Реализации, которые следует изучить

| Подсистема | Что подтверждено в NSH | Что исследовать для SolidJS Native |
|---|---|---|
| Core/tree | Engine-owned shadow tree, commit boundary, typed internal operations | Transaction model, tree invariants, node lifecycle and recovery |
| Batching | One commit produces ordered MountBatch; one platform dispatch/hop per commit on documented backends | Atomic application, coalescing, snapshot recovery and back-pressure |
| Layout | Taffy behind NSH-specific LayoutTree, custom measurement, cache and dirty-boundary relayout | Taffy adapter structure, measure contract, cache keys, invalidation boundaries |
| Text | Engine-side prepare/measure/layout; platform text engines; immutable result passed for drawing | Text model, shaping result DTOs, font fallback, measurement caching and hit testing |
| Windows renderer | Retained NshView tree drawn to Direct2D flip-model swap chain; ordinary nodes do not require one HWND each | Retained drawing, clipping, z-order, transforms, partial invalidation, device loss |
| Windows editable text | Windowless RichEdit via ITextServices2/ITextHost2, Direct2D drawing and Windows text-input integration | Native editor host seam, IME, selection, caret, clipboard and lifecycle |
| Windows accessibility | UI Automation provider tree, semantic properties and supported control patterns | Provider lifetime, semantic mapping, focus and notification behavior |
| Input/focus | Core hit testing and input decisions, platform event normalization, engine dispatch | Event normalization, pointer capture, gesture arbitration, keyboard routing |
| Scrolling/animation | Native scroll state/physics and animation/frame coordination | UI-thread animation driver, frame clock, scroll lifecycle without per-frame JS |
| Apple platforms | UIKit/AppKit and CoreText/Core Animation based integrations | Main-thread ownership, text input, accessibility and view-controller/window lifecycle |
| Android | Android View/JNI, StaticLayout and Choreographer integration | JNI reference ownership, Activity recreation, density and IME |
| Linux | GTK 4/GSK/Pango and desktop integration | Main-context ownership, portals, accessibility and packaging |
| App/runtime services | Native modules, application/window APIs, capabilities, embedding and tooling | Application Bridge, plugin lifecycle, app host, CLI and public embedding contracts |

### 3.1. Windows renderer and platform threading

Useful source paths:
- [Windows backend overview](https://github.com/longbridge/nsh/blob/c16a7009674797eda8781076145c696e22144184/docs/dev/windows.md)
- [Windows platform crate](https://github.com/longbridge/nsh/tree/c16a7009674797eda8781076145c696e22144184/crates/nsh-platform-windows)
- [Direct2D renderer](https://github.com/longbridge/nsh/blob/c16a7009674797eda8781076145c696e22144184/crates/nsh-platform-windows/src/render.rs)
- [Retained view model](https://github.com/longbridge/nsh/blob/c16a7009674797eda8781076145c696e22144184/crates/nsh-platform-windows/src/view.rs)
- [UI Automation provider](https://github.com/longbridge/nsh/blob/c16a7009674797eda8781076145c696e22144184/crates/nsh-platform-windows/src/access/uia.rs)

NSH draws its ordinary Windows view tree into a flip-model swap chain with Direct2D. It does not create a child HWND for every ordinary logical node; native components that require their own windows are a separate case. The engine submits mount changes and the Windows window applies queued batches. Device/resource recreation and partial invalidation have platform-specific handling.

Study this as a proven option, not as a binding decision that every SolidJS Native backend must use identical rendering internals.

### 3.2. Text input is not the same as text rendering

Useful source paths:
- [NSH text architecture](https://github.com/longbridge/nsh/blob/c16a7009674797eda8781076145c696e22144184/docs/dev/text.md)
- [Windows text shaping](https://github.com/longbridge/nsh/blob/c16a7009674797eda8781076145c696e22144184/crates/nsh-platform-windows/src/text.rs)
- [Windows text input host](https://github.com/longbridge/nsh/blob/c16a7009674797eda8781076145c696e22144184/crates/nsh-platform-windows/src/text_input.rs)
- [Windowless RichEdit host](https://github.com/longbridge/nsh/blob/c16a7009674797eda8781076145c696e22144184/crates/nsh-platform-windows/src/richedit.rs)

Keep two separate contracts:
- Text rendering/shaping produces measured and positioned data using platform text engines. On Windows NSH uses DirectWrite and records glyph runs for drawing.
- Editable text uses a specialized host. On Windows NSH hosts windowless RichEdit text services (`ITextServices2` / `ITextHost2`) and integrates with Windows text input/TSF. This is not a generic renderer primitive and must be evaluated as an input-control implementation.

### 3.3. Accessibility

Useful source paths:
- [NSH accessibility API](https://github.com/longbridge/nsh/blob/c16a7009674797eda8781076145c696e22144184/website/docs/accessibility.md)
- [Windows UI Automation adapter](https://github.com/longbridge/nsh/blob/c16a7009674797eda8781076145c696e22144184/crates/nsh-platform-windows/src/access/uia.rs)
- [Windows accessibility bridge](https://github.com/longbridge/nsh/blob/c16a7009674797eda8781076145c696e22144184/crates/nsh-platform-windows/src/access.rs)

Study the split between cross-platform semantic metadata and OS-specific exposure. On Windows NSH supplies a UI Automation provider tree; other platforms use their own native accessibility mechanisms. SolidJS Native still needs independent semantics, focus and action contracts plus real screen-reader tests.

### 3.4. Layout

Useful source paths:
- [NSH layout tree](https://github.com/longbridge/nsh/blob/c16a7009674797eda8781076145c696e22144184/crates/nsh-core/src/core/layout_tree.rs)
- [NSH core text and measurement](https://github.com/longbridge/nsh/blob/c16a7009674797eda8781076145c696e22144184/crates/nsh-core/src/text.rs)
- [Taffy](https://github.com/DioxusLabs/taffy)

Do not equate using the Taffy crate with implementing a layout system. NSH wraps Taffy in its own tree, measurement callbacks, caching, invalidation and platform-independent geometry model. Study those boundaries and test cases. Implement our own adapter around the currently selected stable Taffy version; do not expose Taffy internal types as the public style API.

## 4. Reuse workflow

Every candidate is classified before implementation:

- **Adopt** — a compatible public component or design can be used substantially as-is after API/version/license review.
- **Adapt** — preserve a useful algorithm or behavior, but implement it against SolidJS Native contracts and ownership rules.
- **Reference** — learn from its architecture, tests and trade-offs; do not copy implementation.
- **Reject** — deliberately do not carry the approach forward; record why.

A reuse record must include:
1. subsystem and intended behavior;
2. classification and decision rationale;
3. upstream URL and exact commit SHA;
4. exact file paths and attribution/copyright notices for code reuse;
5. applicable license and NOTICE obligations;
6. local modifications and compatibility changes;
7. tests and platform evidence that validate the adapted behavior.

Repository-level license metadata does not automatically cover bundled third-party code, assets or separately licensed contributions. Inspect file-level provenance and applicable notices before porting.

## 5. License and source provenance

The checked NSH repository contains a root [LICENSE-APACHE](https://github.com/longbridge/nsh/blob/c16a7009674797eda8781076145c696e22144184/LICENSE-APACHE), and its workspace manifest declares `license = "Apache-2.0"` ([Cargo.toml](https://github.com/longbridge/nsh/blob/c16a7009674797eda8781076145c696e22144184/Cargo.toml)).

NSH-authored source covered by Apache-2.0 may be copied or modified subject to that license's requirements, including preserving applicable notices, providing the license, and marking modified files. This is not blanket permission to reuse all content in the repository: inspect the exact file, bundled dependencies, assets and additional notices before a port. This note is engineering provenance guidance, not legal advice.

No NSH source code is copied into SolidJS Native by this reference document or the other documentation changes in this update.
