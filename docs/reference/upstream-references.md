# Upstream references and provenance

- Review date: 2026-11-10.
- Purpose: establish one coherent technology selection from mature upstream projects and document exact reuse boundaries.
- These references do not make an upstream project a runtime dependency. A dependency is adopted only after the subsystem gate in [Dependency Adoption Map](../architecture/dependency-adoption.md) and [Dependency Policy](../dependency-policy.md).

## Reference roles

| Project / component | Role in SolidJS Native | Reuse decision |
|---|---|---|
| Tauri 2 | Product-level completeness: app lifecycle, windows, menus, plugins, permissions, CLI, packaging, updates and developer workflow | Reference; do not use its WebView as our native UI renderer |
| Upstream SolidJS + Universal | Application reactivity, ownership and official custom-renderer contract | Adopt; keep real SolidJS and TSX |
| QuickJS-NG + rquickjs | Embedded JavaScript runtime for native targets | Adopted in current M0 prototype; isolate behind runtime API |
| Native Shell (NSH) | Primary native implementation reference: retained tree, commit batching, per-platform backends, text/input/accessibility and embedding | Adapt architectural patterns; do not port its runtime wholesale |
| GPUI Kit / GPUI Shell | Desktop-native controls, behavior primitives, editor/docking, testing and QuickJS application host | Comparative reference only; no GPUI renderer/backend selected |
| Servo Stylo | Standards-oriented CSS selectors, cascade, computed values and invalidation | CSS prototype candidate only; no Servo browser runtime or renderer adoption |
| Taffy | Flexbox, Grid and Block layout algorithms | Selected internal layout dependency behind sn-layout |
| AccessKit | Semantic accessibility tree, incremental updates, actions and platform adapters | Adopt as semantic accessibility layer; platform integration pending |
| Tailwind CSS v4 | Utility-first CSS authoring and build-time generation | Selected authoring pipeline; native support is defined by our compatibility profile |
| Lightning CSS | CSS AST, parsing, transforms and minification at build time | Build tooling only; not the native runtime style engine |
| style-engine | GUI-oriented Rust CSS resolver with optional layout/text/paint subsystems | Source-level comparison only; MSRV 1.90 excludes it under current project MSRV 1.87 |
| OS text engines | DirectWrite, CoreText, Pango and Android text APIs | Initial platform-specific text and editing strategy |
| read-fonts / skrifa / HarfBuzz | Font parsing, metrics and portable shaping building blocks | Evaluate for specific requirements; do not import Servo fonts wholesale |
| image / resvg | Raster and SVG decoding | Evaluate behind asynchronous resource pipeline |
| euclid | Typed 2D geometry and coordinate spaces | Optional utility; project-owned unit contracts remain authoritative |

The single consolidated decision, including ownership boundaries and prototype gates, is [Architecture Reuse and Dependency Adoption Map](../architecture/dependency-adoption.md).

## Tauri 2 — application-framework scope reference

- Repository: https://github.com/tauri-apps/tauri
- Architecture: https://v2.tauri.app/concept/architecture/
- Inter-process communication: https://v2.tauri.app/concept/inter-process-communication/
- Plugin development: https://v2.tauri.app/develop/plugins/
- Capabilities: https://v2.tauri.app/security/capabilities/
- Runtime authority: https://v2.tauri.app/security/runtime-authority/
- Permission model: https://v2.tauri.app/reference/acl/permission/
- Updater: https://v2.tauri.app/plugin/updater/
- Distribution: https://v2.tauri.app/distribute/

Study Tauri for application lifecycle and configuration, windows, menus, tray, shortcuts, commands/events, plugins, capability scopes, CLI, diagnostics, installers, signing and updates. Tauri's main UI is HTML in an OS WebView. SolidJS Native instead targets a native retained UI on native platforms; Tauri is not our native renderer and its IPC format is not our renderer protocol.

## GPUI Kit / GPUI Shell — desktop and QuickJS host reference

