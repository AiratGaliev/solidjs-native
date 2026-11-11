# ADR-0001: SolidJS Universal and QuickJS-NG

- Status: Accepted for prototype; subject to runtime compatibility tests
- Date: 2026-11-10

## Context

The framework needs a declarative TypeScript UI layer without a browser DOM or WebView. NSH demonstrates a Rust-owned host with an embedded JavaScript runtime, but its custom JSX compiler and reactive graph are not desired.

## Decision

- Use the upstream SolidJS 1.x runtime and its solid-js/universal custom renderer.
- Use the upstream Solid compiler through vite-plugin-solid, configured with generate: universal and moduleName: solid-native.
- Start with QuickJS-NG through the rquickjs Rust bindings; the current pinned binding embeds the QuickJS-NG 0.16.2 source snapshot.
- Emit native TS/JS bundles targeting ES2020 and execute the actual Vite output in the pinned embedded runtime as an acceptance test.
- Keep Rust-specific host state and platform integration in Rust crates.
- Do not duplicate Solid's reactive graph in Rust.

## Consequences

- QuickJS compatibility must be validated against Solid's scheduler and runtime built-ins; passing a basic JS evaluation test is not sufficient.
- Browser APIs are not assumed to exist. Timers, microtasks, MessageChannel, performance, animation-frame scheduling and web globals need an explicit compatibility inventory.
- The VM is not Node.js or a browser. Host globals must be individually implemented and tested; missing APIs must not be silently exposed as incomplete stubs.
- Vite is a development/build tool, not a runtime dependency.
- Bytecode and bundled artifacts must be versioned against the compiler/runtime combination.
- A future JavaScript engine replacement is possible only behind a narrow runtime boundary; this ADR does not promise drop-in VM portability.
