# Architecture Reuse and Dependency Adoption Map

- Status: current architecture selection and adoption policy.
- Review date: 2026-11-10.
- Scope: consolidate the roles of Tauri 2, NSH, GPUI Kit/GPUI Shell, Servo and independent Rust libraries.
- Related: [Target architecture](target-architecture.md), [Platform backends](platform-backends.md), [Styling, CSS and Tailwind CSS](styling-and-css.md), [Dependency policy](../dependency-policy.md), [Upstream references](../reference/upstream-references.md).

## 1. Decision summary

SolidJS Native will reuse mature upstream components where their contracts fit. It will not fork Servo, NSH or GPUI Kit, and it will not make any of those whole projects a mandatory runtime dependency.

The architecture is composed from references with deliberately different responsibilities:

- **Tauri 2 — product completeness.** Use its application-framework scope as the checklist for lifecycle, windows, plugins, permissions, CLI, packaging and updates. Do not adopt its WebView renderer as the native UI.
- **Upstream SolidJS — application runtime contract.** Use the official SolidJS runtime and Universal renderer. Do not reimplement Solid's reactive graph in Rust.
- **QuickJS-NG + rquickjs — native JavaScript execution.** Keep the VM behind the runtime adapter. The Web target uses the normal browser runtime and remains a separate deployment target, not another embedded VM in the native runtime.
- **NSH — primary native implementation and rendering reference.** Reuse its architectural patterns for retained trees, batched commits, Taffy integration, platform ownership, input, text editing, accessibility, native controls and per-platform drawing. Follow the same principle: native platform APIs render each target; do not introduce a separate unified GPU renderer by default.
- **GPUI Kit / GPUI Shell — comparative desktop and application-host reference.** Reuse ideas for unstyled behavior primitives, controls, editing, docking, virtualization, accessibility, headless testing, QuickJS host lifecycle and developer tooling. No GPUI renderer/backend is selected.
- **Servo Stylo — CSS implementation reference only.** Evaluate Stylo as the native CSS resolver prototype for selectors, cascade, computed values, custom properties and style invalidation. Do not adopt Servo's browser runtime, layout/paint pipeline, compositor or embedding stack.
- **Independent Rust crates — subsystem implementations.** Prefer small, separately maintained dependencies behind our own `sn-*` adapters over copying large framework internals.

## 2. Subsystem adoption matrix

“Selected” means the architectural direction is chosen; the crate still needs an implementation milestone and compatibility tests. “Prototype candidate” is not an adopted dependency.

