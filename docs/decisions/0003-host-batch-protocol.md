# ADR-0003: Typed Batched Host Protocol

- Status: Accepted for target architecture.
- Date: 2026-11-10.

## Context

Solid Universal invokes host renderer methods for node creation, insertion, property updates and removal. Forwarding each property as an individual JSON message would create unnecessary serialization, allocation and validation costs.

At the same time, arbitrary JS objects and closures must not become Rust core values.

## Decision

- Host mutations are accumulated by the Solid Native adapter for the current JS task/commit boundary.
- Production transport is a typed, versioned binary batch backed by an ArrayBuffer/byte span.
- The protocol has an explicit header, operation count, payload size, string/value sections and version fields.
- Rust validates all ranges, IDs, value types and operation ordering before commit.
- Rust applies the batch atomically and increments the tree revision once.
- JS functions remain in a QuickJS callback registry and are referenced by opaque handler IDs only.
- JSON may be used for debug dumps or early test harnesses, but not for the production mutation path.

## Initial operations

CreateElement, CreateText, ReplaceText, SetProperty, RemoveProperty, InsertNode, RemoveNode, BindEvent, UnbindEvent, AttachResource and DetachResource.

## Consequences

- The protocol needs golden fixtures and strict compatibility tests.
- The adapter needs an acknowledgement/recovery model.
- Batch encoding adds some JS-side work but bounds JS/Rust boundary crossings.
- Binary transport is not a public app API. It may change only with protocol versioning.

## Rejected alternatives

- JSON per operation: rejected for production due to allocations, parsing and weak typing.
- Raw pointers to JS heap memory: rejected because lifetime and mutability cannot be safely guaranteed.
- Direct JS function serialization: impossible and conceptually invalid; callbacks remain VM-owned.
- Unbatched FFI call per property: rejected as the default due to avoidable boundary overhead.

## Invariants

- A batch is all-or-nothing.
- IDs are stable and scoped to one Surface session.
- Unknown major protocol versions fail before app code executes.
- The UI tree revision advances only after successful application.
