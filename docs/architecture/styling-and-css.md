# Styling, CSS and Tailwind CSS

- Status: target architecture requirement.
- Review date: 2026-11-10.
- Scope: shared styling authoring model for native and web targets.
- Decision: [ADR-0009 — CSS and Tailwind CSS for Native Styling](../decisions/0009-css-and-tailwind-native-styling.md).
- Related: [Target architecture](target-architecture.md), [Platform backends](platform-backends.md), [Roadmap](../roadmap.md), [Upstream references](../reference/upstream-references.md).

## 1. Product requirement

SolidJS Native must support familiar CSS authoring and Tailwind CSS in SolidJS/TSX applications. This is a first-class SDK requirement, not an optional theme layer.

An application should be able to write:

```tsx
<button class="rounded-lg bg-blue-600 px-4 py-2 font-medium text-white hover:bg-blue-700 disabled:opacity-50">
  Save
</button>
```

It must also be possible to use `className`, composed class strings, supported inline `style` objects, application stylesheets, CSS custom properties and theme tokens.

The public authoring model should not force developers to learn a proprietary styling language. CSS concepts and Tailwind utility names should remain familiar. The framework must document where native semantics differ from browser CSS.

## 2. Two targets, one authoring intent

### 2.1. Web target

The web backend delegates styling to the browser:

- real DOM elements;
- browser CSS parser, cascade, style computation, layout and painting;
- standard CSS files and imports;
- Tailwind-generated CSS loaded by Vite;
- browser-native pseudo-classes, media queries, custom properties and accessibility behavior.

Do not emulate browser CSS through the native style engine on the web target.

### 2.2. Native targets

Native targets do not have a DOM or browser CSS engine. They consume the same TSX class/style authoring model through a deliberately specified native CSS compatibility profile:

- Tailwind CSS and its source directives are processed by the official Vite build integration;
- generated CSS is handed to the internal `sn-style` boundary, either as stylesheet text parsed once at registration/replacement or as an optional precompiled, versioned style artifact;
- the `sn-style` implementation resolves supported selectors, cascade, themes and native interaction/environment states against a derived style-tree mirror of the authoritative retained UI tree;
- computed layout properties are translated to the Taffy adapter;
- paint, typography, transform and effect properties flow into the native text/rendering pipeline;
- platform backends receive normalized values, never arbitrary raw CSS text.

The web and native targets share style intent and supported public APIs, but do not claim pixel-identical rendering or full browser CSS parity.

## 3. Build and runtime pipeline

The intended native pipeline is:

```text
TSX class / className + CSS files + @theme
                    |
          Tailwind CSS compiler
             (@tailwindcss/vite)
                    |
       generated CSS + app stylesheet
                    |
       Solid Native sn-style boundary
                    |
       CSS parsed once or optional
       precompiled style artifact
                    |
       Stylo adapter behind sn-style
       (Stylo prototype first; bounded in-house fallback; style-engine source review only)
          /             \
  Taffy layout       Paint / text / effects
          \             /
          Native MountBatch
                    |
            Platform backend
```

Requirements:

1. Use the official Tailwind CSS v4 compiler and its Vite integration as the first supported Tailwind pipeline. Do not reimplement Tailwind utility generation.
2. The `sn-style` integration consumes CSS output produced by the frontend toolchain, not Tailwind source syntax. Tailwind directives such as `@theme`, `@utility`, `@apply` and source detection remain the responsibility of Tailwind.
3. Parse each generated stylesheet at most once when it is registered/replaced, or compile it into an optional compact versioned artifact. Never parse CSS on every render or frame.
4. Resolve selector conditions against native node identity, class lists, semantic host tags, pseudo-state and environment. The resolver may use an internal matched-rule representation or a precompiled selector representation.
5. Cache computed styles. A node update must invalidate only affected style/layout/paint data.
6. Keep any serialized/precompiled style representation independent of QuickJS and platform-specific types.
7. Development mode must retain useful source locations and report unsupported declarations/selectors with file, line and column where available.

