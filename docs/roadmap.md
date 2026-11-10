# SolidJS Native — Implementation Roadmap

- Статус: план реализации целевой архитектуры.
- Дата baseline: 2026-11-10.
- Current milestone: M0 завершён; следующий milestone — M1 Host Protocol.
- Принцип: не переходить к более дорогим слоям, пока предыдущий слой не доказан интеграционными тестами.

## 1. Release definition

Dependency selection is governed by [Dependency Policy and Version Baseline](dependency-policy.md). Do not copy version constraints from NSH. At each milestone, verify the latest stable compatible release using the official registry/release source, update manifests and lockfiles, and run the corresponding compatibility suite. The dependency policy is authoritative if a version mentioned in this roadmap becomes stale.

Цель roadmap — полноценная функциональная альтернатива NSH для SolidJS/TSX приложений на Windows, macOS, Linux, iOS, Android и Web.

Это большой runtime/framework проект. Milestone означает проверенный набор возможностей, а не просто добавление файлов или crate. Каждый milestone должен:
- иметь явный scope;
- иметь измеримые acceptance criteria;
- иметь automated tests;
- иметь демонстрационный example;
- не нарушать ownership/threading/protocol rules;
- обновлять документацию и compatibility matrix;
- указывать платформы, на которых реально проверен.

## 2. Milestone map

| Milestone | Название | Основной результат | Зависимости |
|---|---|---|---|
| M0 | Runtime proof | TSX/Solid bundle запускается в QuickJS-NG | — |
| M1 | Native host protocol | Rust UiTree становится authoritative tree для Solid renderer | M0 |
| M2 | Runtime scheduler | Async Solid effects, Promise jobs, timers и lifecycle | M1 |
| M3 | Layout and commit engine | Taffy, dirty propagation, MountBatch and headless sink | M1, M2 |
| M4 | Windows native vertical slice | Реальное Win32 окно с render/input/layout lifecycle | M3 |
| M5 | Text, input and accessibility foundation | Native TextInput, focus, IME, selection, semantics | M4 |
| M6 | Core controls and resources | Images, scrolling, overlays, navigation, base controls | M5 |
| M7 | Web backend | Browser DOM renderer and web build | M1, M2, M6 |
| M8 | Other native platforms | macOS, Linux, Android, iOS backends | M4-M6 |
| M9 | CLI, HMR and embedding | Dev/build/run/test/package and native host APIs | M4-M8 |
| M10 | Secure runtime packages | Signed modules, capability sandbox, quotas and revocation | M2, M9 |
| M11 | Production hardening | Platform parity, performance baselines, stable public APIs | M7-M10 |

Milestones may overlap only where they do not violate dependencies. For example, web API design can be researched while Windows backend is implemented, but it must not force changes to core before contracts are tested.

## 3. M0 — Runtime proof

Status: complete on Windows x64 / MSVC.

Implemented and verified:
- Cargo workspace with sn-core and sn-runtime.
- pnpm workspace and solid-native package.
- Official SolidJS 1.x and vite-plugin-solid Universal generation.
- QuickJS-NG via rquickjs.
- TSX counter bundle executes in QuickJS.
- Solid universal renderer mounts host nodes.
- Click callback executes.
- Solid signal update changes the text node.
- Minimal Rust UiTree with insert/move/cycle/subtree semantics.
- Four sn-core tree tests.
- cargo tests and clippy passed in the original M0 check.

M0 proves an initial synchronous integration only. It is not proof of the Rust bridge, asynchronous scheduler or native UI.

Exit criteria: complete.

## 4. M1 — Rust-owned host tree and typed protocol

### Goal

Remove the duplicated authoritative tree. SolidJS renderer drives the Rust UiTree through a typed batch protocol.

### Work packages

1. Define stable node identifiers, per-surface generation and handle validation.
2. Extend sn-core node model with text payload and typed element properties.
3. Define versioned batch header and operation vocabulary.
4. Define bounded string/value arenas and wire representation.
5. Implement HostBatchEncoder in solid-native.
6. Implement native QuickJS binding for applying one byte buffer/batch.
7. Implement Rust HostBatchDecoder with strict range/overflow validation.
8. Implement UiTree transaction application.
9. Add atomic failure semantics and revision acknowledgements.
10. Add event handler ID registry without serializing JS functions.
11. Add input-event dispatch from engine into JS handler registry.
12. Add cleanup on node removal and renderer dispose.
13. Replace current synchronous counter inspection with Rust tree assertions.
14. Preserve a test-only diagnostic JSON representation if it simplifies debugging; it must not be used as production transport.

