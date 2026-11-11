# Architecture Governance and ADR Policy

- Status: mandatory design-governance policy.
- Product scope: standalone cross-platform framework and SDK; see [Application Framework Scope](architecture/application-framework.md).
- Architecture decisions are recorded in `docs/decisions/` and linked to the corresponding Linear milestone/issue.

This policy prevents implementation details, external references or temporary prototypes from silently becoming framework requirements.

## 1. Canonical sources and authority

| Source | Authority |
|---|---|
| `README.md` | Current project summary, implemented-state caveats, developer entry point and links to mandatory policy. |
| `docs/README.md` | Documentation navigation, canonical document list and status vocabulary. |
| `docs/roadmap.md` and Linear milestones | Planned outcomes, work packages, milestone order and acceptance criteria. Linear remains the live task/backlog source. |
| Accepted ADRs in `docs/decisions/` | Binding decisions about architecture, ownership, runtime, protocol, threading, layout, security and platform boundaries. |
| `docs/architecture/` | Detailed target contracts and subsystem specifications consistent with accepted ADRs. |
| `docs/development-workflow.md` and `docs/engineering-workflow.md` | Repository delivery, branch/CI and task-state rules. |
| `docs/dependency-policy.md` | Dependency versioning, compatibility, provenance and update policy. |
| Tests and current code | Evidence of what is implemented and currently verified; not automatic approval to change the target architecture. |

When documents disagree, do not silently choose the most convenient description. Identify the conflict, inspect current implementation and tests, and update the authoritative source(s) through a scoped issue and PR. A code path or prototype does not supersede an accepted ADR by existing.

## 2. When an ADR is required

Create or update an ADR before treating a decision as binding when a change materially affects:

- reactive state ownership, SolidJS integration or renderer lifecycle;
- Rust retained-tree ownership, node identity or mutation semantics;
- JavaScript runtime, scheduler, Promise/job semantics or runtime isolation;
- JS/Rust host protocol, wire format, batching, transaction atomicity or callback/event contracts;
- FFI, public Rust/TypeScript API, C ABI, embedding contract or compatibility/versioning;
- thread ownership, queues, synchronization, cancellation or shutdown ordering;
- platform renderer architecture, native control semantics, accessibility or input;
- layout, CSS parsing/resolution, Tailwind compilation or style invalidation;
- application host, Application Bridge, native services, plugins or capability permissions;
- security/trust model, dynamic modules, code signing or package provenance;
- a foundational dependency that changes MSRV, license exposure, ABI, binary/runtime model or platform support;
- a public support promise or platform maturity claim.

Routine implementation within an accepted contract does not require a new ADR. Do not create ADRs for purely local naming or internal refactoring with no lasting contract impact.

## 3. ADR lifecycle and required format

Use sequential filenames such as `0010-short-decision-name.md`. Never reuse a number or silently rewrite history to conceal a changed decision.

Each ADR should contain:
1. **Title and status** — Proposed, Accepted, Superseded or Rejected.
2. **Date and context** — the problem, constraints and relevant milestone.
3. **Decision drivers** — correctness, ownership, platform behavior, security, performance and maintainability factors.
4. **Decision** — the chosen approach and explicit boundaries.
5. **Alternatives considered** — credible alternatives and why they were not selected.
6. **Consequences** — benefits, costs, risks and operational implications.
7. **Compatibility/migration** — impact on existing APIs, data, protocol, ABI, consumers and rollout.
8. **Verification** — prototypes, tests, benchmarks or external evidence required before/after adoption.
9. **References** — official API docs, source code, issues and related ADRs.

A proposal is not accepted merely because it appears in a design document or Linear issue. Mark an ADR Accepted only after the decision owner has approved it through the project workflow. If an accepted decision changes, add a new ADR that supersedes the prior one and explain the transition.

## 4. Prototype and evidence policy

Use prototypes to reduce material uncertainty, not to bypass architecture review. A prototype must state:
- the hypothesis it tests;
- the minimal alternatives and representative scenario;
- target OS/toolchain/runtime versions;
- correctness, lifecycle and failure cases;
- measured resource/performance characteristics when relevant;
- limitations and the decision the evidence can support.

Keep prototype-only dependencies and code out of the production dependency graph unless separately accepted. A benchmark from a synthetic micro-workload does not establish application-level performance. Cross-compilation does not establish platform support.

## 5. Dependency and upstream-reference governance

Follow [Dependency Policy](dependency-policy.md) for version selection, compatibility, lockfiles, security and licensing. Architecture references do not determine the versions or MSRV of this project.

Reference roles are deliberately bounded:
- **NSH** — primary native implementation and platform-rendering reference.
- **GPUI Kit / GPUI Shell** — comparative desktop UI and QuickJS-host reference.
- **Servo Stylo** — CSS implementation reference.
- **Tauri 2** — external application-framework architecture and developer-workflow reference only.

A reference does not become an automatic dependency, platform requirement or feature checklist. Record copied or adapted source/assets with their licenses and provenance. Do not infer a dependency decision from an upstream project's package graph.

## 6. Public contract and compatibility changes

Treat public TypeScript/Rust APIs, host protocol, application bridge, plugin contracts and ABI as explicit interfaces. Before changing one:
- search all in-repository callers, examples, tests and public documentation;
- identify compatibility and migration impact for independent consumers;
- define validation and error behavior;
- update version/compatibility declarations where such policy exists;
- add contract tests and an example demonstrating the new behavior;
- record platform-specific support and known limitations.

Stable API/ABI policy is part of the roadmap's runtime-hardening work. Until a contract is explicitly declared stable, describe it as experimental or target; do not imply semver or cross-version compatibility that has not been adopted and tested.

## 7. Documentation synchronization

When an accepted decision changes, review all affected sources in the same PR:
- the relevant ADR;
- architecture and subsystem contract docs;
- roadmap/milestone acceptance;
- README and documentation index where the current project state or navigation changes;
- examples and test fixtures;
- dependency/support/compatibility matrices;
- Linear issue scope and acceptance criteria.

Keep canonical requirements in one place and link to them elsewhere. Do not copy entire architectural sections across documents where they can drift independently.

## 8. Review checklist

Before accepting a material design change, reviewers should be able to answer:
- Is the proposal consistent with the standalone framework scope?
- Does it preserve single-owner state, tree and thread boundaries?
- Are data, resource and callback lifecycles explicit?
- Are failure, cancellation and shutdown paths specified?
- Are security and platform capabilities enforced at the correct boundary?
- Are alternatives and dependency trade-offs documented?
- Is the required evidence executable and reproducible?
- Are target behavior and implemented behavior clearly distinguished?
