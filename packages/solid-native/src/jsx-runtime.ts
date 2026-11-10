import type { NativeNode } from "./host";

export namespace JSX {
  export type Element = NativeNode | string | number | boolean | null | undefined | Element[];
  export interface ElementChildrenAttribute { children: {}; }
  export interface IntrinsicElements {
    [elementName: string]: Record<string, unknown>;
  }
}
