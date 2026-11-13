# Architecture overview

This file is a short entry point. SolidJS Native is a standalone cross-platform application framework and SDK for upstream SolidJS. Tauri 2 is only an external reference for studying application-framework architecture and developer workflows; it does not define the product goal or required feature set. NSH is the primary native implementation and platform-rendering reference; GPUI Kit/GPUI Shell is the comparative desktop and QuickJS-host reference; Servo Stylo is a CSS implementation reference.

The complete specification is in:

- [Application framework scope](application-framework.md)
- [Target architecture](target-architecture.md)
- [Architecture reuse and dependency adoption map](dependency-adoption.md)
- [Styling, CSS and Tailwind CSS](styling-and-css.md)
- [SolidJS runtime and host protocol](solidjs-runtime-and-host-protocol.md)
- [Platform backends](platform-backends.md)
- [GPUI Kit comparison and reuse map](gpui-kit-comparison.md)
- [NSH implementation reference map](nsh-feature-parity.md)
- [ADR-0008](../decisions/0008-standalone-application-framework.md)
- [ADR-0009: CSS and Tailwind CSS](../decisions/0009-css-and-tailwind-native-styling.md)
- [Upstream references](../reference/upstream-references.md)

## Runtime layers

SolidJS application (TypeScript/TSX)
→ official SolidJS Universal renderer
→ Solid Native host adapter
→ typed batched host protocol
→ Rust runtime bridge
→ authoritative sn-core UI tree
→ native sn-style CSS/cascade resolution
→ sn-layout Taffy custom-tree adapter
→ text/paint/resource/accessibility services
→ immutable MountBatch
→ platform backend
→ native OS views and services

Application APIs use a separate path:

SolidJS application API
→ typed/versioned Application Bridge
→ Rust application services and plugin registry
→ capability/permission authority
→ PlatformApplicationHost / PlatformWindowHost / OS services

The Application Bridge is not the renderer-to-Rust UI Host Protocol. The former invokes native application services and delivers events; the latter commits renderer mutations into the authoritative retained UI tree.

The web target has a separate browser renderer and uses the browser's JS engine and DOM. Browser-only functionality runs through this web target in a normal browser session; it is not embedded into the native UI runtime.

## Ownership rules

- SolidJS owns reactive application state, effects, owners and callback functions.
- Rust owns the authoritative retained UI tree, normalized properties, layout inputs, event metadata and commit revisions.
- sn-layout reads only committed sn-core::UiTree state. It keeps Taffy caches and computed geometry in a derived, disposable sidecar; it never creates a second authoritative node tree or mutates UiTree.
- The application host owns process-level lifecycle, OS application shell and top-level window orchestration; platform backends own per-window/per-Surface OS objects, native input resources and platform lifecycle.
- The JS host-node mirror exists only to satisfy Solid Universal traversal; it is not authoritative for committed UI state.
- QuickJS-specific types stay inside the QuickJS runtime crate.

## Threading

Native runtime uses one owner engine thread per Surface and a separate platform UI thread. Platform UI objects are mutated only on the platform thread. User JS never runs synchronously on that thread. Layout is computed from a committed tree snapshot on the engine side, never on the platform UI thread.

## Current implementation status

M0 proves official SolidJS Universal output executes inside QuickJS-NG, mounts the counter into a JS-side host model, invokes a click callback and updates a signal-driven text node.

M1 has the transactional Rust UiTree and typed host protocol. M3 now introduces the first Taffy-backed sn-layout implementation as a derived sidecar over committed tree state. This does not claim an implemented native renderer, native windows, platform input, stylesheet cascade, or production text shaping.

The next implementation milestone is to connect style resolution and the layout snapshot to the engine's MountBatch orchestration. See [roadmap](../roadmap.md).
