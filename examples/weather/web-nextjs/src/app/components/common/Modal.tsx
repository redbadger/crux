import type { ReactNode } from "react";

/**
 * A fixed overlay with a centred content slot. Used for the delete
 * confirmation dialog; no close-on-backdrop-click — the caller wires up
 * explicit cancel/confirm buttons inside `children`.
 */
export function Modal({ children }: { children: ReactNode }) {
  return (
    <div className="fixed inset-0 z-50 flex items-center justify-center px-4">
      <div className="absolute inset-0 bg-slate-900/50"></div>
      <div className="relative z-10 w-full max-w-sm">{children}</div>
    </div>
  );
}
