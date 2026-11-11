import assert from "node:assert/strict";
import test from "node:test";
import { assertHostValue, HostBatchEncoder } from "../packages/solid-native/src/protocol.ts";

function viewOf(bytes) {
  return new DataView(bytes.buffer, bytes.byteOffset, bytes.byteLength);
}

test("encodes the version 1 CreateElement golden fixture", () => {
  const encoder = new HostBatchEncoder({ surfaceId: 1, sessionGeneration: 1 });
  encoder.createElement("root");
  const bytes = encoder.encode();
  const view = viewOf(bytes);

  assert.equal(bytes.length, 92);
  assert.deepEqual(Array.from(bytes.slice(0, 4)), [0x53, 0x4e, 0x48, 0x42]);
  assert.equal(view.getUint16(4, true), 1);
  assert.equal(view.getUint16(6, true), 0);
  assert.equal(view.getUint16(8, true), 64);
  assert.equal(view.getBigUint64(12, true), 1n);
  assert.equal(view.getBigUint64(20, true), 1n);
  assert.equal(view.getBigUint64(28, true), 1n);
  assert.equal(view.getBigUint64(36, true), 0n);
  assert.equal(view.getUint32(44, true), 1);
  assert.equal(view.getUint32(48, true), 28);
  assert.equal(view.getUint32(52, true), 88);
  assert.equal(view.getUint32(56, true), 4);

  assert.equal(view.getUint8(64), 1);
  assert.equal(view.getUint32(68, true), 24);
  assert.equal(view.getBigUint64(72, true), 1n);
  assert.equal(view.getUint32(80, true), 0);
  assert.equal(view.getUint32(84, true), 4);
  assert.equal(new TextDecoder().decode(bytes.slice(88)), "root");
});

test("writes string references into one UTF-8 arena and preserves empty text", () => {
  const encoder = new HostBatchEncoder({ surfaceId: 2, sessionGeneration: 3 });
  encoder.createText("Привет🙂");
  encoder.replaceText(1, "");
  const bytes = encoder.encode();
  const view = viewOf(bytes);

  assert.equal(view.getUint32(44, true), 2);
  const stringOffset = view.getUint32(52, true);
  const stringLength = view.getUint32(56, true);
  const arena = bytes.slice(stringOffset, stringOffset + stringLength);
  const textRefOffset = view.getUint32(80, true);
  const textRefLength = view.getUint32(84, true);
  const emptyRefOffset = view.getUint32(104, true);
  const emptyRefLength = view.getUint32(108, true);

  assert.equal(new TextDecoder().decode(arena.slice(textRefOffset, textRefOffset + textRefLength)), "Привет🙂");
  assert.equal(emptyRefLength, 0);
  assert.equal(emptyRefOffset, textRefLength);
});

test("rejects unsupported non-finite property values before transport", () => {
  const encoder = new HostBatchEncoder();
  encoder.setProperty(1, "opacity", Number.NaN);
  assert.throws(() => encoder.encode(), /finite/);
});

test("rejects unpaired UTF-16 surrogate input", () => {
  const encoder = new HostBatchEncoder();
  encoder.createText(String.fromCharCode(0xd800));
  assert.throws(() => encoder.encode(), /unpaired high surrogate/);
});

test("rejects NUL in protocol strings before crossing the host boundary", () => {
  const encoder = new HostBatchEncoder();
  encoder.createElement(`root${String.fromCharCode(0)}unsafe`);
  assert.throws(() => encoder.encode(), /cannot contain NUL/);
});

test("rejects unpaired low UTF-16 surrogate input", () => {
  const encoder = new HostBatchEncoder();
  encoder.createText(String.fromCharCode(0xdc00));
  assert.throws(() => encoder.encode(), /unpaired low surrogate/);
});

test("accepts a string exactly at the 1 MiB UTF-8 limit", () => {
  const encoder = new HostBatchEncoder();
  encoder.createText("🙂".repeat(256 * 1024));
  const bytes = encoder.encode();
  assert.equal(bytes.length, 64 + 24 + 1024 * 1024);
});

test("rejects a string above the 1 MiB UTF-8 limit during encoding", () => {
  const encoder = new HostBatchEncoder();
  encoder.createText("🙂".repeat(256 * 1024 + 1));
  assert.throws(() => encoder.encode(), /string length is outside the allowed range/);
});


test("poisons the JS adapter and releases uncommitted callback roots after encode failure", () => {
  const previousRegister = globalThis.__sn_register_callback;
  const previousDiscard = globalThis.__sn_discard_pending_callbacks;
  const previousDispose = globalThis.__sn_dispose;
  const previousApply = globalThis.__sn_apply_batch;
  let discarded = 0;
  let disposed = 0;
  globalThis.__sn_register_callback = () => 7;
  globalThis.__sn_discard_pending_callbacks = () => { discarded += 1; };
  globalThis.__sn_dispose = () => { disposed += 1; };
  globalThis.__sn_apply_batch = () => { throw new Error("Native batch binding must not run after encode failure"); };

  try {
    const encoder = new HostBatchEncoder({ surfaceId: 1, sessionGeneration: 1 });
    const handlerId = encoder.registerCallback(() => {});
    encoder.bindEvent(1, String.fromCharCode(0xd800), handlerId);

    assert.throws(() => encoder.flush(), /unpaired high surrogate/);
    assert.equal(discarded, 1);
    assert.equal(disposed, 0);
    assert.throws(() => encoder.createElement("root"), /poisoned/);
  } finally {
    if (previousRegister === undefined) delete globalThis.__sn_register_callback;
    else globalThis.__sn_register_callback = previousRegister;
    if (previousDiscard === undefined) delete globalThis.__sn_discard_pending_callbacks;
    else globalThis.__sn_discard_pending_callbacks = previousDiscard;
    if (previousDispose === undefined) delete globalThis.__sn_dispose;
    else globalThis.__sn_dispose = previousDispose;
    if (previousApply === undefined) delete globalThis.__sn_apply_batch;
    else globalThis.__sn_apply_batch = previousApply;
  }
});


