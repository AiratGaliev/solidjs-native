# ADR-0005: Taffy Layout and Hybrid Native Rendering

- Status: Accepted for initial implementation direction; individual backend details remain subject to prototypes.
- Date: 2026-11-10.

## Context

The framework needs a platform-neutral layout model but must still use native text, input and accessibility. Mapping every logical node to an operating-system child view would be expensive and would not be necessary for decorative containers.

## Decision

- Use Taffy behind a sn-layout adapter for initial Flexbox/Grid/Block algorithms.
- Keep Taffy types private to implementation layers.
- Use logical points/dp-like units in the engine; convert to physical pixels at the platform boundary.
- Maintain a platform-neutral retained tree and immutable MountBatch.
- Use retained drawing for layout containers and decorative primitives where appropriate.
- Use native platform views/components for text editing, accessibility-critical controls and embedded native resources where system behavior matters.
- Allow backends to coalesce multiple logical nodes into a native drawing surface.
- Use platform-specific text engines and measurement adapters; do not invent a cross-platform glyph shaping implementation in the first milestone.

## Consequences

- Visual behavior is cross-platform by contract, but native backend internals differ.
- Text measurement, focus, IME and accessibility need backend-specific tests.
- A node is not required to map one-to-one to an OS view.
- A style subset must be documented; this is not full CSS.
- Taffy can be replaced later behind sn-layout if benchmark or feature evidence justifies it.

## Rejected alternatives

- Full browser CSS engine: out of scope.
- One OS view/window handle per JSX node: rejected as default strategy.
- One custom GPU renderer for every control and text input: rejected because native input/accessibility semantics would be expensive to recreate.
- Direct exposure of Taffy style structures as public API: rejected to preserve implementation freedom.

## Invariants

- Layout is calculated off the platform UI thread.
- Text intrinsic measurement is cacheable and revision-aware.
- Color-only changes do not trigger layout.
- Accessibility semantics are not inferred solely from drawing nodes.
