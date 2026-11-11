# SolidJS Runtime, Universal Renderer и Host Protocol

- Статус: целевой контракт реализации.
- Связанные документы: target-architecture.md, platform-backends.md, ADR-0001, ADR-0002, ADR-0003.
- Цель: сделать SolidJS/QuickJS/Rust integration проверяемой подсистемой с зафиксированными границами владения.

## 1. Runtime topology

Native runtime surface состоит из:

- EngineSession — владелец жизненного цикла одного UI instance.
- JsRuntime — абстракция над конкретным VM.
- QuickJsRuntime — реализация JsRuntime на QuickJS-NG/rquickjs.
- JsContext — контекст приложения и его native modules.
- ModuleLoader — загрузчик app modules и runtime modules.
- SolidHostAdapter — код пакета solid-native, скомпилированный для Universal renderer.
- HostBatchEncoder — JS-side сборщик операций.
- HostBatchDecoder — Rust-side строгий decoder.
- CallbackRegistry — хранение JS callback roots и идентификаторов.
- Scheduler — микрозадачи, timers, engine tasks и commit boundaries.
- Diagnostics — ошибки, trace events, source map mapping и runtime metrics.

QuickJS types не должны появляться в публичных API EngineSession, sn-core, sn-layout, sn-platform-api или platform crates.

## 2. SolidJS integration

### 2.1. Компиляция

Использовать официальный SolidJS compiler через vite-plugin-solid:

- generate = universal;
- moduleName = solid-native;
- SolidJS package version фиксируется lockfile;
- compile-time настройки и runtime package должны выпускаться совместимыми версиями;
- JSX runtime должен экспортировать необходимые функции custom renderer;
- control-flow helpers должны экспортироваться или быть доступны способом, который требует установленная версия SolidJS.

Нельзя писать собственный TSX/JSX compiler для повторения Solid semantics. Любое расхождение между компилятором, типами и renderer будет трудно отлаживать и станет источником несовместимости.

### 2.2. Universal renderer contract

solid-native реализует контракт createRenderer из solid-js/universal:

- createElement;
- createTextNode;
- replaceText;
- setProperty;
- insertNode;
- removeNode;
- isTextNode;
- getParentNode;
- getFirstChild;
- getNextSibling.

Публичные экспортируемые обёртки Solid renderer — render, effect, memo, createComponent, createElement, createTextNode, insertNode, insert, spread, setProp, mergeProps и use — должны следовать актуальному контракту версии SolidJS из lockfile.

Нельзя менять семантику вставки, ownership или cleanup под видом упрощения. Для каждого метода необходимо зафиксировать:
- вызывается ли он один раз или реактивно;
- допустим ли повторный вызов;
- возможен ли перенос узла;
- как обрабатывается anchor;
- что происходит при невалидном node;
- когда host mutation считается committed;
- как освобождаются JS callback references.

### 2.3. Host node wrapper

NativeNode — lightweight adapter handle, а не копия всего Rust Node.

Минимальные поля:
- stable node ID;
- node kind;
- tag/component kind для диагностики;
- parent wrapper;
- ordered child wrappers;
- local property metadata только там, где это необходимо renderer adapter;
- SurfaceId and SessionGeneration, used to reject stale or foreign-session handles.

Запрещено держать в NativeNode:
- layout frames;
- resolved CSS/Taffy styles;
- native OS objects;
- platform handles;
- authoritative accessibility state;
- копию всех Rust properties;
- копию scroll/focus state без чёткого contract.

Причина существования JS-side topology mirror — Solid Universal traversal callbacks. Без parent/children/sibling информацию renderer не сможет корректно выполнять reconciliation/insert/remove. Этот mirror обязан повторять только структурные изменения, а Rust остаётся источником истины после commit.

### 2.4. Renderer mutation semantics

Каждый renderer callback сначала валидирует входные host handles и применяет изменение к локальной adapter topology. Затем он добавляет соответствующую typed operation в текущий batch.