### Required operation set

- CreateElement
- CreateText
- ReplaceText
- SetProperty
- RemoveProperty
- InsertNode / move
- RemoveNode
- BindEvent / UnbindEvent
- resource attach/detach placeholders only if resource ownership contract exists.

### Tests

- same-parent move and cross-parent move;
- insert before first/middle/last anchor;
- remove subtree;
- text replacement after signal update;
- event binding replacement and removal;
- duplicate/stale/foreign IDs;
- invalid property type;
- invalid anchor and cycle;
- malformed/truncated/oversized protocol payload;
- operation ordering;
- failed transaction leaves tree and revision unchanged;
- one batch increments revision once;
- callback registry returns to baseline after repeated mount/unmount.

### Acceptance

- M0 Counter compiled by official SolidJS compiler is mounted into Rust UiTree.
- A normalized click event enters Rust engine and calls JS callback.
- Solid signal update creates second batch.
- Rust UiTree contains the updated text.
- MountBatch/HeadlessMountSink observes only the expected delta.
- Adapter mirror is clearly non-authoritative and recoverable after transaction failure.
- No JSON serialization exists in the runtime host operation path.

Do not start M2 implementation before this acceptance is green.

## 5. M2 — Runtime scheduler and asynchronous compatibility

### Goal

Make SolidJS runtime behavior deterministic beyond a synchronous click callback.

### Work packages

1. Refactor current executable runner into reusable QuickJS runtime library plus small CLI example.
2. Define JsRuntime/RuntimeSession interface outside QuickJS-specific crate.
3. Implement one engine-thread owner per Surface.
4. Add pending Promise job draining.
5. Implement queueMicrotask and timer scheduler.
6. Implement performance.now and monotonic/wall-clock separation.
7. Implement RAF/cancelRAF semantics.
8. Implement external task and resource completion queue.
9. Define event-loop fairness and maximum job budget.
10. Define unhandled Promise rejection reporting.
11. Add AbortController/Signal if required by selected libraries.
12. Add runtime shutdown, component cleanup and callback registry disposal.
13. Add heap, stack and interrupt controls.
14. Document supported and unsupported browser globals.

### Tests

- Promise then/catch/finally ordering;
- queueMicrotask from event handler;
- multiple updates coalesce into one commit;
- timer cancellation;
- timer after node unmount;
- nested Promise chains;
- event arrives while job queue is non-empty;
- unhandled rejection diagnostics;
- budget exhaustion does not starve native event loop;
- dispose with pending jobs/timers;
- QuickJS GC after handler unbind.

### Acceptance

A Solid application using signals, effects, Promise jobs and timers behaves deterministically inside QuickJS-NG. The UI tree changes only through the host protocol. All jobs run on the owning engine thread.

## 6. M3 — Layout, commit pipeline and headless mount sink

### Goal

Compute geometry in Rust and generate deterministic native mount commands without depending on an OS window.

### Work packages

1. Add sn-layout Taffy adapter.
2. Define platform-independent logical units.
3. Add normalized LayoutStyle and paint/text property types.
4. Build layout tree synchronization from authoritative UiTree.
5. Implement dirty propagation and invalidation reasons.
6. Add viewport and environment metrics.
7. Add intrinsic measurement interface.
8. Add HeadlessTextMeasurer test implementation.
9. Implement ordered MountBatch data model.
10. Implement HeadlessMountSink and tree snapshots.
11. Add clipping, transforms, z-order and overflow representation.
12. Add full snapshot/resync path.
13. Add layout/text measurement cache and invalidation tests.
14. Add golden layout fixtures for flexbox and grid.

### Acceptance

- Taffy computes frames for the Rust-owned tree.
- Updating a color does not invalidate layout.
- Updating text/font/width invalidates only required measurements.
- One committed transaction produces a deterministic MountBatch.
- Headless sink can reconstruct the rendered platform state from a full snapshot and subsequent deltas.
- Layout unit tests run without a desktop session.

## 7. M4 — Windows native vertical slice

### Goal

Show a real native window hosting SolidJS UI without WebView and without JS on the UI thread.

### Work packages