The CSS engine implementation is selected through a prototype gate, not by writing a new cascade by default. **Servo Stylo 0.18.0 is the first M3 prototype to evaluate, not an adopted dependency** because it already implements browser-grade selector matching, cascade, computed values, custom properties and invalidation. It must be adapted behind `sn-style`: provide Stylo's DOM facade over committed `sn-core::UiTree` state and keep Stylo element/style data in a sidecar keyed by stable `UiNodeId`. The tree mirror is derived and never becomes a second source of truth. The GUI-oriented `style-engine` 1.0.0 source is a source-level reference only; its manifest MSRV 1.90 prevents building it under our current 1.87 baseline. A small in-house resolver built from compatible `cssparser` + `selectors` crates is the implementation fallback if Stylo fails the Tailwind/native-profile tests or dependency gates. Lightning CSS remains optional build-time tooling, not the runtime cascade engine. See [upstream references](../reference/upstream-references.md) and [dependency adoption map](dependency-adoption.md).

Do not choose a parser solely because it tokenizes declarations. The complete pipeline must preserve the semantics required by the compatibility profile and prove that Tailwind CSS v4 output can be consumed.

## 4. Native CSS compatibility profile

This is a deliberate cross-platform profile, not an attempt to implement the entire browser CSS specification.

### 4.1. Required authoring features

- `class` and `className` aliases with string values and normal SolidJS reactive updates.
- Static stylesheets imported through the normal Vite pipeline.
- Typed inline `style={{ ... }}` values for supported properties.
- CSS custom properties and `var()` resolution.
- Theme tokens defined through Tailwind `@theme` and compatible application custom properties.
- CSS cascade order, specificity and supported inheritance, with deterministic conflict resolution.
- Responsive media conditions based on the native logical viewport and user environment.
- Native interaction states: hover, active/pressed, focus, focus-visible where supported, disabled and checked.
- Dark/light theme selection through a documented environment or root theme selector.
- Standard Tailwind spacing/sizing utilities and the explicitly supported native display/layout set: block, flow-root, flex, grid and none; Flexbox/Grid alignment and gaps; static, relative, absolute and fixed positioning only with tested containing-block semantics; and backend-owned scrolling/overflow behavior. Unsupported formatting modes are listed below.
- Common typography, color, background, border, radius, outline/ring, opacity, shadow and transform utilities.
- Supported transitions and keyframe effects mapped to the native animation driver, not a JavaScript callback every frame.
- Arbitrary Tailwind values only when the corresponding CSS property/value is inside the native compatibility profile.

### 4.2. Initial native property groups

| Group | Initial support target |
|---|---|
| Layout | block/flow-root/flex/grid/none display; box sizing; width/height and min/max constraints; margin/padding; flexbox/grid tracks, gaps and alignment; static/relative/absolute/fixed positioning with tested containing-block semantics; aspect ratio and overflow/scroll containers |
| Typography | font family, size, weight, style, line height, letter spacing, text alignment, wrapping and supported decoration |
| Color and background | foreground color, background color, opacity, supported gradients and image/resource backgrounds |
| Border and outline | border width/style/color, corner radius, outline and Tailwind ring primitives |
| Effects | box shadow, clipping, z-order and supported transform operations |
| State and responsive | pseudo-state selectors and media conditions explicitly listed by the compatibility profile |
| Animation | supported transitions and a bounded set of native-driven keyframe properties |

The exact property/value matrix must be maintained as a testable compatibility table. A property is not considered supported just because the parser accepts its syntax.

#### Taffy-specific layout boundary

Taffy 0.15.1 supplies Block, Flexbox and Grid algorithms, but it is not a complete browser formatting engine. The initial native profile supports only layout modes and position semantics that the `sn-layout` adapter can prove with conformance fixtures:

