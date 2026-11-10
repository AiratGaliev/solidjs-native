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

const root: NativeNode = {
  kind: "element",
  tag: "root",
  parent: null,
  children: [],
  properties: {},
};

render(() => <App />, root);
(globalThis as typeof globalThis & { __sn_demo_root?: NativeNode }).__sn_demo_root = root;