| Subsystem | Selected source / technology | Classification | Integration boundary and constraints |
|---|---|---|---|
| Application framework scope | Tauri 2 | Reference | Product checklist only: lifecycle, windows, menus, plugins, capabilities, CLI, packaging, updates and developer workflow. No Tauri WebView requirement. |
| Reactive UI and renderer contract | Upstream SolidJS + `solid-js/universal` | Adopt | The real SolidJS runtime owns signals, effects, owners and cleanup. The Universal renderer is the only application UI mutation source. |
| Native JavaScript VM | QuickJS-NG 0.16.2 snapshot through `rquickjs` 0.14.0 | Adopted in current M0 prototype; M2 version/security review required | One owner engine thread per runtime. Keep VM types out of `sn-core`; limits/capabilities are not a sandbox. Upstream QuickJS-NG is now 0.18.0; upgrade only with a compatible binding and runtime conformance tests. `rquickjs` marks Windows MSVC experimental and has no verified iOS/Android support matrix. |
| Optional JIT distribution | Longbridge `quickjs-jit` 0.14.2 + matching `quickjs-jit-runtime` | Comparative M2 candidate; not selected | Downstream `rquickjs` distribution; the binding package alone does not activate machine-code JIT. JIT runtime/core/sys crates must use the exact same release. It is based on QuickJS-NG 0.16.2+, not the upstream 0.18.0 line. Evaluate only with representative SolidJS workloads and per-platform build/support checks. |
| Host protocol and authoritative tree | Project-owned typed/versioned batch + `sn-core::UiTree` | Build here | This is a differentiating boundary: atomic transactions, stable node IDs, revisions and no production JSON transport. Do not duplicate it in a third-party tree. |
| Native UI/platform architecture | NSH patterns | Adapt | Keep platform-neutral engine contracts and per-OS host adapters. Study its ownership, batching, input, text and lifecycle paths; do not port NSH's Rust reactive runtime or renderer wholesale. |
| Desktop controls and app-host workflow | GPUI Kit / GPUI Shell | Comparative reference only | Study behavior primitives, editing, docking, virtualization, accessibility, testing and the QuickJS host. No GPUI renderer/backend is selected; NSH per-platform backends remain the native implementation model. |
| Native CSS semantics | Servo Stylo 0.18.0 | First prototype to evaluate; not adopted | Evaluate the DOM/selector adapter, computed-value normalization, transitive graph, MSRV, build scripts and MPL-2.0 obligations. Keep it behind `sn-style`; do not treat this source review as a dependency selection. |
| GUI-oriented CSS reference | style-engine 1.0.0 | Source-level comparison only; not a dependency under MSRV 1.87 | Its manifest requires Rust 1.90 independently of optional features. Review source/API only; do not raise MSRV or import its layout/text/paint contracts for this review. |
| CSS build transforms | Lightning CSS | Optional build-tool reference; no direct dependency selected | Tailwind's official Vite integration is the selected authoring/compiler path. Add Lightning CSS directly only if a proven build-stage AST/transform requirement is not already covered. It is not the native runtime cascade engine. |
| Utility CSS authoring | Tailwind CSS v4 + official Vite integration | Selected authoring direction; package pin deferred to M3 | Vite 8 requires `@tailwindcss/vite` 4.2.2 or later (4.2.1 peer range stops at Vite 7). Tailwind owns source scanning and utility generation. Feed emitted CSS to `sn-style`; do not build a Tailwind clone or assume every generated browser rule is supported natively. |
| Layout | Taffy 0.15.1 | Selected M3 prototype dependency | Use low-level custom-tree traits over committed `sn-core::UiTree`; no second authoritative `TaffyTree`. Use `default-features = false` with `std`, `flexbox`, `grid`, `block_layout`, `calc`, `content_size`; out-of-flow layout is custom-tree API work (`compute_oof_layout`), not a Cargo feature. Keep Taffy types private. |
| Accessibility semantics and platform exposure | AccessKit 0.25.2 | Selected semantic tree/action contract; integration pending | Project committed state to stable AccessKit IDs and incremental updates. Pin each OS adapter independently at its implementation milestone and test supported roles/actions per platform. It does not replace focus ownership, IME or native editing; do not assume full rich-text/hypertext semantics or a Web adapter. |
| Text input and native editing | NSH patterns + OS text services | Adapt | Use platform-owned text input/IME and native editing services. Do not try to make a renderer or font shaper replace TextInput/selection/IME behavior. |
| Font parsing and shaping | `read-fonts`, `skrifa`, HarfBuzz; platform text APIs | Evaluate by use case | Reuse font parsing/metrics libraries if embedded-font requirements need them. Initially prefer OS text engines per platform for shaping and editing; do not import Servo's entire `fonts` crate. |
| Native platform rendering | NSH per-platform retained renderer + native OS graphics APIs | Adopt as architecture pattern | Apply committed tree/layout/style updates through `MountBatch` to backend-owned state. Render through each OS's native stack; keep damage tracking, batching and drawing details private to the backend. Do not introduce a mandatory generic DisplayList or unified GPU renderer. |
| Unified 2D vector renderer | Vello family (`vello_cpu`, `vello_gpu`, compute `vello`) | Not selected | Vello is a rasterization subsystem, not a native UI framework. Do not add Vello/wgpu to core or make rendering depend on a cross-platform GPU stack. Revisit `vello_gpu` only if benchmarks show duplicated native drawing code is a real cost and a prototype proves a net benefit. |
| First Windows surface renderer | Direct2D + DirectWrite, following NSH's implementation pattern | Initial implementation direction | It is the first vertical slice on the development platform. Other targets use their corresponding native drawing stacks following NSH; platform adapters keep OS-specific drawing details out of shared engine contracts. |
| Browser target and Phaser preview | Standard browser runtime | Separate target | Phaser preview runs as a normal web workload in the browser. It does not require an embedded browser engine or a browser-renderer bridge inside the native UI runtime. |
| Raster and SVG decoding | `image`, `resvg` | Evaluate / adopt where required | Keep decoding off the UI thread and expose immutable resource handles. Verify formats, security limits, target support and licensing before enabling formats. |
| Geometry and coordinate units | Project-owned logical units; `euclid` as a candidate | Adapt / Evaluate | Preserve explicit logical-pixel, physical-pixel and local-coordinate spaces. Adopt a general geometry crate only if it removes duplication without leaking unrelated coordinate types into public contracts. |
| CSS conformance references | Servo Web Platform Tests (WPT) subset | Reference / test data | Reuse relevant CSS parser/cascade/selector fixtures as test inputs where licensing and harness requirements permit. Separate browser layout/paint expectations from our documented native compatibility profile. |

