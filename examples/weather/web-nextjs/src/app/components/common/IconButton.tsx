import clsx from "clsx";
import type { Icon as PhosphorIcon } from "@phosphor-icons/react";

export type IconButtonVariant = "default" | "danger";

const VARIANT_CLASSES: Record<IconButtonVariant, string> = {
  default:
    "text-slate-600 hover:bg-slate-100 focus-visible:ring-slate-300",
  danger: "text-red-600 hover:bg-red-50 focus-visible:ring-red-300",
};

/**
 * A small square button with a single icon and no label — list-row actions,
 * close buttons, and the like.
 */
export function IconButton({
  icon: Icon,
  onClick,
  variant = "default",
  ariaLabel = "",
}: {
  icon: PhosphorIcon;
  onClick: () => void;
  variant?: IconButtonVariant;
  ariaLabel?: string;
}) {
  return (
    <button
      type="button"
      className={clsx(
        "inline-flex items-center justify-center h-9 w-9 rounded-lg",
        "transition-colors duration-150 focus-visible:outline-none",
        "focus-visible:ring-2",
        VARIANT_CLASSES[variant],
      )}
      aria-label={ariaLabel}
      onClick={onClick}
    >
      <Icon size={18} />
    </button>
  );
}
