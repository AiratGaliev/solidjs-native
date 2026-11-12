import { createRenderer } from "solid-js/universal";
import { assertHostValue, HostBatchEncoder, type HostBatchAck } from "./protocol";

declare global {
  var __sn_flush_host_batch: (() => HostBatchAck | undefined) | undefined;
  var __sn_register_session_cleanup: ((callback: () => void) => number) | undefined;
  var __sn_unregister_session_cleanup: ((id: number) => boolean) | undefined;
}

export type NativeNode = {
  surfaceId: number;
  sessionGeneration: number;
  nodeId: number;
  kind: "element" | "text";
  tag?: string;
  parent: NativeNode | null;
  children: NativeNode[];
  eventHandlerIds: Map<string, number>;
};

const encoder = new HostBatchEncoder();

function assertCurrentHandle(node: NativeNode, role: string): void {
  if (node.surfaceId !== encoder.surfaceId || node.sessionGeneration !== encoder.sessionGeneration) {
    throw new TypeError(`${role} belongs to a different or stale Surface session`);
  }
  if (!Number.isSafeInteger(node.nodeId) || node.nodeId <= 0) {
    throw new TypeError(`${role} has an invalid NodeId`);
  }
}

function assertElement(node: NativeNode, role: string): void {
  assertCurrentHandle(node, role);
  if (node.kind !== "element") throw new TypeError(`${role} must be an element node`);
}

function eventName(propertyName: string): string | undefined {
  if (!/^on[A-Z]/.test(propertyName)) return undefined;
  return propertyName.slice(2, 3).toLowerCase() + propertyName.slice(3);
}

function isDescendant(candidate: NativeNode, ancestor: NativeNode): boolean {
  let current = candidate.parent;
  const visited = new Set<NativeNode>();
  while (current) {
    if (current === ancestor) return true;
    if (visited.has(current)) throw new Error("NativeNode parent topology contains a cycle");
    visited.add(current);
    current = current.parent;
  }
  return false;
}

function clearLocalSubtree(node: NativeNode): void {
  const pending = [node];
  const visited = new Set<NativeNode>();
  while (pending.length > 0) {
    const current = pending.pop()!;
    if (visited.has(current)) continue;
    visited.add(current);
    for (const child of current.children) pending.push(child);
    current.eventHandlerIds.clear();
    current.parent = null;
    current.children = [];
  }
}

const renderer = createRenderer<NativeNode>({
  createElement(tag) {
    const nodeId = encoder.createElement(tag);
    return { surfaceId: encoder.surfaceId, sessionGeneration: encoder.sessionGeneration, nodeId, kind: "element", tag, parent: null, children: [], eventHandlerIds: new Map() };
  },
  createTextNode(value) {
    const nodeId = encoder.createText(value);
    return { surfaceId: encoder.surfaceId, sessionGeneration: encoder.sessionGeneration, nodeId, kind: "text", parent: null, children: [], eventHandlerIds: new Map() };
  },
  replaceText(node, value) {
    assertCurrentHandle(node, "replaceText target");
    if (node.kind !== "text") throw new TypeError("replaceText target must be a text node");
    encoder.replaceText(node.nodeId, value);
  },
  setProperty(node, name, value) {
    assertElement(node, "setProperty target");
    const event = eventName(name);
    if (event !== undefined) {
      const previous = node.eventHandlerIds.get(event);
      if (typeof value === "function") {
        const handlerId = encoder.registerCallback(value as (event: unknown) => void);
        node.eventHandlerIds.set(event, handlerId);
        encoder.bindEvent(node.nodeId, event, handlerId);
      } else if (value === undefined || value === null) {
        if (previous !== undefined) {
          node.eventHandlerIds.delete(event);
          encoder.unbindEvent(node.nodeId, event);
        }
      } else {
        throw new TypeError(`Event property ${name} must be a callback or null`);
      }
      return;
    }

    if (value === undefined) {
      encoder.removeProperty(node.nodeId, name);
      return;
    }
    assertHostValue(value, name);
    const hostValue = value;
    encoder.setProperty(node.nodeId, name, hostValue);
  },
  insertNode(parent, node, anchor) {
    assertElement(parent, "insert parent");
    assertCurrentHandle(node, "insert child");
    if (anchor) assertCurrentHandle(anchor, "insert anchor");
    if (parent === node || isDescendant(parent, node)) throw new TypeError("Cannot insert a node into itself or its descendant");
    if (anchor && (anchor.parent !== parent || !parent.children.includes(anchor))) {
      throw new TypeError("Insert anchor must be a child of the target parent");
    }
    if (anchor === node) return;

    if (node.parent) {
      const previousIndex = node.parent.children.indexOf(node);
      if (previousIndex < 0) throw new Error("NativeNode parent/children topology is inconsistent");
      node.parent.children.splice(previousIndex, 1);
    }
    const index = anchor ? parent.children.indexOf(anchor) : -1;
    if (index < 0) parent.children.push(node);
    else parent.children.splice(index, 0, node);
    node.parent = parent;
    encoder.insertNode(parent.nodeId, node.nodeId, anchor?.nodeId);
  },
  isTextNode(node) {
    return node.kind === "text";
  },
  removeNode(parent, node) {
    assertElement(parent, "remove parent");
    assertCurrentHandle(node, "removed node");
    if (node.parent !== parent) throw new Error("Removed node is not a child of the specified parent");
    const index = parent.children.indexOf(node);
    if (index < 0) throw new Error("NativeNode parent/children topology is inconsistent");
    parent.children.splice(index, 1);
    encoder.removeNode(node.nodeId);
    clearLocalSubtree(node);
  },
  getParentNode(node) {
    return node.parent ?? undefined;
  },
  getFirstChild(node) {
    return node.children[0] ?? undefined;
  },
  getNextSibling(node) {
    if (!node.parent) return undefined;
    const index = node.parent.children.indexOf(node);
    if (index < 0) throw new Error("NativeNode parent/children topology is inconsistent");
    return node.parent.children[index + 1] ?? undefined;
  },
});

