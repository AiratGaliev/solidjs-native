# ADR-0006: SolidJS/TSX as Primary Application Model

- Status: Accepted for initial product direction.
- Date: 2026-11-10.

## Context

The project is explicitly SolidJS Native. A Rust-first UI authoring model like NSH would introduce a second application-facing component system and duplicate examples/tooling. The engine and platform layers are Rust regardless of the UI authoring language.

## Decision

- SolidJS/TSX is the primary and first-class application authoring API.
- Rust is used for the engine, platform backends and host-registered native extensions/services.
- A Rust-only declarative UI authoring path is not required for the first stable release.
- Native components are registered through typed schemas, event metadata and lifecycle contracts.
- Web uses browser JavaScript and a DOM renderer, not embedded QuickJS.
- Native platforms use QuickJS-NG through the runtime adapter.
- Shared components use a documented cross-platform API subset, with capability checks for platform-only features.

## Consequences

- App developers use the normal SolidJS mental model and TypeScript tooling.
- Rust business logic can be exposed through explicit native modules where necessary.
- Native extension APIs need strong typing and ABI/version boundaries.
- Browser and native runtime compatibility must be tested independently.

## Rejected alternatives

- Recreate NSH Rust signal/computed/effect APIs: rejected as a duplicate reactive model.
- Use QuickJS inside a browser page: rejected because the browser already provides a JS runtime and DOM.
- Require native application logic to be authored in Rust: rejected because TSX/SolidJS is the product focus.