- Support `display: block`, `flex`, `grid` and `none`; `flow-root`, inline formatting (`inline`, `inline-block`), tables and `display: contents` are not part of the initial profile.
- Taffy 0.15.1 models `static`, `relative`, `absolute` and `fixed`. The custom-tree adapter must implement the out-of-flow trait methods and call `compute_oof_layout`; `TaffyTree` performs this internally, but our custom tree must supply containing-block resolution and hoisted descendants. Transforms/filters that establish containing blocks must be reflected by `CoreStyle::is_containing_block()` before descendants are computed.
- `position: sticky` is explicitly unsupported by Taffy 0.15.1 (it currently behaves as static); do not expose Tailwind sticky utilities as supported.
- Floats and `clear` are not in the selected feature set; do not enable Taffy's optional float layout merely to claim broader CSS coverage.
- Scroll offsets, wheel/touch input, kinetic scrolling and viewport lifecycle remain platform-backend responsibilities; Taffy provides layout/overflow data, not a complete native scrolling system.

A CSS declaration outside this matrix must produce the documented unsupported-rule diagnostic. The profile may expand only with layout fixtures and an explicit compatibility update.

### 4.3. Selectors and variants

The initial resolver should support the selectors required by core Tailwind utilities and common component styles:

- class selectors and compound class selectors;
- registered semantic host element selectors;
- root/theme selectors;
- supported descendant and direct-child relationships;
- supported pseudo-classes for native interaction state;
- media queries for configured breakpoints, color scheme and reduced motion.

Tailwind variants that require more advanced relationships (for example complex `group`/`peer`/relational selectors), pseudo-elements, generated content, browser-only state or DOM-specific semantics must be individually evaluated and added only with tests. Do not advertise all Tailwind variants as supported merely because Tailwind can generate them.

### 4.4. Unsupported CSS

Unsupported native CSS must not silently disappear.

- Build-time compilation should emit a diagnostic for unsupported selectors, at-rules, properties or value functions.
- Strict mode should fail the native build for unsupported rules that affect application UI.
- A documented warning mode may be used during migration, but it must report every omitted rule.
- CSS features requiring a browser engine remain available on the web target or inside an explicit WebView/browser surface.
- A custom native component may expose a typed native-specific extension where a real product need cannot be represented by the common profile.

Do not use a WebView as an invisible fallback for ordinary native controls. The platform boundary must remain explicit.

## 5. Tailwind CSS integration contract

Tailwind CSS v4 is the first target, using the official Vite plugin and CSS-first configuration.

Expected application setup:

```ts
// vite.config.ts
import { defineConfig } from "vite";
import tailwindcss from "@tailwindcss/vite";
import solid from "vite-plugin-solid";

export default defineConfig({
  plugins: [
    tailwindcss(),
    solid({ /* Solid Native Universal build settings */ }),
  ],
});
```

```css
/* src/styles.css */
@import "tailwindcss";

@theme {
  --color-brand: #2563eb;
  --radius-panel: 0.75rem;
}
```

Exact plugin ordering and Universal build configuration must be verified in the actual Vite integration; this snippet defines the intended developer experience, not a validated config for the current prototype.

Tailwind's source scanner requires complete class tokens to be present in scanned source. Applications should map variants to complete class strings rather than dynamically concatenate fragments such as `bg-${color}-500`. Packages outside the default scan paths must register their sources explicitly. This is standard Tailwind behavior and should be documented in the Solid Native starter template.

The native target must provide a compatibility report for the generated stylesheet. Tailwind may generate a valid browser rule that is outside the native profile; that rule must be reported rather than silently treated as implemented.

### 5.1. Preflight and native semantic defaults

Tailwind Preflight resets browser element defaults. Native primitives do not have browser default styles, and native controls have platform-specific semantics.

Therefore:

- native base styles are defined by Solid Native's semantic component baseline;
- Tailwind reset behavior is either translated through an explicit native reset profile or excluded from the native artifact;
- platform-native control behavior (focus, text editing, accessibility, hit targets) must not be erased by a generic CSS reset;
- web target continues to use standard Tailwind Preflight when imported.

