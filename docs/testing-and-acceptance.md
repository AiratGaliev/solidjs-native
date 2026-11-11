# Testing, CI and Functional Acceptance Policy

- Status: mandatory verification policy.
- Fast CI: every pull request to `dev`, `test` or `main`.
- Integrated runtime acceptance: promotion pull request `dev → test`.
- Current acceptance limitation: M0 QuickJS smoke proof; not native-window GUI acceptance or cross-platform support.

## 1. Verification principles

Tests are authored with the subsystem or behavior they protect. Separate writing/maintaining test coverage from the frequency and cost of running the complete integrated suite.

- Test the changed contract, including relevant invalid input, edge cases, errors and lifecycle cleanup.
- Prefer deterministic tests close to the owning subsystem; add higher-level tests for interactions between subsystems.
- Keep ordinary PR feedback fast. Do not move the full integration/E2E suite into every topic PR.
- A green build proves that a build completed; it does not prove platform behavior, accessibility, security or performance.
- Record the exact commit SHA and environment for important acceptance evidence.

## 2. Test layers

| Layer | Purpose | Expected coverage |
|---|---|---|
| Unit | Validate isolated algorithms and invariants. | UiTree operations, IDs/revisions, parsers, encoders/decoders, reducers and resource state machines. |
| Component/contract | Validate a public or subsystem boundary. | Solid Universal adapter, typed host batches, JS/Rust boundary, callback registry, protocol validation and error contracts. |
| Integration | Validate collaborating runtime subsystems. | QuickJS execution, scheduler/job queue, event dispatch, transaction application/rollback, layout/style integration and resource cleanup. |
| Headless acceptance | Validate deterministic output without a desktop session. | UiTree snapshots, mount batches, layout results, style compatibility, semantic/accessibility trees and lifecycle transitions. |
| Platform integration | Validate a real OS backend. | Window lifecycle, rendering, input, text editing/IME, focus, accessibility, DPI/scale, resize and disposal on the target OS/device. |
| Consumer acceptance | Validate actual public SDK use from a clean checkout. | Install, build, run and package an independent sample app using only documented public contracts. |
| Security/adversarial | Validate hostile or malformed inputs and permission boundaries. | Invalid batches/IDs/lengths/versions, stale handles, unauthorized capability requests, path/permission boundaries and resource exhaustion where implemented. |
| Performance | Measure representative workloads and regressions. | Startup, first frame, input latency, tree/batch cost, layout, memory, idle CPU and artifact size. See [Performance Baseline](performance-baseline.md). |

Not every issue requires every layer. The test plan must explain which layers are affected and why the omitted layers are not relevant to that change.

## 3. Current project commands and CI gates

Install exact locked dependencies:

```bash
pnpm install --frozen-lockfile
```

Run the current fast engineering gate:

```bash
pnpm check
```

The actual `pnpm check` script runs:
- `pnpm typecheck` — TypeScript compiler check;
- `pnpm build` — Vite production build;
- `pnpm test:host-protocol` — Node.js built-in test runner for the TypeScript binary encoder and golden wire fixture;
- `cargo fmt --all -- --check` — Rust formatting;
- `cargo clippy --workspace --all-targets --locked -- -D warnings` — Clippy with warnings treated as errors;
- `cargo test --workspace --locked` — Rust workspace tests.

The current runtime smoke command is:

```bash
pnpm smoke
```

It builds the TSX demo and executes it through the reusable `EngineSession` library on its dedicated QuickJS owner thread. The smoke verifies the initial typed host mount, one Solid signal update through a single `ReplaceText` operation, root cleanup, and repeated mount/unmount callback reclamation. It is a narrow runtime integration proof, not a native window, platform input, accessibility or full GUI E2E suite.

The runtime scheduler's focused Rust tests cover:
- dedicated-thread session ownership and string-only evaluation results;
- Promise/queueMicrotask ordering;
- timeout cancellation and deterministic due-timer delivery with `ManualClock`;
- RAF request ordering, same-frame cancellation, and deferral of callbacks requested during a frame;
- one-shot external completions with an owned typed-array payload;
- Promise-job budget yielding and unhandled rejection diagnostics with bounded diagnostic text;
- bounded timer deadline storage after repeated set/clear cycles;
- command payload byte-budget enforcement and cancellation state cleanup after completion;
- shutdown interruption of a running JS turn followed by LIFO root cleanup;
- callback-root cleanup during session teardown.


