# Development Workflow and GitHub CI Policy

- Status: mandatory engineering workflow.
- Branch model: `main` / `dev` / `test`.
- CI baseline: Node.js 24, pnpm 12.18.0, Rust 1.99.1 (from repository manifests).

## 1. Branch roles

| Branch | Purpose | Incoming pull requests |
|---|---|---|
| `main` | Release-ready history; releases only | `test` or `hotfix/*` |
| `dev` | Active engineering integration branch | Topic branches |
| `test` | Promotion candidate for integrated acceptance | `dev` only |

Topic branch prefixes:
- `feat/*` or `feature/*` — new behavior;
- `fix/*` — defect correction;
- `refactor/*` — internal restructuring;
- `test/*` — tests and test infrastructure;
- `docs/*` — documentation;
- `chore/*` — tooling, CI and maintenance;
- `dependabot/*` — automated dependency update PRs targeting `dev`.

Urgent production corrections use `hotfix/*` and target `main`. After release, port or cherry-pick the fix into a fresh topic branch based on `dev` and merge it through a normal PR so the branches do not diverge.

Do not commit routine engineering changes directly to `main`, `dev` or `test`. Work in a focused topic branch and use a pull request.

## 2. Standard delivery flow

1. Start from the latest `dev`; create a topic branch associated with a Linear issue.
2. Inspect relevant architecture documents, ADRs, source code, tests and repository status before editing.
3. Implement only the agreed issue scope. Add tests alongside the subsystem change.
4. Run the fast local checks and the relevant focused integration tests.
5. Open a PR from the topic branch into `dev`. Link the Linear issue and attach exact verification evidence.
6. Review the complete diff, CI checks, compatibility, lifecycle, threading, memory safety and security impact.
7. Merge only after required checks and review are satisfied.
8. Promote a coherent, integrated candidate using a dedicated `dev` → `test` PR.
9. Run promotion acceptance and the available end-to-end suite on that PR. Fix failures through normal topic branches into `dev`, then promote again.
10. Release only from validated `main`, normally through `test` → `main`.

Expected route:
`feat/* | feature/* | fix/* | refactor/* | test/* | docs/* | chore/* | dependabot/* → dev → test → main`.

## 3. CI responsibilities

### Fast CI — every PR to dev, test or main

Workflow: `.github/workflows/ci.yml`.

- Validate the branch routing above.
- Install JavaScript dependencies from `pnpm-lock.yaml` using `pnpm install --frozen-lockfile`.
- Use Node.js 24 in CI, pnpm 12.18.0 from the `packageManager` field, and the Rust 1.99.1 toolchain pinned in `rust-toolchain.toml`.
- Run `pnpm check`: TypeScript typecheck, Vite production build, JavaScript host-protocol encoder tests, Rust formatting, Clippy and workspace tests.
- Use read-only GitHub token permissions and cancel superseded PR runs.

Fast CI must remain suitable for the ordinary feature feedback loop. Do not put the full GUI/platform E2E suite into this workflow.

### Promotion E2E / integration acceptance — dev → test only

Workflow: `.github/workflows/e2e.yml`.

- Runs only for a PR whose base is `test` and head is `dev`; manual dispatch is accepted only from `test`.
- Executes `pnpm smoke`, which builds the TSX bundle and executes it in the embedded QuickJS runtime.
- This is the current M0 runtime integration acceptance. It is **not yet a native-window GUI E2E suite**: the native platform UI is not implemented. As new native/platform scenarios become executable, extend this promotion workflow with the full suite rather than moving it into feature PRs.
- Tests must be authored with the corresponding feature; expensive full E2E runs remain isolated to promotion.

### Release workflow

No automated package/release publication is configured yet. This repository has not reached its CLI, packaging or public SDK release milestones. Introduce release automation with the corresponding M9/M15 deliverables and explicit artifact/signing requirements; do not publish arbitrary source archives as if they were usable framework binaries.

## 4. Local checks

Install dependencies:

```bash
pnpm install --frozen-lockfile
```

Fast engineering gate:

```bash
pnpm check
```

Runtime integration check:

```bash
pnpm smoke
```

The `pnpm check` command is the single fast gate and includes:
- `pnpm typecheck`;
- `pnpm build`;
- `cargo fmt --all -- --check`;
- `cargo clippy --workspace --all-targets --locked -- -D warnings`;
- `cargo test --workspace --locked`.

