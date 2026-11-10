# Platform Backends и Native Embedding

- Статус: целевая архитектура платформенного слоя.
- Связанные документы: target-architecture.md, solidjs-runtime-and-host-protocol.md.
- Основное правило: общая семантика UI, разные нативные реализации.

## 1. Область ответственности

Platform backend — адаптер между платформенно-независимым Solid Native engine и конкретной операционной системой.

Он отвечает за:
- создание и жизненный цикл окна/embedded host view;
- создание, обновление, переупорядочивание и удаление native view resources;
- применение MountBatch;
- преобразование OS events в typed EngineEvent;
- focus, keyboard, pointer, touch, gestures, scroll и IME;
- native text input и selection;
- text shaping/measurement adapter;
- accessibility tree exposure;
- display scale, viewport, safe area, keyboard insets, locale и color scheme;
- clipboard, file dialogs, URL opening и platform notifications через capability interface;
- native resource handles и их teardown;
- UI-thread scheduling и display frame notification.

Platform backend не отвечает за:
- SolidJS reactivity;
- JS compilation и module resolution;
- authoritative UI tree;
- Taffy layout decisions;
- application business state;
- generic event handler execution;
- app package policy.

## 2. Две отдельные абстракции

Не следует сводить всё к одному огромному Platform trait.

### 2.1. PlatformHost

PlatformHost представляет интеграцию с окном/встраиваемым view и OS lifecycle:

- create_surface_host;
- attach;
- detach;
- resize;
- set_scale;
- set_environment;
- dispatch_to_ui_thread;
- request_redraw;
- request_frame_callback;
- dispose.

Методы UI-thread-only должны быть явно обозначены и проверяться в debug builds.

### 2.2. MountSink

MountSink принимает уже рассчитанный MountBatch и применяет его к platform resources:

- apply_batch;
- apply_full_snapshot;
- create/destroy native resource;
- update frame/clip/transform;
- update semantic accessibility state;
- update text/image/media resource;
- apply focus/input configuration;
- apply scroll/animation state.

MountSink получает DTO, opaque handles и immutable snapshots. Он не получает ссылку на QuickJS Runtime или JS Function.

### 2.3. Дополнительные capabilities

Платформенные функции подключаются через отдельные interfaces:

- TextMeasurer;
- TextInputHost;
- ClipboardHost;
- DialogHost;
- FileAccessHost;
- NetworkHost;
- AccessibilityHost;
- AnimationHost;
- NativeComponentRegistry;
- AppLifecycleHost.

Наличие функции зависит от платформы. Общее API возвращает capability result или typed unsupported error. Не использовать пустые заглушки, которые выглядят как успешная операция.

## 3. Представление элементов

### 3.1. Не каждый node — отдельный OS view

Retained UI tree — логическая структура. Platform backend вправе сгруппировать несколько layout/paint nodes в один retained drawing surface.

Рекомендуемая модель:

- контейнеры, простые background/border/transform/clip и текстовые spans могут рисоваться совместно;
- TextInput, native selection controls, embedded map, WebView, video surface и другие OS-integrated controls могут быть отдельными native objects;
- hit-testing остаётся связанным с engine geometry, а platform input передаёт target и local coordinates;
- accessibility semantics могут иметь больше узлов, чем визуальная native view hierarchy;
- platform view creation выполняется только если элементу нужна OS-level behavior.

Это позволяет не создавать сотни HWND/NSView/Android View для каждого декоративного элемента, но сохраняет реальные нативные controls там, где системное поведение критично.

### 3.2. Backend renderer

Backend имеет две cooperating части:

1. Native surface renderer — рисование retained primitives и текстовых snapshots.
2. Native control host — размещение настоящих системных controls и внешних native components.

Обе части используют одни logical frames, clipping, transforms и z-order, рассчитанные engine.

## 4. Windows backend

### 4.1. Цель

Первый полноценный native backend. Разработка ведётся на Windows x64/MSVC, поэтому именно здесь должна появиться первая end-to-end native vertical slice.

### 4.2. Технологии и ownership

- Win32 window/message loop — top-level window и native lifecycle.
- Direct2D — retained 2D drawing surface.
- DirectWrite — font metrics, shaping and glyph rendering.
- Native IME/input method integration — composition, caret, selection and keyboard input.
- Win32 accessibility/UI Automation adapter — semantic exposure.
- Native controls where the engine requires OS-owned semantics or input behavior.

Не делать всю UI hierarchy дочерними HWND. Декоративные элементы должны оставаться retained-drawn primitives. Создание отдельных HWND допускается для native embedded components и специализированных controls.

### 4.3. Обязательные сценарии

- окно создаётся и закрывается без утечек;
- resize и DPI scale обновляют environment/layout;
- мышь, клавиатура, wheel и focus корректно маршрутизируются;
- TextInput обрабатывает IME composition, selection и clipboard;
- z-order, clipping, transforms и hit testing согласованы;
- resize не вызывает синхронный QuickJS execution на UI thread;
- окно может detach/reattach к host при embedding;
- device loss/recreation восстанавливает native resources из engine snapshot.

