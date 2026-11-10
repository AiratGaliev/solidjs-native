import { createRenderer } from "solid-js/universal";

export type NativeNode = {
  kind: "element" | "text";
  tag?: string;
  text?: string;
  parent: NativeNode | null;
  children: NativeNode[];
  properties: Record<string, unknown>;
};

declare global {
  var __sn_emit: ((operation: string, payload: unknown) => void) | undefined;
}

function emit(operation: string, payload: unknown) {
  globalThis.__sn_emit?.(operation, payload);
}

const renderer = createRenderer<NativeNode>({
  createElement(tag) {
    const node: NativeNode = { kind: "element", tag, parent: null, children: [], properties: {} };
    emit("create-element", { tag });
    return node;
  },
  createTextNode(value) {
    const node: NativeNode = { kind: "text", text: value, parent: null, children: [], properties: {} };
    emit("create-text", { value });
    return node;
  },
  replaceText(node, value) {
    node.text = value;
    emit("replace-text", { value });
  },
  setProperty(node, name, value) {
    node.properties[name] = value;
    emit("set-property", { name, value: typeof value === "function" ? "[Function]" : value });
  },
  insertNode(parent, node, anchor) {
    if (node.parent) {
      const oldIndex = node.parent.children.indexOf(node);
      if (oldIndex >= 0) node.parent.children.splice(oldIndex, 1);
    }
    const index = anchor ? parent.children.indexOf(anchor) : -1;
    if (index < 0) parent.children.push(node);
    else parent.children.splice(index, 0, node);
    node.parent = parent;
    emit("insert-node", { parent: parent.tag ?? parent.kind, child: node.tag ?? node.kind });
  },
  isTextNode(node) {
    return node.kind === "text";
  },
  removeNode(parent, node) {
    const index = parent.children.indexOf(node);
    if (index >= 0) parent.children.splice(index, 1);
    node.parent = null;
    emit("remove-node", { child: node.tag ?? node.kind });
  },
  getParentNode(node) { return node.parent ?? undefined; },
  getFirstChild(node) { return node.children[0] ?? undefined; },
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

export function render(code: () => unknown, root: NativeNode) {
  return renderer.render(() => code() as NativeNode, root);
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
