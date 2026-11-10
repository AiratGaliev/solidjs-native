# SolidJS Native — целевая архитектура

- Статус: целевая архитектурная спецификация (Target Architecture).
- Версия документа: 1.0.
- Дата: 2026-11-10.
- Кодовая база: текущий прототип M0; спецификация описывает конечную систему и явно отделяет её от уже реализованного.
- Основной ориентир: Native Shell (NSH), но без копирования его внутренней реализации и без отказа от настоящего SolidJS.

## 1. Назначение и ожидаемый результат

SolidJS Native — нативный UI runtime и toolkit для создания приложений на TypeScript/TSX с использованием настоящего SolidJS. Один и тот же прикладной код должен запускаться в нативных приложениях Windows, macOS, Linux, iOS и Android, а также в браузере. На нативных платформах UI должен быть встроен в приложение и взаимодействовать с системными окнами, вводом, текстом, доступностью, навигацией, файлами и другими платформенными возможностями без обязательного WebView.

Проект должен стать функциональной альтернативой Native Shell, а не демонстрацией SolidJS внутри QuickJS. Конечный результат включает:

- SolidJS как официальный и единственный JS reactive runtime для TSX-приложений.
- QuickJS-NG как встраиваемый JS engine для нативных платформ.
- Rust как владелец retained UI tree, layout, commit pipeline, ресурсов, событийной маршрутизации и platform integration.
- Нативные backend-адаптеры для системных окон, представлений, текста, ввода и доступности.
- Отдельный web backend, использующий браузер и его DOM, а не эмуляцию native UI в canvas.
- Единый компонентный API и типизированные декларации для TypeScript.
- Инструменты сборки, запуска, тестирования, hot reload и упаковки.
- Контролируемую загрузку динамических UI-модулей с capability-based security.
- Встраивание в существующие Rust, Swift, Kotlin, C/C++ и другие host-приложения через стабильные ABI/API adapters.

Проект не должен быть копией исходного кода NSH. NSH используется как архитектурный референс: разделение engine/UI threads, Rust-owned UI tree, пакетирование mount operations, layout через Taffy, платформенные backend-слои, базовые unstyled-компоненты, sandbox и единый CLI.

## 2. Главная архитектурная идея

Система состоит из четырёх независимых, но согласованных частей:

1. **SolidJS application runtime.** Выполняет пользовательские компоненты, signals, memos, effects, context, owners, cleanup и обработчики событий.
2. **Solid Native renderer adapter.** Реализует контракт SolidJS Universal. Он превращает вызовы renderer в host mutations и поддерживает минимальные JS-объекты-обёртки, необходимые SolidJS для обхода дерева.
3. **Rust UI engine.** Принимает пакет изменений, валидирует его, обновляет единственный authoritative retained tree, вычисляет layout, разрешает ресурсы и формирует mount batch.
4. **Platform backend.** Применяет mount batch к нативным представлениям и системным API, принимает input и возвращает нормализованные события в engine.

Основное правило владения состоянием:

- SolidJS владеет реактивным состоянием приложения.
- Rust владеет структурой и платформенным состоянием UI.
- Platform backend владеет реальными OS objects и системными ресурсами.
- JS renderer adapter владеет только временным reconciliation mirror, необходимым для контракта Solid Universal; этот mirror не является вторым источником истины для layout, styles, frames, accessibility или фактического native tree.

В проекте запрещено создавать вторую реактивную систему на Rust, имитирующую SolidJS. Rust получает уже вычисленные изменения от SolidJS. Прикладная логика не должна вынужденно работать с двумя конкурирующими графами реактивности.

## 3. Цели и ограничения

### 3.1. Обязательные цели

- Использовать upstream SolidJS, а не форк Solid и не переписывать его reactive graph.
- Поддержать актуальный SolidJS 1.x API и Solid Universal.
- Сохранять TypeScript/TSX compile-time semantics и нормальные type declarations.
- Не требовать браузерного DOM или WebView для native UI.
- Не блокировать UI thread выполнением пользовательского JS.
- Не пересылать отдельный mount/update вызов на каждое свойство через межпоточный канал.
- Не сериализовать production host operations в JSON.
- Не выполнять layout, text shaping и пользовательский JS на UI thread.
- Обеспечить детерминированную очистку обработчиков, timers, resources и runtime contexts.
- Иметь headless режим для тестов без создания системного окна.
- Поддерживать отдельный backend на каждой целевой OS.
- Поддерживать платформенные функции, не сводя API к наименьшему общему знаменателю.
- Обеспечить предсказуемую модель версионирования протокола, пакетов и runtime compatibility.
- Измерять производительность на репрезентативных UI-сценариях, а не заявлять преимущество на основании архитектурных предположений.

### 3.2. Не является целью первой версии

- Собственный browser engine.
- Полный CSS layout/rendering engine.
- Совместимость со всем Web Platform API.
- React compatibility или общий VDOM.
- Автоматическая конвертация произвольного DOM приложения в native UI.
- Собственный язык разметки вместо TSX.
- Обязательная поддержка GPUI, TUI, game engine или WebGPU как единственного renderer.
- Полная замена SwiftUI, Jetpack Compose, AppKit, UIKit или WinUI для платформенных экранов.
- Произвольный доступ исполняемых модулей к файловой системе, процессам или сети.
- Первая версия с несколькими конкурентными JS engines.
- Обязательный Rust-only UI authoring path. Rust используется для engine, platform extensions и native modules; основной прикладной декларативный интерфейс — SolidJS/TSX.

