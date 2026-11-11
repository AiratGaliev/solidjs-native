export type HostValue = string | number | boolean | null;

export type HostOperation =
  | { kind: "create-element"; id: number; tag: string }
  | { kind: "create-text"; id: number; text: string }
  | { kind: "replace-text"; id: number; text: string }
  | { kind: "set-property"; id: number; name: string; value: HostValue }
  | { kind: "remove-property"; id: number; name: string }
  | { kind: "insert-node"; parent: number; child: number; anchor?: number }
  | { kind: "remove-node"; id: number }
  | { kind: "bind-event"; id: number; event: string; handlerId: number }
  | { kind: "unbind-event"; id: number; event: string }
  | { kind: "attach-resource"; id: number; name: string; resourceId: number }
  | { kind: "detach-resource"; id: number; name: string };

export type HostBatchAck = {
  ok: boolean;
  surfaceId: number;
  sessionGeneration: number;
  transactionId: number;
  revision: number;
  nextNodeId: number;
  error?: string;
};

type StringReference = { offset: number; length: number };

const HEADER_SIZE = 64;
const MAX_BATCH_BYTES = 4 * 1024 * 1024;
const MAX_OPERATIONS = 65_535;
const MAX_STRING_BYTES = 1024 * 1024;
const PROTOCOL_MAJOR = 1;
const PROTOCOL_MINOR = 0;

declare global {
  var __sn_host_config: { surfaceId: number; sessionGeneration: number } | undefined;
  var __sn_register_callback: ((callback: (event: unknown) => void) => number) | undefined;
  var __sn_discard_pending_callbacks: (() => void) | undefined;
  var __sn_apply_batch: ((bytes: Uint8Array) => HostBatchAck) | undefined;
  var __sn_dispose: (() => void) | undefined;
}

function encodeUtf8(value: string): Uint8Array {
  if (value.includes(String.fromCharCode(0))) throw new TypeError("Host protocol strings cannot contain NUL");
  const bytes: number[] = [];
  for (let index = 0; index < value.length; index += 1) {
    let codePoint = value.charCodeAt(index);
    if (codePoint >= 0xd800 && codePoint <= 0xdbff) {
      const low = value.charCodeAt(index + 1);
      if (!(low >= 0xdc00 && low <= 0xdfff)) throw new TypeError("String contains an unpaired high surrogate");
      codePoint = 0x10000 + ((codePoint - 0xd800) << 10) + (low - 0xdc00);
      index += 1;
    } else if (codePoint >= 0xdc00 && codePoint <= 0xdfff) {
      throw new TypeError("String contains an unpaired low surrogate");
    }

    if (codePoint <= 0x7f) {
      bytes.push(codePoint);
    } else if (codePoint <= 0x7ff) {
      bytes.push(0xc0 | (codePoint >> 6), 0x80 | (codePoint & 0x3f));
    } else if (codePoint <= 0xffff) {
      bytes.push(0xe0 | (codePoint >> 12), 0x80 | ((codePoint >> 6) & 0x3f), 0x80 | (codePoint & 0x3f));
    } else {
      bytes.push(0xf0 | (codePoint >> 18), 0x80 | ((codePoint >> 12) & 0x3f), 0x80 | ((codePoint >> 6) & 0x3f), 0x80 | (codePoint & 0x3f));
    }
  }
  if (bytes.length > MAX_STRING_BYTES) throw new RangeError("Host protocol string length is outside the allowed range");
  return Uint8Array.from(bytes);
}

class StringArena {
  private readonly bytes: number[] = [];

  add(value: string): StringReference {
    const encoded = encodeUtf8(value);
    const reference = { offset: this.bytes.length, length: encoded.length };
    if (reference.offset + reference.length > MAX_BATCH_BYTES) throw new RangeError("Host protocol string arena is too large");
    for (const byte of encoded) this.bytes.push(byte);
    return reference;
  }

  toUint8Array(): Uint8Array {
    return Uint8Array.from(this.bytes);
  }
}

function checkedId(value: number, label: string): bigint {
  if (!Number.isSafeInteger(value) || value <= 0) throw new RangeError(`${label} must be a positive safe integer`);
  return BigInt(value);
}

function checkedU32(value: number, label: string): number {
  if (!Number.isInteger(value) || value <= 0 || value > 0xffff_ffff) throw new RangeError(`${label} must fit in u32 and be non-zero`);
  return value;
}

function writeRef(view: DataView, offset: number, reference: StringReference): void {
  view.setUint32(offset, reference.offset, true);
  view.setUint32(offset + 4, reference.length, true);
}

function makeRecord(opcode: number, size: number, write: (view: DataView) => void): Uint8Array {
  const bytes = new Uint8Array(size);
  const view = new DataView(bytes.buffer);
  view.setUint8(0, opcode);
  view.setUint8(1, 0);
  view.setUint16(2, 0, true);
  view.setUint32(4, size, true);
  write(view);
  return bytes;
}

