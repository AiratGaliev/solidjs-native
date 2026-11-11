# SolidJS Native — Implementation Roadmap

- Статус: план реализации целевой архитектуры.
- Дата baseline: 2026-11-10.
- Current milestone: M0 завершён; следующий milestone — M1 Host Protocol.
- Принцип: не переходить к более дорогим слоям, пока предыдущий слой не доказан интеграционными тестами.

## 1. Release definition

Dependency selection is governed by [Dependency Policy and Version Baseline](dependency-policy.md). Do not copy version constraints from NSH. At each milestone, verify the latest stable compatible release using the official registry/release source, update manifests and lockfiles, and run the corresponding compatibility suite. The dependency policy is authoritative if a version mentioned in this roadmap becomes stale.

Цель roadmap — развитие самостоятельного cross-platform application framework и SDK для SolidJS/TSX. Tauri 2 используется только как внешний референс для изучения архитектуры подобных framework и developer workflow; продуктовые цели и feature scope SolidJS Native определяются независимо. NSH — технический референс native UI subsystems, а не целевой продукт или обязательная dependency. Собственный scope framework описан в [Application Framework Scope](architecture/application-framework.md), архитектурное решение — в [ADR-0008](decisions/0008-standalone-application-framework.md).

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
| M10 | Secure runtime packages | Signed TSX modules, runtime capability sandbox, quotas and revocation | M2, M9 |
| M11 | Native runtime hardening | Platform parity, performance baselines, diagnostics and stable runtime APIs | M7-M10 |
| M12 | Application shell and windows | Application lifecycle, top-level windows, menus, shortcuts and platform shell | M4 |
| M13 | Application Bridge and native services | Typed commands/events and permission-checked OS services | M12 |
| M14 | Native plugin system | Versioned Rust/TypeScript plugin contracts and platform adapters | M13, M10 |
| M15 | Packaging and distribution | Installers, signing, update channels and recovery | M9, M12-M14 |
| M16 | Public SDK and independent consumer validation | Public SDK integration in representative sample applications | M11-M15 |

Milestones may overlap only where they do not violate dependencies. For example, web API design can be researched while Windows backend is implemented, but it must not force changes to core before contracts are tested.

M0-M11 establish and harden the cross-platform UI/runtime foundation. M12-M15 add the application shell, native service bridge, plugin integration and distribution/update workflow. M16 validates the public SDK in a representative external consumer; it is not a migration away from another application host.

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

Status: implementation submitted in PR #4. The topic Fast CI gate passes on the implementation branch; the Counter host-protocol smoke acceptance remains pending on the integrated `dev → test` promotion candidate. M2 stays blocked until the M1 acceptance gate is green.

### Goal

Remove the duplicated authoritative tree. SolidJS renderer drives the Rust UiTree through a typed batch protocol.

### Work packages

1. Define stable node identifiers, per-Surface SessionGeneration and handle validation. Use a monotonic NodeId that is never reused within a session; add per-node generation only if a future design introduces ID recycling.
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
14. Keep JSON outside the production host mutation transport; the implementation uses no JSON encoding or parsing on the operation path.

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
- HeadlessMountSink observes the expected initial Counter mount delta and exactly one ReplaceText delta after the click-driven signal update. The platform MountBatch pipeline remains a separate M3 deliverable.
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

## 6. M3 — Layout, CSS/Tailwind styling, commit pipeline and headless mount sink

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
14. Add golden layout fixtures for flexbox, grid, absolute positioning and fixed positioning, including transformed containing blocks.
15. Add explicit unsupported-layout tests for sticky, floats/clear, inline/inline-block, table layout and `display: contents`; verify source-linked CSS diagnostics rather than silently approximating them.
16. Define the internal `sn-style` facade and its contract with the authoritative `sn-core::UiTree`; a styling engine may maintain only a derived selector/style mirror.
17. Prototype **Servo Stylo first** behind `sn-style`. Implement the minimum DOM adapter required by the Stylo traits over committed `sn-core::UiTree` state; keep style data in a sidecar keyed by stable `UiNodeId`. Verify stylesheet registration, selector/class/state synchronization, invalidation, computed values and normalized style DTOs without introducing a second authoritative tree.
18. Review `style-engine` 1.0.0 as a **source-level comparison only**. Its Rust 1.90 requirement excludes it from the current 1.87 workspace; do not add it or raise MSRV merely to run a second prototype. Compare only the CSS semantic boundary, not its Taffy/text/paint architecture.
19. Run real Tailwind CSS v4 output from the official Vite integration through the Stylo prototype and the in-house fallback only if Stylo fails. Cover utilities, `@layer`, `@theme`, custom properties, `var()`, `@property`, color functions (including OKLCH/color-mix where required), responsive rules and interaction variants used by examples. Reuse applicable Servo WPT CSS parser/selector/cascade fixtures as conformance input, but do not import browser-only layout/paint expectations.
20. Resolve Stylo 0.18.0's actual transitive graph and default `servo` feature, determine whether `default-features = false` is viable, establish MSRV/build-script/toolchain requirements, check target-platform support and review MPL-2.0 obligations before any dependency addition. Keep `style-engine` source-only while the project remains at MSRV 1.87; do not silently raise MSRV or introduce a parallel Taffy line.
21. Promote Stylo only if its adapter, dependency graph, security/license review and full acceptance suite pass. If it fails, implement a bounded resolver using compatible `cssparser` + `selectors`. `style-engine` remains source-level reference only while MSRV is 1.87. Lightning CSS remains optional build-time tooling, not a runtime cascade. Record the final choice and evidence in an ADR before adding a dependency.
22. Map computed layout properties to Taffy and paint/typography/effect properties to the appropriate services, independent of which CSS resolver is selected.
23. Add strict compatibility diagnostics for unsupported native CSS rules and source-linked build errors. A precompiled stylesheet artifact is optional and must be justified by startup, binary-size or compatibility measurements.