### 3.3. Что означает «лучше NSH»

«Лучше» должно подтверждаться конкретными критериями:

- upstream SolidJS без собственной реализации его реактивного графа;
- более узкая и документированная граница между JS runtime и Rust engine;
- отсутствие JSON в штатном renderer-to-host transport;
- строгая транзакционность host updates;
- единая модель ownership и потоков;
- совместимость с обычными инструментами TypeScript/Vite;
- явные capability boundaries и отсутствие неявных системных привилегий;
- воспроизводимые headless и platform integration tests;
- модульная архитектура без зависимости core от QuickJS или конкретной OS;
- измеряемые startup, commit latency, memory и input latency.

Это архитектурные гипотезы, а не доказательство превосходства. NSH переносит Solid-подобный reactive algorithm в Rust и может иметь меньшие JS runtime расходы. SolidJS Native сознательно принимает затраты QuickJS execution ради настоящего upstream SolidJS и его экосистемы. Нельзя обещать производительность выше NSH без прямых сопоставимых benchmark-ов.

## 4. Что берём у NSH и что проектируем иначе

### 4.1. Сохраняем как архитектурные идеи

- Разделение engine thread и platform UI thread.
- Один изолированный UI engine/runtime на один Surface или AppView instance.
- Rust-owned shadow/retained tree.
- Одна транзакция изменений на JS task/commit boundary.
- Пакет MountBatch от engine к platform.
- Taffy как layout engine.
- Отдельные backend crates на целевые платформы.
- Нативные text input, scrolling, clipboard, accessibility и system integration.
- Native animation driver для анимаций, не требующих JS на каждом frame.
- Unstyled base components отдельно от визуально оформленной component library.
- Стабильное embedding API.
- Development server, hot updates, package format и host-controlled capabilities.

### 4.2. Отличия SolidJS Native

- SolidJS остаётся реальным JS runtime dependency, а не только алгоритмом, переписанным на Rust.
- JSX компилируется официальным Solid compiler с generate=universal и moduleName=solid-native.
- JS renderer adapter реализует upstream Solid Universal contract.
- Rust не отслеживает зависимости Solid signals и не переисполняет JS expressions сам.
- Между JS adapter и Rust tree используется typed, versioned, batched protocol.
- Rust core не знает о QuickJS types. QuickJS-specific APIs находятся внутри runtime crate.
- Веб-платформа использует browser-native JS engine и DOM backend, а не встраивает QuickJS в браузер.
- Первой реализацией является TSX runtime; Rust native modules подключаются как расширения с явными типами, а не как вторая модель пользовательского UI.

## 5. Слои и зависимости

### 5.1. Логическая схема

Application TSX
→ SolidJS compiler
→ SolidJS runtime / Universal renderer
→ Solid Native host adapter
→ Host operation batch
→ Rust runtime bridge
→ sn-core retained UI tree
→ sn-layout / text / resource / accessibility services
→ MountBatch
→ platform backend
→ OS-native views and APIs

Input идёт обратно:

OS event
→ platform normalization
→ engine event queue
→ event target + handler ID
→ QuickJS event handler
→ Solid signals/effects
→ next host batch
→ native mount updates

### 5.2. Целевая структура workspace

Структуру нужно вводить постепенно. Не создавать десятки пустых crates до появления реальных границ ответственности.

- crates/sn-types — общие идентификаторы, units, geometry, error codes и компактные typed values, когда они начнут использоваться несколькими слоями.
- crates/sn-core — authoritative UI tree, node lifecycle, typed properties, transaction validation, tree invariants, render-independent state.
- crates/sn-layout — адаптер Taffy, style normalization, dirty propagation, measurement contracts и layout snapshots.
- crates/sn-runtime-api — VM-independent contracts для runtime lifecycle, timers, host callbacks и diagnostics; не содержит QuickJS types.
- crates/sn-runtime-quickjs — QuickJS-NG/rquickjs integration, module loader, JS globals, runtime limits, pending jobs и callback registry.
- crates/sn-bridge — wire protocol, decoder, validation, transaction application и bridge tests.
- crates/sn-engine — orchestration: runtime, tasks, input dispatch, commits, layout, resources, frame scheduling и mount batches.
- crates/sn-platform-api — PlatformHost, MountSink, TextMeasurer, WindowHost, InputSource, Clipboard, dialogs и accessibility contracts.
- crates/sn-platform-windows — Windows backend.
- crates/sn-platform-macos — macOS backend.
- crates/sn-platform-linux — Linux backend.
- crates/sn-platform-ios — UIKit backend and iOS embedding.
- crates/sn-platform-android — Android Views backend and JNI embedding.
- crates/sn-platform-web — DOM backend compiled to WebAssembly or a browser-side host, according to the proven runtime design.
- crates/sn-base — behavior-oriented base controls, focus/navigation/overlay primitives and accessibility patterns, independent from app visual theme.
- crates/sn-cli — init/dev/build/test/package/doctor commands.
- packages/solid-native — TypeScript package exposing custom renderer, host components, runtime APIs and declarations.
- packages/solid-native-base — optional unstyled Solid components and behaviors, only after primitive APIs stabilize.
- examples — counter, forms, lists, navigation, image loading, embedding, multi-window and platform examples.

A crate is extracted only when it has a stable public contract, independent tests or a genuinely separate compilation/platform boundary. An implementation module inside sn-core is preferable to an unnecessary crate split.

### 5.3. Dependency direction