- Repository: https://github.com/longbridge/gpui-kit
- Reviewed snapshot: https://github.com/longbridge/gpui-kit/commit/6808964c3a5e0ef970e47b696e8a800575cb21a7
- Base architecture: https://github.com/longbridge/gpui-kit/blob/6808964c3a5e0ef970e47b696e8a800575cb21a7/docs/ARCHITECTURE.md
- Base behavior layer: https://github.com/longbridge/gpui-kit/blob/6808964c3a5e0ef970e47b696e8a800575cb21a7/crates/base/README.md
- Shell crate: https://github.com/longbridge/gpui-kit/blob/6808964c3a5e0ef970e47b696e8a800575cb21a7/crates/shell/README.md
- GPUI Shell architecture: https://github.com/longbridge/gpui-kit/blob/6808964c3a5e0ef970e47b696e8a800575cb21a7/docs/gpui-shell.md
- Mobile scope: https://gpui-kit.com/docs/mobile
- Detailed comparison: [GPUI Kit comparison](../architecture/gpui-kit-comparison.md)

Study GPUI Kit for the separation of unstyled behavior/state from visual components, focus and keyboard behavior, complex editing, docking, virtualization, accessibility, headless integration tests and developer tooling. GPUI Shell is a close analogue for a QuickJS-hosted native application/plugin model, but it uses JavaScript builder APIs and GPUI View descriptions, not upstream SolidJS Universal or TSX. Its runtime is experimental. GPUI remains a comparative desktop/application-host reference only; NSH per-platform backends are the selected native implementation model.

## Native Shell (NSH) — primary native implementation reference

