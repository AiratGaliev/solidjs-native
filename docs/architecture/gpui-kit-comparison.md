# GPUI Kit Comparison and Reuse Map

- Статус: comparative engineering reference; not a dependency decision.
- Upstream: https://github.com/longbridge/gpui-kit
- Pinned source snapshot: [6808964c3a5e0ef970e47b696e8a800575cb21a7](https://github.com/longbridge/gpui-kit/commit/6808964c3a5e0ef970e47b696e8a800575cb21a7), observed as `main` head on 2026-11-10.
- Product direction: [ADR-0008](../decisions/0008-tauri-class-application-framework.md).
- Application scope: [Tauri replacement scope](tauri-replacement-scope.md).
- Related native architecture: [NSH implementation reference and reuse map](nsh-feature-parity.md).
- Consolidated dependency decisions: [Architecture Reuse and Dependency Adoption Map](dependency-adoption.md).

## 1. Executive assessment

**GPUI Kit is a close adjacent product and a partial analogue to SolidJS Native. GPUI Shell is the closest runtime analogue.** The overlap is substantial at the product level: both combine a Rust-owned native UI host, a QuickJS-based JavaScript layer, native rendering without a mandatory WebView, application/system capabilities and an extension/application story.

They are not the same architecture or target:

- GPUI Kit is a Rust-first desktop application framework built on GPUI. Its reusable foundation is `gpui-base`; `gpui-component` adds a complete styled component system.
- `gpui-shell` adds an experimental JavaScript application/plugin runtime on top of GPUI and `gpui-base`.
- SolidJS Native is designed around actual upstream SolidJS, its official Universal renderer and TypeScript/TSX as the primary application model. Rust owns the authoritative committed UI tree and platform runtime.

This means the project is **not inventing the broad category**, and we must not duplicate GPUI Kit's interaction primitives, editor, docking, accessibility or testing solutions without first studying them. It does not mean we should replace SolidJS with GPUI or copy GPUI Shell's API.

## 2. What GPUI Kit provides

Primary sources:

- Repository and feature overview: https://github.com/longbridge/gpui-kit
- Architecture of `gpui-base`: https://github.com/longbridge/gpui-kit/blob/6808964c3a5e0ef970e47b696e8a800575cb21a7/docs/ARCHITECTURE.md
- Base crate: https://github.com/longbridge/gpui-kit/blob/6808964c3a5e0ef970e47b696e8a800575cb21a7/crates/base/README.md
- Shell crate: https://github.com/longbridge/gpui-kit/blob/6808964c3a5e0ef970e47b696e8a800575cb21a7/crates/shell/README.md
- GPUI Shell architecture: https://github.com/longbridge/gpui-kit/blob/6808964c3a5e0ef970e47b696e8a800575cb21a7/docs/gpui-shell.md
- Runtime and engine measurements: https://gpui-kit.com/shell/engine
- Capabilities: https://gpui-kit.com/shell/capabilities
- Mobile scope: https://gpui-kit.com/docs/mobile

### 2.1. GPUI Kit layers

| Layer | Responsibility |
|---|---|
| `gpui` / GPUI platform | Native element model, windowing, input, layout, GPU rendering and desktop platform integration |
| `gpui-base` | Unstyled behavior, state, focus, keyboard interactions, accessibility semantics, editing, virtualization and infrastructure |
| `gpui-component` | Styled components, theming and ready-to-use visual system |
| `gpui-shell` | QuickJS-based JavaScript application/plugin runtime, bindings, capability policy, CLI and script host |

The `gpui-kit` façade pins a matching GPUI release and re-exports GPUI, base, component and assets. Applications can use Rust directly, use the styled component system, or use `gpui-shell` for JavaScript composition and extensions.

### 2.2. GPUI Shell

GPUI Shell is especially relevant to this project:

- standalone app and plugin code is authored in JavaScript;
- the runtime embeds QuickJS and runs in the native host process;
- scripts import GPUI/GPUI Base APIs and describe their UI through JavaScript builder APIs and `View.render(cx)`;
- GPUI host owns layout, text input, focus, overlays, rendering and GPU presentation;
- script output is captured as a snapshot/description and materialized into GPUI elements; unchanged views can reuse that description on later frames;
- capabilities are granted explicitly for APIs such as filesystem, storage, clipboard, process execution and network;
- the toolchain includes run/dev/watch/check/types and application bundle workflows.

The shell's own documentation explicitly says there is no DOM, HTML, CSS, JSX or Node.js compatibility layer. Generated `.d.ts` files describe the script API; this is not a TypeScript/TSX compiler pipeline.

The shell is documented as experimental. Its architecture document also records incomplete plugin contribution registration/authorization UI and other unfinished ecosystem pieces. Treat feature claims as the exact current source snapshot, not as a stable product guarantee.

## 3. Direct comparison with SolidJS Native

| Concern | GPUI Kit / GPUI Shell | SolidJS Native target |
|---|---|---|
| Product orientation | Rust-first native desktop framework, with optional JavaScript application/plugin runtime | SolidJS/TypeScript-first native application framework and SDK |
| Primary app language | Rust/GPUI; JavaScript for Shell apps and extensions | TypeScript/TSX with upstream SolidJS |
| Reactivity | GPUI entities/state and View invalidation; script updates explicitly notify the host | Actual SolidJS signals, memos, effects, owners and cleanup |
| UI authoring | JavaScript builder chains and `View.render(cx)`; no JSX/TSX | Official Solid compiler and `solid-js/universal` custom renderer |
| Runtime boundary | Script builds a host-consumable description/snapshot; Rust materializes GPUI elements | Universal renderer mutations cross a typed/versioned protocol into the Rust authoritative retained tree |
| Rendering | GPUI element/layout/paint pipeline and GPU renderer | Solid Native UI engine and platform renderers; no required GPUI dependency |
| VM | QuickJS-based Shell runtime; engine hidden behind a Shell seam | QuickJS-NG through rquickjs is the initial native VM; own runtime adapter and scheduler |
| Threading | GPUI App and VM are documented as main-thread-owned; no script Workers | One owner engine thread per Surface and separate platform UI thread; UI thread never synchronously executes user JS |
| Components | Mature and extensive Base + styled Component catalog, including editing, tables, docking and rich content | Minimal foundation first; public Solid Native control/component layers remain target work |
| Accessibility/testing | AccessKit integration and real headless UI integration testing | Platform semantics and accessibility backends plus dedicated contract/platform test suites are target requirements |
| Desktop | Primary focus: macOS, Windows and Linux | Windows first, then macOS and Linux |
| Mobile | Experimental GPUI mobile integration; mobile is explicitly not GPUI Kit's primary target | iOS and Android are first-class target platforms, each with its own native backend |
| Web | WebAssembly support and showcases; not the same thing as a browser DOM target | Separate browser-native JavaScript/DOM backend |
| App/dev workflow | Shell CLI, hot reload, type declaration generation, script app bundles | Planned Solid Native CLI, Vite/Solid integration, HMR, native packaging and distribution |

The main difference is not merely the syntax used by application authors. The renderer boundary, reactive ownership, state lifecycle, threading and platform strategy are different.

## 4. Relationship between GPUI Kit and NSH

GPUI Kit and NSH must not be treated as unrelated references.

The current NSH documentation states that GPUI Shell was the starting point for its original mobile plugin layer. NSH then evolved into a cross-platform framework for whole applications. It deliberately uses each operating system's own UI backend on mobile and desktop standalone applications, rather than forcing GPUI's desktop renderer onto phones.

The current backend roles are distinct:

- NSH standalone desktop apps use AppKit on macOS, Win32 on Windows and GTK 4 on Linux.
- The optional NSH GPUI backend uses GPUI on macOS, Windows and Linux and embeds an NSH `AppView` inside a GPUI/GPUI Kit application.
- GPUI Kit is therefore a desktop host/rendering integration for NSH as well as a standalone framework in its own right; it is not the native backend used by NSH standalone applications by default.

Upstream evidence, pinned to the NSH snapshot reviewed for this project:

- NSH project origin and relationship: https://github.com/longbridge/nsh/blob/c16a7009674797eda8781076145c696e22144184/website/docs/index.md
- NSH backend matrix: https://github.com/longbridge/nsh/blob/c16a7009674797eda8781076145c696e22144184/website/docs/platforms.md
- GPUI embedding backend: https://github.com/longbridge/nsh/blob/c16a7009674797eda8781076145c696e22144184/website/docs/gpui.md
- NSH design goals and GPUI Kit influence: https://github.com/longbridge/nsh/blob/c16a7009674797eda8781076145c696e22144184/website/docs/goals.md

This lineage matters when evaluating a proposed reuse: a pattern found in NSH may have originated in GPUI Shell or GPUI Kit, while the current NSH native backend may have evolved it significantly.

## 5. What SolidJS Native should learn from GPUI Kit

| Area | Classification | Why |
|---|---|---|
| Base behavior separated from visual design | **Adopt as a design principle** | Keeps interaction, keyboard, focus and accessibility reusable without forcing one design system |
| Stateful controls and complex editing | **Reference / Adapt** | Study explicit state facades, edit history, selection, IME, editor architecture and presentation seams |
| Docking and large data controls | **Reference / Adapt** | Study drag/drop, serialization, virtualization, selection and resize behavior before implementing equivalents |
| Accessibility semantics | **Reference / Adapt** | Use its AccessKit integration and headless interaction tests as additional design/test references; still implement each OS accessibility bridge |
| UI integration testing | **Adopt as a quality bar** | Exercise real mounted controls, input, focus, layout and semantic output instead of testing only reducer state |
| QuickJS host and capability policy | **Reference / Adapt** | Compare callback lifetime, snapshots, explicit grants, host APIs, reload and script isolation with our ADR-0003/0007 contracts |
| CLI, HMR, type generation and packaging | **Reference / Adapt** | Study a functioning application workflow and known limitations rather than designing these tools from scratch |
| GPUI as mandatory renderer/runtime | **Do not adopt by default** | It introduces GPUI's element, layout, focus, text and event model as a core dependency and does not satisfy the complete mobile/native platform target |
| GPUI as a general Solid Native renderer/backend | **Do not plan** | NSH already demonstrates independent OS backends for standalone applications. GPUI is a desktop application-host reference, not a substitute rendering target for this framework |

## 6. Why we are not simply building GPUI Kit again

The risk of duplicating work is real in these product areas: native UI, QuickJS scripting, system capabilities, app execution, component behavior and developer tooling.

The differentiating contract for SolidJS Native must remain explicit:

1. Application code is real upstream SolidJS/TypeScript/TSX, not a custom JavaScript builder API.
2. SolidJS owns reactive application state. Rust does not reimplement its reactive graph.
3. The official Universal renderer drives a typed, bounded, versioned host protocol.
4. Rust owns the authoritative retained UI tree after atomic commit.
5. Native platform backends are independent from GPUI and use their own OS integration. The Web target uses browser DOM and browser JavaScript.
6. Native UI thread never synchronously executes user JavaScript or waits for engine work.
7. Desktop, mobile and Web have explicit independent backend/maturity requirements; WebAssembly compilation alone is not counted as browser-DOM support.

These are architectural/product contracts, not a claim that SolidJS Native is already implemented or more mature than GPUI Kit.

## 7. Decision and reuse policy

- Keep GPUI Kit as a **primary comparative reference** for native desktop application framework design, component behavior, editing, accessibility, tests and JavaScript-host integration.
- Keep GPUI Shell as the closest implementation reference for a QuickJS-powered native JavaScript app/plugin host, but do not copy its API or replace SolidJS with its custom View model.
- Keep NSH as the primary reference for multi-platform native backend architecture, especially native text/input/accessibility and platform-owned rendering.
- Do not make GPUI a mandatory dependency of SolidJS Native at this stage.
- Do not prototype a GPUI renderer merely as an alternative to the NSH per-platform backends. Consider a narrow embedding adapter only if a concrete external application already built on GPUI needs to host a Solid Native Surface; that is a separate product integration, not a renderer strategy.
- Before any code port from GPUI Kit, inspect the exact source file and preserve its Apache-2.0 notices. GPUI Kit's documentation prose and original illustrations also have a separate CC BY 4.0 licensing statement; do not copy documentation material as code or assume one license covers the entire repository.

The project should reuse proven behavior and engineering knowledge, not build a second GPUI implementation or abandon the SolidJS contracts that define its purpose.