- sn-types has no dependency on runtime or platform.
- sn-core depends only on shared types and narrowly selected pure-Rust utilities.
- sn-layout depends on sn-core abstractions and Taffy; sn-core must not depend on Taffy APIs.
- sn-runtime-quickjs depends on sn-runtime-api and the QuickJS binding.
- sn-bridge depends on shared types and sn-core application contracts; it must not depend on a platform implementation.
- sn-engine composes core, runtime API, bridge, layout and platform API.
- platform implementations depend inward on platform API and engine-facing DTOs, never the reverse.
- solid-native package depends on solid-js and exposes the renderer adapter.
- sn-core must never import rquickjs, JS values, HWND, AppKit, UIKit, JNI, GTK or winit types.

Запрещённые зависимости: core → runtime; core → platform; core → TypeScript-generated code; QuickJS API → public UI tree API; platform crate → SolidJS.

## 6. Владение состоянием

### 6.1. SolidJS side

SolidJS owns:

- signals, memos and effects;
- component execution and props tracking;
- context/owners and cleanup;
- callback functions and closures;
- application state and domain state;
- application-level async tasks;
- event handlers before dispatch.

Rust does not inspect the dependency graph and does not attempt to infer which expression must rerun.

### 6.2. Rust engine side

Rust owns:

- node identity and lifecycle after commit;
- parent/child relationship in authoritative tree;
- node kind and normalized property values;
- style and layout state;
- resolved geometry and clipping;
- event handler metadata and native event routing IDs;
- accessibility semantics and focus state;
- scroll state, native animation state and resource handles;
- per-surface runtime limits and capabilities;
- pending mount operations and platform resources.

Rust does not retain arbitrary JS values in sn-core. JS callback references are retained in the runtime-owned callback registry and referenced from core only by opaque handler IDs.

### 6.3. Platform side

Platform backend owns:

- window and native view objects;
- OS event registrations and platform subscriptions;
- actual native text controls and text input sessions;
- GPU/device/context resources where applicable;
- display scale, safe areas, keyboard insets and platform metrics;
- OS-specific animation drivers;
- native accessibility objects;
- native image/video/webview instances and handles.

No platform object crosses into sn-core as a concrete OS type. Core holds opaque resource identifiers or platform-independent properties.

## 7. Threading and scheduling

### 7.1. Threads

- **Platform UI thread:** owns OS UI objects and event loop. Never executes arbitrary app JS. Never blocks waiting for JS, layout or resource I/O.
- **Engine thread:** owns one QuickJS runtime/context, executes SolidJS, processes engine tasks, applies host batches, computes layout and emits mount batches.
- **Worker threads:** decode images, perform file/network I/O, prepare expensive assets and other jobs that do not require UI thread affinity.
- **GPU/render thread:** optional per backend. It exists only where the platform rendering API benefits from separate ownership. Do not introduce it before measurements justify it.

For one native Surface, exactly one engine thread is the owner of its QuickJS runtime. A QuickJS context is not accessed concurrently. All cross-thread interactions use bounded queues or platform-safe dispatchers.

### 7.2. One turn, one commit

A normal interaction follows this sequence:

1. Platform receives a pointer, keyboard, text, gesture or accessibility event.
2. Platform normalizes it into an EngineEvent with surface ID, node ID, event kind, timestamp and relevant coordinates/modifiers.
3. Event is enqueued for the engine thread; platform UI thread returns immediately.
4. Engine resolves event target and handler ID, then invokes the callback in QuickJS.
5. SolidJS updates its own signals and executes only the affected effects/bindings.
6. Renderer adapter accumulates host operations.
7. Runtime drains pending Promise jobs/microtasks according to the documented scheduler.
8. At the task boundary, engine flushes one operation batch.
9. Rust validates and applies the batch as a transaction.
10. Layout and derived services update only dirty regions/nodes.
11. Engine creates one ordered MountBatch for the resulting revision.
12. Platform applies the MountBatch in one UI-thread dispatch.
13. Native animation and scrolling continue under platform control without JS per frame, unless the application explicitly subscribes to frame events.

The exact microtask and task boundary must be specified by tests. Do not assume that a call to a JS handler is the only source of pending updates: Promise jobs, queueMicrotask, timers and scheduler jobs can create follow-up work.

### 7.3. UI thread non-blocking invariant

The platform main thread must never synchronously call into QuickJS. It may answer OS synchronous queries only from state already mounted or cached on the platform side: max text length, whether a control can accept input, whether a swipe is allowed, current focusability and similar local properties.

When a synchronous platform decision depends on app logic, the decision must be represented by a prop or policy that was committed earlier. The engine can update that policy asynchronously after the event.

### 7.4. Surface isolation

Each Surface/AppView owns an isolated engine session, JS runtime/context, event queue, UI tree, resource scope, capability set and diagnostics context. No global singleton may hold app state or callbacks. Multiple surfaces in one host process must not share node IDs or VM heaps accidentally.

## 8. Host tree and node model

### 8.1. Retained tree

sn-core contains one authoritative retained tree per surface. It stores logical UI structure, not direct native object pointers.

Node kinds initially:

- Element with a registered tag/kind.
- Text with a text payload or text-run reference.
- NativeComponent with a registered component type and typed properties.
- Portal/Overlay root attachment, represented without violating logical ownership.
- Resource-backed nodes such as Image or Video only after their lifecycle contract is defined.

Do not use arbitrary strings as an unrestricted instruction language for platform backends. Tags resolve through a registry to known semantic kinds and capability requirements.

### 8.2. Node identity

