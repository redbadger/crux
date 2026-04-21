import clsx from "clsx";
import type { Icon as PhosphorIcon } from "@phosphor-icons/react";

export type StatusTone = "neutral" | "error";

const ICON_CLASSES: Record<StatusTone, string> = {
  neutral: "text-slate-400",
  error: "text-red-500",
};

const TEXT_CLASSES: Record<StatusTone, string> = {
  neutral: "text-slate-600",
  error: "text-red-600",
};

/**
 * A centred icon + message used for empty, loading, and failure states
 * inside cards.
 */
export function StatusMessage({
  icon: Icon,
  message,
  tone = "neutral",
}: {
  icon: PhosphorIcon;
  message: string;
  tone?: StatusTone;
}) {
  return (
    <div className="flex flex-col items-center gap-2 py-6 text-center">
      <span className={ICON_CLASSES[tone]}>
        <Icon size={32} />
      </span>
      <p className={clsx("text-sm", TEXT_CLASSES[tone])}>{message}</p>
    </div>
  );
}