Для insert/move:
1. Проверить, что parent и node живы.
2. Проверить, что anchor либо отсутствует, либо текущий child указанного parent.
3. Проверить отсутствие циклов локально.
4. Отсоединить node от предыдущего родителя.
5. Вычислить anchor index уже после detach.
6. Вставить в новое место.
7. Добавить InsertNode operation в тот же порядок, в котором Solid вызвал renderer.
8. Синхронизировать logical lifecycle metadata.

Для remove:
1. Удалить node из parent topology.
2. Пометить весь subtree как pending removal.
3. Сгенерировать RemoveNode operation для корня subtree, если Rust удаляет subtree целиком.
4. После подтверждения commit освободить host wrapper metadata, event handlers и связанные ресурсы.

Нельзя немедленно уничтожать callback registry entries до успешного commit, иначе откат batch оставит живой node без callback. Использовать pending-release list, применяемый после подтверждённой транзакции.

## 3. Stable node IDs

В M1 handle состоит из `SurfaceId + SessionGeneration + NodeId`. SessionGeneration меняется при создании новой runtime session для Surface и защищает от delayed events/batches предыдущего жизненного цикла.

Требования:
- `NodeId` — monotonic integer в пределах одной session.
- `NodeId` не повторяется и не переиспользуется внутри session; удаление навсегда инвалидирует ID.
- Полный handle валиден только в пределах соответствующих `SurfaceId + SessionGeneration`.
- `NodeId` не превышает `Number.MAX_SAFE_INTEGER`, если передаётся в JS как Number.
- Rust отклоняет дублирование, unknown references, stale references и handles другой Surface/session.
- Batch header и callback/event metadata несут достаточно данных, чтобы engine мог проверить Surface/session scope.

Per-node generation-aware packed handle понадобится только перед введением повторного использования NodeId (например, slot recycling). До появления такой возможности ID остаётся monotonic и never-reused внутри session; это проще и исключает ABA/use-after-free сценарии без усложнения первого wire format.

## 4. Typed host batch protocol

### 4.1. Why not JSON

JSON в текущем прототипе может использоваться для диагностического вывода, но не должен стать shipping bridge. JSON требует аллокаций, кодирования строк, повторного парсинга и runtime validation, плохо описывает бинарные ресурсы и не имеет естественной модели typed handles.

Production path:

Solid renderer callbacks
→ JS operation records
→ compact encoded ArrayBuffer
→ one native binding call at commit
→ strict Rust decoder
→ validated transaction
→ UiTree commit.

### 4.2. Batch data model

Batch header:
- magic;
- protocol version major/minor;
- flags;
- SurfaceId;
- SessionGeneration;
- transaction ID;
- expected tree revision;
- operation count;
- payload length;
- section offsets/lengths.

Operation records:
- opcode;
- record flags;
- record size;
- typed payload.

Variable data:
- UTF-8 string arena;
- typed value arena;
- optional asset/resource reference table.

All offsets are relative to the beginning of the batch. All integers use explicitly fixed little-endian representation. Every offset+length operation must use checked arithmetic. Decoder must never trust the header length or count.

### 4.3. Protocol versioning

- Major version changes are incompatible.
- Minor version adds backward-compatible records/optional fields only.
- Runtime package declares the protocol range it supports.
- The engine negotiates compatibility before app module evaluation.
- A mismatch fails at startup with expected/actual version in diagnostics.
- Golden binary fixtures protect the wire layout from accidental changes.

### 4.4. Operation semantics

CreateElement and CreateText reserve IDs in transaction scope. A later operation in the same batch may reference a node created earlier in that batch.

ReplaceText updates only Text node content.

SetProperty and RemoveProperty are validated against the registered element contract. Function values are forbidden.

InsertNode moves the existing node; it does not create a duplicate logical node.

RemoveNode removes the node and owned descendants according to core ownership semantics. A node cannot receive events after removal commit.

BindEvent carries a handler ID, not the JS function object. The callback registry owns the function and releases it on confirmed unbind/removal.

AttachResource references a runtime-created resource token. It cannot contain arbitrary pointers or paths.

### 4.5. Typed value algebra

The value wire format should have explicit variants:

- null;
- boolean;
- finite number;
- UTF-8 string reference;
- interned enum identifier;
- color;
- length;
- point/size/rect/insets;
- transform;
- typed numeric array;
- typed layout style;
- typed text style;
- typed accessibility metadata;
- resource handle.

