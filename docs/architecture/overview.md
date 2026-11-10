# Architecture overview

This file is a short entry point. The complete specification is in:

- [Target architecture](target-architecture.md)
- [SolidJS runtime and host protocol](solidjs-runtime-and-host-protocol.md)
- [Platform backends](platform-backends.md)
- [NSH feature parity](nsh-feature-parity.md)

## Runtime layers

SolidJS application (TypeScript/TSX)
→ official SolidJS Universal renderer
→ Solid Native host adapter
→ typed batched host protocol
→ Rust runtime bridge
→ authoritative sn-core UI tree
→ layout/resource/accessibility services
→ immutable MountBatch
→ platform backend
→ native OS views and services

The web target has a separate browser renderer and uses the browser's JS engine and DOM.

## Ownership rules

- SolidJS owns reactive application state, effects, owners and callback functions.
- Rust owns the authoritative retained UI tree, normalized properties, layout inputs, event metadata and commit revisions.
- Platform backends own OS objects, native input resources and platform lifecycle.
- The JS host-node mirror exists only to satisfy Solid Universal traversal; it is not authoritative for committed UI state.
- QuickJS-specific types stay inside the QuickJS runtime crate.

## Threading

Native runtime uses one owner engine thread per Surface and a separate platform UI thread. Platform UI objects are mutated only on the platform thread. User JS never runs synchronously on that thread.

## Current implementation status

M0 proves official SolidJS Universal output executes inside QuickJS-NG, mounts the counter into a JS-side host model, invokes a click callback and updates a signal-driven text node.

The Rust UiTree is currently not connected to the renderer. The typed host protocol, asynchronous scheduler, Taffy layout, native windows, platform rendering, input, text and accessibility are target work, not implemented features.

The next implementation milestone is M1: connect the SolidJS renderer to the Rust UiTree through an atomic typed batch protocol. See [roadmap](../roadmap.md).