Node handles must be stable from creation until removal and unique within a Surface. Initial wire-level IDs may use a monotonically increasing integer with an explicit safe-integer bound, because the adapter must keep the token in a JS value without precision loss. Rust validates the bound and rejects duplicates, stale references and IDs from another surface.

Before handle reuse is introduced, switch to a generation-aware handle scheme. Reusing an ID without a generation is forbidden because delayed events or MountBatch items could target a newly created node.

The JS wrapper contains the opaque stable ID and only the minimal parent/children links required by Solid Universal traversal. The wrapper does not contain computed layout, final style, platform frames, accessibility tree or duplicated runtime state.

SolidJS uses fine-grained reactive updates. SolidJS Native must not introduce a React-style virtual DOM or a full-tree diff as an intermediate layer. The Universal renderer writes only the host mutations caused by changed reactive computations into the current batch.

### 8.3. Tree invariants

- Every live node belongs to at most one parent.
- Roots have no parent and are present in the root registry.
- A child cannot be its own ancestor.
- An insertion anchor must belong to the specified parent.
- Moving a node detaches it from the old location before calculating the new insertion index.
- A removal detaches and destroys the entire owned subtree, unless a separately specified portal/resource ownership rule says otherwise.
- Every committed node ID maps to exactly one node.
- Text nodes cannot own children.
- Element/component child acceptance is defined by its registered type.
- Removed nodes cannot receive later events.
- Batch application preserves operation order.
- Failed batches do not leave a partially mutated tree.

All invariants must be tested in sn-core, bridge integration tests and randomized/property-based tree mutation tests.

## 9. Renderer-to-host protocol

### 9.1. Design goals

The protocol is an internal versioned ABI between the SolidJS adapter and Rust engine. It is not a public app API and must not expose QuickJS types.

It must support:

- create element and text;
- update text;
- set/remove typed properties;
- insert, move and remove nodes;
- bind/unbind native event metadata;
- attach resource references;
- commit boundaries and revision acknowledgement;
- protocol version negotiation;
- validation errors with operation index and node ID;
- bounded payload size and operation count;
- failure atomicity.

### 9.2. Transport

Production transport must not be JSON. Preferred shape:

- JS adapter accumulates compact typed operation records in a per-surface buffer.
- Strings and variable-size typed values are stored in a side arena referenced by record offsets.
- One explicit flush transfers one ArrayBuffer/typed byte span to a native QuickJS binding.
- Rust decodes the binary batch with strict bounds and version validation.
- Rust applies the batch transactionally and returns a compact acknowledgement containing committed revision, created IDs if required, and structured error details.
- The exact byte layout is documented and covered by golden tests.

Do not use raw memory pointers into a JS heap after the call returns. The native binding must copy or take ownership of the byte payload before JS can mutate or release it.

A JSON bridge may exist only as an early debugging adapter or external diagnostics format. It is not the shipping protocol.

### 9.3. Batch header

The initial protocol header should contain:

- magic bytes;
- protocol major/minor version;
- flags;
- surface/session identifier;
- transaction sequence;
- expected base tree revision;
- operation count;
- total byte length;
- string/value section offsets and lengths.

All integer widths, byte order, maximum lengths and alignment must be fixed. The decoder must reject unknown major versions, impossible offsets, integer overflow, truncated records, out-of-range handles and oversized batches.

Minor versions may add backward-compatible optional records. A major version changes record semantics or representation and requires explicit runtime compatibility.

### 9.4. Operation set

Initial operation vocabulary:

- CreateElement(nodeId, elementKind)
- CreateText(nodeId, initialText)
- ReplaceText(nodeId, textValue)
- SetProperty(nodeId, propertyKey, typedValue)
- RemoveProperty(nodeId, propertyKey)
- InsertNode(parentId, nodeId, optionalAnchorId)
- RemoveNode(nodeId)
- BindEvent(nodeId, eventKind, handlerId, eventFlags)
- UnbindEvent(nodeId, eventKind)
- AttachResource(nodeId, resourceId, resourceKind)
- DetachResource(nodeId, resourceId)

Do not encode JS functions as strings or serialize closures. The function remains in the QuickJS callback registry. Rust stores only an opaque handler ID and the event metadata required by the platform.

### 9.5. Typed property values

The protocol must not be a generic object dump. Define a stable value algebra:

- Null
- Boolean
- Number (finite values only unless a field explicitly permits NaN/Infinity)
- UTF-8 string
- Enum/interned identifier
- Point/Size/Rect/Insets
- Color
- Length (points, dp/logical px, percent, auto, min/max-content as supported)
- Transform
- Typed arrays for supported vector-like values
- Structured layout/text/image/accessibility values

Style and layout values should be normalized into stable records before crossing the boundary. Unsupported values fail loudly with the property name and source location in development mode. Never pass arbitrary JavaScript objects to Rust and hope each platform interprets them consistently.

### 9.6. Event callbacks

Each callback is registered in a runtime-owned registry and receives a stable handler ID within that runtime session. Rust core stores handler ID metadata but not a QuickJS Function object.

On node removal:

1. engine determines all event bindings owned by the removed subtree;
2. callback registry releases those references;
3. outstanding queued events are ignored by node generation/revision validation;
4. native event subscriptions/resources are removed in the resulting MountBatch.

If the same handler is assigned to multiple nodes, define whether the registry uses a shared callback handle with reference counting or per-binding entries. The implementation must not leak functions after unmount.

### 9.7. Transaction and atomicity

A batch is all-or-nothing:

