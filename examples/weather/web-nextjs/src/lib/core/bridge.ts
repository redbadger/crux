import { CoreFfi } from "shared";
import type { CoreBridge } from "shared_types/app";

/// The generated `CoreBridge`, implemented over the wasm bindings: bytes in,
/// bytes out, nothing else. This is the only file in the shell that knows the
/// Rust core exists — swap it for a fake and the rest is unchanged.
export class LiveBridge implements CoreBridge {
  private readonly ffi = CoreFfi.new();

  update(event: Uint8Array): Uint8Array {
    return this.ffi.update(event);
  }

  resolve(id: number, output: Uint8Array): Uint8Array {
    return this.ffi.resolve(id, output);
  }

  view(): Uint8Array {
    return this.ffi.view();
  }
}
