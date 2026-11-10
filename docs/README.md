# Documentation index

Документация описывает SolidJS Native как полноценный native UI runtime/framework, целевой архитектурный аналог NSH с настоящим upstream SolidJS.

## Start here

1. [Target architecture](architecture/target-architecture.md) — целевая система, слои, ownership, threading, tree, protocol, layout, rendering, security, performance.
2. [SolidJS runtime and host protocol](architecture/solidjs-runtime-and-host-protocol.md) — Universal renderer, QuickJS integration, typed batched bridge, scheduler, callback lifecycle and tests.
3. [Platform backends](architecture/platform-backends.md) — Windows/macOS/Linux/iOS/Android/Web, rendering strategy, embedding and platform acceptance.
4. [NSH feature parity](architecture/nsh-feature-parity.md) — что входит в понятие полноценной замены, parity mapping и release scope.
5. [Implementation roadmap](roadmap.md) — milestones M0-M11, work packages and acceptance criteria.

## Engineering policy

- [Dependency policy and current stable version baseline](dependency-policy.md)

## Architecture decisions

- [ADR-0001: SolidJS Universal and QuickJS-NG](decisions/0001-solidjs-and-runtime.md)
- [ADR-0002: SolidJS reactivity and Rust-owned UI tree](decisions/0002-state-ownership-and-solidjs.md)
- [ADR-0003: Typed batched host protocol](decisions/0003-host-batch-protocol.md)
- [ADR-0004: Engine/UI thread separation](decisions/0004-engine-and-platform-threading.md)
- [ADR-0005: Taffy and hybrid native rendering](decisions/0005-layout-and-native-rendering.md)
- [ADR-0006: TSX-first application model and Web backend](decisions/0006-application-model-and-web.md)
- [ADR-0007: Capability security for dynamic modules](decisions/0007-capabilities-and-runtime-packages.md)

## Reference material

- [Upstream architecture and API references](reference/upstream-references.md)

## How to read statuses

- **Implemented** means code exists and the relevant automated/manual checks have been run.
- **Target** means the behavior is part of the architecture specification but is not yet implemented.
- **Candidate** means the approach needs a prototype or benchmark before becoming a binding decision.
- **Experimental** means an API or backend may change before stable release.

Architecture documents are not proof of implementation. The current code status is maintained in README and roadmap. When implementation diverges from a target contract, update the relevant ADR/specification rather than silently treating the code as the new design.
