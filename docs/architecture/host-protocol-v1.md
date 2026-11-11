# Host Protocol v1

- Status: implemented wire contract for M1; binary format version `1.0`.
- Canonical decision: [ADR-0003 — Typed Batched Host Protocol](../decisions/0003-host-batch-protocol.md).
- Implementation: `crates/sn-core/src/host_protocol.rs`, `packages/solid-native/src/protocol.ts`.

This document specifies the exact v1 bytes. It is an internal runtime contract, not a public application API.

## 1. Framing and limits

All integer fields are little-endian. Offsets in the header are absolute from the first batch byte; string references are relative to the string arena.

| Field | Offset | Size | Meaning |
|---|---:|---:|---|
| Magic | 0 | 4 | ASCII `SNHB` |
| Major | 4 | 2 | `1` |
| Minor | 6 | 2 | `0` |
| Header size | 8 | 2 | `64` |
| Flags | 10 | 2 | Must be zero |
| SurfaceId | 12 | 8 | Non-zero Surface identifier |
| SessionGeneration | 20 | 8 | Non-zero generation for the Surface session |
| TransactionId | 28 | 8 | Non-zero, strictly sequential within the session |
| Expected revision | 36 | 8 | Current committed UiTree revision |
| Operation count | 44 | 4 | Number of records; range 1–65,535 |
| Payload size | 48 | 4 | Exact number of bytes after the 64-byte header |
| String arena offset | 52 | 4 | Absolute start of UTF-8 arena |
| String arena length | 56 | 4 | Exact UTF-8 arena size |
| Reserved | 60 | 4 | Must be zero |

The payload is the concatenation of exactly `operation_count` records followed by the string arena. The arena must end at the end of the batch. No trailing bytes are accepted.

Limits:
- Maximum complete batch: 4 MiB.
- Maximum operations: 65,535.
- Maximum individual UTF-8 string: 1 MiB.
- Maximum cumulative decoded string bytes per batch: 4 MiB. This prevents repeated references to the same arena range from amplifying memory during decoding.
- Every record, offset and length is range-checked with checked arithmetic.
- Strings must be valid UTF-8 and must not contain NUL.
- Empty text/property string values are valid; names and tags are validated by the tree contract.

## 2. Operation record framing

Every record starts with an 8-byte header:

| Field | Record offset | Size |
|---|---:|---:|
| Opcode | 0 | 1 |
| Flags | 1 | 1 |
| Reserved | 2 | 2 |
| Record size | 4 | 4 |

Flags and reserved bits must be zero. Record size includes this 8-byte header and must match the exact size for the opcode.

String references are two consecutive `u32` values: arena-relative byte offset, then byte length. Node IDs are non-zero `u64`. Handler IDs are non-zero `u32`. A zero `u64` anchor means no anchor.

## 3. Operation vocabulary

| Opcode | Operation | Record size | Payload after record header |
|---:|---|---:|---|
| 1 | CreateElement | 24 | NodeId `u64`, tag string reference |
| 2 | CreateText | 24 | NodeId `u64`, initial text string reference |
| 3 | ReplaceText | 24 | NodeId `u64`, replacement text reference |
| 4 | SetProperty | 40 | NodeId `u64`, property-name reference, value kind `u8`, 7 reserved zero bytes, value bits `u64` |
| 5 | RemoveProperty | 24 | NodeId `u64`, property-name reference |
| 6 | InsertNode | 32 | Parent NodeId `u64`, child NodeId `u64`, anchor NodeId `u64` or zero |
| 7 | RemoveNode | 16 | Subtree root NodeId `u64` |
| 8 | BindEvent | 32 | NodeId `u64`, event-name reference, HandlerId `u32`, reserved `u32 = 0` |
| 9 | UnbindEvent | 24 | NodeId `u64`, event-name reference |
| 10 | AttachResource | 32 | NodeId `u64`, resource-name reference, non-zero resource token `u64` |
| 11 | DetachResource | 24 | NodeId `u64`, resource-name reference |

### SetProperty value kinds

| Kind | Meaning | Value bits |
|---:|---|---|
| 0 | Null | Must be zero |
| 1 | Boolean | Only 0 or 1 |
| 2 | Number | IEEE-754 `f64`; NaN and infinities are rejected |
| 3 | String | Low 32 bits = arena offset; high 32 bits = byte length |

Arbitrary JavaScript objects and functions are not property values. Event functions use the callback registry and are represented only by opaque HandlerIds.

## 4. Identity and ordering

A host node handle is `SurfaceId + SessionGeneration + NodeId`.

- NodeId starts at 1 and increases monotonically for the lifetime of the session.
- Removed NodeIds are never reused.
- The JS adapter keeps the Surface/session alongside each NativeNode handle.
- Rust rejects batches for another Surface/session and unknown or out-of-sequence node IDs.
- A later operation in the same batch may reference a node created earlier in that batch.
- InsertNode validates the anchor against the target parent before handling the self-anchor no-op. Moves are evaluated after detaching the child.

TransactionId starts at 1 and must advance by exactly one after a successful commit. Expected revision must match the current tree revision. A successful non-empty batch increments revision exactly once. Any decoder, validation or apply error leaves the committed tree, revision and previous active event bindings unchanged.

## 5. Commit and callback lifecycle

The native QuickJS binding:
1. checks the input is a bounded Uint8Array;
2. copies its bytes to an owned Rust `Vec<u8>` during the binding call;
3. strictly decodes and validates the complete batch;
4. verifies every BindEvent HandlerId exists in the session callback registry;
5. applies operations to a staged UiTree clone;
6. commits the staged tree and advances revision once;
7. releases callback roots no longer referenced by committed tree events;
8. returns an acknowledgement with SurfaceId, SessionGeneration, TransactionId, revision and next NodeId.

No ArrayBuffer/TypedArray backing pointer is retained after the binding returns. JS callbacks are rooted with `Persistent<Function>` in the QuickJS runtime. Failed transactions discard uncommitted callback roots while preserving callbacks referenced by the last committed tree. RemoveNode releases all handlers in the removed subtree after commit; disposing the Surface releases all callback roots.

The JS adapter becomes poisoned after a rejected batch. Because the Rust transaction is atomic, the committed tree, revision and active callback bindings remain unchanged; only uncommitted operations and callback roots are discarded. The adapter does not continue with a potentially divergent JS mirror, and its owner must dispose and remount the Surface before further use. If the native binding returns an invalid, missing or mismatched acknowledgement, the adapter aborts the whole Surface session because the committed revision can no longer be trusted.

## 6. Validation coverage

The Rust decoder tests golden framing, version rejection, malformed lengths, truncated and oversized payloads, unknown opcodes, reserved bits, invalid UTF-8 and string-range violations. Tree tests cover monotonic IDs, same-parent/cross-parent moves, self/foreign anchors, cycles, subtree deletion, revision conflicts and atomic rollback. The JS encoder test runner verifies a golden v1 record, UTF-8 arena references, empty text and rejection of non-finite values and malformed surrogate pairs.

Full scheduler ordering, asynchronous jobs, platform input queues, MountBatch generation, layout and native OS rendering remain outside M1.
