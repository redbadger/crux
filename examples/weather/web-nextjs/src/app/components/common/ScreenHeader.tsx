import type { Icon as PhosphorIcon } from "@phosphor-icons/react";

/**
 * The big white app header — title, optional subtitle, optional icon.
 * Lives at the top of the page above all cards.
 */
export function ScreenHeader({
  title,
  subtitle = "",
  icon: Icon,
}: {
  title: string;
  subtitle?: string;
  icon?: PhosphorIcon;
}) {
  return (
    <header className="text-center text-white mb-6">
      <h1 className="text-3xl font-semibold flex items-center justify-center gap-2">
        {Icon ? <Icon size={28} /> : null}
        <span>{title}</span>
      </h1>
      {subtitle ? (
        <p className="text-white/80 text-sm mt-1">{subtitle}</p>
      ) : null}
    </header>
  );
}