1. Create sn-platform-api contracts: PlatformHost, MountSink, TextMeasurer and InputSource.
2. Add Win32 window lifecycle and message pump.
3. Add engine-thread creation/ownership.
4. Add cross-thread task and mount-batch queues.
5. Add Direct2D surface and resource lifecycle.
6. Add basic frame/clip/background/border/transform rendering.
7. Add DirectWrite measurement/shaping adapter.
8. Add pointer, keyboard, wheel and focus event normalization.
9. Add DPI and viewport environment updates.
10. Add input event dispatch to QuickJS engine.
11. Add backend recovery after resize/resource recreation.
12. Add a native Counter example and launcher.

### Acceptance

- Application opens a real Win32 top-level window.
- No browser or WebView is used to draw the UI.
- Counter initial state renders.
- Button interaction updates Solid signal and rendered text.
- Resize recomputes layout and frames.
- UI thread never calls QuickJS and does not wait for engine.
- Close/dispose releases runtime and platform resources.
- Headless core/bridge tests still pass.

M4 is a vertical slice, not a complete Windows toolkit.

## 8. M5 — Text input, focus, keyboard and accessibility

### Goal

Deliver correct native interaction, not only pixels and click handlers.

### Work packages

1. Text node and TextRun model.
2. Text style and attributed text normalization.
3. DirectWrite text shaping/measurement cache.
4. Native editable input host.
5. IME composition and commit/cancel semantics.
6. Selection/caret and clipboard.
7. Focus tree, focus scopes and traversal.
8. Keyboard and key-equivalent mapping.
9. Accessibility semantic tree and actions.
10. Windows UI Automation adapter.
11. Test harness to inspect focus/accessibility state.
12. Lifecycle tests for focus/IME while nodes are removed/reparented.

### Acceptance

- Text rendering handles multiline, font fallback and scaling.
- Native text editing supports IME composition, selection and paste/copy.
- Keyboard navigation/focus is deterministic.
- Screen reader can discover and operate primitive controls.
- No text/IME processing is delegated to JS synchronously on UI thread.
- Accessibility semantics are testable in headless and native tests.

## 9. M6 — Core primitives, resources and base behaviors

### Goal

Make the framework useful for real application screens.

### Primitives

- View
- Text
- Image
- Button/Pressable
- TextInput
- ScrollView
- Overlay/Portal
- Native component host.

### Resource pipeline

- file/app assets;
- image decoding off UI thread;
- intrinsic dimensions;
- memory cache;
- cancellation;
- disposal and memory budget;
- error state and placeholders.

### Base behavior

- Dialog and AlertDialog;
- Checkbox, Radio, Switch;
- Tabs and roving focus;
- Menu, Popover and Tooltip;
- Select/Combobox/Listbox;
- Collapsible/Accordion;
- Slider;
- navigation stack;
- scroll/gesture state;
- keyboard shortcuts.

### Acceptance

Build at least three example apps:
- responsive settings/form app;
- navigation/dialog/list app;
- media/image loading app.

Each example must exercise keyboard/touch behavior, accessibility and cleanup. Styled components remain outside core and base behavior contracts.

## 10. M7 — Web backend

### Goal

Run the same application-facing API in a browser with browser-native DOM, accessibility and text input.

### Work packages

1. Define cross-platform primitives surface.
2. Implement browser renderer adapter.
3. Use browser JS runtime and native DOM semantics.
4. Reuse common component contracts without importing QuickJS runtime.
5. Implement browser environment adapter.
6. Implement resource APIs backed by browser facilities.
7. Add web build configuration/static output.
8. Add API parity tests shared with native examples.
9. Document platform-specific APIs and capability differences.

### Acceptance

- Counter/form/navigation examples run in modern browsers.
- Native builds have no dependency on browser DOM packages at runtime.
- Web build has no QuickJS embedded.
- Shared component API types are consistent.
- Browser accessibility and input are native to the platform.
- Documented limitations have explicit error/fallback behavior.

## 11. M8 — Remaining native platforms

### Linux

- GTK 4 integration and main context ownership;
- GSK renderer and Pango text;
- AT-SPI accessibility;
- file chooser/desktop portal integration;
- Wayland/X11 validation;
- packaging/runtime dependencies.

### macOS

- AppKit window and view integration;
- Core Animation/CoreText;
- text input and accessibility;
- window/menu/drag-drop semantics;
- embedding and lifecycle.

### Android

- JNI and Android library packaging;
- native View host and Choreographer;
- density/font-scale/configuration changes;
- text input/IME/selection;
- touch/gesture/scroll;
- Activity recreation and lifecycle;
- ABI splits and arm64 devices.