- Validate header and payload.
- Validate referenced nodes, parents and anchors.
- Validate operation ordering and allowed node kinds.
- Validate property names/types against the element registry.
- Validate resource and event IDs.
- Apply the batch to a transaction staging area or mutation journal.
- Commit the tree revision only after all operations succeed.
- Generate downstream layout/mount invalidation from the committed delta.
- Return a structured failure without changing the authoritative tree if any operation is invalid.

A full clone of the entire UiTree is acceptable only as a simple first implementation or test oracle. Production should use a staging transaction or reversible mutation journal to avoid O(tree-size) copies on small updates.

### 9.8. Revision consistency

Each successful transaction increments the tree revision once. MountBatch carries the source revision and a monotonically increasing mount sequence. Platform backends reject stale batches, preserve ordering and request a full resync if they detect a gap or unsupported operation.

## 10. Layout and styling

### 10.1. Layout engine

Taffy is the selected initial layout engine because it implements CSS Block, Flexbox and Grid algorithms as a pure Rust library. It remains an internal dependency behind sn-layout; Solid Native must not expose Taffy-specific types as public app API.

### 10.2. Supported layout model

Start with a deliberately bounded native layout model:

- Flex direction, wrapping, alignment, justification and gaps.
- Grid tracks, placement, gaps and alignment.
- Block/stack behavior only where it has a well-defined native meaning.
- Width/height, min/max constraints, aspect ratio.
- Padding, margin, border widths and corner radii.
- Absolute positioning, clipping and overflow.
- Scroll containers with content and viewport sizes.
- Intrinsic size through explicit measurement callbacks.

The API should resemble familiar CSS concepts where semantics are stable, but must not claim full browser CSS compatibility.

### 10.3. Units

The engine uses logical units (platform-independent points/dp-like values) internally. Platform backend converts logical coordinates to physical pixels using the current display scale. Percentages resolve against a defined containing block. Auto/intrinsic values are explicit enum variants, not magic numeric sentinels.

Device scale, font scale, safe-area insets and keyboard insets are environment inputs. Environment changes dirty only the affected layout branches.

### 10.4. Style ownership

- JSX props and typed style objects are normalized by the host adapter.
- Core stores platform-independent computed inputs.
- sn-layout maps these inputs to Taffy styles.
- Layout produces frames, baselines and overflow data.
- Platform backend receives frames and paint/native properties, not raw arbitrary CSS.
- A theme is an application-level design token layer, not hidden global state in the engine.

### 10.5. Incremental layout

Track dirty reasons independently:

- tree topology changed;
- layout style changed;
- intrinsic content changed;
- viewport/environment changed;
- font/text metrics changed;
- image/resource intrinsic size changed.

A color-only update must not recompute layout. A layout-only change must not rebuild unrelated native controls. Cache invalidation keys include width constraints, font, font scale, direction, locale where relevant and content revision.

### 10.6. Intrinsic measurement

Text and images provide measurement functions. Taffy can request size under min/max constraints. Measurement must be deterministic for a fixed content revision and environment. Cache results and invalidate only when relevant inputs change.

## 11. Text subsystem

Text is a first-class subsystem, not merely a string property on a generic view.

Responsibilities:

- UTF-8/Unicode-safe text storage;
- attributed text runs and nested spans;
- font family/weight/size/line height/letter spacing;
- shaping, glyph fallback and line breaking;
- baseline and intrinsic measurement;
- selection, cursor and composition;
- accessibility labels and text ranges;
- locale, writing direction and font scaling;
- text measurement cache and invalidation.

Platform engines:

- Apple platforms: CoreText/CoreGraphics for shaping and drawing, with UIKit/AppKit text controls where editable text is required.
- Android: Android text APIs such as StaticLayout for measurement/layout and native EditText for editable input.
- Windows: DirectWrite for shaping/measurement and native input method integration; editable controls may use a dedicated native host component.
- Linux: Pango and GTK text widgets.
- Web: browser text layout and native HTML input/textarea.

The engine thread should prepare immutable text layout snapshots. Platform thread draws or applies those snapshots but does not run arbitrary user JS. When a platform text API is thread-affine, implement an explicit measurement service or asynchronous cache; do not silently call a UI-only API on the engine thread.

Do not attempt to build a full custom text editor before native text input works. Correct IME, selection, clipboard, accessibility and bidirectional text are more important than custom glyph painting.

## 12. Rendering and platform mounting

### 12.1. Two kinds of visual nodes

A node may be:

- layout-only container: participates in layout and clipping but requires no separate native OS control;
- platform view: maps to a native view/widget or a retained drawing primitive;
- native component: embeds a platform-owned object such as a map, video surface or web view;
- text/image resource node: rendered by native text/image infrastructure.

A logical UI node does not have to map one-to-one to an OS child window. Backend may coalesce drawing nodes into a retained surface and use actual native controls only where platform behavior is essential.

### 12.2. MountBatch

MountBatch is an ordered, immutable description of a committed engine revision. It may contain:

- create/destroy platform handle;
- insert/remove/reorder child;
- set normalized native property;
- set frame/transform/clip/opacity;
- update text snapshot;
- attach/detach image/video/native resource;
- update accessibility node;
- update focus and input configuration;
- start/cancel native animation;
- update scroll viewport/content/offset.

MountBatch must be delta-based. Do not resend an unchanged property or layout frame. Each backend receives the same semantic operations but may translate them into different OS-specific calls.

### 12.3. Idempotency and recovery

