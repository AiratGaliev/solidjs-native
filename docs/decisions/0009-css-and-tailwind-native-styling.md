# ADR-0009: CSS and Tailwind CSS for Native Styling

- Status: Accepted
- Date: 2026-11-10
- Decision owners: SolidJS Native project
- Related: [Target architecture](../architecture/target-architecture.md), [Styling, CSS and Tailwind CSS](../architecture/styling-and-css.md), [ADR-0005: Taffy and hybrid native rendering](0005-layout-and-native-rendering.md), [Tailwind upstream references](../reference/upstream-references.md).

## Context

SolidJS Native targets real upstream SolidJS and TypeScript/TSX. Developers need a familiar styling workflow and the ability to use existing CSS knowledge, design tokens and Tailwind CSS utilities. Styling is a product requirement for the SDK and for Codogma Engine's editor UI.

The architecture deliberately avoids a browser/WebView as the primary native renderer. Consequently, native targets cannot delegate style computation to a browser CSS engine. Taffy supplies layout algorithms, but it is not a CSS parser, cascade engine, theme resolver or paint system.

Tailwind CSS generates CSS from utility classes at build time. It does not provide the runtime style/cascade/layout engine required to apply those rules to a retained native UI tree. A native implementation therefore needs a defined boundary between CSS authoring, build-time compilation, native style resolution, layout and painting.

## Decision

1. **CSS and Tailwind CSS are first-class public SDK requirements.** The same SolidJS/TSX authoring model must work on native and web targets.
2. **Target Tailwind CSS v4 first**, using the official Tailwind Vite integration. With Vite 8, pin `@tailwindcss/vite` 4.2.2 or later. SolidJS Native will not fork Tailwind or reimplement its utility generator.
3. **The web target uses browser CSS.** Generated and authored stylesheets are handed to the browser's normal CSS engine through the web backend.
4. **Native targets implement a documented CSS compatibility profile**, not the complete browser CSS specification. The profile covers the common layout, typography, color, background, border, radius, effects, transform, responsive and interaction-state features required by the framework and its first consumers.
5. **Tailwind CSS is compiled during the Vite build; native CSS semantics are resolved by `sn-style`.** The selected resolver may parse stylesheet text once at registration/replacement or consume an optional versioned precompiled style artifact. CSS must never be parsed on every render or frame. The serialized artifact format is not mandated before the M3 prototype.
6. **A dedicated sn-style subsystem owns native style resolution.** It is responsible for stylesheet resources, selector matching, cascade, supported inheritance, CSS custom properties, Tailwind theme tokens, pseudo-state and environment conditions, computed style snapshots and compatibility diagnostics.
7. **Taffy remains a layout engine only.** sn-style computes normalized styles; sn-layout maps the layout subset to Taffy; text and paint/effect services consume their own normalized values.
8. **Unsupported native CSS is explicit.** Strict build mode fails for unsupported selectors/properties/value functions that affect native UI. Warning mode may be provided for migration, but silent dropping is forbidden.
9. **Native CSS compatibility is not a hidden WebView fallback.** Browser-only content uses the web target or an explicitly embedded WebView surface.

## Alternatives considered

### Inline typed style objects only

Rejected as the sole public styling model. It would create a proprietary API, make normal CSS/Tailwind reuse difficult and force developers to learn another styling system. Typed inline styles remain supported as one authoring option.

### WebView for all CSS and Tailwind rendering

Rejected as the default native UI architecture. It would undermine the native retained renderer, application lifecycle, platform input/accessibility integration and the product goal of native UI without a system WebView. Explicit WebView components remain supported for genuine browser content.

### Full browser CSS implementation in Rust

Deferred/rejected for the initial architecture. Full standards compatibility is a browser-engine-sized project with substantial parsing, cascade, layout, paint, text, animation and compatibility costs. It is not required to provide a practical Tailwind and CSS workflow for native application UI.

### A hand-written Tailwind class-to-style table

Rejected. It would duplicate Tailwind utility generation, fragment compatibility, miss custom utilities/arbitrary values and become version-sensitive. Use Tailwind's official compiler and pass its generated CSS to the selected native style resolver.

### Select a CSS engine without a prototype

Rejected. Stylo is the primary prototype candidate because it provides browser-grade selector matching, cascade and computed-value semantics, but its DOM facade and style-data lifecycle must be adapted to our authoritative UI tree. `style-engine` 1.0.0 is retained as a source-level comparison only because its Rust 1.90 MSRV exceeds the project's 1.87 baseline. Stylo is the only planned executable CSS prototype at M3; if it fails, use compatible `cssparser` + `selectors` as the fallback unless a separate MSRV decision is approved.

### Make Taffy the CSS engine

Rejected. Taffy is responsible for layout algorithms; it does not provide the full stylesheet parsing, selector, cascade, theme, typography, paint or platform state system required here.

## Consequences

Positive:
- SolidJS developers can use familiar CSS, class/className, Tailwind utilities, theme tokens and responsive/state variants.
- Web target retains standard browser CSS semantics.
- Native style resolution is deterministic, versioned and independent from QuickJS.
- Layout, paint and text invalidation can be tracked separately.
- Unsupported CSS is observable at build time rather than failing silently at runtime.

Costs and constraints:
- A native CSS compatibility matrix must be maintained and tested.
- The build must expose Tailwind-generated CSS to `sn-style` and retain useful source locations in diagnostics; a precompiled native artifact may be added if the prototype demonstrates a measurable benefit.
- sn-style requires selector matching, cascade, custom-property resolution and computed-style caching. Stylo is the first executable prototype to evaluate; `style-engine` is source-level reference only under MSRV 1.87. A bounded in-house resolver using compatible `cssparser` + `selectors` is the implementation fallback if Stylo fails the required compatibility profile.
- Tailwind's source detection constraints apply: complete class tokens must be discoverable, or explicitly registered/safelisted.
- CSS features that require browser semantics remain web-only or require an explicit WebView component.
- Pixel-identical output across native platforms is not guaranteed.

## Acceptance

The decision is considered implemented only when the M3 acceptance criteria in [Roadmap](../roadmap.md) pass, including end-to-end TSX → Tailwind Vite compiler → sn-style resolver (stylesheet parsed once or optional precompiled artifact) → Taffy/native paint pipeline, reactive classes, responsive/state variants, theme variables, diagnostics and web-target browser rendering.

A successful Tailwind CSS build alone is insufficient evidence of native styling support.