## 3. Target subsystem graph

The target architecture composes these choices without creating parallel sources of truth:

```text
TypeScript / TSX
      │
 upstream SolidJS
      │
 official Universal renderer
      │
 typed, versioned Host UI Protocol
      │
 sn-core::UiTree  ─────────────────────────────── authoritative tree
      │
      ├── sn-style
      │     └── Stylo adapter (primary prototype)
      │           ├── CSS parsing / selectors / cascade
      │           ├── computed values / custom properties
      │           └── invalidation and style snapshots
      │
      ├── sn-layout
      │     └── Taffy adapter
      │           └── frames / intrinsic measurement / dirty layout
      │
      ├── sn-accessibility
      │     └── AccessKit projection and action mapping
      │
      ├── sn-text
      │     └── platform text engines / font resources
      │
      └── sn-render
            └── normalized paint values / resource references
                  │
             platform-owned retained state + native control host
                  │
             Windows / macOS / Linux / iOS / Android

Browser-only path:
      Web target → normal browser runtime (including Phaser preview)
```

## 4. Non-negotiable integration contracts

1. **One authoritative UI tree.** `sn-core::UiTree` is the only committed native UI structure. Stylo's DOM traits, an AccessKit tree, Taffy node data and platform retained state are derived projections/caches.
2. **No foreign types in public contracts.** Taffy, Stylo, AccessKit and GPUI types stay behind subsystem adapters. The public SolidJS component API uses stable project-owned types.
3. **Commit before derive.** A validated UI batch commits atomically; style, layout, accessibility and render projections update from the committed revision and record that revision.
4. **Independent invalidation.** Style, layout, text measurement, accessibility and paint dirtiness are tracked separately. A paint-only change does not trigger layout.
5. **Platform ownership remains explicit.** OS windows, UI objects, text-input sessions, GPU contexts and accessibility objects are owned by their platform host/backend, not `sn-core`.
6. **Browser runtime stays separate.** The normal browser's JavaScript engine and DOM are used only by the Web target; they are not embedded into or merged with the QuickJS-NG native SolidJS runtime.
7. **Platform rendering stays replaceable.** Native drawing follows NSH's per-platform model. Platform backends own retained state, damage tracking and OS API details; those details never enter sn-core or public SDK types.

## 5. Servo Stylo reuse boundary

Servo is not adopted as a browser runtime or rendering stack. The only planned reuse is the Stylo CSS engine, subject to the prototype gate:

- standards-oriented selector matching and cascade;
- computed values, custom properties and CSS invalidation;
- CSS parser/selector/cascade fixtures from the Web Platform Tests where relevant.

Adapt Stylo behind sn-style: provide a read-only DOM facade over committed sn-core::UiTree state and keep its element/style data in a sidecar keyed by stable UiNodeId. Normalize computed values into project-owned DTOs consumed by sn-layout and the platform renderers.

Do not import Servo's browser runtime, networking/storage, browser layout and paint pipeline, compositor, embedding layer or event loop. Phaser preview is a separate Web target running in a normal browser; it does not use Servo or a renderer embedded in SolidJS Native.

## 6. Adoption gates

Before a candidate becomes an unconditional dependency:

- [ ] Verify the current stable release or exact upstream commit and maintenance status.
- [ ] Review license, NOTICE/provenance, transitive licenses and redistribution obligations.
- [ ] Check MSRV against the project's declared Rust 1.87 MSRV and pinned Rust 1.99.1 development toolchain; raise MSRV only through an explicit decision and CI update.
- [ ] Check supported OS/CPU targets, feature flags, native SDK requirements and cross-compilation.
- [ ] Confirm the API can be adapted without a second authoritative tree, scheduler or event loop.
- [ ] Measure build time, binary size, memory, startup and representative runtime workloads.
- [ ] Add subsystem conformance and failure-path tests before changing the selected implementation.
- [ ] Keep a rollback path behind the internal `sn-*` facade.

Candidate decisions are reviewed at the milestone that implements the subsystem. The presence of a dependency in an upstream Cargo manifest is evidence to investigate, not evidence that it is automatically suitable for SolidJS Native.