### 4.4. Windows delivery

Сначала standalone executable. После стабилизации — Rust API для embedding и C ABI для других языков. Не выпускать внешний stable ABI до тестов lifecycle, threading и disposal.

## 5. macOS backend

### 5.1. Технологии

- AppKit — NSWindow, NSView, menu and lifecycle integration.
- Core Animation — retained layers and native animation integration.
- CoreText/CoreGraphics — text shaping and drawing snapshots.
- NSTextInputClient and native text controls — text input/IME.
- NSAccessibility — accessibility semantics.

### 5.2. Требования

- AppKit objects создаются и изменяются только на main thread.
- Engine thread никогда не обращается к AppKit object напрямую.
- Text layout preparation не должна требовать вызова main-thread-only objects; если требуется platform dispatcher, результат кешируется и возвращается асинхронно.
- Retina scale, backing size and logical size не смешиваются.
- Window focus, key equivalents, menu commands и native drag/drop отражаются в typed events.
- macOS close/resize/fullscreen semantics не эмулируются одинаково с Windows, если это противоречит платформенным привычкам.

## 6. Linux backend

### 6.1. Технологии

- GTK 4 — native window/view lifecycle and controls.
- GSK — retained scene drawing.
- Pango — shaping, measurement and text layout.
- AT-SPI/GTK accessibility integration.
- Desktop portals for file chooser/open URL and other sandbox-aware integrations where appropriate.

### 6.2. Требования

- GTK main context owns GTK objects.
- Linux environments vary: Wayland/X11, desktop themes, portals and graphics drivers.
- Backend reports capabilities rather than assuming a specific desktop shell.
- GTK/GSK frame scheduling is coordinated with engine MountBatch.
- Headless tests do not require a running compositor.
- Packaging and runtime dependencies are documented separately for major distributions.

## 7. iOS backend

### 7.1. Технологии

- UIKit — embedding view and native control lifecycle.
- CoreText — text measurement/shaping.
- Core Animation — display-linked and native animations.
- UIAccessibility — accessibility.
- Swift host wrapper plus stable C ABI from Rust.

### 7.2. Требования

- Native views and UIKit calls execute on main thread.
- Touch input, gesture recognizers, safe area, keyboard frame and view-controller lifecycle feed engine environment/events.
- Background/foreground transitions suspend/resume Surface and timers according to policy.
- TextInput must support marked text, dictation and hardware keyboard where available.
- Screen scale and content size category affect measurement/layout.
- Embedding must not require the app to surrender its navigation/controller hierarchy.

## 8. Android backend

### 8.1. Технологии

- Android View system — native view integration and critical controls.
- JNI — Rust to Java/Kotlin boundary.
- StaticLayout or equivalent Android text measurement/layout APIs.
- Editable TextView/EditText semantics for IME, selection and accessibility.
- Android accessibility node provider.
- Choreographer — display frame coordination.

### 8.2. Требования

- UI objects are main-thread owned.
- Rust engine thread communicates by bounded queues and typed batches.
- Density (dp/px), font scale and configuration changes update engine environment.
- Keyboard resize/pan and insets must not double-apply.
- Scroll offsets are restored after layout changes and clamped to content bounds.
- Activity recreation does not leave callbacks pointing to stale Java objects.
- JNI local/global references have explicit ownership and release rules.
- Native library loading and ABI splits are tested on arm64 and supported emulator targets.

## 9. Web backend

### 9.1. Принцип

Web is a first-class target but not a native backend with fake OS APIs. It uses browser primitives:

- browser JavaScript engine;
- DOM;
- browser CSS/layout only where the web adapter intentionally delegates;
- browser accessibility tree;
- native HTML text inputs and text selection;
- browser event loop and requestAnimationFrame.

The same application-facing package should remain available, but renderer implementation is separate.

### 9.2. Renderer strategy

The native Solid renderer and browser DOM renderer are distinct adapters. Shared components should use the supported cross-platform primitives contract. Web-specific components may use DOM features only when they are guarded by platform capability and have a native fallback or documented unsupported behavior.

Do not force web to create a fake native tree in Rust. Do not make native targets depend on browser DOM APIs.

### 9.3. Runtime strategy

The browser backend should use the browser's JS engine rather than embedding QuickJS into a page. Build output is a normal web app/static package. Runtime API compatibility is tested separately because browser built-ins may have different behavior and capabilities.

## 10. Optional backends

GPUI and terminal renderers are optional extension targets. They are not part of the first platform parity promise.

A GPUI backend is useful for embedding in applications already built with GPUI. A TUI backend may be useful for tests, diagnostics and terminal dashboards. Both must implement the same platform-neutral contracts and must not change sn-core semantics.

These backends should be implemented only after MountBatch, layout, input and accessibility interfaces are stable.

## 11. Native components

A native component is a host-registered platform resource, not a custom instruction hidden in JSX.

