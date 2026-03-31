import { reactRouter } from "@react-router/dev/vite";
import wasm from "vite-plugin-wasm";
import { defineConfig } from "vite";

export default defineConfig({
  plugins: [wasm(), reactRouter()],
});
