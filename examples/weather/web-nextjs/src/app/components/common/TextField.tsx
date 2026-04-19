import clsx from "clsx";
import type { Icon as PhosphorIcon } from "@phosphor-icons/react";

/**
 * An input field with an optional leading icon.
 */
export function TextField({
  value,
  placeholder,
  onInput,
  icon: Icon,
  autoFocus = false,
}: {
  value: string;
  placeholder: string;
  onInput: (value: string) => void;
  icon?: PhosphorIcon;
  autoFocus?: boolean;
}) {
  return (
    <div className="relative">
      {Icon ? (
        <span className="pointer-events-none absolute inset-y-0 left-0 flex items-center pl-3 text-slate-400">
          <Icon size={18} />
        </span>
      ) : null}
      <input
        type="text"
        className={clsx(
          "w-full rounded-lg border border-slate-200 bg-white",
          "py-2.5 text-sm text-slate-900 placeholder:text-slate-400",
          "focus:outline-none focus:ring-2 focus:ring-sky-400 focus:border-sky-400",
          Icon ? "pl-10 pr-3" : "px-3",
        )}
        placeholder={placeholder}
        autoFocus={autoFocus}
        value={value}
        onChange={(e) => onInput(e.target.value)}
      />
    </div>
  );
}