Registration includes:
- stable component type ID;
- supported platform set;
- typed property schema;
- accepted children/slots policy;
- event schema;
- accessibility mapping;
- resource lifecycle;
- required capabilities;
- thread ownership;
- cleanup contract;
- test provider or unsupported test behavior.

Examples: map, video, web view, native share sheet, file picker, platform tab bar.

Native component implementation must not hold arbitrary JS runtime references unless the component contract explicitly requires a callback registry handle.

## 12. Embedding model

The same EngineSession runs standalone or embedded.

### 12.1. Rust embedding

Rust host creates a Surface, passes platform host adapter and app configuration, attaches to window/view, grants capabilities and manages lifecycle.

### 12.2. C ABI

The ABI exposes opaque handles and fixed-size versioned structs only.

Required exports eventually include:
- create_surface;
- destroy_surface;
- attach_surface;
- detach_surface;
- resize_surface;
- set_environment;
- send_input_event;
- grant_capability;
- revoke_capability;
- register_native_component;
- get_last_error / diagnostics callback.

No Rust references, Vec, String, Result, enum layout or trait objects cross C ABI. All errors have stable integer codes and optional owned UTF-8 diagnostic buffers with matching free function.

### 12.3. Swift wrapper

Swift wrapper owns an opaque handle, provides a UIView/NSView wrapper where appropriate, dispatches lifecycle on main thread and disposes deterministically. No Swift code calls QuickJS directly.

### 12.4. Kotlin wrapper

Kotlin/Java wrapper owns a native handle, attaches a custom View/ViewGroup, registers JNI callbacks and disposes references during detach/Activity destruction. JNI entry points validate handle generation and thread.

### 12.5. Compatibility

Embedding ABI version is independent from Cargo crate versions. Host must check the runtime ABI before using the Surface. Breaking changes require a new ABI major or explicit compatibility shim.

## 13. Platform capability matrix

| Capability | Windows | macOS | Linux | iOS | Android | Web |
|---|---|---|---|---|---|---|
| Window / embedding | Win32 | AppKit | GTK 4 | UIKit | Android View | DOM |
| Text shaping | DirectWrite | CoreText | Pango | CoreText | StaticLayout | Browser |
| Editable text | Native host | Native host | GTK | UIKit | Android View | HTML input |
| Accessibility | UI Automation | NSAccessibility | AT-SPI/GTK | UIAccessibility | Android accessibility | Browser tree |
| Clipboard | Win32 | AppKit | GTK/portal | UIKit | Android | Browser permissions |
| File picker | Win32/dialog | AppKit | Portal/GTK | UIKit | Storage Access Framework | Browser |
| Frame clock | OS/compositor | Display link | GTK frame clock | CADisplayLink | Choreographer | RAF |
| Native graphics | Direct2D | Core Animation/Graphics | GSK | Core Animation/Graphics | Canvas/Render host | DOM/CSS |

Matrix is a target. Blank implementation is not evidence of support. Every capability must have a test status per target.

## 14. Platform maturity levels

Use the same maturity levels across targets:

- L0 — compiles; no runtime proof.
- L1 — creates an empty Surface and tears it down.
- L2 — renders basic layout/text and handles resize.
- L3 — input, focus, native text entry, scroll and accessibility basics.
- L4 — navigation, overlays, images/resources, lifecycle and embedding.
- L5 — production hardening, packaging, automated platform tests and performance baseline.

A platform is not marked supported at L1/L2. Public claims must distinguish preview, experimental and stable support.

## 15. Implementation order

Recommended order:

1. Headless backend and sn-core/bridge integration. This allows most logic to be tested without OS UI.
2. Windows backend vertical slice: one window, layout containers, text, button, input, resize, focus and disposal.
3. Web backend with browser-native renderer and API parity tests.
4. Linux backend; validate GTK/Pango and distribution packaging.
5. macOS backend; validate AppKit/CoreText and embedding.
6. Android backend; validate JNI, density, IME and Activity lifecycle.
7. iOS backend; validate UIKit, keyboard, safe areas and Swift embedding.
8. Optional GPUI/TUI backends if real consumers justify them.

The order may be adjusted for available build hosts and hardware, but no target may be declared complete based only on cross-compilation.

## 16. Platform backend acceptance checklist

Before a target is considered complete:

- [ ] Clean create/attach/resize/detach/dispose lifecycle.
- [ ] No UI-thread QuickJS execution or blocking waits.
- [ ] Basic layout and DPI/logical-unit correctness.
- [ ] Text shaping and native editable text.
- [ ] IME composition, selection and clipboard.
- [ ] Focus traversal, keyboard and pointer/touch input.
- [ ] Scroll physics and viewport changes.
- [ ] Native accessibility semantics and actions.
- [ ] Resource decode/load/cancel/release.
- [ ] Safe-area/window insets and environment changes.
- [ ] Native component lifecycle.
- [ ] Device/context loss or view recreation recovery.
- [ ] Integration tests on the target OS.
- [ ] Performance and memory baseline.
- [ ] Embedding sample app and distribution documentation.
