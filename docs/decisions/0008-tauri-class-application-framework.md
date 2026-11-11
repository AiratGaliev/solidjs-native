# ADR-0008: Tauri-Class Application Framework for Codogma

- Status: Accepted product direction.
- Date: 2026-11-10.
- Related: ADR-0001 through ADR-0007, [Tauri replacement scope](../architecture/tauri-replacement-scope.md), [NSH reference map](../architecture/nsh-feature-parity.md), [GPUI Kit comparison](../architecture/gpui-kit-comparison.md), [dependency adoption map](../architecture/dependency-adoption.md), [Servo Stylo CSS reference](../reference/upstream-references.md#servo-stylo--css-implementation-reference).

## Context

SolidJS Native started as a SolidJS/TSX native UI runtime with Native Shell (NSH) as its primary native implementation reference. GPUI Kit / GPUI Shell is a close comparative reference for desktop UI and a QuickJS-powered JavaScript application host. The intended consumer is Codogma Engine, whose editor currently uses Tauri 2 as its native application host.

A UI renderer alone is not enough to replace that host. Codogma needs an installable application framework with native windows, application lifecycle, system services, plugins, permissions, developer tooling, packaging, signing and updates. This capability set should be available to other SolidJS applications as well.

Tauri 2 is the product-level reference for application-framework completeness. NSH is the primary implementation and platform-rendering reference for native UI and platform backends. GPUI Kit/GPUI Shell is a comparative desktop-controls and QuickJS-host reference. Servo Stylo is used only as a CSS implementation reference; the full Servo browser runtime and rendering stack are not part of the target architecture.

## Decision

1. SolidJS Native is developed as a standalone, cross-platform application framework and SDK, not only as a UI renderer or Codogma-specific host.
2. The target is a functional Tauri 2 alternative for Codogma Engine's selected application scenarios. It is not a drop-in, source-compatible or API-compatible Tauri replacement.
3. Native application UI uses SolidJS Universal and SolidJS Native platform renderers; an OS WebView is not the primary native UI renderer.
4. The Web target remains a separate browser-native renderer using the browser JavaScript engine and DOM. Browser-only workloads, including Phaser preview, run in a normal browser session and remain separate from the native application runtime.
5. Upstream SolidJS owns application reactivity. QuickJS-NG is the initial embedded JavaScript engine for native targets. Rust owns the authoritative retained UI tree, application host services, permissions and platform orchestration.
6. The UI Host Protocol and Application Bridge are separate versioned contracts: one synchronizes renderer mutations; the other exposes native application services and events.
7. Tauri 2 is used as a product and developer-workflow reference: app/window lifecycle, menus/tray, commands/events, plugins, permissions/capabilities, CLI, packaging, signing and updates.
8. NSH is used as the primary technical reference for native rendering, platform adaptation, layout, text, input, accessibility, batching and embedding. Each proposed reuse is classified as Adopt, Adapt, Reference or Reject before implementation.
9. GPUI Kit / GPUI Shell is a close comparative reference for Rust native desktop frameworks and QuickJS-powered JavaScript application/plugin hosts. Its behavior, accessibility, editor, docking, testing and tooling solutions should be studied, but GPUI is not a mandatory dependency and SolidJS Universal/TSX remain our application contract.
10. Servo Stylo is a selective CSS implementation reference. Stylo is the first CSS resolver prototype behind sn-style for selectors, cascade, computed values, custom properties and invalidation. Reuse relevant CSS Web Platform Test fixtures; do not adopt Servo's browser runtime, browser layout/paint pipeline, compositor or embedding stack.
11. Codogma Engine is the first major consumer, but it must depend on public versioned SDK packages. Codogma's game runtime and project core do not depend on SolidJS Native.
12. Existing Tauri integration in Codogma remains until the Tauri replacement acceptance gate is satisfied by a real editor application.

## Consequences

- The roadmap must include application-shell and distribution milestones beyond the existing renderer/runtime milestones.
- Native application APIs and plugin interfaces require explicit platform support, versioning, lifecycle and permission contracts.
- QuickJS-NG, the renderer and native backends become framework-owned runtime components whose memory, scheduling, binary-size and security costs must be measured.
- The framework must preserve a Web deployment path without forcing browser APIs into native targets.
- Tauri API compatibility is not a goal; Codogma migration will use a deliberate adapter/refactor against Solid Native's public SDK.
- NSH source reuse is possible only after checking the exact Apache-2.0-covered file, notices, dependencies and provenance; architectural study alone does not imply code has been copied.

## Rejected alternatives

- Keep SolidJS Native as only a renderer/runtime and leave all application hosting permanently to Tauri: rejected because it does not meet the product goal.
- Make SolidJS Native a Codogma-only fork: rejected because the SDK should be reusable by other SolidJS projects.
- Reproduce Tauri's WebView frontend architecture: rejected because the primary native UI must use the SolidJS Native renderer.
- Reimplement NSH's Rust reactive graph or JSX compiler: rejected because upstream SolidJS and its official Universal renderer are required.
- Port NSH internals wholesale before auditing contracts and provenance: rejected because its types, threading details and runtime model differ.

## Invariants

- Native UI thread never synchronously executes user JavaScript or waits for engine work.
- UI Host Protocol and Application Bridge remain separate.
- Privileged native services enforce authorization in Rust.
- Native plugins are trusted code, not sandboxed runtime modules.
- No Tauri dependency is introduced into SolidJS Native itself.
- Codogma does not remove Tauri until the documented migration gate passes.
