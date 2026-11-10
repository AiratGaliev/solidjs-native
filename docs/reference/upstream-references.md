# Upstream reference material

Дата проверки ссылок: 2026-11-10.

Документы ниже используются как источники API/архитектурных фактов. Они не означают, что SolidJS Native должен копировать конкретную реализацию или API этих проектов.

## Native Shell

- Repository: https://github.com/longbridge/nsh
- Product documentation: https://nsh.dev/docs/
- Architecture: https://nsh.dev/docs/architecture/
- Developer architecture: https://github.com/longbridge/nsh/blob/main/docs/dev/architecture.md
- Native pipeline: https://github.com/longbridge/nsh/blob/main/docs/dev/native-pipeline.md
- Developer docs index: https://github.com/longbridge/nsh/blob/main/docs/dev/README.md
- Agent-facing architecture summary: https://github.com/longbridge/nsh/blob/main/AGENTS.md

Observed architecture worth studying:
- one engine thread per surface/runtime;
- platform UI thread applies native mount batches;
- native Rust reactive graph and native renderer;
- authoritative Rust tree and commit pipeline;
- Taffy layout;
- per-platform text systems, native input and accessibility;
- native embedding and signed/capability-controlled runtime modules;
- distinct core, base-component and application design-system layers.

Important difference: NSH's reactive graph is implemented in Rust with Solid-like semantics. This project intentionally uses the actual upstream SolidJS runtime. Do not transfer assumptions about NSH's native reactive implementation into this project.

## SolidJS Universal

- Universal renderer README: https://github.com/solidjs/solid/blob/main/packages/solid/universal/README.md
- SolidJS repository: https://github.com/solidjs/solid
- vite-plugin-solid repository: https://github.com/solidjs/vite-plugin-solid

The Universal package is the official custom-renderer mechanism. Its contract is built around createRenderer and the host operations for elements, text, insertion, removal, properties and tree traversal. The build plugin supports generate=universal and a configured moduleName. Keep the custom renderer compatible with the installed SolidJS release.

## QuickJS-NG and Rust bindings

- QuickJS-NG: https://github.com/quickjs-ng/quickjs
- rquickjs: https://github.com/DelSkayn/rquickjs

rquickjs describes itself as high-level Rust bindings to QuickJS-NG, including JS/Rust value conversion, module loader integration, callback and async integration. Its existence does not make a QuickJS runtime thread-safe: Solid Native must maintain a single owner thread per runtime and explicitly design task scheduling, callback roots, shutdown and quotas.

## Taffy layout

- Taffy: https://github.com/DioxusLabs/taffy
- Taffy API documentation: https://docs.rs/taffy/

Taffy is a Rust layout library implementing CSS Block, Flexbox and Grid algorithms. Integrate through sn-layout and do not expose its internal types as the public Solid Native style API.

## Platform references

The native backend design should be grounded in the primary platform APIs:

- Windows: Win32, Direct2D, DirectWrite, UI Automation.
- macOS: AppKit, Core Animation, CoreText, NSAccessibility.
- Linux: GTK 4, GSK, Pango, AT-SPI and desktop portals.
- iOS: UIKit, Core Animation, CoreText, UIAccessibility.
- Android: Android View, JNI, Choreographer, StaticLayout and accessibility APIs.
- Web: DOM, CSS, browser event loop and browser accessibility tree.

Platform-specific documentation and version support must be rechecked when each backend begins. A platform matrix in platform-backends.md is a target, not proof of current implementation.

## Legal/provenance caution

GitHub repository metadata for longbridge/nsh reported a non-standard license marker (Other/NOASSERTION); fetching the expected root LICENSE path did not return a file in the checked repository state. Before copying NSH source or adapting a substantial code fragment, independently inspect current licensing/provenance and obtain appropriate legal review if required. The architecture documents here capture design ideas and requirements; they do not authorize source-code copying.
