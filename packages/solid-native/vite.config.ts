import { fileURLToPath } from "node:url";
import { defineConfig } from "vite";
import solid from "vite-plugin-solid";

const exampleRoot = fileURLToPath(new URL("../../examples/counter", import.meta.url));
const outputDir = fileURLToPath(new URL("../../dist", import.meta.url));
const rendererEntry = fileURLToPath(new URL("./src/index.ts", import.meta.url));

export default defineConfig({
  root: exampleRoot,
  resolve: { alias: { "solid-native": rendererEntry } },
  plugins: [solid({ solid: { moduleName: "solid-native", generate: "universal" } })],
  build: {
    outDir: outputDir,
    emptyOutDir: true,
    lib: {
      entry: "src/main.tsx",
      name: "SolidNativeDemo",
      formats: ["iife"],
      fileName: () => "solid-native-demo.js",
    },
  },
});
