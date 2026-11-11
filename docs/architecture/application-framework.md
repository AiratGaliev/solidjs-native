# Application Framework Scope

- Status: accepted target architecture; implementation is milestone-driven.
- Decision: [ADR-0008](../decisions/0008-standalone-application-framework.md).
- Technical references: [NSH implementation reference and reuse map](nsh-feature-parity.md), [GPUI Kit comparison](gpui-kit-comparison.md), [Tauri 2 reference](../reference/upstream-references.md#tauri-2--application-framework-reference).

## 1. Product intent

SolidJS Native is developed as a standalone, installable, cross-platform application framework and SDK for applications built with upstream SolidJS and TypeScript/TSX. It is intended to serve independent applications and framework consumers. Product requirements, architecture and priorities are defined by the framework itself; no particular consuming product is privileged or required.

Tauri 2 is an external reference for studying how an established application framework approaches lifecycle, windows, system services, plugins, permissions, developer tooling and distribution. SolidJS Native's requirements and priorities are defined independently; Tauri does not define its product goal, required feature set or compatibility contract.

## 2. UI execution and rendering model

SolidJS Native is designed around a native retained UI renderer for the primary native application interface.

- Native desktop/mobile UI is rendered by SolidJS Native's platform backends; a system WebView is not the main application renderer.
- Web builds use the browser JavaScript engine and DOM as a separate target.
- An embedded OS WebView is not part of the initial selected architecture. Browser-only workloads use the separate Web target in a normal browser session.
- QuickJS-NG is embedded on native targets to execute the SolidJS/TSX application runtime. SolidJS Native therefore owns the lifecycle, scheduling, memory budget and distribution of its JavaScript VM and renderer.

This is a deliberate trade-off, not a claim that a native renderer is universally better than WebView. Startup time, memory, binary size, rendering quality, input latency, accessibility and maintenance cost require measurement on real applications.

## 3. Application framework capability areas

The following capability areas describe SolidJS Native's own framework responsibilities. They are derived from the intended product, platform requirements and consumer scenarios; they do not imply one-to-one parity with Tauri or any other framework.

| Capability area | SolidJS Native design direction | Scope guardrail |
|---|---|---|
| Application lifecycle | Versioned lifecycle owned by the native application runtime | Define explicit startup, activation, shutdown and error semantics |
| Windows | Native Window API and window manager, separated from the UI Surface renderer | Keep top-level window lifecycle distinct from per-Surface rendering |
| Application shell | Platform application host for menus, context menus, tray, shortcuts, icons and notifications | Implement only where the target OS and product require them |
| UI runtime | SolidJS Universal + QuickJS-NG on native targets; browser DOM on Web | Preserve upstream SolidJS as the application contract |
| Host communication | Separate typed/versioned Application Bridge | Keep application services separate from renderer mutations |
| Native services | Modular APIs for filesystem, dialogs, clipboard, URL opening, process/network and OS information | Explicit per-platform support and permission checks; no ambient privileges |
| Plugins | Versioned Rust interface plus TypeScript package; platform adapters where required | Do not make arbitrary dynamic native-library loading the default |
| Permissions | Deny-by-default capability authority scoped to app, window/Surface and resource | Enforce authorization at the Rust service boundary |
| Developer workflow | CLI, Vite/Solid integration, native runner, diagnostics and safe reload | Lifecycle-safe behavior and reproducible local/CI workflows |
| Distribution | Native packaging, signing/notarization, update channels and recovery | Implement per-platform policy and protect user data during updates |
| Embedding | Rust API first, then versioned C ABI and Swift/Kotlin bindings | Explicit ownership, thread, lifecycle and ABI contracts |
| Web | Independent first-class browser target | Do not require an embedded browser for native UI or browser-only workloads |

These areas are a planning model, not a claim that every capability is implemented today. Prioritize them from actual SolidJS Native requirements; do not add features merely to imitate another framework.

## 4. Two independent communication protocols

The UI renderer protocol and the application bridge solve different problems and must remain separate.

### 4.1. UI Host Protocol

Purpose: keep the SolidJS Universal renderer synchronized with Rust's authoritative retained UI tree.

- Carries renderer mutations: create/remove/insert nodes, text changes, typed properties and event binding metadata.
- Uses stable node handles, bounded typed batches, explicit revisions and atomic application.
- Does not serialize JavaScript functions; it uses runtime-owned callback IDs.
- Is defined by ADR-0003 and the [SolidJS runtime and host protocol](solidjs-runtime-and-host-protocol.md).
- Must not become a generic operating-system command bus.

### 4.2. Application Bridge

Purpose: let application code call native services and receive native events asynchronously.

Examples of the intended shape (illustrative API only; not implemented):

```tsx
import { app, window, clipboard, dialog } from "solid-native";

await window.setTitle("Sample Application");
await clipboard.writeText("Copied from the editor");

const selected = await dialog.openFile({
  multiple: false,
  filters: [{ name: "Sample project", extensions: ["demo"] }],
});

app.on("second-instance", event => {
  // Focus or route to an existing project window.
});
```

The Application Bridge requires:
- named, typed commands and typed results/errors;
- asynchronous request/response and one-way events;
- cancellation and shutdown semantics;
- resource handles for long-lived streams or OS objects;
- protocol/API version negotiation;
- permission checks in Rust before privileged work;
- bounded payloads, timeouts and back-pressure where relevant;
- testable ordering and delivery guarantees.

Choose the transport based on the typed contract and measured requirements. The Application Bridge is not required to use the same wire representation as the UI Host Protocol.

## 5. Plugin model

The public plugin model is designed around explicit separation of native implementation, application-facing API and platform integration:

- Rust crate: privileged native implementation and platform adapters.
- TypeScript package: application-facing typed API and event declarations.
- Optional Swift/Kotlin platform module: only when the native platform requires host-language integration.
- Manifest: plugin ID, version, runtime/API compatibility, platform support, commands/events, resources and required capabilities.
- Lifecycle: registration, initialization, application/window attachment, shutdown and resource cleanup.
- Permissions: plugins declare permissions; the host application grants them.

Keep two plugin categories distinct:

1. **Trusted native plugins** are Rust/platform code linked into or explicitly installed into the native host. They run with native process privileges; a capability API is not a sandbox against malicious Rust code. Their provenance, signing/build trust and review are separate concerns.
2. **Untrusted or separately delivered TSX runtime modules** execute inside a constrained QuickJS runtime and are governed by the signed package, capability and resource-limit model in ADR-0007 / M10.

Do not implement arbitrary dynamic native-library loading as the default plugin mechanism.

## 6. Capability and permission authority

Every system API must be protected at its Rust implementation boundary. TypeScript wrappers improve ergonomics but are not the security boundary.

Target rules:
- deny by default for privileged host services;
- application manifest declares requested capabilities;
- host/operator grants only the required capabilities;
- window/Surface identity is part of authorization context;
- resource scopes restrict concrete filesystem paths, URL origins, process names/arguments and other resource identifiers;
- user-selected files may produce narrow, explicit grants rather than ambient filesystem access;
- permission denial is typed, observable and testable;
- revocation and application shutdown invalidate handles and stop dependent work;
- native plugins remain trusted code and are not sandboxed by QuickJS limits.

QuickJS-NG resource limits and host capabilities constrain application modules, but do not provide an OS process sandbox. Strong isolation for genuinely untrusted native code requires a separate process boundary and OS-level sandboxing.

## 7. Platform and release strategy

- **First end-to-end target:** Windows x64, because it is the current development and first native vertical-slice environment.
- **Desktop targets:** Windows, macOS and Linux, each with its own windowing, rendering, text, input, accessibility and distribution backend.
- **Mobile targets:** Android and iOS after the desktop application lifecycle, service contracts and SDK boundaries are stable.
- **Web target:** browser-native JavaScript and DOM; it is not a native OS backend.
- **Embedding:** the same engine/session should support standalone applications and native host embedding, with explicit lifecycle and thread contracts.

Do not advertise a platform as supported because it compiles. Use platform maturity and acceptance requirements from [Platform Backends](platform-backends.md).

## 8. Independent consumer integration boundary

SolidJS Native is designed to be consumed as a standalone SDK by unrelated applications. Its core, public contracts and roadmap must not depend on a specific game engine, editor, domain model or application host.

- Consumers integrate through public, versioned SDK packages and documented platform capabilities.
- The framework owns UI/runtime/application-host primitives; each consumer owns its domain model, workflows, business logic and product-specific integrations.
- Native and Web targets expose the documented shared contracts while retaining explicit platform-specific capabilities and limitations.
- A consumer may embed SolidJS Native into an existing host or use it for a standalone application. Neither integration style is mandatory for framework development.
- Examples and test consumers must be independent applications, not forks or ports of a particular product.

## 9. Public SDK and consumer validation

Validate SolidJS Native using one or more independent sample consumer applications that use only its public, versioned SDK. This proves the framework can be consumed outside its own repository without making any particular product the roadmap driver.

Acceptance for a representative consumer integration:

- The consumer builds from a clean checkout using public, versioned Rust crates, NPM packages and CLI; no source vendoring or private-module imports.
- A representative application workflow exercises application lifecycle, windows, native controls, keyboard/input, dialogs and supported system services.
- Native text input/IME, focus, accessibility, windowing, file/clipboard access and permission enforcement pass applicable platform tests.
- The Web target builds and runs independently through the browser-native DOM/runtime.
- Development workflow, diagnostics, packaging/install/update and recovery behavior are validated for the platforms claimed by the sample.
- Startup, memory, idle CPU, input latency, first-frame time and package size meet explicit framework budgets.
- SDK integration guidance, examples, support matrix and known limitations are documented.

These checks validate SolidJS Native's own public contracts and product requirements, not integration with or migration from a particular consumer or external framework.

## 10. Non-goals

- Drop-in compatibility with another framework's JavaScript APIs, configuration files, plugin packages or IPC wire format.
- Tauri API/configuration/plugin compatibility or one-to-one feature parity.
- Replacing the browser engine or rendering arbitrary HTML/CSS with full browser CSS semantics natively. Native UI still requires the documented CSS/Tailwind compatibility profile; browser-only content uses the Web target or an explicit WebView.
- Removing embedded WebView support when a product genuinely needs browser content.
- Exposing every OS API in the first release.
- Implementing all desktop and mobile backends simultaneously.
- Treating a capability declaration as a security boundary for trusted native plugins.
