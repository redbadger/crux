import type { Core } from "../../lib/core";
import type { HomeViewModel } from "shared_types/app";
import {
  LocalWeatherViewModelVariantCheckingPermission,
  LocalWeatherViewModelVariantLocationDisabled,
  LocalWeatherViewModelVariantFetchingLocation,
  LocalWeatherViewModelVariantFetchingWeather,
  LocalWeatherViewModelVariantFetched,
  LocalWeatherViewModelVariantFailed,
  EventVariantActive,
  ActiveEventVariantHome,
  ActiveEventVariantResetApiKey,
  HomeEventVariantGoToFavorites,
} from "shared_types/app";
import { WeatherDetail } from "./WeatherDetail";
import { FavoriteWeatherCard } from "./FavoriteWeatherCard";

// ANCHOR: home_view
export function HomeView({
  model,
  core,
}: {
  model: HomeViewModel;
  core: React.RefObject<Core | null>;
}) {
  const lw = model.local_weather;

  return (
    <>
      <div className="card">
        {lw instanceof LocalWeatherViewModelVariantCheckingPermission && (
          <div className="status-message">
            <i className="ph ph-map-pin-line" />
            <p>Checking location permission...</p>
          </div>
        )}
        {lw instanceof LocalWeatherViewModelVariantLocationDisabled && (
          <div className="status-message">
            <i className="ph ph-map-pin-line" />
            <p>
              Location is disabled. Enable location access to see local
              weather.
            </p>
          </div>
        )}
        {lw instanceof LocalWeatherViewModelVariantFetchingLocation && (
          <div className="status-message">
            <i className="ph ph-gps" />
            <p>Getting your location...</p>
          </div>
        )}
        {lw instanceof LocalWeatherViewModelVariantFetchingWeather && (
          <div className="status-message">
            <i className="ph ph-cloud" />
            <p>Loading weather data...</p>
          </div>
        )}
        {lw instanceof LocalWeatherViewModelVariantFetched && (
          <WeatherDetail data={lw.value} />
        )}
        {lw instanceof LocalWeatherViewModelVariantFailed && (
          <div className="status-message">
            <i className="ph ph-cloud-slash" style={{ color: "#ef4444" }} />
            <p style={{ color: "#ef4444" }}>Failed to load weather.</p>
          </div>
        )}
      </div>
      {model.favorites.length > 0 && (
        <div className="card">
          <p className="section-title">
            <i className="ph ph-star" />
            Favorites
          </p>
          {model.favorites.map((fav, i) => (
            <FavoriteWeatherCard key={i} fav={fav} />
          ))}
        </div>
      )}
      <div className="buttons is-centered" style={{ marginTop: "1rem" }}>
        <button
          className="button is-info btn"
          onClick={() =>
            core.current?.update(
              new EventVariantActive(
                new ActiveEventVariantHome(
                  new HomeEventVariantGoToFavorites(),
                ),
              ),
            )
          }
        >
          <span className="icon">
            <i className="ph ph-star" />
          </span>
          <span>Favorites</span>
        </button>
        <button
          className="button is-light btn"
          onClick={() =>
            core.current?.update(
              new EventVariantActive(new ActiveEventVariantResetApiKey()),
            )
          }
        >
          <span className="icon">
            <i className="ph ph-key" />
          </span>
          <span>Reset API Key</span>
        </button>
      </div>
    </>
  );
}
// ANCHOR_END: home_view