test("poisons the JS adapter but preserves the committed Surface when a batch is rejected", () => {
  const previousDiscard = globalThis.__sn_discard_pending_callbacks;
  const previousDispose = globalThis.__sn_dispose;
  const previousApply = globalThis.__sn_apply_batch;
  let discarded = 0;
  let disposed = 0;
  globalThis.__sn_discard_pending_callbacks = () => { discarded += 1; };
  globalThis.__sn_dispose = () => { disposed += 1; };
  globalThis.__sn_apply_batch = () => { throw new Error("native rejection"); };

  try {
    const encoder = new HostBatchEncoder({ surfaceId: 1, sessionGeneration: 1 });
    encoder.createElement("root");
    assert.throws(() => encoder.flush(), /native rejection/);
    assert.equal(discarded, 1);
    assert.equal(disposed, 0);
    assert.throws(() => encoder.createElement("after-failure"), /poisoned/);
  } finally {
    if (previousDiscard === undefined) delete globalThis.__sn_discard_pending_callbacks;
    else globalThis.__sn_discard_pending_callbacks = previousDiscard;
    if (previousDispose === undefined) delete globalThis.__sn_dispose;
    else globalThis.__sn_dispose = previousDispose;
    if (previousApply === undefined) delete globalThis.__sn_apply_batch;
    else globalThis.__sn_apply_batch = previousApply;
  }
});

test("aborts the Surface when the native acknowledgement cannot be trusted", () => {
  const previousDiscard = globalThis.__sn_discard_pending_callbacks;
  const previousDispose = globalThis.__sn_dispose;
  const previousApply = globalThis.__sn_apply_batch;
  let discarded = 0;
  let disposed = 0;
  globalThis.__sn_discard_pending_callbacks = () => { discarded += 1; };
  globalThis.__sn_dispose = () => { disposed += 1; };
  globalThis.__sn_apply_batch = () => ({
    ok: true,
    surfaceId: 1,
    sessionGeneration: 1,
    transactionId: 1,
    revision: 0,
    nextNodeId: 2,
  });

  try {
    const encoder = new HostBatchEncoder({ surfaceId: 1, sessionGeneration: 1 });
    encoder.createElement("root");
    assert.throws(() => encoder.flush(), /mis-acknowledged/);
    assert.equal(discarded, 1);
    assert.equal(disposed, 1);
    assert.throws(() => encoder.createElement("after-invalid-ack"), /disposed/);
  } finally {
    if (previousDiscard === undefined) delete globalThis.__sn_discard_pending_callbacks;
    else globalThis.__sn_discard_pending_callbacks = previousDiscard;
    if (previousDispose === undefined) delete globalThis.__sn_dispose;
    else globalThis.__sn_dispose = previousDispose;
    if (previousApply === undefined) delete globalThis.__sn_apply_batch;
    else globalThis.__sn_apply_batch = previousApply;
  }
});

test("aborts a failed mount session and releases pending callbacks without applying a batch", () => {
  const previousRegister = globalThis.__sn_register_callback;
  const previousDiscard = globalThis.__sn_discard_pending_callbacks;
  const previousDispose = globalThis.__sn_dispose;
  const previousApply = globalThis.__sn_apply_batch;
  let discarded = 0;
  let disposed = 0;
  let applied = 0;
  globalThis.__sn_register_callback = () => 8;
  globalThis.__sn_discard_pending_callbacks = () => { discarded += 1; };
  globalThis.__sn_dispose = () => { disposed += 1; };
  globalThis.__sn_apply_batch = () => { applied += 1; throw new Error("Aborted batch must not be applied"); };

  try {
    const encoder = new HostBatchEncoder({ surfaceId: 1, sessionGeneration: 1 });
    encoder.createElement("root");
    encoder.registerCallback(() => {});
    encoder.abort();

    assert.equal(discarded, 1);
    assert.equal(disposed, 1);
    assert.equal(applied, 0);
    assert.throws(() => encoder.flush(), /disposed/);
  } finally {
    if (previousRegister === undefined) delete globalThis.__sn_register_callback;
    else globalThis.__sn_register_callback = previousRegister;
    if (previousDiscard === undefined) delete globalThis.__sn_discard_pending_callbacks;
    else globalThis.__sn_discard_pending_callbacks = previousDiscard;
    if (previousDispose === undefined) delete globalThis.__sn_dispose;
    else globalThis.__sn_dispose = previousDispose;
    if (previousApply === undefined) delete globalThis.__sn_apply_batch;
    else globalThis.__sn_apply_batch = previousApply;
  }
});

test("accepts supported primitive host values and rejects symbols, bigints and objects", () => {
  for (const value of ["text", 42, true, false, null]) {
    assert.doesNotThrow(() => assertHostValue(value, "data"));
  }
  assert.throws(() => assertHostValue(Symbol("unsupported"), "data"), /supported primitive value/);
  assert.throws(() => assertHostValue(1n, "data"), /supported primitive value/);
  assert.throws(() => assertHostValue({ nested: true }, "data"), /supported primitive value/);
  assert.throws(() => assertHostValue(() => undefined, "data"), /supported primitive value/);
});
