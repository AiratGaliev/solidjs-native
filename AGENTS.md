# Repository contributor instructions

These rules apply to human contributors and automated coding agents working in this repository. They supplement the product and architecture documents; they do not authorize work outside the linked Linear issue.

## Before changing anything

1. Confirm the repository is `AiratGaliev/solidjs-native`, inspect `git status`, the current branch, its base and the complete existing diff. Preserve unrelated work.
2. Find the current task in the [SolidJS Native Linear project](https://linear.app/solidjs-native). Read its full description, parent milestone, dependencies, acceptance criteria and current state. Do not use an old chat summary as the current task specification.
3. Read [README.md](README.md), [Engineering Workflow](docs/engineering-workflow.md), [Development Workflow](docs/development-workflow.md), and the relevant architecture documents and accepted ADRs before implementation.
4. Start from the latest `dev` in a focused topic branch. Follow the allowed PR routes in `docs/development-workflow.md`; do not commit directly to permanent branches.

## Product and architecture boundaries

- SolidJS Native is a standalone, reusable cross-platform application framework and SDK. Do not introduce requirements or dependencies on Codogma Engine, a game engine, editor, or any single consumer application.
- Tauri 2 is an external architecture and workflow reference only. Do not turn it into a replacement, compatibility, migration or feature-parity target.
- Follow the documented roles of NSH, GPUI Kit/GPUI Shell and Servo Stylo. A reference is not automatically a source-code dependency or an architectural decision.
- Upstream SolidJS owns reactive application state. Do not create a second reactive graph in Rust.
- Keep `sn-core` independent of QuickJS and operating-system APIs. Keep runtime-specific and platform-specific implementation details behind their documented boundaries.
- The Rust retained UI tree and typed, versioned host protocol are the authoritative native contracts. Do not bypass them with untyped production JSON transport.
- Native platform UI threads must not synchronously execute user JavaScript or wait for runtime, layout or resource I/O.
- Make thread ownership, resource lifetime, cancellation, queue capacity, back-pressure and cleanup explicit.

## Implementation rules

- Make the smallest coherent change that satisfies the issue. Do not create empty future crates, speculative abstractions or unrelated refactors.
- Validate all data crossing JS/Rust, FFI, platform and capability boundaries. Never allow a panic to unwind across FFI.
- Prefer typed contracts and stable identifiers over string-keyed maps of untyped values.
- Use explicit error paths; do not silently discard unsupported platform behavior or failed operations.
- Add or update focused tests with the behavior. A test must exercise the changed contract, including relevant failure and lifecycle paths.
- Follow [Dependency Policy](docs/dependency-policy.md) before adding or updating dependencies. Keep manifests, lockfiles and compatibility evidence consistent.
- Update API docs, ADRs, examples, roadmap and support claims in the same PR whenever the change affects them.

## Verification and reporting

- Run `pnpm check` before requesting review. It is the current fast gate: TypeScript typecheck, Vite build, Rust formatting, Clippy and workspace tests.
- Run focused checks while iterating. Run `pnpm smoke` when changing the embedded QuickJS/TSX runtime integration; in CI the promotion workflow runs it only on `dev → test`.
- Do not describe `pnpm smoke` as native-window or GUI acceptance. The current M0 proof is not native UI or cross-platform support.
- Do not claim platform support from cross-compilation alone. Use real target-OS integration evidence as defined in `docs/testing-and-acceptance.md`.
- Do not invent benchmark results, performance budgets, completed platform support or security guarantees.
- Record exact commands, target environment, commit SHA, outcomes, unrun checks and known limitations in the Linear issue and PR.
- Do not weaken tests or bypass failing required checks to obtain a green result.
- Do not merge a PR, change a release branch or mark an issue Done merely because an implementation or PR exists. Merge only when explicitly authorized; mark Done only after its acceptance criteria and project DoD are actually satisfied.

## Canonical project guidance

- [Engineering Workflow](docs/engineering-workflow.md) — Linear tasks, states, readiness and completion.
- [Development Workflow](docs/development-workflow.md) — branches, PRs, CI and repository rules.
- [Architecture Governance](docs/architecture-governance.md) — design decisions and ADRs.
- [Testing and Acceptance](docs/testing-and-acceptance.md) — test layers, CI boundaries and evidence.
- [Code Quality Baseline](docs/code-quality-baseline.md) — implementation and review expectations.
- [Performance Baseline](docs/performance-baseline.md) — measurement protocol.
- [Security Baseline](docs/security-baseline.md) — trust boundaries and security requirements.