export const {
  effect, memo, createComponent, createElement, createTextNode,
  insertNode, insert, spread, setProp, mergeProps, use,
} = renderer;

export function flushHostBatch(): HostBatchAck | undefined {
  return encoder.flush();
}

globalThis.__sn_flush_host_batch = flushHostBatch;

export function disposeHostSession(): void {
  encoder.dispose();
}

export function render(code: () => unknown, root: NativeNode) {
  if (root.kind !== "element") throw new TypeError("Render root must be an element node");
  if (!Number.isSafeInteger(root.nodeId) || root.nodeId <= 0) {
    root.surfaceId = encoder.surfaceId;
    root.sessionGeneration = encoder.sessionGeneration;
    root.nodeId = encoder.createElement(root.tag ?? "root");
  }
  assertElement(root, "render root");
  let dispose: (() => void) | undefined;
  try {
    dispose = renderer.render(() => code() as NativeNode, root);
    encoder.flush();
  } catch (error) {
    try {
      dispose?.();
    } catch {
      // Preserve the original mount/commit failure; the adapter is poisoned below.
      // Its owner must dispose and remount this Surface before further use.
    }
    clearLocalSubtree(root);
    encoder.poison();
    throw error;
  }

  let disposed = false;
  let cleanupId: number | undefined;
  const cleanup = () => {
    if (disposed) return;
    disposed = true;
    if (cleanupId !== undefined) {
      globalThis.__sn_unregister_session_cleanup?.(cleanupId);
    }

    try {
      dispose?.();
    } catch (error) {
      try {
        encoder.abort();
      } finally {
        clearLocalSubtree(root);
      }
      throw error;
    }

    try {
      encoder.removeNode(root.nodeId);
      encoder.flush();
    } catch (error) {
      try {
        encoder.abort();
      } finally {
        clearLocalSubtree(root);
      }
      throw error;
    }
    clearLocalSubtree(root);
  };

  try {
    cleanupId = globalThis.__sn_register_session_cleanup?.(cleanup);
  } catch (error) {
    try {
      cleanup();
    } catch {
      // Preserve the cleanup-registration failure; the disposer has already
      // attempted to release the local Solid root and encoder transaction.
    }
    throw error;
  }
  return cleanup;
}

export function View(props: { children?: unknown; onClick?: (event: unknown) => void }) {
  const node = createElement("view");
  if (props.onClick) setProp(node, "onClick", props.onClick);
  insert(node, () => props.children);
  return node;
}

export function Text(props: { children?: unknown }) {
  const node = createElement("text");
  insert(node, () => props.children);
  return node;
}

export function Button(props: { children?: unknown; onClick?: (event: unknown) => void }) {
  const node = createElement("button");
  if (props.onClick) setProp(node, "onClick", props.onClick);
  insert(node, () => props.children);
  return node;
}
