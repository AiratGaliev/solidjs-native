# ADR-0002: SolidJS Reactivity and Rust-Owned UI Tree

- Status: Accepted for target architecture.
- Date: 2026-11-10.

## Context

The project uses the real SolidJS Universal renderer inside QuickJS-NG. The Rust prototype already has a UiTree, while the current renderer also maintains a JavaScript host-node topology. If both structures become authoritative, they can diverge on moves, removals, property updates and failure recovery.

NSH uses a native Rust reactive graph following the Solid 1.9 algorithm. SolidJS Native intentionally chooses the actual upstream SolidJS runtime instead.

## Decision

1. SolidJS is the only owner of JS reactive semantics: signals, memos, effects, owners, context and cleanup.
2. Rust does not implement a parallel reactive graph.
3. Rust sn-core owns the authoritative retained UI tree after every successful commit.
4. JavaScript NativeNode objects are lightweight renderer adapters. They preserve only the topology and handles required by Solid Universal traversal.
5. Layout, normalized properties, accessibility state, native resources and committed revisions are authoritative in Rust/platform layers.
6. QuickJS-specific references stay in sn-runtime-quickjs and are not stored in sn-core.
7. Every cross-boundary mutation is validated and committed through a versioned batch.

## Consequences

Positive:
- Upstream SolidJS behavior and ecosystem are retained.
- Rust core remains VM-independent.
- Layout/platform code is not required to understand JavaScript closures.
- A future VM integration is possible behind a narrow runtime interface.

Costs:
- Native apps require a JS runtime.
- JS-to-Rust synchronization must be implemented carefully.
- The adapter needs a minimal local structure for Solid traversal.
- Runtime scheduling and garbage-collection lifecycle are critical subsystems.
- Performance cannot be assumed to exceed a Rust-native reactive implementation.

## Rejected alternatives

- Reimplement Solid's reactive graph in Rust: rejected because it duplicates semantics and creates two runtimes to maintain.
- Use a browser DOM/WebView as the native host: rejected because it undermines native UI integration.
- Maintain full authoritative trees in both JS and Rust: rejected because it creates synchronization ambiguity.

## Invariants

- No Rust signal/memo/effect graph is introduced.
- JS mirror is not the source of truth after commit.
- No platform UI thread calls into QuickJS synchronously.
- Failed host transactions cannot partially mutate the authoritative tree.
