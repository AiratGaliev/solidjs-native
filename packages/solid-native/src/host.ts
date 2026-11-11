import { createRenderer } from "solid-js/universal";
import { HostBatchEncoder, type HostBatchAck, type HostValue } from "./protocol";

declare global {
  var __sn_flush_host_batch: (() => HostBatchAck | undefined) | undefined;
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
  while (current) {
    if (current === ancestor) return true;
    current = current.parent;
  }
  return false;
}

function clearLocalSubtree(node: NativeNode): void {
  for (const child of node.children) clearLocalSubtree(child);
  node.eventHandlerIds.clear();
  node.parent = null;
  node.children = [];
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
    if (
      value !== null &&
      typeof value !== "string" &&
      typeof value !== "number" &&
      typeof value !== "boolean"
    ) {
      throw new TypeError(`Property ${name} must use a supported primitive value`);
    }
    const hostValue = value as HostValue;
    encoder.setProperty(node.nodeId, name, hostValue);
  },
  insertNode(parent, node, anchor) {
    assertElement(parent, "insert parent");
    assertCurrentHandle(node, "insert child");
    if (anchor) assertCurrentHandle(anchor, "insert anchor");
    if (parent === node || isDescendant(parent, node)) throw new TypeError("Cannot insert a node into itself or its descendant");
    if (anchor && anchor.parent !== parent) throw new TypeError("Insert anchor must be a child of the target parent");
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
  if (!Number.isSafeInteger(root.nodeId) || root.nodeId <= 0) {
    root.surfaceId = encoder.surfaceId;
    root.sessionGeneration = encoder.sessionGeneration;
    root.nodeId = encoder.createElement(root.tag ?? "root");
  }
  assertElement(root, "render root");
  const dispose = renderer.render(() => code() as NativeNode, root);
  encoder.flush();
  let disposed = false;
  return () => {
    if (disposed) return;
    disposed = true;
    dispose();
    encoder.removeNode(root.nodeId);
    encoder.flush();
  };
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