### Acceptance

- Taffy computes frames from Rust-owned tree state and computed native styles.
- A SolidJS Universal TSX example uses `class`/`className` and Tailwind utilities; generated CSS reaches the selected `sn-style` resolver and produces correct computed values and native output.
- Stylo has passed the `sn-style` contract and Tailwind/native-profile acceptance suite, or a tested `cssparser` + `selectors` fallback is selected with documented evidence. `style-engine` remains a source-level reference only under MSRV 1.87; MSRV, Taffy and platform constraints are recorded.
- `sn-core::UiTree` remains authoritative; any third-party style tree is a derived mirror synchronized from committed native tree state.
- Stylesheet parsing occurs once per registration/replacement or via an optional precompiled artifact; no CSS parsing occurs on the per-frame hot path.
- Common spacing, sizing, flex/grid, typography, color, background, border and radius utilities resolve to expected typed native values.
- Reactive class changes update styles without remounting nodes.
- A supported hover/focus/disabled state, dark theme and responsive breakpoint update the expected properties.
- CSS custom properties and Tailwind `@theme` tokens resolve deterministically.
- Unsupported native CSS rules produce source-linked diagnostics; strict mode fails instead of silently dropping them.
- Updating a color does not invalidate layout; updating spacing does.
- Updating text/font/width invalidates only required measurements.
- One committed transaction produces a deterministic MountBatch.
- Headless sink can reconstruct the rendered platform state from a full snapshot and subsequent deltas.
- Layout and style resolver tests run without a desktop session.

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
9. Project committed UiTree semantics into an AccessKit tree with stable UiNodeId-to-NodeId mapping and incremental TreeUpdate generation.
10. Integrate the AccessKit Windows UI Automation adapter and route AccessKit action requests back through the engine event queue.
11. Add a headless accessibility-tree/action test harness; keep focus ownership and native editing semantics in Solid Native.
12. Lifecycle tests for focus/IME/accessibility actions while nodes are removed/reparented.

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
- image decoding off UI thread through a project-owned async resource-decoder contract; evaluate `image` for raster and `resvg` for SVG support;
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

## 14. M11 — Native runtime hardening

- public API stability policy;
- versioned runtime/protocol/ABI;
- platform-specific CI;
- benchmark suite and regression budgets;
- startup and memory budgets;
- crash/error diagnostics;
- accessibility audit;
- security review;
- documentation, examples and migration guides;
- compatibility matrix and support policy.

A stable release is not declared until every platform included in that release reaches its published maturity level.

## 15. M12 — Application shell and window management

### Goal

Provide the process-level application host and top-level native window lifecycle required by real desktop applications. Keep this separate from the per-Surface PlatformHost and renderer MountSink.

### Work packages

1. Define PlatformApplicationHost and PlatformWindowHost contracts, lifecycle, ownership and thread affinity.
2. Implement application startup, activation, single-instance handoff, quit/restart and shutdown ordering on Windows first.
3. Implement top-level window creation, close requests, activation/focus, resize, minimize/maximize/fullscreen, title, icon and decorations.
4. Add application/window menus, context menus and keyboard shortcut routing required by the first consumer.
5. Add tray/status item, notifications and URL/file activation only where the target OS and documented framework use cases justify them.
6. Keep Surface attach/detach and MountBatch application in the existing per-Surface contracts.
7. Add lifecycle and multi-window integration tests, including failure and disposal cases.

### Acceptance

- A standalone app can start, activate, open and close its main and secondary windows, and shut down without leaked surfaces or callbacks.
- Window events have documented ordering and are delivered through the Application Bridge/event contract.
- Menus and shortcuts work with keyboard focus and text editing; they do not steal input unexpectedly.
- Unsupported platform facilities return explicit capability results.
- Platform UI threads do not synchronously wait for QuickJS or user application logic.

## 16. M13 — Application Bridge and native services

### Goal

Expose typed, asynchronous native application services to SolidJS applications without coupling them to renderer mutations.

### Work packages

