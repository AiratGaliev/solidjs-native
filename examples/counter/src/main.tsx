import { createSignal } from "solid-js";
import { render, View, Text, Button, type NativeNode } from "solid-native";

function App() {
  const [count, setCount] = createSignal(0);
  return (
    <View>
      <Text>SolidJS Native renderer smoke test</Text>
      <Text>Count: {count()}</Text>
      <Button onClick={() => setCount((value) => value + 1)}>Increment</Button>
    </View>
  );
}

function createRoot(): NativeNode {
  return {
    surfaceId: 0,
    sessionGeneration: 0,
    nodeId: 0,
    kind: "element",
    tag: "root",
    parent: null,
    children: [],
    properties: {},
    eventHandlerIds: new Map(),
  };
}

(globalThis as typeof globalThis & {
  __sn_demo_dispose?: () => void;
  __sn_mount_counter?: () => () => void;
}).__sn_mount_counter = () => render(() => <App />, createRoot());

(globalThis as typeof globalThis & {
  __sn_demo_dispose?: () => void;
}).__sn_demo_dispose = (globalThis as typeof globalThis & {
  __sn_mount_counter: () => () => void;
}).__sn_mount_counter();
