import { reactRouter } from "@react-router/dev/vite";
import wasm from "vite-plugin-wasm";
import { defineConfig } from "vite";

export default defineConfig({
  plugins: [wasm(), reactRouter()],
  ssr: {
    // The server loads `shared` from node_modules, because its Node.js glue
    // reads the .wasm file next to it at run time.
    external: ["shared"],
    // The generated `shared_types` package holds CommonJS modules, so Vite
    // bundles it into the server build.
    noExternal: ["shared_types"],
  },
});