No implicit serialization of arbitrary JS objects. Public API conversion functions must be explicit and covered by TypeScript tests.

### 4.6. String and property interning

Intern property names and common enum strings per runtime session or per protocol version where safe. Never treat a client-provided string index as trusted: validate it against the string table and bounds.

String interning must not become permanent global state. Release session tables on runtime shutdown.

## 5. Transaction lifecycle

A batch lifecycle consists of:

- Open — runtime begins collecting mutations for one JS task.
- Encode — JS adapter serializes records into a bounded transferable payload.
- Decode — Rust validates structure and ranges.
- Validate — tree references, operation order, types, capabilities and sizes are checked.
- Apply — changes are applied to a transaction staging area.
- Commit — tree revision increments exactly once.
- Derive — layout/resource/accessibility invalidation is created.
- Mount — engine produces a MountBatch.
- Ack — adapter receives committed revision and releases pending resources.
- Fail — no tree mutation is committed; error reports operation index and transaction ID.

The adapter must not silently reset its local mirror after a failure. It must either:
- restore the last acknowledged mirror snapshot using a mutation journal; or
- dispose and reconstruct the root from the last committed Rust snapshot.

The recovery strategy must be selected explicitly. Best-effort continuing with mismatched trees is forbidden.

## 6. Scheduler contract

### 6.1. Required job classes

- JS synchronous call stack;
- Promise jobs;
- queueMicrotask;
- timers;
- engine external tasks;
- platform input events;
- resource/network completions;
- animation/frame notifications;
- shutdown/dispose tasks.

All job classes must execute on the owning engine thread for the relevant JS runtime.

### 6.2. Turn ordering

The scheduler must define exact ordering for:
- nested Promise callbacks;
- queueMicrotask from inside event handlers;
- timer callback while a batch is pending;
- resource completion after the target node was removed;
- new events arriving while an earlier event is executing;
- runtime shutdown with pending timers;
- exceptions thrown in Promise jobs;
- reentrant calls from native bindings.

No host binding may recursively enter the same QuickJS context from a platform callback. Native input is queued; it is not delivered by recursive JS invocation.

### 6.3. Event loop strategy

The initial engine loop should be deterministic and single-owner:
1. take the next engine task;
2. set the surface clock/context;
3. execute the task;
4. drain the supported pending JS jobs until quiescent or budget exhausted;
5. flush host operations;
6. commit the core transaction;
7. emit mount batch;
8. schedule timers/next frame as required.

A fairness/budget policy is required so an infinite Promise chain cannot starve native input forever. Budget exhaustion yields the engine loop and records a diagnostic. It must not deadlock the platform UI thread.

### 6.4. Timers and clocks

Use a monotonic clock for duration and a wall clock for Date/time formatting. Tests need a manual clock. Define:
- minimum timer resolution;
- task cancellation;
- repeating timer drift policy;
- callback behavior after component unmount;
- ordering between timeout and input events with equal deadlines;
- app background/suspend behavior.

### 6.5. requestAnimationFrame

requestAnimationFrame must have one documented meaning:
- on native, callback is scheduled from the Surface frame clock and delivered on engine thread;
- it is coalesced to one callback per animation frame;
- callbacks queued before a frame run in deterministic order;
- cancellation prevents delivery when it has not started;
- runtime does not wake a suspended surface solely for an unnecessary RAF.
Native transitions should use platform animation driver instead of scheduling a JS callback every frame.

## 7. Runtime API compatibility inventory

The runtime must maintain an explicit support table for globals and modules:

- ECMAScript language features emitted by the pinned compiler;
- ESM import and module resolution;
- Promise, async/await and async iteration where used;
- queueMicrotask and pending Promise jobs;
- timers and cancellation;
- performance.now;
- requestAnimationFrame;
- AbortController/AbortSignal;
- URL/URLSearchParams;
- TextEncoder/TextDecoder;
- ArrayBuffer/DataView/TypedArray;
- structured clone if exposed;
- fetch/Request/Response/Headers/ReadableStream only when implemented fully enough for documented use;
- console and source-mapped stack traces;
- crypto primitives only when implemented securely;
- locale/timezone APIs only when behavior can be defined consistently.

