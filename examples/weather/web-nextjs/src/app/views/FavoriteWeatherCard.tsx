import type { FavoriteWeatherViewModel } from "shared_types/app";
import {
  FavoriteWeatherStateViewModelVariantFetching,
  FavoriteWeatherStateViewModelVariantFetched,
  FavoriteWeatherStateViewModelVariantFailed,
} from "shared_types/app";

export function FavoriteWeatherCard({
  fav,
}: {
  fav: FavoriteWeatherViewModel;
}) {
  const w = fav.weather;
  return (
    <div className="fav-card">
      <span className="fav-name">{fav.name}</span>
      {w instanceof FavoriteWeatherStateViewModelVariantFetching && (
        <span className="fav-detail">Loading...</span>
      )}
      {w instanceof FavoriteWeatherStateViewModelVariantFetched && (
        <>
          <span className="temp-medium" style={{ fontSize: "1.5rem" }}>
            {w.value.main.temp.toFixed(1)}&deg;
          </span>
          {w.value.weather?.[0] && (
            <span className="fav-detail">
              {w.value.weather[0].description}
            </span>
          )}
          <span className="fav-detail">
            <i className="ph ph-drop" style={{ marginRight: "0.2rem" }} />
            {Number(w.value.main.humidity)}%
          </span>
        </>
      )}
      {w instanceof FavoriteWeatherStateViewModelVariantFailed && (
        <span style={{ color: "#ef4444" }}>
          <i className="ph ph-warning" /> Failed
        </span>
      )}
    </div>
  );
}
