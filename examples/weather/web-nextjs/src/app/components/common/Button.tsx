import clsx from "clsx";
import type { Icon as PhosphorIcon } from "@phosphor-icons/react";

export type ButtonVariant = "primary" | "secondary" | "danger";

const VARIANT_CLASSES: Record<ButtonVariant, string> = {
  primary: "bg-sky-600 text-white hover:bg-sky-700 focus-visible:ring-sky-300",
  secondary:
    "bg-slate-100 text-slate-800 hover:bg-slate-200 focus-visible:ring-slate-300",
  danger: "bg-red-600 text-white hover:bg-red-700 focus-visible:ring-red-300",
};

/**
 * A labelled action button. Mirrors the Leptos `Button` — same variant set,
 * same Tailwind classes, same behaviour.
 */
export function Button({
  label,
  onClick,
  variant = "primary",
  icon: Icon,
  fullWidth = false,
  enabled = true,
}: {
  label: string;
  onClick: () => void;
  variant?: ButtonVariant;
  icon?: PhosphorIcon;
  fullWidth?: boolean;
  enabled?: boolean;
}) {
  return (
    <button
      type="button"
      className={clsx(
        "inline-flex items-center justify-center gap-2 rounded-lg px-5 py-2.5",
        "text-sm font-semibold transition-colors duration-150",
        "focus-visible:outline-none focus-visible:ring-2",
        "disabled:cursor-not-allowed disabled:opacity-50",
        fullWidth && "w-full",
        VARIANT_CLASSES[variant],
      )}
      disabled={!enabled}
      onClick={onClick}
    >
      {Icon ? <Icon size={18} /> : null}
      <span>{label}</span>
    </button>
  );
}
