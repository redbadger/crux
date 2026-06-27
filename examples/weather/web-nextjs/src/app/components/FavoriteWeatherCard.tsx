import { Drop, Warning } from "@phosphor-icons/react";

import type { FavoriteWeatherViewModel } from "shared_types/app";
import { matchFavoriteWeatherStateViewModel } from "shared_types/app";

export function FavoriteWeatherCard({
  fav,
}: {
  fav: FavoriteWeatherViewModel;
}) {
  const w = fav.weather;
  return (
    <div className="bg-slate-50 rounded-xl px-4 py-3 flex items-center justify-between gap-4">
      <span className="font-semibold text-slate-900">{fav.name}</span>
      {matchFavoriteWeatherStateViewModel(w, {
        Fetching: () => (
          <span className="text-sm text-slate-500">Loading...</span>
        ),
        Fetched: (v) => (
          <div className="flex items-center gap-3 text-sm text-slate-600">
            <span className="text-2xl font-bold text-slate-900">
              {v.value.main.temp.toFixed(1)}&deg;
            </span>
            {v.value.weather?.[0] ? (
              <span className="capitalize">{v.value.weather[0].description}</span>
            ) : null}
            <span className="flex items-center gap-1">
              <Drop size={14} />
              {Number(v.value.main.humidity)}%
            </span>
          </div>
        ),
        Failed: () => (
          <span className="text-red-500 text-sm flex items-center gap-1">
            <Warning size={14} />
            Failed
          </span>
        ),
      })}
    </div>
  );
}
