# SolidJS Native

**TSX-first native UI framework powered by the real SolidJS runtime.**

SolidJS Native is an experimental cross-platform UI runtime being designed as a functional alternative to Native Shell (NSH). The application API is based on SolidJS and TypeScript/TSX. Native runtime, retained UI tree, layout, event orchestration and platform integration are implemented in Rust.

This is not a WebView wrapper. Native targets are designed to use platform backends; the Web target uses the browser's JavaScript engine and DOM.

## Product direction

- Upstream SolidJS 1.x is the only owner of reactive application state and scheduling.
- solid-js/universal is the official custom-renderer boundary.
- QuickJS-NG via rquickjs is the initial embedded JS engine for native targets.
- Rust owns the authoritative retained UI tree, host transaction pipeline, layout inputs, event metadata and native resource orchestration.
- Platform backends own OS windows/views, text input, accessibility, native resources and UI-thread lifecycle.
- Renderer mutations are designed to cross the JS/Rust boundary as typed, versioned batches.
- Layout is planned through a Taffy adapter.
- Native UI thread must never synchronously execute user JS.
- Web uses a separate browser-native renderer rather than QuickJS.

## Documentation

Start with [the documentation index](docs/README.md).

Dependency versions follow the [latest-stable dependency policy](docs/dependency-policy.md); NSH versions are not inherited.

Key documents:

- [Target architecture](docs/architecture/target-architecture.md)
- [SolidJS runtime and host protocol](docs/architecture/solidjs-runtime-and-host-protocol.md)
- [Platform backends](docs/architecture/platform-backends.md)
- [NSH feature parity](docs/architecture/nsh-feature-parity.md)
- [Implementation roadmap](docs/roadmap.md)
- [Architecture decisions](docs/decisions/)

## Current workspace

- crates/sn-core — minimal platform-independent retained UI tree and invariants.
- crates/sn-runtime — current QuickJS-NG runner/smoke host; it will be refactored into a reusable runtime library.
- packages/solid-native — Solid Universal renderer adapter and initial host primitives.
- examples/counter — TSX runtime smoke test.
- docs — target architecture, runtime contracts, platform design, roadmap and ADRs.

The future crate graph is specified in the architecture documents. It should be extracted incrementally when interfaces and ownership boundaries become real, not created as a set of empty packages.

## Prerequisites

- Rust 1.99.1 stable for development (Windows MSVC; pinned in `rust-toolchain.toml`); declared MSRV is 1.87.
- Node.js 22.12+ (local checks use Node 24.22.1; Node 26 Active LTS is the current upgrade target).
- pnpm 12.18.0 (pinned through `packageManager` and Corepack).

## Development

Install dependencies:

    pnpm install --frozen-lockfile

Type-check and build:

    pnpm typecheck
    pnpm build

Run the current QuickJS smoke test:

    pnpm smoke

Run Rust tests and lint:

    cargo test --workspace --locked
    cargo clippy --workspace --all-targets -- -D warnings
    cargo fmt --all -- --check

Combined project check:

    pnpm check

## Architectural invariants

1. Do not introduce a second reactive graph in Rust.
2. sn-core must not depend on QuickJS or OS APIs.
3. QuickJS-specific types stay inside the QuickJS runtime crate.
4. Rust is the authoritative owner of the committed retained UI tree.
5. JS host-node objects are only the minimal Solid renderer adapter mirror.
6. Native UI thread never calls into JS synchronously.
7. Production host operations use a typed/versioned batch, not JSON.
8. Batch application is atomic and tree revisions are explicit.
9. Platform-specific implementations depend inward on platform-neutral contracts.
10. Native text input, focus, IME and accessibility are first-class requirements.
11. Do not claim platform support without target-OS integration tests.
12. Performance claims require reproducible benchmarks.

## License and source references

Project license and third-party dependency notices must be finalized before distributing binaries. NSH is an architectural reference, not an assumed source-code dependency. See [upstream references and provenance notes](docs/reference/upstream-references.md).

## TODO

- [ ] Implement the typed, versioned JavaScript-to-Rust host protocol and stable node handles.
- [ ] Make Rust `UiTree` authoritative by applying renderer batches atomically.
- [ ] Add integration coverage for renderer mount, signal-driven updates, node moves/removal and event dispatch.
- [ ] Implement deterministic QuickJS job scheduling, Promise jobs, timers and runtime cleanup.
- [ ] Add Taffy-backed layout and a deterministic headless mount pipeline.

See the [implementation roadmap](docs/roadmap.md) for the complete milestones and acceptance criteria.