function encodeOperation(operation: HostOperation, strings: StringArena): Uint8Array {
  switch (operation.kind) {
    case "create-element": {
      const tag = strings.add(operation.tag);
      return makeRecord(1, 24, view => {
        view.setBigUint64(8, checkedId(operation.id, "NodeId"), true);
        writeRef(view, 16, tag);
      });
    }
    case "create-text": {
      const text = strings.add(operation.text);
      return makeRecord(2, 24, view => {
        view.setBigUint64(8, checkedId(operation.id, "NodeId"), true);
        writeRef(view, 16, text);
      });
    }
    case "replace-text": {
      const text = strings.add(operation.text);
      return makeRecord(3, 24, view => {
        view.setBigUint64(8, checkedId(operation.id, "NodeId"), true);
        writeRef(view, 16, text);
      });
    }
    case "set-property": {
      const name = strings.add(operation.name);
      let valueKind = 0;
      let valueBits = 0n;
      if (typeof operation.value === "boolean") {
        valueKind = 1;
        valueBits = operation.value ? 1n : 0n;
      } else if (typeof operation.value === "number") {
        if (!Number.isFinite(operation.value)) throw new TypeError("Host property numbers must be finite");
        valueKind = 2;
        const valueBuffer = new ArrayBuffer(8);
        new DataView(valueBuffer).setFloat64(0, operation.value, true);
        valueBits = new DataView(valueBuffer).getBigUint64(0, true);
      } else if (typeof operation.value === "string") {
        valueKind = 3;
        const value = strings.add(operation.value);
        valueBits = BigInt(value.offset) | (BigInt(value.length) << 32n);
      }
      return makeRecord(4, 40, view => {
        view.setBigUint64(8, checkedId(operation.id, "NodeId"), true);
        writeRef(view, 16, name);
        view.setUint8(24, valueKind);
        view.setBigUint64(32, valueBits, true);
      });
    }
    case "remove-property": {
      const name = strings.add(operation.name);
      return makeRecord(5, 24, view => {
        view.setBigUint64(8, checkedId(operation.id, "NodeId"), true);
        writeRef(view, 16, name);
      });
    }
    case "insert-node":
      return makeRecord(6, 32, view => {
        view.setBigUint64(8, checkedId(operation.parent, "Parent NodeId"), true);
        view.setBigUint64(16, checkedId(operation.child, "Child NodeId"), true);
        view.setBigUint64(24, operation.anchor === undefined ? 0n : checkedId(operation.anchor, "Anchor NodeId"), true);
      });
    case "remove-node":
      return makeRecord(7, 16, view => view.setBigUint64(8, checkedId(operation.id, "NodeId"), true));
    case "bind-event":
      return makeRecord(8, 32, view => {
        const event = strings.add(operation.event);
        view.setBigUint64(8, checkedId(operation.id, "NodeId"), true);
        writeRef(view, 16, event);
        view.setUint32(24, checkedU32(operation.handlerId, "HandlerId"), true);
        view.setUint32(28, 0, true);
      });
    case "unbind-event": {
      const event = strings.add(operation.event);
      return makeRecord(9, 24, view => {
        view.setBigUint64(8, checkedId(operation.id, "NodeId"), true);
        writeRef(view, 16, event);
      });
    }
    case "attach-resource": {
      const name = strings.add(operation.name);
      return makeRecord(10, 32, view => {
        view.setBigUint64(8, checkedId(operation.id, "NodeId"), true);
        writeRef(view, 16, name);
        view.setBigUint64(24, checkedId(operation.resourceId, "ResourceId"), true);
      });
    }
    case "detach-resource": {
      const name = strings.add(operation.name);
      return makeRecord(11, 24, view => {
        view.setBigUint64(8, checkedId(operation.id, "NodeId"), true);
        writeRef(view, 16, name);
      });
    }
  }
}

export class HostBatchEncoder {
  readonly surfaceId: number;
  readonly sessionGeneration: number;
  private revision = 0;
  private transactionId = 1;
  private nextNodeId = 1;
  private operations: HostOperation[] = [];
  private poisoned = false;
  private disposed = false;

  constructor(config = globalThis.__sn_host_config ?? { surfaceId: 1, sessionGeneration: 1 }) {
    checkedId(config.surfaceId, "SurfaceId");
    checkedId(config.sessionGeneration, "SessionGeneration");
    this.surfaceId = config.surfaceId;
    this.sessionGeneration = config.sessionGeneration;
  }

  createElement(tag: string): number {
    this.assertWritable();
    const id = this.allocateNodeId();
    this.operations.push({ kind: "create-element", id, tag });
    return id;
  }

  createText(text: string): number {
    this.assertWritable();
    const id = this.allocateNodeId();
    this.operations.push({ kind: "create-text", id, text });
    return id;
  }

  replaceText(id: number, text: string): void {
    this.assertWritable();
    this.operations.push({ kind: "replace-text", id, text });
  }

  setProperty(id: number, name: string, value: HostValue): void {
    this.assertWritable();
    this.operations.push({ kind: "set-property", id, name, value });
  }