A browser global must not be exposed as a stub that behaves differently in subtle ways. For each API, choose supported, unsupported, or platform-specific and write conformance tests.

## 8. Callback registry and garbage collection

QuickJS GC must be able to reclaim callback objects when no longer registered or referenced by app state.

Rules:
- registry owns references only while event binding is active;
- pending batch references are retained until transaction ack;
- removing a subtree schedules release after commit;
- replacing an event handler releases the previous registry entry after the new binding is committed;
- failures preserve the old active handler until a corrective transaction succeeds;
- runtime shutdown releases all callback roots and native resources in a defined order;
- no callback may hold a strong Rust reference cycle that prevents Surface shutdown.

Test repeated mount/unmount, callback replacement, event dispatch after removal and forced GC. Measure registry size before and after cleanup.

## 9. Error propagation

Runtime boundary must convert QuickJS exception into structured RuntimeError:

- error category;
- message;
- stack;
- source URL/module ID;
- line/column;
- runtime/session ID;
- transaction ID if inside commit;
- fatal/non-fatal classification.

Rust errors crossing into JS become typed Error instances with stable error codes. Do not expose internal Rust debug strings as an API contract.

A JS exception in one event handler must not corrupt the tree or leave the scheduler in a reentrant state. The engine reports it, closes the current JS call stack, drains or cancels pending work according to policy, then preserves the last committed tree.

## 10. Resource and lifecycle teardown

Surface disposal order:

1. Mark session as closing; reject new input and new host operations.
2. Cancel timers and scheduled frame callbacks.
3. Stop accepting resource completions or mark them stale.
4. Notify Solid root disposer/cleanup.
5. Unbind native event registrations.
6. Release callback registry and native JS roots.
7. Commit or discard pending host transaction according to shutdown policy.
8. Detach platform views/resources on the UI thread.
9. Release QuickJS contexts/runtime on the owning engine thread.
10. Release tree, layout caches and remaining resource scopes.
11. Publish final diagnostics and mark Surface disposed.

Every operation is idempotent where possible. Calling dispose twice must not double-free handles. No platform callback may enter a disposed runtime.

## 11. Testing matrix

Renderer tests:
- create element/text;
- initial mount;
- update text via signal;
- update properties through effects;
- insert before anchor;
- move sibling after detach;
- move across parents;
- remove subtree;
- nested keyed list updates;
- conditional mount/unmount;
- cleanup on disposal;
- invalid anchor/cycle handling.

Bridge tests:
- valid golden batch;
- wrong magic/version;
- truncated header;
- overflowing offset;
- wrong payload size;
- unknown opcode;
- duplicate ID;
- stale node ID;
- cross-surface ID;
- event binding to removed node;
- failed transaction leaves tree/revision unchanged;
- successful transaction increments once.

Runtime scheduler tests:
- Promise job after event callback;
- queueMicrotask ordering;
- nested signals/effects;
- nested batch;
- timeout cancellation;
- runtime shutdown with pending timer;
- infinite job fairness;
- Promise rejection reporting;
- repeated mount/unmount and GC.

End-to-end acceptance:
- compile a normal TSX Counter using Vite/Solid;
- run it inside QuickJS-NG;
- create the tree in Rust;
- click via normalized EngineEvent;
- verify Solid signal update;
- verify second typed host batch;
- verify Rust tree text content and revision;
- verify MountBatch contains only expected text mutation;
- verify no duplicate authoritative JS tree exists.

## 12. Current state and next implementation step

Current code already proves that official SolidJS Universal output can run in embedded QuickJS-NG for a synchronous counter interaction. It does not yet implement the Rust bridge described here.

The next milestone is therefore not Taffy or graphics. It is the M1 vertical slice:
- stable host IDs;
- host batch contract;
- native binary/typed transport;
- Rust decoder;
- atomic UiTree::apply_batch;
- acknowledgement and error path;
- event dispatch from engine into JS;
- second commit after Solid signal update;
- integration tests proving Rust tree is the authoritative tree.

Only after this slice is green should runtime async compatibility become the next major focus.
