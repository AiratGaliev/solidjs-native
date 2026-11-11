# Tauri 2 Replacement Scope

- Status: accepted product direction; implementation target.
- Decision: [ADR-0008](../decisions/0008-tauri-class-application-framework.md).
- Technical reference maps: [NSH implementation reference and reuse map](nsh-feature-parity.md), [GPUI Kit comparison](gpui-kit-comparison.md).
- External reference catalog: [Upstream references](../reference/upstream-references.md).

## 1. Product intent

SolidJS Native is intended to become a standalone, installable, cross-platform application framework that can replace Tauri 2 as the application host for Codogma Engine.

It must also remain usable by unrelated SolidJS/TypeScript projects. Codogma Engine is the first demanding product consumer, not a privileged fork or the only supported application.

"Replace Tauri 2" means **functional replacement for the selected Codogma application scenarios**, not source, binary, plugin, configuration or JavaScript API compatibility with Tauri. SolidJS Native may design a migration layer for Codogma, but application code must move to its own public API.

## 2. The decisive architectural difference

Tauri 2 combines a Rust application host with HTML rendered by an operating-system WebView. SolidJS Native is designed around a native retained UI renderer for the primary native application interface.

- Native desktop/mobile UI is rendered by SolidJS Native's platform backend; a system WebView is not the main application renderer.
- Web builds use the browser JavaScript engine and DOM as a separate target.
- An embedded OS WebView is not part of the initial selected architecture. Browser-only workloads, including Codogma/Phaser preview, use the separate Web target in a normal browser session.
- QuickJS-NG is embedded on native targets to execute the SolidJS/TSX application runtime. Unlike Tauri's normal WebView-hosted frontend, SolidJS Native therefore owns the lifecycle, scheduling, memory budget and distribution of its JavaScript VM and renderer.

This is a deliberate trade-off, not a claim that the native renderer is universally better than WebView. Startup time, memory, binary size, rendering quality, input latency, accessibility and maintenance cost require measurement on real applications.

## 3. Framework capability inventory

The following table defines the Tauri-class application-framework responsibilities that must be covered. It is a target inventory, not a statement that these features are implemented today or that every Tauri plugin must be reproduced.

| Capability area | Tauri 2 reference | SolidJS Native target |
|---|---|---|
| Application lifecycle | App identity, setup, ready/exit/restart and lifecycle events | Versioned app lifecycle owned by the native application runtime |
| Windows | Window creation, destruction, state, events and multiple windows | Native Window API and window manager, separated from the UI Surface renderer |
| Application shell | Menus, context menus, tray, global shortcuts, icons and notifications | Platform application host with typed commands/events and capability checks |
| UI runtime | WebView frontend | SolidJS Universal + QuickJS-NG on native targets; browser DOM on Web |
| Host communication | Commands and events | Separate typed/versioned Application Bridge |
| Native services | Filesystem, dialogs, clipboard, opener, process/shell, networking, OS information and notifications | Modular native service APIs, each with explicit platform support and permissions |
| Plugins | Rust crate plus optional NPM package; mobile plugins may include Swift/Kotlin code | Versioned Rust plugin interface plus TypeScript package; platform-specific adapters where required |
| Permissions | ACL permissions, capabilities, scopes and runtime checks | Deny-by-default capability authority scoped to app, window/surface and resource |
| Developer workflow | CLI, dev server, build, diagnostics and HMR | Solid Native CLI, Vite/Solid integration, native runner, diagnostics and lifecycle-safe HMR |
| Distribution | Platform bundles, signing and updater | Native packaging, signing/notarization, update channel and recovery policy |
| Embedding | Rust/native host integration | Rust API first, then versioned C ABI and Swift/Kotlin bindings |
| Web | WebView frontend inside a native shell | Independent first-class browser target, executed by the normal browser; embedded browser hosting is not in the initial scope |

The selected baseline should cover the services Codogma actually needs first. Features such as process spawning, shell access, arbitrary filesystem access, dynamic native loading and automatic updates must not be enabled merely to achieve superficial parity.

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

await window.setTitle("Codogma Engine");
await clipboard.writeText("Copied from the editor");

const selected = await dialog.openFile({
  multiple: false,
  filters: [{ name: "Codogma project", extensions: ["cgp"] }],
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

Do not copy Tauri's JSON IPC implementation by default. Choose the transport based on the typed contract and measured requirements. The Application Bridge is not required to use the same wire representation as the UI Host Protocol.

## 5. Plugin model

The public plugin model should preserve the useful separation demonstrated by Tauri:

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
- window/surface identity is part of authorization context;
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

## 8. Codogma Engine integration boundary

Codogma Engine is the first production-oriented consumer.

- Codogma's editor UI may depend on SolidJS Native as an external SDK/package.
- Codogma project model, game engine/core, Phaser runtime and game output must not depend on SolidJS Native.
- Native editor chrome and panels can run in the native renderer.
- Phaser preview runs through the separate Web target in a normal browser session; it is not hosted by the native UI renderer or an embedded browser engine.
- The editor should use public Solid Native APIs only. It must not import private runtime internals or vendor the framework source.
- Keep the existing Tauri host working until a feature-complete native replacement passes the migration acceptance gate.

Shared editor domain logic, commands, project state and validated workflows should remain independent of whether the host UI is Web or Native. DOM-dependent components are not automatically portable to the Universal renderer and need explicit native equivalents or cross-platform primitives.

## 9. Tauri replacement acceptance gate

Codogma may remove its Tauri 2 dependency only after a real editor build demonstrates all selected product-critical scenarios:

- standalone native application opens, closes, restarts and reports lifecycle correctly;
- main and secondary windows have correct create/close/focus/resize/state behavior;
- application menus, context menus, shortcuts, tray/notification features required by Codogma work on the target OS;
- file and folder dialogs, project file access, clipboard, external URL opening and required process/network services work through scoped native APIs;
- permission checks reject unauthorized operations at the Rust boundary;
- plugin registration, errors, version compatibility and cleanup are deterministic;
- development workflow supports run/build/test/diagnostics and safe reload/restart;
- release artifacts install, launch, sign and update according to the supported platform's policy;
- keyboard, text input/IME, focus, accessibility, DPI/scaling and window behavior pass real platform tests;
- startup, memory, idle CPU, input latency, first-frame time and package size meet budgets agreed for Codogma;
- the migration is reproducible from a clean checkout and does not require importing Solid Native private source.

The full Tauri plugin ecosystem is not a release requirement. The required baseline is the subset Codogma uses plus the documented public framework contracts.

## 10. Non-goals

- Drop-in compatibility with Tauri's JavaScript APIs, configuration files, plugin packages or IPC wire format.
- Replacing the browser engine or rendering arbitrary HTML/CSS with full browser CSS semantics natively. Native UI still requires the documented CSS/Tailwind compatibility profile; browser-only content uses the Web target or an explicit WebView.
- Removing embedded WebView support when a product genuinely needs browser content.
- Exposing every OS API in the first release.
- Implementing all desktop and mobile backends simultaneously.
- Treating a capability declaration as a security boundary for trusted native plugins.
- Declaring Tauri replacement complete based only on a Counter example or native window screenshot.