## 4. CI responsibilities and execution boundary

The workflows are:
- `.github/workflows/ci.yml` — CI Fast; validates PR routing, installs frozen dependencies and runs `pnpm check` on PRs targeting `dev`, `test` or `main`.
- `.github/workflows/e2e.yml` — Promotion E2E / Runtime Integration Acceptance; executes `pnpm smoke` only for a PR from `dev` into `test` (or manual dispatch from `test`).

Rules:
1. Do not add the full integrated E2E suite to Fast CI or ordinary topic PRs.
2. Add/update focused tests and fixtures in the same topic PR as the feature.
3. Run focused local integration tests while developing when they materially reduce risk.
4. A promotion PR is an integrated candidate. Run all acceptance workflows required for the current maturity level against that candidate.
5. If promotion fails, fix the cause through the normal topic-branch workflow, merge to `dev`, then create a fresh promotion candidate.
6. Do not bypass a failed required check, rename a required check or weaken tests to force a pass.

Current active GitHub Rulesets:
| Target branch | Required checks |
|---|---|
| `dev` | `validate-branch-routing`, `fast-checks` |
| `test` | `validate-branch-routing`, `fast-checks`, `runtime-integration-acceptance` |
| `main` | `validate-branch-routing`, `fast-checks` |

The active rulesets require pull requests, block branch deletion and non-fast-forward updates, and currently require zero reviewer approvals. Repository-level enforcement is maintained in GitHub Rulesets, not by workflow YAML alone.

## 5. Platform support and acceptance

A platform capability is accepted only against the maturity/support criteria in [Platform Backends](architecture/platform-backends.md). For the target OS/device, record the applicable scenarios, such as:
- application/window and Surface startup, activation, resize, close and shutdown;
- rendering and resource recreation;
- pointer, keyboard, scroll and focus behavior;
- native text editing, selection, clipboard and IME;
- accessibility tree, actions and screen-reader interaction;
- cancellation, failure recovery, resource release and repeated create/destroy cycles.

Cross-compiling a crate or passing a platform-independent unit test does not establish platform support. Do not advertise a backend as available when only a prototype or compile target exists. Test on physical devices when the OS behavior cannot be represented by a simulator.

The current M0 implementation is a Windows x64/MSVC runtime proof. It does not prove the existence of a native UI backend, a native window, or support for Windows as a production platform.

## 6. Acceptance evidence

For any acceptance result that supports a milestone, platform or release claim, record:
- repository and exact commit SHA;
- target OS, version, architecture, device or simulator;
- toolchain and relevant runtime/dependency versions;
- fixture/sample application and setup steps;
- exact commands or manual reproduction steps;
- expected and actual result;
- pass/fail and links to logs, CI runs or artifacts;
- tests not run, known limitations and residual issues.

Keep evidence types distinct: unit tests, integration checks, platform manual tests, visual inspection and benchmarks are not interchangeable. A screenshot alone cannot prove lifecycle, input, accessibility or correctness.

## 7. Failure and flaky-test handling

For a deterministic failure, preserve the failing command, environment and first relevant log. Find the cause before changing production code or the test. Add a regression test for the violated invariant where feasible.

For a flaky test, capture the run URL/SHA, frequency, environment and relevant timing/order data. Do not hide flakiness by adding unbounded retries or weakening assertions. A retry is diagnostic evidence, not proof of reliability.

If the test infrastructure is unavailable, report the precise blocker and do not claim the omitted check passed. Create or update a Linear issue when the missing infrastructure prevents acceptance of a required capability.

## 8. Definition of acceptance

A test suite is accepted when it reliably exercises the stated contract and failure paths, cleans up resources, runs in its declared environment and provides actionable failure output.

A platform/milestone acceptance is complete only when the required real-target evidence is recorded, relevant limitations are explicit and the support/roadmap documents reflect the verified state. The existence of tests or a CI workflow alone is not acceptance.
