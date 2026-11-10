# ADR-0004: Engine Thread and Platform UI Thread Separation

- Status: Accepted for target architecture.
- Date: 2026-11-10.

## Context

QuickJS execution, Solid effects, layout and platform UI object mutation have different ownership constraints. OS frameworks commonly require UI objects to be used from a main/UI thread. An application handler may be slow, throw, or enter an infinite loop.

## Decision

- Each Surface owns one EngineSession and one QuickJS runtime owner thread on native targets.
- Platform UI objects are created and mutated only on the platform UI thread.
- The engine thread executes user code, applies host transactions, computes layout and produces MountBatch data.
- Cross-thread communication uses bounded queues or explicit platform dispatchers.
- Platform input is normalized and queued to the engine thread; it never recursively enters QuickJS.
- The UI thread does not synchronously wait for JS, layout or resource I/O.
- Synchronous OS decisions are answered from previously mounted/cached properties.
- Runtime work uses fairness budgets and cancellation/interrupt mechanisms.

## Consequences

- Input handling and JS scheduling are asynchronous relative to OS event callbacks.
- The engine may lag under heavy app work, but should not freeze the platform event loop.
- Platform adapters need explicit thread assertions and queue lifecycle.
- Resize/layout updates can be delayed by engine load; the UI backend must preserve responsiveness and use the last committed state safely.

## Rejected alternatives

- Execute JS on the platform UI thread: rejected due to responsiveness and platform-thread constraints.
- Share one QuickJS context across threads: rejected because VM ownership and callback lifetimes become unsafe.
- Let platform callbacks synchronously call handlers: rejected because it creates reentrancy and deadlock risks.

## Invariants

- A QuickJS Runtime/Context has one owning thread.
- No JS call crosses from platform main thread.
- MountBatch is immutable after publication.
- All queues have defined capacity, overflow and shutdown behavior.
