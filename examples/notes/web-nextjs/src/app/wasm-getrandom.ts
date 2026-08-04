// WORKAROUND: automerge uses `features = ["wasm"]` in its Cargo.toml, which
// enables the `getrandom/js` feature. That compiles a wasm-bindgen import
// (`__wbg_getRandomValues_<hash>`) into the WASM binary so that automerge can
// call `crypto.getRandomValues` to generate random actor IDs.
//
// boltffi deliberately stubs ALL `__wbindgen_placeholder__` imports as
// "Unimplemented" — it replaces wasm-bindgen interop entirely and doesn't
// provide browser API bindings. There is currently no way to inject custom
// `__wbindgen_placeholder__` implementations through the `instantiateBoltFFI`
// API, so we intercept `WebAssembly.instantiate` to replace the stub with the
// real `crypto.getRandomValues` before the module is loaded.
//
// This lives in its own module, imported for its side effect *before* `shared`,
// because ES module imports are hoisted and evaluated in source order: patching
// inline in a module that also imports `shared` would install the patch only
// after `shared` had already been evaluated.
//
// Only `WebAssembly.instantiate` is patched, which is what boltffi's loader
// calls. If that ever changes to `instantiateStreaming`, the stub returns to
// throwing "Unimplemented" as soon as automerge needs randomness.
//
// The import name ends in a hash of its wasm-bindgen signature, which changes
// whenever `getrandom` is bumped (0.4.2 and 0.4.3 differ), so we match on the
// `__wbg_getRandomValues_` prefix rather than pinning one hash — pinning meant
// a lockfile refresh silently reverted the stub to throwing. To see the
// current imports:
//   node -e "const fs=require('fs'); WebAssembly.compile(fs.readFileSync('generated/pkg/shared_bg.wasm')).then(m=>console.log(WebAssembly.Module.imports(m).map(i=>i.module+'::'+i.name).join('\n')))"
//
// The proper fix is either:
//   a) Remove `features = ["wasm"]` from the `automerge` dependency in
//      shared/Cargo.toml and configure `getrandom` with `features = ["custom"]`
//      plus a boltffi-compatible random implementation, or
//   b) Ask boltffi to natively provide `crypto.getRandomValues` for the
//      `__wbindgen_placeholder__` namespace (feature request to boltffi).
if (typeof WebAssembly !== "undefined" && typeof crypto !== "undefined") {
  const origInstantiate = WebAssembly.instantiate.bind(WebAssembly);

  // Memory of the most recently instantiated module. `getRandomValues` is only
  // reachable through that module's own exports, and the shell instantiates a
  // single WASM module, so this is the one automerge is calling from.
  let memory: WebAssembly.Memory | null = null;

  (WebAssembly as any).instantiate = (
    source: BufferSource | WebAssembly.Module,
    importObject?: WebAssembly.Imports,
  ) => {
    if (importObject?.["__wbindgen_placeholder__"]) {
      const stubs = importObject["__wbindgen_placeholder__"] as object;
      importObject["__wbindgen_placeholder__"] = new Proxy(stubs, {
        get(target, prop) {
          if (
            typeof prop === "string" &&
            prop.startsWith("__wbg_getRandomValues_")
          ) {
            return (ptr: number, len: number) => {
              crypto.getRandomValues(new Uint8Array(memory!.buffer, ptr, len));
            };
          }
          return Reflect.get(target, prop);
        },
      }) as WebAssembly.ModuleImports;
    }
    return (
      origInstantiate(
        source as any,
        importObject,
      ) as Promise<WebAssembly.WebAssemblyInstantiatedSource>
    ).then((result) => {
      memory = result.instance.exports["memory"] as WebAssembly.Memory;
      return result;
    });
  };
}
