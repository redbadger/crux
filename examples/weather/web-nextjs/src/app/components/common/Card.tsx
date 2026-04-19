import clsx from "clsx";
import type { ReactNode } from "react";

/**
 * A rounded white panel with a soft shadow. The main layout container for
 * screen content — every screen stacks one or more cards.
 */
export function Card({
  children,
  className,
}: {
  children: ReactNode;
  className?: string;
}) {
  return (
    <div className={clsx("bg-white rounded-2xl shadow-lg p-6", className)}>
      {children}
    </div>
  );
}