Platform batch application must be deterministic and ordered. Each operation validates the target handle. If backend state is lost (native view recreation, window restore, surface detach/reattach), backend can request a full snapshot for the current revision. The engine must be able to rebuild native state from its authoritative tree.

## 13. Native platform architecture

Platform support is an explicit product matrix, not an incidental compilation detail.

- Windows: Win32 windowing, Direct2D/DirectWrite retained drawing, native focus/input/IME/clipboard/accessibility bridge and platform views for controls that need true OS behavior.
- macOS: AppKit window/view lifecycle, Core Animation, CoreText, native menus, accessibility and text input.
- Linux: GTK 4 integration, GSK/Pango and desktop portal integration where appropriate.
- iOS: UIKit embedding/view lifecycle, CoreText, keyboard/safe-area handling, touch/gesture and accessibility.
- Android: Android View hierarchy for required native controls, JNI boundary, density/configuration changes, keyboard/IME and accessibility.
- Web: browser DOM/CSS and browser event loop; no native host APIs are emulated where browser equivalents exist.

Each target backend has its own platform crate and tests. Do not implement all native APIs in a monolithic cfg-heavy crate.

A generic PlatformHost trait must remain small. OS-specific extensions are exposed through capability interfaces and typed native component registries, not by adding every platform-specific method to the common trait.

## 14. Input, focus, scrolling and accessibility

### 14.1. Input model

Normalize platform events into typed events:

- pointer down/up/move/cancel/enter/leave;
- wheel/trackpad scroll;
- keyboard down/up and text input;
- IME composition start/update/commit/cancel;
- focus/blur;
- touch and gesture events;
- accessibility actions;
- platform navigation/back events.

Events carry timestamp, logical coordinates, pointer identity, modifiers, key identity and surface/node identity as appropriate. Coordinate transforms and hit testing must be consistent with layout, clipping, scrolling and nested transforms.

### 14.2. Focus

Focus is engine state but native focus is applied by the backend. Define:

- focusable/disabled/inert state;
- focus order and roving tabindex behavior;
- tab traversal and platform-native traversal;
- focus scopes and modal traps;
- focus restoration after overlay dismissal;
- programmatic focus and accessibility focus;
- focus loss on removal/reparenting.

Never infer that a visually selected element owns keyboard focus.

### 14.3. Scrolling

Native scrolling must not depend on JS being available every frame. Backend owns gesture tracking and kinetic scrolling, with state/offset notifications delivered to engine at a controlled cadence. Engine controls layout bounds, scroll policies and optional linked signals. Large lists must support virtualization with a well-defined row lifecycle.

### 14.4. Accessibility

Accessibility semantics are independent of visual styling and native widget choice. Each semantic node describes role, label, value, hint, state, actions, relationships and reading order. Backend maps this to UIAccessibility, Android AccessibilityNodeInfo, Windows UI Automation, AT-SPI/GTK or browser accessibility tree.

Accessibility tree must be testable in headless mode. A UI is not considered complete because it is clickable with a mouse.

## 15. Animation and frame scheduling

- Layout/paint/property transitions may be driven natively once committed.
- Time-based animations use a monotonic clock.
- The engine emits immutable animation definitions and target values.
- Platform backend advances animations with display-linked drivers when possible.
- Gesture-driven values can be updated by platform input and sampled by engine at frame/commit boundaries.
- JS requestAnimationFrame exists only with a documented scheduling guarantee; it must not be used as the default mechanism for simple native opacity/transform transitions.
- Cancellation and node removal stop associated animation resources.
- Reduced-motion preference is part of environment state.
- Manual clock support is required for deterministic tests.

## 16. Resources, images, networking and media

Resource pipeline:

1. Parse and validate the requested resource URI/capability.
2. Resolve package asset, application asset, local file grant or network request.
3. Decode/prepare off the UI thread.
4. Apply memory and disk cache policy.
5. Return an immutable resource handle and intrinsic metadata.
6. Mount a platform resource object or renderer representation.
7. Cancel and release resources when no longer retained.

Images should decode near their displayed size when practical and respect memory budgets. Large images and video must not be copied through JS strings. Use resource IDs and streaming/copy-aware buffers.

Network access is provided by runtime services behind explicit capabilities. The JS API may present familiar Promise/Fetch abstractions, but implementation and support boundaries must be documented. Cancellation, response streaming, redirects, TLS errors and back-pressure need tests.

Video, WebView and maps are optional native component providers. They do not belong in sn-core; they register typed native component implementations and lifecycle hooks.

## 17. Application API and component layers

Three layers are required.

### 17.1. Primitives

Low-level, stable host primitives such as:

- View / Fragment;
- Text / TextSpan;
- Image;
- Pressable / Button;
- ScrollView;
- TextInput;
- Overlay / Portal;
- Canvas or CustomView only when the drawing contract is defined;
- native component extension point.

These primitives define structure and behavior required by the engine. They should not impose a complete visual design system.

### 17.2. Base behavior components

A separate base package may provide accessible behavior and interaction logic without enforcing colors, typography, radius or spacing:

- Dialog/AlertDialog;
- Checkbox/Radio/Switch;
- Select/Combobox/Listbox;
- Tabs/Accordion/Collapsible;
- Menu/Popover/Tooltip;
- Slider;
- navigation primitives;
- focus scopes and roving focus;
- keyboard shortcuts;
- safe-area and keyboard-aware layout helpers.

The base layer is not the final styled component library.

### 17.3. Application/community design systems

