# Code Quality Baseline

- Status: mandatory engineering conventions.
- Enforced fast gate: `pnpm check`.
- Scope: TypeScript/TSX packages, Rust workspace, runtime boundaries, examples and tests.

This document describes currently enforceable checks and the quality expectations for new code. It does not claim that tools or policies absent from the repository are already enabled.

## 1. Current automated baseline

The root `package.json` defines the following scripts:

| Script | Responsibility |
|---|---|
| `pnpm typecheck` | TypeScript compiler check (`tsc --noEmit`). |
| `pnpm build` | Vite production build using `packages/solid-native/vite.config.ts`. |
| `pnpm format:rust:check` | `cargo fmt --all -- --check`. |
| `pnpm lint:rust` | Clippy for all workspace targets with warnings denied. |
| `pnpm test:rust` | Locked Rust workspace tests. |
| `pnpm check` | The complete fast gate: typecheck, build, Rust formatting, Clippy and tests. |

JavaScript/TypeScript ESLint, Prettier, Rust coverage thresholds and additional static-analysis jobs are not currently declared in the root scripts. Do not report them as enabled checks. Any future tool addition must have a clear ownership, configuration, CI integration and migration plan.

## 2. TypeScript and SolidJS

- Use strict, explicit TypeScript types at public and cross-package boundaries.
- Prefer discriminated unions, typed result/error contracts and stable identifier types over `any` or generic untyped property bags.
- Keep the public application API aligned with upstream SolidJS and the documented Universal renderer boundary.
- SolidJS owns reactive state and scheduling. Do not create shadow signals, a second scheduler or a competing reactive graph in Rust.
- Keep renderer adapter objects minimal. Do not treat JS host-node mirrors as the authoritative native UI tree.
- Keep native runtime code independent of browser DOM globals. The Web target is a separate browser-native backend.
- Handle async errors, cancellation and resource disposal explicitly. Do not leave callbacks, subscriptions, timers or handles alive after owner teardown.
- Avoid hidden global state and implicit platform behavior. Make platform-specific capability differences observable and documented.

## 3. Rust workspace

- Keep `sn-core` platform-neutral and independent of QuickJS and OS APIs.
- Keep QuickJS-specific types and operations inside the runtime integration boundary.
- Use ownership and borrowing to make resource lifetime explicit; avoid unnecessary shared mutable state.
- Use `Result` for recoverable failures. Reserve panic for violated internal invariants that cannot be represented as normal input errors; never allow panic to cross an FFI boundary.
- Validate all externally supplied IDs, lengths, revisions, enum tags, buffer sizes and state transitions before mutation.
- Make transaction boundaries atomic where the host protocol requires atomicity. A failed batch must not leave a partially mutated authoritative tree.
- Use bounded queues and explicit overload/back-pressure behavior. Every callback, handle, subscription and native resource requires a defined release path.
- Document thread affinity and `Send`/`Sync` assumptions where crossing execution contexts.
- Prefer small typed structures and enums over `HashMap<String, serde_json::Value>` for core contracts.
- Keep unsafe blocks minimal, justified with a local safety comment and covered by tests for the relevant invariant.

## 4. FFI and runtime boundaries

- Do not retain borrowed JS heap references beyond their documented lifetime.
- Validate pointers, lengths, handles, generations and version fields before dereference or dispatch.
- Do not call arbitrary user JS synchronously from the native platform UI thread.
- Keep diagnostics/serialization formats separate from the production typed host transport.
- Make callback registration, invocation, cancellation and deregistration deterministic.
- Return stable, actionable errors for malformed input, unsupported capabilities, stale handles and shutdown races.
- Do not expose privileged operating-system operations through renderer mutation APIs.

## 5. Maintainability and scope

- Keep changes focused on one coherent Linear issue.
- Avoid premature crate extraction, speculative feature flags and abstractions with no current consumer.
- Do not duplicate canonical contracts across source code, ADRs, examples and test fixtures; define the contract once and reference it.
- Do not silently change public API behavior, protocol layouts or platform semantics. Use [Architecture Governance](architecture-governance.md).
- Do not update dependency versions as part of unrelated feature work. Follow [Dependency Policy](dependency-policy.md).
- Do not introduce generated build outputs, local IDE state, secrets or machine-specific files into commits.

## 6. Review checklist

Reviewers should check:
- correctness, state transitions and null/empty/invalid-input behavior;
- error propagation and observability;
- ownership, callback/resource cleanup and cancellation;
- thread-affinity, deadlock and race risks;
- protocol/API compatibility and boundary validation;
- platform-specific behavior and unsupported-target diagnostics;
- test coverage of the changed contract and failure paths;
- dependency, license, security and performance impact;
- documentation, examples and roadmap accuracy.

Style-only findings must not distract from correctness or contract defects. Do not demand broad rewrites when a minimal safe change satisfies the project conventions.