1. Define a versioned command/event contract with typed payloads, results, errors, cancellation and resource handles.
2. Keep this Application Bridge independent from the UI Host Protocol and its renderer batch format.
3. Implement Rust-side permission checks before every privileged service operation.
4. Add the first required services: app/window control, clipboard, file and directory dialogs, scoped file access, external URL opening, notifications and platform/app information.
5. Add network services and process/shell execution only when a concrete product requirement justifies their permission and threat model.
6. Define resource lifetime, event ordering, back-pressure, timeout and shutdown behavior.
7. Add TypeScript declarations and contract tests for command success, denial, cancellation, unsupported platforms and stale handles.

### Acceptance

- Application calls are asynchronous and typed; errors are stable and actionable.
- Authorization is enforced by Rust implementations, not only by TypeScript wrappers.
- Resource handles are scoped, revocable and released on app/window shutdown.
- Renderer operations cannot invoke arbitrary OS services and Application Bridge calls cannot mutate the retained tree outside its commit contract.
- Compatibility is negotiated before app modules call the bridge.

## 17. M14 — Native plugin system

### Goal

Allow applications to extend the framework with versioned native services while keeping trusted native code distinct from sandboxed runtime modules.

### Work packages

1. Define plugin manifest, identity, version compatibility, platform support and declared commands/events.
2. Define the trusted Rust plugin registration, initialization, application/window attachment and shutdown contracts.
3. Provide optional TypeScript packages for typed application-facing bindings.
4. Add Swift/Kotlin adapters only for capabilities requiring those platform host languages.
5. Reuse Application Bridge authorization and declare the capabilities each plugin requires.
6. Define diagnostics, dependency conflicts, registration order and deterministic cleanup.
7. Keep dynamic/untrusted TSX runtime modules governed by M10 and ADR-0007; do not treat a native Rust plugin as sandboxed by QuickJS.
8. Require provenance review and trusted build/distribution policy for native plugin code.

### Acceptance

- A sample native plugin exposes a typed command and event on the first supported platform.
- Runtime/API incompatibility and missing permissions fail before the protected operation executes.
- Plugin shutdown releases handlers, resources, subscriptions and platform registrations.
- Native plugins are explicitly classified as trusted process code; untrusted app modules cannot load arbitrary native code.

## 18. M15 — Packaging, signing and updates

### Goal

Build and distribute standalone applications through a reproducible platform-aware release pipeline.

### Work packages

1. Define application bundle manifest, executable layout, assets, metadata, icons and runtime dependencies.
2. Produce installable Windows packages first; add macOS, Linux, Android and iOS delivery according to their platform maturity and signing requirements.
3. Integrate code signing, timestamping and notarization where required by the target platform.
4. Define update manifest, release channels, compatibility checks, download verification and staged rollout policy.
5. Implement install/update/rollback/recovery behavior without corrupting user data.
6. Record build provenance, dependency inventory and artifact checksums.
7. Test clean installation, upgrade, downgrade rejection, interrupted update and uninstall behavior on real targets.

### Acceptance

- Release artifacts are reproducible from a clean checkout and include all required native runtime assets.
- Signing/verification policy is explicit for every distributed target.
- Interrupted or invalid updates do not leave the application unlaunchable or silently corrupt user data.
- Unsupported packaging/update capabilities are not advertised as available.

## 19. M16 — Public SDK and independent consumer validation

### Goal

Validate SolidJS Native as a reusable, versioned SDK through independent sample consumer applications. M16 proves that consumers can install and use the public framework contracts from a clean checkout; it is not tied to a specific engine, editor, company or application host.

### Work packages

1. Publish or consume the public Rust crates, NPM packages and CLI from a separate sample consumer project; no source vendoring or private-module imports.
2. Implement a representative standalone application that exercises application/window lifecycle, navigation, native controls, keyboard/input, dialogs and supported native services.
3. Provide a separate Web-target sample using browser-native DOM/runtime contracts.
4. Validate SDK installation, build, run, diagnostics and packaging from a clean checkout on the platforms claimed by the samples.
5. Run native lifecycle, text/input/IME, accessibility, permissions, security, packaging and recovery tests; record platform-specific differences.
6. Measure startup, memory, idle CPU, input latency, first-frame time and package size against explicit framework budgets.
7. Document public API usage, integration guidance, examples, compatibility/support matrix and known limitations.

### Acceptance

- Independent sample consumers build from a clean checkout using only public, versioned SDK packages and documented APIs.
- Sample workflows pass on every platform claimed by the sample; unsupported capabilities are explicit.
- Native and Web target contracts, platform-specific behavior, permissions and lifecycle are covered by repeatable evidence.
- Security, accessibility, packaging and performance results meet the declared framework maturity and budgets.
- SDK documentation, samples, compatibility/support matrix and limitations are published and consistent.
- The result is based on reproducible application workflows and platform checks, not a prototype-only screenshot. No particular product integration, host replacement or migration is implied.

## 20. Cross-cutting engineering rules

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

## 21. Definition of Done for each milestone

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
