# Documentation index

Документация описывает SolidJS Native как самостоятельный cross-platform application framework для настоящего upstream SolidJS. Tauri 2 используется только как внешний референс для изучения архитектуры подобных framework и developer workflow. Продуктовые требования SolidJS Native определяются самостоятельно. Роли источников разделены: NSH — основной native implementation и platform-rendering reference; GPUI Kit/GPUI Shell — comparative desktop UI and QuickJS-host reference; Servo Stylo — reference для CSS implementation.

## Start here

1. [Application framework scope](architecture/application-framework.md) — independent framework scope, Application Bridge, plugins, permissions, platforms and consumer validation.
2. [Target architecture](architecture/target-architecture.md) — system layers, ownership, threading, tree, protocols, layout, rendering, security and performance.
3. [Architecture reuse and dependency adoption map](architecture/dependency-adoption.md) — final reference roles, selected components, prototype candidates and reuse boundaries.
4. [Styling, CSS and Tailwind CSS](architecture/styling-and-css.md) — shared styling model, native CSS compatibility profile, Tailwind build integration and Stylo adapter.
5. [SolidJS runtime and host protocol](architecture/solidjs-runtime-and-host-protocol.md) — Universal renderer, QuickJS integration, typed batched UI bridge, scheduler and callback lifecycle.
6. [Platform backends](architecture/platform-backends.md) — Windows/macOS/Linux/iOS/Android/Web, native rendering, application host, embedding and platform acceptance.
7. [GPUI Kit comparison](architecture/gpui-kit-comparison.md) — closest adjacent desktop framework, GPUI Shell runtime and reuse decisions.
8. [NSH implementation reference map](architecture/nsh-feature-parity.md) — concrete upstream native implementations, architectural differences and reuse rules.
9. [Implementation roadmap](roadmap.md) — milestones M0-M16, work packages and acceptance criteria.

## Engineering policy

- [Development workflow and GitHub CI policy](development-workflow.md)
- [Dependency policy and current stable version baseline](dependency-policy.md)

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

- **Implemented** means code exists and the relevant automated/manual checks have been run.
- **Target** means the behavior is part of the architecture specification but is not yet implemented.
- **Candidate** means the approach needs a prototype or benchmark before becoming a binding decision.
- **Experimental** means an API or backend may change before stable release.

Architecture documents are not proof of implementation. The current code status is maintained in README and roadmap. When implementation diverges from a target contract, update the relevant ADR/specification rather than silently treating the code as the new design.
