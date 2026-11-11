# Security Baseline and Trust-Boundary Policy

- Status: mandatory security design policy.
- Current state: security architecture and capability requirements are specified, but future dynamic-module enforcement and the full adversarial test suite are roadmap work, not completed product capabilities.
- Related decisions: [ADR-0007 — Capability Security for Dynamic Modules](decisions/0007-capabilities-and-runtime-packages.md).

This document records required security properties and the current implementation boundary. It must not be used to claim that the framework already provides a sandbox, signed package loading or complete isolation.

## 1. Security principles

- **Least privilege:** application/runtime code receives only the capabilities explicitly exposed and granted by the host.
- **Default deny:** absence of a capability or unsupported platform service must not silently grant access.
- **Enforcement at the authority boundary:** privileged operations are validated and authorized by Rust/native host implementations, not only by TypeScript wrappers.
- **Explicit trust:** trusted native plugins and runtime-loaded application modules are different security classes.
- **Validated boundaries:** all cross-language, FFI, protocol, filesystem and platform inputs are untrusted until validated.
- **Deterministic lifecycle:** handles, callbacks, subscriptions, resources and temporary privileges have explicit ownership, revocation and cleanup.
- **Reproducible supply chain:** dependency provenance, lockfiles, licenses and updates follow the dependency policy.

## 2. Trust boundaries

Treat these as explicit boundaries in design and review:
- application TSX/JavaScript versus the embedded runtime;
- QuickJS runtime versus Rust host and retained UI tree;
- renderer Host Protocol versus Application Bridge/native services;
- application-level requests versus privileged OS operations;
- trusted Rust/native plugins versus dynamic or third-party application modules;
- platform backend versus platform-neutral core;
- downloaded or generated package contents versus executable code and native resources.

Do not allow renderer mutations to become an implicit route to arbitrary filesystem, process, network or clipboard access. Keep the Host Protocol and Application Bridge as separate contracts.

## 3. Protocol, FFI and memory safety

- Validate protocol version, message type, IDs, generations, lengths, offsets, enum values, anchors and state transitions before applying a batch.
- Apply transactions atomically where specified; invalid input must not partially mutate authoritative state.
- Reject stale handles and callbacks after session/window/runtime disposal.
- Do not retain borrowed JavaScript heap memory outside its valid lifetime.
- Do not unwind a Rust panic across FFI.
- Keep unsafe code minimal and locally justified; test the safety invariants.
- Bound queues, payload sizes, resource allocations and runtime work where the corresponding runtime supports such controls.
- Define cancellation, timeout, back-pressure and cleanup behavior for all asynchronous/host operations.

## 4. Native capabilities and permissions

For application services such as window control, clipboard, file dialogs, scoped file access, notifications or future native plugins:
- define typed commands, events, results and stable errors;
- declare the required capability and platform support;
- enforce permission checks in the Rust/native service before performing the operation;
- scope file/resource handles to the authorized application or session;
- support cancellation and deterministic release;
- return an explicit denial or unsupported-capability result instead of silently degrading.

Do not add shell/process execution or unrestricted network/file access without a concrete product requirement, threat model and separately reviewed decision.

## 5. Dynamic modules and plugins

The M10 roadmap work defines signed package manifests, content hashes, trust store, capability grants, resource quotas, interrupt/termination and revocation. These are target requirements until implemented and tested.

A signature establishes package provenance/integrity; it does not bypass runtime capability policy. Native Rust plugins execute trusted process code and must never be described as sandboxed merely because the JS runtime has its own isolation.

Do not load untrusted native code into the process. Do not advertise dynamic-module isolation, package-signature enforcement or runtime quotas until adversarial tests demonstrate those properties on the claimed targets.

## 6. Dependencies, source and secrets

- Follow [Dependency Policy](dependency-policy.md) for stable version selection, lockfiles, provenance, licenses and advisory review.
- Preserve upstream license and attribution notices for copied/adapted source, assets and generated material.
- Do not commit credentials, tokens, private keys, signing material, local environment files or user data.
- Do not print secrets or full sensitive payloads in logs, test failures, PRs or Linear comments; redact diagnostics.
- Keep GitHub Actions permissions at the minimum needed. Review token permissions and external actions when workflows change.
- Review dependency and build-script changes for unexpected network access, executable build steps and native toolchain implications.

## 7. Security testing and issue handling

Security-sensitive changes require tests for both allowed and denied paths. Add malformed-input, stale-handle, permission-denial, cancellation, resource-limit and lifecycle tests as the relevant capability is implemented.

For a suspected vulnerability:
- preserve a minimal reproduction without publishing exploit details or secrets;
- identify affected versions, boundary and impact;
- create/link a Linear Bug or security issue with restricted evidence where appropriate;
- fix the authority boundary rather than hiding the symptom in a wrapper;
- add a regression test and review adjacent callers/permissions;
- record what was tested and what remains uncertain.

A passing build or a code review is not a security audit. Record external audit status separately; do not claim certification or complete isolation without formal evidence.
