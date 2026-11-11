# Documentation index

SolidJS Native is a standalone, cross-platform application framework and SDK powered by upstream SolidJS. Its product requirements and roadmap are independent of any single consumer application. Tauri 2 is an external architecture and workflow reference only; it is not a replacement, compatibility, migration or feature-parity target. NSH is the primary native implementation and platform-rendering reference; GPUI Kit/GPUI Shell and Servo Stylo have bounded comparative and implementation-reference roles.

## Start here

1. [Application framework scope](architecture/application-framework.md) — independent framework scope, Application Bridge, plugins, permissions, platforms and consumer validation.
2. [Target architecture](architecture/target-architecture.md) — system layers, ownership, threading, tree, protocols, layout, rendering, security and performance.
3. [Architecture reuse and dependency adoption map](architecture/dependency-adoption.md) — reference roles, selected components, prototype candidates and reuse boundaries.
4. [Styling, CSS and Tailwind CSS](architecture/styling-and-css.md) — shared styling model, native CSS compatibility profile, Tailwind build integration and Stylo adapter.
5. [SolidJS runtime and host protocol](architecture/solidjs-runtime-and-host-protocol.md) — Universal renderer, QuickJS integration, typed batched UI bridge, scheduler and callback lifecycle.
6. [Host Protocol v1](architecture/host-protocol-v1.md) — exact binary header, operation records, value tags, bounds and transaction semantics.
7. [Platform backends](architecture/platform-backends.md) — Windows/macOS/Linux/iOS/Android/Web, native rendering, application host, embedding and platform acceptance.
8. [GPUI Kit comparison](architecture/gpui-kit-comparison.md) — adjacent desktop framework, GPUI Shell runtime and reuse decisions.
9. [NSH implementation reference map](architecture/nsh-feature-parity.md) — concrete upstream native implementations, architectural differences and reuse rules.
10. [Implementation roadmap](roadmap.md) — milestones M0–M16, work packages and acceptance criteria.

## Mandatory engineering policy

- [Engineering workflow and Linear working agreement](engineering-workflow.md) — issue scope, hierarchy, priorities, current SOL states, readiness and completion.
- [Development workflow and GitHub CI policy](development-workflow.md) — branch routes, PR requirements, CI responsibilities and repository protection.
- [Architecture governance and ADR policy](architecture-governance.md) — canonical sources, material decisions, ADR lifecycle and reference boundaries.
- [Testing, CI and functional acceptance](testing-and-acceptance.md) — test layers, promotion-only runtime acceptance, platform evidence and limitations.
- [Code quality baseline](code-quality-baseline.md) — currently enforced checks and implementation/review conventions.
- [Performance baseline protocol](performance-baseline.md) — reproducible measurement and future budgets; no unverified numerical claims.
- [Security baseline](security-baseline.md) — trust boundaries, capability requirements and security evidence.
- [Dependency policy and current stable version baseline](dependency-policy.md) — dependency selection, compatibility, lockfiles, security and licensing.
- [Repository contributor instructions](../AGENTS.md) — mandatory first steps and human/agent contribution boundaries.

## Architecture decisions

- [ADR-0001: SolidJS Universal and QuickJS-NG](decisions/0001-solidjs-and-runtime.md)
- [ADR-0002: SolidJS reactivity and Rust-owned UI tree](decisions/0002-state-ownership-and-solidjs.md)
- [ADR-0003: Typed batched host protocol](decisions/0003-host-batch-protocol.md)
- [ADR-0004: Engine/UI thread separation](decisions/0004-engine-and-platform-threading.md)
- [ADR-0005: Taffy and hybrid native rendering](decisions/0005-layout-and-native-rendering.md)
- [ADR-0006: TSX-first application model and Web backend](decisions/0006-application-model-and-web.md)
- [ADR-0007: Capability security for dynamic modules](decisions/0007-capabilities-and-runtime-packages.md)
- [ADR-0008: Standalone cross-platform application framework](decisions/0008-standalone-application-framework.md)
- [ADR-0009: CSS and Tailwind CSS for native styling](decisions/0009-css-and-tailwind-native-styling.md)

## Reference material

- [Upstream architecture and API references](reference/upstream-references.md)

## How to read statuses

- **Implemented** means code exists and the relevant automated/manual checks have actually been run.
- **Target** means the behavior is part of the architecture specification but is not yet implemented.
- **Candidate** means the approach needs a prototype or benchmark before becoming a binding decision.
- **Experimental** means an API or backend may change before stable release.

Architecture documents are not proof of implementation. The current code status is maintained in README and roadmap. When implementation diverges from a target contract, update the relevant ADR/specification rather than silently treating the code as the new design. Do not advertise platform support, security properties or performance results beyond the evidence recorded for the exact implementation and target.