Run focused package/test commands while iterating. Before PR, run the complete relevant fast gate and record any omitted checks.

## 5. Linear project and issue hygiene

[Linear](https://linear.app/solidjs-native) is the source of truth for SolidJS Native project tasks and planning. Use it to find the current backlog, priorities, milestones, dependencies, issue status and acceptance criteria before starting work. Do not treat GitHub Issues as the project task backlog; GitHub is used for repository history, pull requests, code review and CI.

- Every implementation, documentation or maintenance PR for project work must link the corresponding Linear issue from the SolidJS Native workspace.
- Work against the current issue scope and acceptance criteria; do not rely on an outdated chat summary or an unlinked task description.
- Use milestone parent issues and child issues only for independently verifiable work packages.
- Update issue state from observed work only: keep active implementation in `In Progress`; move to `In Review` when a non-draft PR is ready for review and focused verification is complete; return to `In Progress` when review requests changes; move to `Done` only after merge and the Definition of Done. Full state semantics are defined in [Engineering Workflow](engineering-workflow.md).
- Record follow-up defects as separate Linear issues; do not silently expand scope.
- Link PRs, commits and test evidence to the Linear issue.

See [Engineering Workflow](engineering-workflow.md) for the complete Linear working agreement.

## 6. Definition of Ready

An issue is ready when its milestone, scope, constraints, acceptance criteria, tests and dependencies are clear; relevant ADRs have been reviewed; and required target environments are available or limitations are explicitly recorded.

If implementation requires a material change to architecture, ownership, threading, protocol, security, dependency/MSRV, license or public API, prepare/update an ADR before treating the choice as settled.

## 7. Definition of Done

An issue is Done only when:
1. All agreed behavior and acceptance criteria are implemented.
2. Focused regression tests exist and pass.
3. Required fast CI checks pass.
4. Relevant integration/platform checks have run, or their absence is explicitly recorded and the issue is not represented as platform-accepted.
5. No unresolved critical correctness, memory/lifecycle, security or threading defect remains.
6. Public API, architecture docs, ADRs, examples and compatibility matrix are updated where required.
7. The PR has been reviewed and merged into the intended branch when merge is authorized.
8. Linear reflects the actual repository/PR state and includes verification evidence.

A compiling crate, green fast CI, merged PR or screenshot alone does not complete a milestone.

## 8. Architecture-specific engineering invariants

- Upstream SolidJS is the single reactive owner; do not introduce a second Rust reactive graph.
- Rust `sn-core::UiTree` owns committed UI state; the JS host mirror is not authoritative.
- Each native Surface has one QuickJS-owning EngineSession thread.
- Platform UI threads never execute user JS synchronously or wait for JS/layout/resource I/O.
- Production renderer operations use a bounded, typed, versioned and atomic batch; JSON is diagnostics-only.
- Host Protocol and Application Bridge are separate contracts.
- `sn-core` remains independent of QuickJS and OS APIs.
- Validate FFI input, IDs, lengths, versions and capability requests; never retain borrowed JS heap memory or panic across FFI.
- Every queue/resource has explicit capacity, ownership, cancellation, backpressure and disposal semantics.
- Do not claim a target platform is supported based on cross-compilation alone.
- Performance claims require reproducible representative benchmarks.

## 9. GitHub repository protection

Repository protection is enforced by the active GitHub Rulesets `protect-dev`, `protect-test` and `protect-main`. The rulesets require pull requests, block branch deletion and non-fast-forward updates, and require the following status checks:

| Protected branch | Required status checks |
|---|---|
| `dev` | `validate-branch-routing`, `fast-checks` |
| `test` | `validate-branch-routing`, `fast-checks`, `runtime-integration-acceptance` |
| `main` | `validate-branch-routing`, `fast-checks` |

The rulesets currently require zero reviewer approvals and allow merge, squash and rebase methods. No actor is configured to bypass the rules. Treat GitHub Rulesets as the enforcement layer and this document as the contributor-facing explanation; verify the live rulesets before changing this section.

Workflow YAML also uses least-privilege `contents: read` permissions. Changes to Actions permissions, third-party actions, secrets, Dependabot, secret scanning or push protection require a separate security review. Never infer repository protection from workflow YAML alone.