- Repository: https://github.com/longbridge/nsh
- Source snapshot reviewed: [c16a7009674797eda8781076145c696e22144184](https://github.com/longbridge/nsh/commit/c16a7009674797eda8781076145c696e22144184)
- Product docs: https://nsh.dev/docs/
- Architecture: https://nsh.dev/docs/architecture/
- Platform overview: https://nsh.dev/docs/platforms/
- Developer architecture: https://github.com/longbridge/nsh/blob/c16a7009674797eda8781076145c696e22144184/docs/dev/architecture.md
- Native pipeline: https://github.com/longbridge/nsh/blob/c16a7009674797eda8781076145c696e22144184/docs/dev/native-pipeline.md
- Windows backend: https://github.com/longbridge/nsh/blob/c16a7009674797eda8781076145c696e22144184/docs/dev/windows.md
- Text subsystem: https://github.com/longbridge/nsh/blob/c16a7009674797eda8781076145c696e22144184/docs/dev/text.md
- Embedding: https://github.com/longbridge/nsh/blob/c16a7009674797eda8781076145c696e22144184/docs/dev/embedding.md

Useful implementation paths:
- Taffy-backed layout tree: https://github.com/longbridge/nsh/blob/c16a7009674797eda8781076145c696e22144184/crates/nsh-core/src/core/layout_tree.rs
- Core text model and measurement: https://github.com/longbridge/nsh/blob/c16a7009674797eda8781076145c696e22144184/crates/nsh-core/src/text.rs
- Windows retained drawing: https://github.com/longbridge/nsh/blob/c16a7009674797eda8781076145c696e22144184/crates/nsh-platform-windows/src/render.rs
- Windows text shaping: https://github.com/longbridge/nsh/blob/c16a7009674797eda8781076145c696e22144184/crates/nsh-platform-windows/src/text.rs
- Windows text input / RichEdit: https://github.com/longbridge/nsh/blob/c16a7009674797eda8781076145c696e22144184/crates/nsh-platform-windows/src/text_input.rs
- Windows UI Automation: https://github.com/longbridge/nsh/blob/c16a7009674797eda8781076145c696e22144184/crates/nsh-platform-windows/src/access/uia.rs

Reuse patterns: platform-independent logical tree; committed operation batches; engine-thread versus OS UI-thread ownership; Taffy behind a project-specific tree/cache/measurement adapter; platform text engines; specialized native text-input hosts; native accessibility exposure; and OS-specific resource lifecycle. NSH's Windows backend draws ordinary retained nodes into a Direct2D surface instead of creating a child HWND for every logical node.

Do not copy NSH's Rust-native reactive graph, JSX compiler or runtime as the application layer. SolidJS Native uses actual upstream SolidJS, the official Universal renderer, QuickJS-NG and its own typed/versioned host protocol. NSH's platform-specific behavior is evidence to study, not proof that every platform must use identical implementation details. See [NSH implementation reference map](../architecture/nsh-feature-parity.md).

## Servo Stylo — CSS implementation reference

- Repository: https://github.com/servo/stylo
- Reviewed release/source snapshot: [Stylo 0.18.0](https://github.com/servo/stylo/tree/8bde0e96db888b7f188830bea261e72b2006fd73) / [commit `8bde0e96db888b7f188830bea261e72b2006fd73`](https://github.com/servo/stylo/commit/8bde0e96db888b7f188830bea261e72b2006fd73)
- Servo style-system overview: https://book.servo.org/design-documentation/style.html
- WPT CSS fixtures: https://github.com/servo/servo/tree/363532e7c428b3974c94de828f97684474cb6f77/tests/wpt

Stylo is relevant as an implementation reference for CSS selector matching, cascade, computed values, custom properties and style invalidation. The candidate integration is limited to sn-style: provide a read-only DOM facade over committed sn-core::UiTree state, store Stylo style data in a sidecar keyed by stable UiNodeId, and normalize computed values into project-owned DTOs for layout and platform rendering.

Servo is not selected as an embedded browser, native renderer, compositor or runtime dependency. Do not adopt its browser layout/paint pipeline, embedding/event-loop integration or browser process topology. Phaser preview is a separate Web target executed in a normal browser session.

Stylo 0.18.0's `style/Cargo.toml` enables the `servo` feature by default. That feature pulls additional browser-oriented crates/features including `encoding_rs`, `web_atoms`, `mime`, `url`, `string_cache`, `stylo_atoms` and serialization support; the `gecko` path additionally brings bindgen/Gecko integration. The style crate does not declare a `rust-version`, so MSRV cannot be inferred from its version or Servo workspace alone. The M3 prototype must inspect the full resolved graph/build scripts, test whether `default-features = false` is viable for our adapter, and avoid enabling Gecko-only features. Stylo adoption remains subject to a clean build, supported target platforms, API stability, MPL-2.0 review and Tailwind v4/native-profile conformance. Its integration surface is accepted only if it saves substantial standards-engine work and can be isolated behind `sn-style`.

## CSS and layout building blocks

### Tailwind CSS v4

- Documentation: https://tailwindcss.com/docs
- Official Vite integration: https://tailwindcss.com/docs/installation/using-vite
- Theme variables: https://tailwindcss.com/docs/theme
- Source detection: https://tailwindcss.com/docs/detecting-classes-in-source-files
- State variants: https://tailwindcss.com/docs/hover-focus-and-other-states

Tailwind owns source scanning, utility generation and CSS-first directives. Native targets feed its generated stylesheet into `sn-style` and implement a documented native CSS compatibility profile. A successful Tailwind build does not prove a generated rule is supported natively.

### Lightning CSS

- Repository: https://github.com/parcel-bundler/lightningcss
- Rust API: https://docs.rs/lightningcss/latest/lightningcss/
- Documentation: https://lightningcss.dev/

Lightning CSS is an optional build-stage reference for typed AST inspection, transforms and minification. Do not add it directly unless a concrete build pipeline requirement is not already covered by Tailwind's Vite integration. It is not a runtime cascade/computed-style/layout engine.

### Taffy

- Repository: https://github.com/DioxusLabs/taffy
- API documentation: https://docs.rs/taffy/

Taffy 0.15.1 is selected for the M3 layout prototype behind `sn-layout` (MSRV 1.71, MIT). Prefer its low-level custom-tree traits over the built-in `TaffyTree` when adapting the authoritative `sn-core::UiTree`; keep Taffy types private. Set `default-features = false` and enable only `std`, `flexbox`, `grid`, `block_layout`, `calc` and `content_size`. Implement out-of-flow layout through the custom-tree trait methods and `compute_oof_layout` API; this is not a Cargo feature. A custom tree must also implement containing-block resolution and hoisted-child traversal for absolute/fixed descendants. Taffy 0.15.1 exposes a `Position::Sticky` enum value but explicitly lays sticky boxes out like static boxes; it does not provide sticky scrolling behavior. It is not an inline formatting engine or a full table layout engine; those semantics remain outside the initial CSS profile. The adapter still owns intrinsic measurement, caching, dirty propagation, unit normalization and stable output contracts.

### style-engine — source-level CSS reference (not an executable candidate at current MSRV)

- Repository: https://github.com/chh-itt/style-engine
- Crates.io: https://crates.io/crates/style-engine
- Reviewed v1.0.0 source: https://github.com/chh-itt/style-engine/commit/ade9a961cdac2a5a29ccc7d3a897cd527ca59eba
- Feature registry: https://github.com/chh-itt/style-engine/blob/ade9a961cdac2a5a29ccc7d3a897cd527ca59eba/docs/FEATURES.md

The crate offers CSS parsing/selectors/cascade and computed values, alongside optional layout, text and paint APIs. Its v1.0.0 manifest requires Rust 1.90, so it cannot be built under the project's current MSRV 1.87 even with optional features disabled. Keep it as a source-level reference only; do not add it to the dependency graph, raise MSRV or adopt its layout/text/paint contracts merely to compare CSS semantics.

## Unified vector renderer alternative — Vello (not selected)

- Repository: https://github.com/linebender/vello
- Reviewed source snapshot: https://github.com/linebender/vello/tree/6293f3b74498582752c3734888608030dfd8cc0e
- Architecture: https://github.com/linebender/vello/blob/6293f3b74498582752c3734888608030dfd8cc0e/ARCHITECTURE.md
- GPU renderer: https://github.com/linebender/vello/tree/6293f3b74498582752c3734888608030dfd8cc0e/vello_gpu

The Vello family contains distinct implementations, not one interchangeable renderer: `vello_cpu` 0.3.0 is the CPU path; `vello_gpu` 0.3.0 is the wgpu-based GPU path and requires the host to manage the device, queue, encoder and render target; `vello` 0.11.0 is the earlier compute-centric research renderer. `vello_gpu` is still under active development and has documented limitations in masks, filter graphs, blend modes and parts of resource/glyph handling. The compute-centric `vello` 0.11 line is experimental and is not the GPU path to evaluate for this project.

Vello supplies vector rasterization only. It does not replace the retained UI tree, CSS/layout, window lifecycle, native controls, text input/IME, accessibility or per-platform resource ownership. NSH already provides the better-matched architecture reference: platform-owned retained state and each OS's native drawing stack. Therefore no Vello crate, `wgpu` dependency or generic GPU renderer is selected. Revisit `vello_gpu` only if representative benchmarks show that maintaining native renderers duplicates substantial drawing work and an isolated prototype measurably reduces total complexity without compromising native text, controls or accessibility.

## Accessibility, rendering, text and resources

### AccessKit

- Repository: https://github.com/AccessKit/accesskit
- API: https://docs.rs/accesskit/
- Platform adapters: https://github.com/AccessKit/accesskit#platform-adapters

AccessKit provides a platform-neutral semantic tree, incremental tree-update model and action-request contract. Its Windows, macOS, Unix/Linux, Android and iOS adapters are separate crates with independently versioned releases and only rough feature parity. It is selected as the semantic tree/action layer for `sn-accessibility`: map stable `UiNodeId` values to AccessKit IDs, publish updates after tree commits and route actions through the engine event queue. Per-platform adapter integration and supported roles/actions must be tested in the Solid Native host; do not assume full parity, a released Web adapter or rich-text/hypertext support.

It does not replace focus ownership, native text editing, IME, selection or platform lifecycle. Validate advanced text-range requirements separately because rich-text/hypertext semantics are not a complete replacement for a dedicated editor model.

### Text and fonts

- `read-fonts`: https://github.com/googlefonts/fontations/tree/main/read-fonts
- `skrifa`: https://github.com/googlefonts/fontations/tree/main/skrifa
- HarfBuzz: https://harfbuzz.github.io/

Initial text and editing use the native platform services described in [Platform Backends](../architecture/platform-backends.md): DirectWrite, CoreText, Pango and Android text APIs. Evaluate `read-fonts`/`skrifa` for font parsing and metrics and HarfBuzz for portable shaping only where requirements justify them. Do not import Servo's entire `fonts` crate: it includes browser-specific platform and font infrastructure. Editable controls, selection and IME remain separate native host responsibilities.

### Image, SVG and geometry utilities

- image-rs: https://github.com/image-rs/image
- resvg: https://github.com/linebender/resvg
- euclid: https://github.com/servo/euclid

Evaluate `image` and `resvg` behind an asynchronous resource decoder with explicit limits, immutable resource handles and cache/lifecycle ownership. `euclid` may help with typed coordinate spaces, but project-owned logical/physical unit contracts remain authoritative unless a measured integration removes duplication without coupling public APIs to its types.

## SolidJS Universal and QuickJS-NG

- SolidJS Universal README: https://github.com/solidjs/solid/blob/main/packages/solid/universal/README.md
- SolidJS repository: https://github.com/solidjs/solid
- vite-plugin-solid: https://github.com/solidjs/vite-plugin-solid
- QuickJS-NG: https://github.com/quickjs-ng/quickjs
- rquickjs: https://github.com/DelSkayn/rquickjs

SolidJS Universal is the official custom renderer contract. Do not introduce another reactive graph or a custom JSX runtime. The current project pins `rquickjs` 0.14.0, whose published `rquickjs-sys` source embeds a QuickJS-NG 0.16.2 snapshot; upstream QuickJS-NG is 0.18.0, so M2 must review the delta and binding compatibility before upgrading. The rquickjs 0.14.0 platform table marks Windows MSVC experimental and does not establish iOS/Android support; cross-target builds and on-device runtime tests are release gates. rquickjs does not itself provide sandboxing or concurrent VM access. Preserve one owner thread per VM and explicitly implement scheduling, callback lifetime, quotas, cleanup and capability enforcement.

### Longbridge quickjs-jit — JIT distribution candidate

- Repository: https://github.com/longbridge/quickjs-jit
- Reviewed package: https://docs.rs/crate/quickjs-jit/0.14.2

Longbridge's `quickjs-jit` is a downstream distribution of `rquickjs`: the Cargo package is named `quickjs-jit`, but the Rust library is still imported as `rquickjs`. It is based on `rquickjs` 0.14.0 and QuickJS-NG 0.16.2+ (commit `0fdea21`), so it does not provide the upstream QuickJS-NG 0.18.0 update being evaluated separately.

The binding crate does not by itself establish that machine-code JIT is active. JIT support requires the separate, exactly version-matched `quickjs-jit-runtime` crate with its `compiler` feature and explicit runtime attachment. Keep all JIT crates on the same patch release because their ABI depends on internal QuickJS layouts.

NSH's workspace manifest selects this distribution under the `rquickjs` alias; this is evidence of the binding choice, not proof that its application runtime activates JIT. Inspect the runtime attachment path before making that claim. SolidJS Native keeps upstream `rquickjs` for M0; this distribution is only an M2 benchmark candidate after the Host Protocol is stable.

## NSH source license and code reuse

The reviewed NSH repository contains a root Apache-2.0 license and its workspace manifest declares Apache-2.0 for the project. NSH-authored source may be copied or modified only after checking the exact file and satisfying the license conditions, including retaining license/copyright/patent/trademark and attribution notices, including the license text, and marking modified files where required. This does not automatically cover third-party dependencies, bundled assets or contributed files, which need their own provenance review.

- License: https://github.com/longbridge/nsh/blob/c16a7009674797eda8781076145c696e22144184/LICENSE-APACHE
- Workspace manifest: https://github.com/longbridge/nsh/blob/c16a7009674797eda8781076145c696e22144184/Cargo.toml

No NSH source code has been copied into SolidJS Native by these documentation changes.

## Reuse classification and provenance

Classify each proposed reuse before implementation:

- **Adopt** — use a compatible public dependency or design behind our own stable adapter after version/license/platform checks.
- **Adapt** — preserve proven behavior/architecture but implement it against Solid Native contracts.
- **Reference** — study design, tests and trade-offs without copying implementation.
- **Reject** — record why the approach does not fit.

For copied source code, record the exact upstream commit, file path, copyright notices, license, required NOTICE content, local modifications and validation tests. A repository-level license does not automatically cover every bundled dependency, asset or contributed file. No source code from Servo, NSH or GPUI Kit is copied by these documentation references.