### iOS

- UIKit embedding and lifecycle;
- Swift wrapper and C ABI;
- safe areas, keyboard and touch;
- native editable controls and accessibility;
- background/foreground lifecycle;
- simulator and physical device validation.

### Acceptance for each platform

Each backend must reach at least maturity L4 from platform-backends.md for a preview claim. Stable support requires L5 and a maintained CI/test environment.

Do not infer parity by compiling a crate on another OS. Real interaction testing is required.

## 12. M9 — CLI, development server, HMR and embedding

### CLI stages

First:
- init;
- dev;
- build;
- run;
- test;
- doctor.

Later:
- package;
- inspect;
- signing and key management;
- device and simulator management.

### Development server

- source update protocol;
- local asset server;
- compiler diagnostics;
- runtime error overlay/log forwarding;
- source maps;
- session lifecycle.

### HMR

Only support state-preserving HMR when owner graph and component compatibility can be proved. Otherwise restart the root deterministically and report the reset.

### Embedding

- Rust API;
- C ABI with opaque handles;
- Swift wrapper;
- Kotlin wrapper;
- lifecycle and thread contract;
- native component registration.

### Acceptance

- one command creates and runs example;
- native and web build paths work from clean checkout;
- host app embeds Surface and disposes it;
- HMR cannot leak callbacks or resources;
- ABI has version negotiation and integration tests.

## 13. M10 — Dynamic modules and security

### Work packages

1. Define package manifest and runtime ABI range.
2. Define dependency graph and content hashes.
3. Define package signature/trust store model.
4. Define capability request and host grant model.
5. Enforce file/network/process/clipboard/native component capabilities.
6. Add runtime heap/stack/instruction/time/operation quotas.
7. Add interrupt and forced termination path.
8. Add package revocation/update policy.
9. Add dependency provenance and third-party notices.
10. Add adversarial package tests.

### Acceptance

- module cannot access a capability it was not granted;
- path traversal and host filesystem escapes are rejected;
- time/heap/stack exhaustion terminates safely;
- signed package integrity is checked before execution;
- revoked/unsupported module is rejected with actionable diagnostic;
- signature does not bypass capability policy.

## 14. M11 — Production hardening

- public API stability policy;
- versioned runtime/protocol/ABI;
- platform-specific CI;
- benchmark suite and regression budgets;
- startup and memory budgets;
- crash/error diagnostics;
- installer/package signing;
- upgrade/recovery behavior;
- accessibility audit;
- security review;
- documentation, examples and migration guides;
- compatibility matrix and support policy.

A stable release is not declared until every platform included in that release reaches its published maturity level.

## 15. Cross-cutting engineering rules

### Architecture

- sn-core never depends on QuickJS or OS APIs.
- no duplicated reactive graph;
- no synchronous JS invocation from platform UI thread;
- no production JSON renderer bridge;
- no unversioned runtime/bridge/embedding contract;
- no platform-specific code in core.

### Code quality

- keep public types minimal and explicit;
- avoid generic map-of-string-to-untyped-JSON-value for core style/property model;
- use typed enums/structs and stable IDs;
- validate all boundary data;
- avoid panics across FFI;
- use explicit cancellation and disposal;
- document thread ownership.

### Testing

- unit tests at subsystem boundaries;
- bridge/runtime/layout integration tests in headless CI;
- platform-specific UI tests in platform jobs;
- expensive full E2E suite separated from ordinary feature loop;
- benchmark same workload before/after a performance claim.

### Documentation

Every new public API requires:
- type declaration;
- semantics and default value;
- event ordering;
- cleanup/lifecycle contract;
- platform support matrix;
- accessibility behavior;
- error/capability behavior;
- runnable example;
- test.

## 16. Definition of Done for each milestone

A milestone is Done only when:

1. Code is implemented on the intended branch.
2. Tests exist for each acceptance criterion.
3. All relevant fast checks pass.
4. Platform checks run on every claimed target.
5. Architecture and API docs reflect actual implementation.
6. Roadmap and parity matrix are updated.
7. Known limitations are recorded.
8. No critical correctness, memory/lifecycle, security or threading issue remains.
9. Example app demonstrates the capability.
10. The milestone is reviewed against target-architecture.md and relevant ADRs.

A compiling crate, merged code, or demo screenshot alone is not sufficient.
