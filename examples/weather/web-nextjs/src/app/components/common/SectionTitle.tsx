import type { Icon as PhosphorIcon } from "@phosphor-icons/react";

/**
 * An inline card header — icon + title, used at the top of a `Card` to label
 * the section. Smaller and denser than `ScreenHeader`.
 */
export function SectionTitle({
  icon: Icon,
  title,
}: {
  icon: PhosphorIcon;
  title: string;
}) {
  return (
    <div className="flex items-center gap-2 text-slate-700 font-semibold text-lg mb-4">
      <Icon size={20} />
      <span>{title}</span>
    </div>
  );
}