  removeProperty(id: number, name: string): void {
    this.assertWritable();
    this.operations.push({ kind: "remove-property", id, name });
  }

  insertNode(parent: number, child: number, anchor?: number): void {
    this.assertWritable();
    this.operations.push({ kind: "insert-node", parent, child, anchor });
  }

  removeNode(id: number): void {
    this.assertWritable();
    this.operations.push({ kind: "remove-node", id });
  }

  bindEvent(id: number, event: string, handlerId: number): void {
    this.assertWritable();
    this.operations.push({ kind: "bind-event", id, event, handlerId });
  }

  unbindEvent(id: number, event: string): void {
    this.assertWritable();
    this.operations.push({ kind: "unbind-event", id, event });
  }

  attachResource(id: number, name: string, resourceId: number): void {
    this.assertWritable();
    this.operations.push({ kind: "attach-resource", id, name, resourceId });
  }

  detachResource(id: number, name: string): void {
    this.assertWritable();
    this.operations.push({ kind: "detach-resource", id, name });
  }

  registerCallback(callback: (event: unknown) => void): number {
    this.assertWritable();
    const register = globalThis.__sn_register_callback;
    if (!register) throw new Error("Native callback registry is not installed");
    return checkedU32(register(callback), "HandlerId");
  }

  encode(): Uint8Array {
    this.assertWritable();
    if (this.operations.length === 0) throw new Error("Cannot encode an empty host batch");
    if (this.operations.length > MAX_OPERATIONS) throw new RangeError("Host batch contains too many operations");

    const strings = new StringArena();
    const records = this.operations.map(operation => encodeOperation(operation, strings));
    const recordsLength = records.reduce((total, record) => total + record.length, 0);
    const stringBytes = strings.toUint8Array();
    const stringOffset = HEADER_SIZE + recordsLength;
    const totalLength = stringOffset + stringBytes.length;
    if (totalLength > MAX_BATCH_BYTES) throw new RangeError("Host batch exceeds the configured byte limit");

    const bytes = new Uint8Array(totalLength);
    bytes.set([0x53, 0x4e, 0x48, 0x42], 0); // SNHB
    const view = new DataView(bytes.buffer);
    view.setUint16(4, PROTOCOL_MAJOR, true);
    view.setUint16(6, PROTOCOL_MINOR, true);
    view.setUint16(8, HEADER_SIZE, true);
    view.setUint16(10, 0, true);
    view.setBigUint64(12, checkedId(this.surfaceId, "SurfaceId"), true);
    view.setBigUint64(20, checkedId(this.sessionGeneration, "SessionGeneration"), true);
    view.setBigUint64(28, checkedId(this.transactionId, "TransactionId"), true);
    view.setBigUint64(36, BigInt(this.revision), true);
    view.setUint32(44, records.length, true);
    view.setUint32(48, totalLength - HEADER_SIZE, true);
    view.setUint32(52, stringOffset, true);
    view.setUint32(56, stringBytes.length, true);
    view.setUint32(60, 0, true);

    let offset = HEADER_SIZE;
    for (const record of records) {
      bytes.set(record, offset);
      offset += record.length;
    }
    bytes.set(stringBytes, stringOffset);
    return bytes;
  }

  flush(): HostBatchAck | undefined {
    this.assertWritable();
    if (this.operations.length === 0) return undefined;
    const apply = globalThis.__sn_apply_batch;
    if (!apply) throw new Error("Native host batch binding is not installed");

    let bytes: Uint8Array;
    try {
      bytes = this.encode();
    } catch (error) {
      globalThis.__sn_discard_pending_callbacks?.();
      this.operations = [];
      this.poisoned = true;
      throw error;
    }

    let ack: HostBatchAck;
    try {
      ack = apply(bytes);
    } catch (error) {
      this.poisoned = true;
      this.operations = [];
      throw error;
    }

    if (!ack.ok || ack.surfaceId !== this.surfaceId || ack.sessionGeneration !== this.sessionGeneration ||
        ack.transactionId !== this.transactionId || ack.revision !== this.revision + 1) {
      this.poisoned = true;
      this.operations = [];
      throw new Error(ack.error ?? "Native host rejected or mis-acknowledged a batch");
    }

    this.revision = ack.revision;
    this.nextNodeId = ack.nextNodeId;
    this.transactionId += 1;
    this.operations = [];
    return ack;
  }

  dispose(): void {
    if (this.disposed) return;
    globalThis.__sn_dispose?.();
    this.operations = [];
    this.disposed = true;
    this.poisoned = true;
  }

  private allocateNodeId(): number {
    if (this.nextNodeId > Number.MAX_SAFE_INTEGER) throw new RangeError("NodeId space exceeds JavaScript's safe integer range");
    const id = this.nextNodeId;
    this.nextNodeId += 1;
    return id;
  }

  private assertWritable(): void {
    if (this.disposed) throw new Error("Host session has been disposed");
    if (this.poisoned) throw new Error("Host adapter is poisoned after a rejected transaction; dispose and remount the Surface");
  }
}
