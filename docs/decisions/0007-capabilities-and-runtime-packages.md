# ADR-0007: Capability Security for Dynamic Modules

- Status: Accepted as target requirement; implementation deferred to M10.
- Date: 2026-11-10.

## Context

A complete NSH alternative should eventually load UI modules at runtime, including team-authored or generated screens. Dynamic code runs inside a native host process and may otherwise inherit dangerous privileges.

## Decision

- Runtime modules are packaged with manifest, version requirements, content hashes, dependencies and requested capabilities.
- Host application grants capabilities explicitly; a module cannot grant privileges to itself.
- Default policy is deny for filesystem, network, process, clipboard, external URLs, native components and other sensitive host services.
- QuickJS runtime limits cover heap, stack, execution budget and interruption.
- Package signatures verify provenance/integrity but do not replace capability checks.
- Module loading validates runtime/protocol compatibility before execution.
- Revocation and shutdown release timers, handlers, resources and native registrations.

## Consequences

- Module API must be designed around capabilities from the start.
- Package format and trust store are separate from the renderer protocol.
- AI-generated UI is treated as untrusted code until reviewed/granted.
- Dynamic package delivery is deferred until static app runtime and lifecycle are stable.

## Rejected alternatives

- Give every module unrestricted native host access: rejected.
- Treat signed packages as automatically trusted: rejected.
- Build dynamic package delivery before runtime and module loader stabilize: rejected.

## Invariants

- Untrusted modules cannot access ambient filesystem/process/network privileges.
- Capability denial is explicit and observable.
- Resource limits cannot rely solely on cooperative app code.
