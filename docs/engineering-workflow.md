# Engineering Workflow and Linear Working Agreement

- Status: mandatory project delivery policy.
- Scope: the SolidJS Native repository, its SDK packages, Rust crates, examples, tooling and project documentation.
- Task source of truth: [SolidJS Native in Linear](https://linear.app/solidjs-native).
- Linear team: **Solid-native (SOL)**.
- Git repository: [AiratGaliev/solidjs-native](https://github.com/AiratGaliev/solidjs-native).

This document governs issue planning, ownership, task state and engineering completion. Branch routing and CI implementation are specified in [Development Workflow](development-workflow.md). Technical decision authority is specified in [Architecture Governance](architecture-governance.md).

## 1. Project and ownership model

SolidJS Native is one standalone framework project with one Linear delivery team, **SOL**. The SOL team owns framework implementation, architecture, tests, platform integration, security, performance, documentation, tooling and release engineering.

Do not create a second QA team or copy the Codogma Engine COD/CODQA split. Functional acceptance and release-candidate verification remain work within SOL. A separate acceptance issue is appropriate only when the verification is independently scoped, substantial and has its own measurable result; routine testing belongs to the implementation issue.

Use the [SolidJS Native Linear project](https://linear.app/solidjs-native) for all project work. Linear is authoritative for:
- the backlog and task ownership;
- milestone hierarchy, priorities and dependencies;
- issue scope, acceptance criteria and current state;
- implementation, research, test and acceptance evidence.

Use GitHub for source code, branch history, pull requests, review and CI. GitHub Issues are not a parallel project backlog. Every project-work PR must link the corresponding SOL issue.

## 2. Issue hierarchy and scope

Use Linear milestones M0–M16 as the project's milestone structure. A milestone issue captures an independently verifiable outcome; child issues describe work packages that can be implemented and accepted independently.

Create a child issue when it has its own:
- outcome and bounded scope;
- acceptance criteria;
- relevant tests or evidence;
- meaningful dependency or delivery boundary.

Do not create a child for every commit, small refactor, test file or PR. Do not split one coherent result across multiple issues merely to make the board look busy. Avoid duplicate backlogs in documents, GitHub Issues or chat.

An issue should use a clear outcome-oriented title and describe:
1. **Context/problem** — what is missing or incorrect and why it matters.
2. **Goal** — the observable result, not a list of implementation guesses.
3. **Scope** — included behavior and affected subsystem/contracts.
4. **Out of scope** — nearby capabilities intentionally excluded.
5. **Constraints** — accepted ADRs, platform/toolchain/API boundaries and compatibility requirements.
6. **Dependencies** — blocking issues, milestone parent and required upstream work.
7. **Acceptance criteria** — specific, verifiable expected behavior.
8. **Verification** — focused tests, CI checks, platform scenarios, benchmark or security evidence as applicable.
9. **Limitations/risks** — unsupported targets, missing environments and unresolved assumptions.

If implementation reveals a materially different problem, update the issue scope or create a follow-up issue before silently expanding the change.

## 3. Priorities and labels

Use the existing SOL labels according to their intent:
- `Feature` — a new user-facing or framework capability.
- `Improvement` — an improvement to an existing capability, workflow or quality attribute.
- `Bug` — behavior that violates an existing contract or expected result.

Do not use a label as a substitute for clear issue scope. Create new labels only when a repeated classification need is demonstrated and the team agrees on its meaning.

Priority guidance:
- **Urgent** — active blocker, critical correctness/security issue or release-blocking failure that requires immediate attention.
- **High** — blocks the active milestone or a critical dependency chain.
- **Medium** — normal planned work.
- **Low** — non-blocking cleanup, optional improvement or research without an immediate dependency.

Priority is not a substitute for milestone order or dependency relations. Do not raise priority simply because an issue is old.

## 4. Linear state semantics

The SOL team uses these states: `Backlog`, `Todo`, `In Progress`, `In Review`, `Done`, `Canceled` and `Duplicate`. `In Review` is a `Started`-category state placed after `In Progress` and before the completed `Done` stage.

| State | Meaning | Transition guidance |
|---|---|---|
| `Backlog` | Valid work that is not yet ready or scheduled for active delivery. | Move to `Todo` when scope, dependencies and acceptance criteria are ready. |
| `Todo` | Refined work that has not started. | Move to `In Progress` when implementation or the planned investigation actually begins. |
| `In Progress` | Active implementation, research, tests, documentation or PR preparation. | Move to `In Review` when implementation and focused verification are complete and a non-draft PR is ready for review. Return to `In Progress` if review findings require further implementation. |
| `In Review` | Implementation submitted for review; the PR is under review and/or required CI checks are pending. | Keep here while review and required checks are pending. Return to `In Progress` when changes are requested or implementation resumes. Move to `Done` only after merge and all applicable DoD conditions are met. |
| `Done` | The issue's own Definition of Done is satisfied and evidence is recorded. | Do not use merely because a PR exists, code compiles or a milestone parent is mostly complete. |
| `Canceled` | The work was intentionally stopped and will not be delivered in this scope. | Record the reason and any replacement decision. |
| `Duplicate` | The issue duplicates a canonical issue. | Link the canonical issue and preserve relevant context. |

Move an issue to `In Review` only when its non-draft PR is ready for review and focused verification is complete. An open draft PR or ongoing implementation remains `In Progress`. If review requires code or documentation changes, return the issue to `In Progress`; once the revision is ready for review again, move it back to `In Review`.

State changes must reflect observed work, not intended progress or board cleanup. Do not mass-close or reclassify issues without checking their individual acceptance criteria.

## 5. Definition of Ready

An issue is Ready (`Todo`) when:
- the goal and scope are understandable without relying on chat history;
- acceptance criteria describe an observable result;
- relevant milestone and dependencies are linked;
- applicable architecture documents and accepted ADRs are identified;
- the required target platform/toolchain is available, or the limitation is explicit;
- the intended test and evidence strategy is clear;
- no unresolved design choice blocks a safe implementation.

A major architecture choice must be resolved through an ADR before implementation treats it as settled. A small detail that does not affect an approved contract does not require an ADR.

## 6. Delivery workflow

1. Start from the latest `dev` and create a focused topic branch associated with the SOL issue.
2. Inspect the current source, complete diff, tests, relevant contracts, ADRs and CI state.
3. Implement only the issue's agreed scope. Add or update tests alongside the subsystem change.
4. Run focused checks during development and the full fast gate before requesting review.
5. Open one focused PR to the correct target branch, link the Linear issue and record verification evidence.
6. Review the complete diff for behavior, API compatibility, lifecycle, threading, memory safety, security and maintainability.
7. Resolve review comments and required CI failures. Do not weaken checks to get a green result.
8. Merge only after the required checks and review are satisfied and merge is explicitly authorized.
9. Update the Linear issue with the PR, commit/SHA, checks, limitations and actual resulting state.
10. Promote integrated work using the dedicated `dev → test` PR. Release only through the documented `test → main` route.

Allowed routes, branch names, required status checks and protection rules are maintained in [Development Workflow](development-workflow.md). Do not repeat or redefine them in an issue description.

## 7. Engineering completion and acceptance

Engineering work is complete when its own criteria have been implemented and verified, focused regression tests pass, the required fast CI checks pass, the PR has been reviewed and merged into the intended integration branch, and the issue contains the evidence.

Do not keep an Engineering issue open waiting for a future full promotion acceptance that is intentionally run against an integrated `test` candidate. Conversely, do not mark platform support accepted merely because a topic PR or cross-compilation succeeded.

For independently scoped functional/platform/release acceptance:
- create a separate top-level SOL issue or a clearly scoped work package under the relevant milestone;
- identify the exact integrated candidate SHA and target OS/device/environment;
- record reproducible steps, expected and actual results, logs/artifacts and limitations;
- keep acceptance evidence distinct from unit/contract test results;
- create a separate Bug issue for a discovered defect and link it to the original implementation/acceptance context.

The current CI boundary is intentional: ordinary topic PRs run Fast CI; integrated runtime acceptance is run by the promotion workflow on `dev → test`. See [Testing and Acceptance](testing-and-acceptance.md).

## 8. Definition of Done

An issue may move to `Done` only when all applicable conditions hold:

1. All agreed acceptance criteria are met; unimplemented criteria are not silently waived.
2. Relevant focused unit, component, contract or integration tests exist and pass.
3. Required fast CI checks pass on the final PR commit.
4. The PR has completed review and is merged into its intended branch, when the issue changes repository content.
5. Required platform, security, performance or acceptance evidence is recorded; missing target environments are explicitly identified and do not support a stronger maturity claim.
6. Public API, architecture docs, ADRs, examples, dependency/support matrices and roadmap entries are updated where affected.
7. No unresolved critical correctness, lifecycle, memory-safety, security or threading blocker remains.
8. Linear contains links to the PR/commit, test and CI evidence, and known limitations.

For research/design-only issues, Done means the requested decision or report is recorded in the canonical project documents, alternatives and consequences are described, and follow-up implementation issues are created where needed. A proposal is not an accepted implementation.

## 9. Reporting and change discipline

At completion, report:
- the Linear issue and actual final state;
- branch, PR and changed areas;
- commands and CI checks actually run, with results;
- checks not run and the reason;
- platform/security/performance evidence, or the absence of it;
- known risks, residual work and linked follow-up issues.

Do not state that a task, platform or milestone is complete without evidence. Do not merge, release, delete branches or mark an issue Done solely to make the board look finished.