Applications choose their own visual language, tokens, typography and styled components. A first-party optional component package may exist, but it must not be coupled into sn-core or force all apps into one visual system.

### 17.4. TypeScript contract

Every exported primitive and native module must have accurate declarations. Type declarations are generated from or checked against the same source of truth as runtime exports. CI must fail if public runtime exports and declarations diverge.

## 18. QuickJS-NG runtime

### 18.1. Decision

QuickJS-NG through rquickjs is the initial native JS engine. It is selected for embeddability, compact integration and Rust bindings. Its use is an explicit runtime implementation, not a dependency of the core architecture.

### 18.2. Isolation boundary

All QuickJS-specific types and operations stay inside sn-runtime-quickjs:

- Runtime and Context creation;
- module loading/linking;
- JS function references;
- typed host bindings;
- pending job execution;
- timers/microtasks integration;
- memory/stack limits and interruption;
- error and stack trace extraction;
- garbage collection and runtime shutdown.

sn-core and sn-engine APIs should refer to VM-independent handles, tasks and error DTOs.

### 18.3. Runtime compatibility

A real SolidJS runtime requires more than evaluating an IIFE. Maintain a compatibility inventory and automated tests for:

- ES language/runtime features used by the pinned Solid compiler output;
- modules/import resolution;
- Promise jobs and microtasks;
- queueMicrotask;
- setTimeout/clearTimeout and interval semantics;
- MessageChannel or an explicit scheduler alternative if used by dependencies;
- performance.now and monotonic time;
- requestAnimationFrame/cancelAnimationFrame;
- AbortController/AbortSignal;
- TextEncoder/TextDecoder;
- URL and URLSearchParams;
- typed arrays and ArrayBuffer;
- error stacks and source maps;
- unhandled Promise rejection reporting;
- GC behavior for native callback references.

Do not silently add browser globals that only partially behave like browser APIs. Each API is either implemented with documented semantics, unavailable with an explicit error, or provided by the platform.

### 18.4. Runtime portability

A future runtime replacement is possible only behind sn-runtime-api and a conformance suite. That does not imply drop-in VM portability: module loaders, GC, callback roots, interrupt behavior, scheduler integration and native binding models differ between engines.

Do not add Hermes, V8, JavaScriptCore or Boa as additional backends until a concrete requirement and measured advantage justify the engineering cost.

## 19. Build, bundling and developer tooling

### 19.1. Development toolchain

The current toolchain direction remains:

- Rust workspace managed by Cargo.
- TypeScript packages managed by pnpm.
- Vite and vite-plugin-solid for development and TSX compilation.
- official SolidJS compiler with Universal generation.
- TypeScript declarations for application APIs.
- Rust CLI as the unified user-facing command once its scope is clear.

Vite is a build/dev dependency, not a native runtime dependency.

### 19.2. Build modes

- Development: fast bundle, source maps, diagnostics, local dev server and hot updates.
- Production: optimized bundle, deterministic asset manifest, content hashes, optional bytecode compilation, signing and capability manifest.
- Test: deterministic build with test hooks and virtual/manual clock.
- Web: browser-native module output and static assets.

Never assume a bundle built for one QuickJS-NG version or host ABI is compatible with another. Runtime ABI and package metadata must record required protocol/runtime versions.

### 19.3. CLI target

The eventual CLI should expose stable commands:

- init — create a typed application template.
- dev — run with live diagnostics/hot update.
- build — produce target-specific build artifacts.
- test — run headless and selected platform integration tests.
- run — launch on a selected target/device.
- package — create signed or unsigned distributable bundle.
- doctor — diagnose required toolchains, SDKs and capabilities.
- inspect — show manifest, module dependencies, runtime compatibility and capabilities.

Do not implement all commands in M1. The first CLI should remain a thin wrapper over existing build/test scripts.

### 19.4. Hot reload and HMR

Hot reload is development-only infrastructure until lifecycle correctness is proven. A patch may update code while preserving state only when the runtime can safely preserve compatible component owners and subscriptions. If compatibility cannot be guaranteed, perform a controlled reload and clearly report state loss.

HMR must release old callback references, subscriptions, timers and resource handles. A successful code patch cannot leave both old and new owner trees active.

## 20. Dynamic modules and package security

For NSH-equivalent runtime module delivery, introduce a package format only after stable runtime and host API versions exist.

Package contents:

- manifest with package ID, version, entry point and required runtime ABI;
- module code or bytecode;
- content-addressed assets;
- dependencies with exact versions and hashes;
- requested capabilities;
- signature metadata;
- source map/debug metadata where policy allows.

Security model:

- host grants capabilities; package requests them but cannot grant them to itself;
- deny-by-default for filesystem, process, network, clipboard, notifications and platform-native extensions;
- paths are restricted to app/package sandbox and explicit file grants;
- network origin/host permissions are explicit;
- CPU, memory, stack, timer and message budgets are enforced by runtime/engine;
- interruption and termination must not depend on a cooperative JS callback;
- native module registration is host-controlled;
- package signature verifies publisher/integrity, but does not replace runtime capability enforcement;
- revocation and runtime compatibility are checked before module execution.

Do not execute untrusted TSX with full host privileges. Dynamic UI and AI-generated screens are a security boundary, not just a bundler feature.

## 21. Embedding API

Standalone app and embedded UI use the same Surface/Engine model.

Native host API should provide:

- create/destroy Surface;
- attach/detach to native host view/window;
- set available size and scale;
- submit platform input;
- deliver environment changes;
- grant/revoke capabilities;
- register native components/services;
- expose lifecycle events;
- retrieve structured diagnostics;
- suspend/resume when host view is not visible.

