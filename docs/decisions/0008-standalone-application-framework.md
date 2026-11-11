# ADR-0008: Standalone Cross-Platform Application Framework

- Status: Accepted product direction.
- Date: 2026-11-11.
- Related: ADR-0001 through ADR-0007, [Application Framework Scope](../architecture/application-framework.md), [Tauri 2 reference](../reference/upstream-references.md#tauri-2--application-framework-reference), [NSH reference map](../architecture/nsh-feature-parity.md), [GPUI Kit comparison](../architecture/gpui-kit-comparison.md), [dependency adoption map](../architecture/dependency-adoption.md), [Servo Stylo CSS reference](../reference/upstream-references.md#servo-stylo--css-implementation-reference).

## Context

SolidJS Native started as a SolidJS/TSX native UI runtime with Native Shell (NSH) as its primary native implementation reference. GPUI Kit / GPUI Shell is a close comparative reference for desktop UI and a QuickJS-powered JavaScript application host. The project direction is to develop an independent, reusable application framework and SDK for upstream SolidJS and TypeScript/TSX applications.

Tauri 2 is a useful external reference because it is an established application framework with documented approaches to lifecycle, windows, commands/events, plugins, permissions, developer workflow and distribution. Its role is limited to comparative study and learning from proven patterns. It does not define the SolidJS Native product goal, required feature set or compatibility contract.

NSH is the primary implementation and platform-rendering reference for native UI and platform backends. GPUI Kit/GPUI Shell is a comparative desktop-controls and QuickJS-host reference. Servo Stylo is used only as a CSS implementation reference; the full Servo browser runtime and rendering stack are not part of the target architecture.

## Decision

1. SolidJS Native is developed as a standalone, cross-platform application framework and SDK for independent applications, not as a UI renderer or host tied to one consuming product.
2. Tauri 2 is a non-binding reference for studying application-framework architecture and developer workflows. It is not a product target, required parity checklist, compatibility target or runtime dependency.
3. Framework capabilities and implementation priorities are selected from SolidJS Native's own requirements, supported-platform constraints, security model and consumer scenarios. Studying a Tauri feature does not make that feature mandatory.
4. Native application UI uses SolidJS Universal and SolidJS Native platform renderers; an OS WebView is not the primary native UI renderer.
5. The Web target remains a separate browser-native renderer using the browser JavaScript engine and DOM. Browser-only workloads run in a normal browser session and remain separate from the native application runtime.
6. Upstream SolidJS owns application reactivity. QuickJS-NG is the initial embedded JavaScript engine for native targets. Rust owns the authoritative retained UI tree, application host services, permissions and platform orchestration.
7. The UI Host Protocol and Application Bridge are separate versioned contracts: one synchronizes renderer mutations; the other exposes native application services and events.
8. NSH is used as the primary technical reference for native rendering, platform adaptation, layout, text, input, accessibility, batching and embedding. Each proposed reuse is classified as Adopt, Adapt, Reference or Reject before implementation.
9. GPUI Kit / GPUI Shell is a close comparative reference for Rust native desktop frameworks and QuickJS-powered JavaScript application/plugin hosts. Its behavior, accessibility, editor, docking, testing and tooling solutions should be studied, but GPUI is not a mandatory dependency and SolidJS Universal/TSX remain our application contract.
10. Servo Stylo is a selective CSS implementation reference. Stylo is the first CSS resolver prototype behind sn-style for selectors, cascade, computed values, custom properties and invalidation. Reuse relevant CSS Web Platform Test fixtures; do not adopt Servo's browser runtime, browser layout/paint pipeline, compositor or embedding stack.
11. No consuming product is a privileged or required target. All consumers integrate through public, versioned SDK packages; each consumer keeps its own domain model and business logic outside the framework.
12. Standalone operation and integration into an existing host are both supported consumer scenarios. Neither requires the framework to replace or migrate an existing application host.

## Consequences

- The roadmap includes application-shell, native service, plugin, distribution and external-consumer validation milestones beyond the renderer/runtime foundation.
- Native application APIs and plugin interfaces require explicit platform support, versioning, lifecycle and permission contracts.
- QuickJS-NG, the renderer and native backends become framework-owned runtime components whose memory, scheduling, binary-size and security costs must be measured.
- The framework must preserve a Web deployment path without forcing browser APIs into native targets.
- Tauri APIs, configuration, plugins, IPC and feature coverage are not compatibility requirements. Tauri-derived ideas are evaluated individually against SolidJS Native requirements.
- NSH source reuse is possible only after checking the exact Apache-2.0-covered file, notices, dependencies and provenance; architectural study alone does not imply code has been copied.

## Rejected alternatives

- Make Tauri 2 the product target and use feature parity as the definition of success: rejected because Tauri is a reference, not the product goal.
- Tie the public API, architecture or roadmap to one first-party consumer: rejected because the framework must remain reusable by independent SolidJS applications.
- Reproduce Tauri's WebView frontend architecture: rejected because the primary native UI must use the SolidJS Native renderer.
- Reimplement NSH's Rust reactive graph or JSX compiler: rejected because upstream SolidJS and its official Universal renderer are required.
- Port NSH internals wholesale before auditing contracts and provenance: rejected because its types, threading details and runtime model differ.

## Invariants

- Native UI thread never synchronously executes user JavaScript or waits for engine work.
- UI Host Protocol and Application Bridge remain separate.
- Privileged native services enforce authorization in Rust.
- Native plugins are trusted code, not sandboxed runtime modules.
- Tauri is studied as an external reference only; no Tauri dependency, compatibility layer or feature-parity requirement is introduced by this decision.
