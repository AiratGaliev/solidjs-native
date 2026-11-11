# ADR-0003: Typed Batched Host Protocol

- Status: Accepted for target architecture.
- Date: 2026-11-10.

## Context

Solid Universal invokes host renderer methods for node creation, insertion, property updates and removal. Forwarding each property as an individual JSON message would create unnecessary serialization, allocation and validation costs.

At the same time, arbitrary JS objects and closures must not become Rust core values.

## Decision

- Host mutations are accumulated by the Solid Native adapter for the current JS task/commit boundary.
- Production transport is a typed, versioned binary batch backed by an ArrayBuffer/byte span.
- The Rust binding copies or validates the payload into owned Rust memory before returning control to JavaScript; it must not retain a borrowed ArrayBuffer/TypedArray pointer across JS execution, buffer detachment/resizing or GC.
- The protocol has an explicit header, operation count, payload size, string/value sections and version fields.
- Node handles are scoped to `SurfaceId + SessionGeneration + NodeId`. `SessionGeneration` fences delayed work from a previous Surface session; `NodeId` is monotonic and never reused within that session.
- Per-node generation is required before any future node-ID recycling is enabled.
- Rust validates all ranges, IDs, value types and operation ordering before commit.
- Rust applies the batch atomically and increments the tree revision once.
- JS functions remain in a QuickJS callback registry and are referenced by opaque handler IDs only.
- JSON may be used for debug dumps or early test harnesses, but not for the production mutation path.

## Initial operations

CreateElement, CreateText, ReplaceText, SetProperty, RemoveProperty, InsertNode, RemoveNode, BindEvent, UnbindEvent, AttachResource and DetachResource.

The exact v1.0 byte layout, operation sizes, value tags, arenas, limits and acknowledgement semantics are specified in [Host Protocol v1](../architecture/host-protocol-v1.md). V1 uses a 64-byte header, little-endian fixed-width records and a bounded UTF-8 string arena; property values are inline tagged values. The decoder accepts only major version 1/minor 0 in this implementation. Any incompatible layout change requires a new major version; backward-compatible additions require a minor-version policy and matching compatibility tests.

## Consequences

- The protocol needs golden fixtures and strict compatibility tests.
- The adapter needs an acknowledgement/recovery model.
- Batch encoding adds some JS-side work but bounds JS/Rust boundary crossings.
- Binary transport is not a public app API. It may change only with protocol versioning.

## Rejected alternatives

- JSON per operation: rejected for production due to allocations, parsing and weak typing.
- Raw pointers to JS heap memory or retained ArrayBuffer backing slices: rejected because JS can detach/reallocate buffers and the borrowed lifetime cannot extend beyond a validated binding call.
- Direct JS function serialization: impossible and conceptually invalid; callbacks remain VM-owned.
- Unbatched FFI call per property: rejected as the default due to avoidable boundary overhead.

## Invariants

- A batch is all-or-nothing.
- IDs are stable and scoped to one Surface session.
- Unknown major protocol versions fail before app code executes.
- The UI tree revision advances only after successful application.
