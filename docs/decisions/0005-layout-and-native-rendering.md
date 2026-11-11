# ADR-0005: Taffy Layout and Hybrid Native Rendering

- Status: Accepted for initial implementation direction; individual backend details remain subject to prototypes.
- Date: 2026-11-10.

## Context

The framework needs a platform-neutral layout model but must still use native text, input and accessibility. Mapping every logical node to an operating-system child view would be expensive and would not be necessary for decorative containers.

## Decision

- Use Taffy 0.15.1 behind a `sn-layout` adapter for initial Flexbox/Grid/Block algorithms; disable default features and adapt its low-level custom-tree API to `sn-core::UiTree`.
- Keep Taffy types private to implementation layers.
- Use logical points/dp-like units in the engine; convert to physical pixels at the platform boundary.
- Maintain a platform-neutral retained tree and immutable MountBatch.
- Use MountBatch to apply committed layout/style/text/resource updates to the platform backend's retained state. The backend paints that state through native OS drawing APIs; do not add a mandatory generic DisplayList or GPU abstraction between the engine and platform renderer.
- Use platform drawing APIs for initial native surfaces: Direct2D/DirectWrite on the first Windows vertical slice, and corresponding CoreGraphics/CoreAnimation, GTK/GSK, UIKit/CoreAnimation and Android drawing integrations on other targets.
- Use retained drawing for layout containers and decorative primitives where appropriate.
- Use native platform views/components for text editing, accessibility-critical controls and embedded native resources where system behavior matters.
- Allow backends to coalesce multiple logical nodes into a native drawing surface.
- Use platform-specific text engines and measurement adapters; do not invent a cross-platform glyph shaping implementation in the first milestone.
- Adopt NSH's per-platform retained rendering model. Use each OS's native drawing stack and native controls where platform semantics require them. Keep batching/damage implementation-specific to each backend; do not select a separate unified renderer. Keep GPUI only as a comparative desktop reference; no GPUI renderer/backend is selected.

## Consequences

- Visual behavior is cross-platform by contract, but native backend internals differ.
- Text measurement, focus, IME and accessibility need backend-specific tests.
- A node is not required to map one-to-one to an OS view.
- A style subset must be documented; this is not full CSS.
- Taffy can be replaced later behind sn-layout if benchmark or feature evidence justifies it.

## Rejected alternatives

- Full browser engine with DOM, browser layout/formatting contexts and browser paint/compositing: out of scope. A browser-grade CSS style resolver (Stylo) may be adapted for the bounded native CSS profile; it does not bring Servo browser layout or renderer into the native UI engine.
- One OS view/window handle per JSX node: rejected as default strategy.
- One custom GPU renderer for every control and text input: rejected because native input/accessibility semantics would be expensive to recreate.
- Making GPUI or another external UI framework the mandatory renderer: rejected. Native rendering follows the per-platform model demonstrated by NSH; GPUI introduces its own element/input/focus/text model and remains a comparative desktop reference.
- Direct exposure of Taffy style structures as public API: rejected to preserve implementation freedom.

## Invariants

- Layout is calculated off the platform UI thread.
- Text intrinsic measurement is cacheable and revision-aware.
- Color-only changes do not trigger layout.
- Accessibility semantics are not inferred solely from drawing nodes.