### 5.2. Dynamic classes and runtime themes

SolidJS may change the complete class string reactively. The resolver must update the node's matched style rules without rebuilding the stylesheet.

Utilities must exist in the compiled artifact. Runtime-generated class fragments that Tailwind did not detect cannot be expected to work. Use complete string variants, explicit Tailwind source registration or documented safelisting.

Theme changes should update token bindings and affected computed styles without requiring a full application remount. Theme variables are data in the style system, not global mutable state hidden in the renderer.

## 6. Ownership and invalidation

- SolidJS owns reactive class/style inputs and application-level theme selection.
- The JS Universal adapter forwards class/style changes as typed properties; it does not calculate final layout or run a second CSS engine.
- Rust `sn-style` owns native stylesheet resources, selector matching, cascade resolution, supported inheritance, custom-property resolution and computed style snapshots.
- `sn-layout` owns geometry and maps only layout-relevant computed values to Taffy.
- Text services own text shaping/measurement based on computed typography.
- Platform renderer owns native painting and platform resources based on normalized paint values.

Dirty reasons must remain independent:

- selector/class/theme change;
- layout property change;
- paint/effect property change;
- typography/text measurement change;
- environment/media condition change;
- interaction pseudo-state change.

A color-only update must not trigger Taffy layout. A font/line-height change may invalidate text measurement and layout. A hover background change should update paint only. A viewport breakpoint change may invalidate both style and layout for affected nodes.

Computed-style caches must account for stylesheet revision, node classes/semantic identity, parent selector context, interaction state and environment inputs. Caches must be bounded and invalidated deterministically on node removal or stylesheet replacement.

## 7. Public API and compatibility rules

- Keep `class` and `className` as aliases; recommend `class` in docs only if it remains consistent with SolidJS conventions and TypeScript declarations.
- Keep inline style values typed and documented. Do not accept arbitrary JavaScript objects as an unvalidated native property bag.
- Use semantic native host primitives and registered component types; arbitrary HTML element semantics must not be assumed on native targets.
- Keep browser-only CSS APIs out of the universal public contract unless the native backend has a documented equivalent.
- Expose feature detection/target capability information for styles that differ by platform.
- Keep stylesheet registration and any optional compiled-style artifact versioning independent from the UI Host Protocol; record the `sn-style` artifact format/version compatibility in the app bundle manifest.

## 8. Acceptance criteria

Native CSS/Tailwind support is not accepted based on a stylesheet compiling successfully. The end-to-end test must show the final values reaching the Rust tree and rendering pipeline.

Minimum acceptance scenarios:

1. A SolidJS Universal TSX example uses `class`/`className` with Tailwind utilities and runs in QuickJS.
2. The Vite Tailwind pipeline emits CSS that is parsed once by the selected `sn-style` implementation or converted into an optional precompiled artifact.
3. Padding, flex alignment, width/height, typography, foreground/background color, border and radius produce the expected normalized values.
4. Reactive class changes update style without remounting the node.
5. `hover`, `focus`, `disabled`, dark theme and one responsive breakpoint update only affected style properties.
6. `@theme` tokens and CSS custom properties resolve consistently on native.
7. A color-only class change does not invalidate layout; a spacing change does.
8. Unsupported rules produce source-linked diagnostics and strict mode fails as configured.
9. The same example renders through browser CSS on the web target.
10. Golden tests cover cascade order, specificity, inheritance, variable resolution, selector matching, responsive conditions and interaction-state transitions.

## 9. Non-goals

- Full browser CSS specification and layout/painting engine on native platforms.
- DOM emulation or automatic conversion of arbitrary web applications into native controls.
- Guaranteed pixel parity between browser rendering and every OS-native backend.
- Silent fallback from unsupported native CSS into a hidden WebView.
- Running JavaScript for every native animation frame.
- A proprietary Tailwind fork or a duplicate Tailwind utility generator.
