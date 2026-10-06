// Run a wasm32-wasip1 build of the heap probe under Node's WASI, for 32-bit heap numbers
// (4-byte pointers, as on the nRF52840). Usage: node run-wasi.mjs <probe.wasm>
import { readFile } from "node:fs/promises";
import { WASI } from "node:wasi";

const wasi = new WASI({ version: "preview1", args: [], env: {} });
const module = await WebAssembly.compile(await readFile(process.argv[2]));
const instance = await WebAssembly.instantiate(module, wasi.getImportObject());
wasi.start(instance);
