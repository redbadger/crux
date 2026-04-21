import { MapPin, Plus } from "@phosphor-icons/react";

import type { GeocodingResponse } from "shared_types/app";

import { Button } from "./common";

export function SearchResultItem({
  result,
  onAdd,
}: {
  result: GeocodingResponse;
  onAdd: () => void;
}) {
  const subtitle = result.state
    ? `${result.state}, ${result.country}`
    : result.country;

  return (
    <div className="bg-slate-50 rounded-xl px-4 py-3 flex items-center justify-between gap-4">
      <div>
        <div className="font-semibold text-slate-900 flex items-center gap-1">
          <MapPin size={16} />
          {result.name}
        </div>
        <small className="text-slate-500">{subtitle}</small>
      </div>
      <Button label="Add" icon={Plus} onClick={onAdd} />
    </div>
  );
}