ABI rules:

- use opaque handles, fixed-width integers, explicit ownership and versioned structs;
- no Rust references, String, Vec, trait objects or Rust enums cross C ABI;
- callback lifetime and thread on which callbacks occur are documented;
- panics never unwind across FFI;
- Swift/Kotlin wrappers own native handles explicitly and provide RAII/dispose semantics;
- Android JNI methods validate thread and object lifetimes;
- iOS callbacks respect main-thread requirements;
- embedding API is versioned independently from internal Rust crates.

## 22. Error model and observability

Errors are typed and have explicit boundaries:

- compile/build error;
- module resolution/link error;
- runtime JS exception;
- unhandled Promise rejection;
- host protocol validation failure;
- tree invariant failure;
- layout/measurement failure;
- resource load/decode failure;
- native mounting failure;
- capability denial;
- platform integration failure.

Every error should include surface ID, runtime/session ID, transaction sequence, node ID when relevant, module/source location and a stable error code. Do not log secret values or arbitrary user data by default.

Development mode adds:

- source-mapped JS stack traces;
- structured engine/platform logs;
- commit/revision tracing;
- tree and layout inspector;
- runtime heap/host resource stats;
- dropped/stale event and batch counters;
- native backend operation timing;
- performance marks for event-to-mount latency.

Production logs should be bounded and configurable. Diagnostics must not require a WebView.

## 23. Performance and resource budgets

Define and benchmark at minimum:

- process startup and first window;
- VM/runtime creation;
- first TSX module compilation/evaluation;
- first committed frame;
- event-to-handler latency;
- handler-to-native-mount latency;
- host batch size and decode cost;
- layout time at 100, 1,000 and 10,000 nodes;
- text measurement cache hit rate;
- scroll frame stability;
- image decode and memory;
- steady-state JS heap and native retained tree memory;
- multi-Surface scaling;
- idle CPU usage;
- package load time.

Benchmark dimensions must be recorded: hardware, OS build, compiler mode, runtime build, display scale, tree shape, assets, input cadence and sample duration.

Performance budgets are established from a measured baseline. A change is not a performance improvement because it uses fewer abstraction layers or fewer lines of code.

## 24. Testing strategy

- sn-core unit tests: ownership, node lifecycle, ordering, cycle detection, roots, stale IDs and removal.
- bridge tests: wire decoding, malformed payloads, unknown opcodes, oversized strings, atomicity, revision checks and compatibility.
- runtime tests: real Solid signals/effects/cleanup, task/microtask order, timers, Promises and callback lifetime in QuickJS-NG.
- layout tests: golden layout fixtures, intrinsic measurement, flex/grid, RTL, safe areas, scaling and invalidation.
- engine tests: event dispatch → Solid update → host batch → retained tree → mount batch.
- platform tests: actual focus, text input/IME, selection, scrolling, native accessibility and lifecycle.
- HMR tests: update, rollback, stale callback disposal and state preservation policy.
- security tests: denied capabilities, path traversal, package signature, memory/stack/CPU limit, interruption and malicious event floods.
- API tests: TS declaration parity and compile examples against the package.
- performance tests: stable workloads, regression thresholds and reproducible artifacts.

The first reliable CI layer should be fast and headless. Heavy cross-platform UI/E2E suites run only in dedicated platform integration jobs or promotion pipelines. Feature-level correctness should be tested at the subsystem boundary; do not require every developer change to boot all OS environments.

## 25. Current implementation versus target

Implemented and previously verified in the local prototype:

- Cargo and pnpm workspaces.
- SolidJS 1.x universal generation through vite-plugin-solid.
- Custom JS host renderer.
- QuickJS-NG execution through rquickjs.
- Counter component mounts in QuickJS.
- Button handler updates a Solid signal and text node.
- Minimal Rust UiTree with node creation, insertion/move, cycle rejection and subtree removal.
- Four sn-core tree tests.
- Windows x64/MSVC build of QuickJS-NG and Rust workspace.

Not implemented yet:

- Rust tree bridge and operation protocol.
- Single authoritative tree connected to Solid renderer.
- Runtime scheduler and compatibility layer beyond synchronous smoke path.
- Taffy integration.
- Native window and platform backend.
- Real input, focus, IME, text selection and accessibility.
- Native graphics/text/image/resource pipeline.
- Component library and navigation/overlay infrastructure.
- Web, iOS, Android, macOS and Linux backends.
- Hot reload, CLI, packaging, signatures and sandbox capability enforcement.
- Performance baselines and platform parity tests.

Passing the current smoke test proves a synchronous Solid/QuickJS integration path only. It does not prove a complete native UI framework.

## 26. Architecture acceptance principles

A future implementation or refactor is acceptable only if:

1. SolidJS remains the sole owner of JS reactivity.
2. sn-core remains independent from QuickJS and OS APIs.
3. Rust retained tree is the sole authoritative UI structure after commit.
4. JS mirror contains only what the Solid renderer contract needs.
5. Native UI thread never waits synchronously for JS.
6. Host mutations cross the runtime boundary in bounded batches.
7. Production bridge is typed/versioned and does not use JSON.
8. Batch application is atomic and revisioned.
9. Platform implementations depend on platform-neutral contracts.
10. Native text input, focus and accessibility are first-class requirements.
11. All asynchronous runtime APIs have tests for ordering, cancellation and shutdown.
12. No feature is marked complete without the matching automated and platform verification.
