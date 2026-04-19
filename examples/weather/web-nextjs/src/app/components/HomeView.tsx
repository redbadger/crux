import {
  CloudSlash,
  Key,
  MapPinLine,
  Star,
} from "@phosphor-icons/react";

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

import { useDispatch } from "../../lib/core/provider";
import {
  Button,
  Card,
  SectionTitle,
  Spinner,
  StatusMessage,
} from "./common";
import { FavoriteWeatherCard } from "./FavoriteWeatherCard";
import { WeatherDetail } from "./WeatherDetail";

// ANCHOR: home_view
export function HomeView({ model }: { model: HomeViewModel }) {
  const dispatch = useDispatch();
  const lw = model.local_weather;

  return (
    <>
      <Card className="mb-4">
        {lw instanceof LocalWeatherViewModelVariantCheckingPermission && (
          <StatusMessage
            icon={MapPinLine}
            message="Checking location permission..."
          />
        )}
        {lw instanceof LocalWeatherViewModelVariantLocationDisabled && (
          <StatusMessage
            icon={MapPinLine}
            message="Location is disabled. Enable location access to see local weather."
          />
        )}
        {lw instanceof LocalWeatherViewModelVariantFetchingLocation && (
          <Spinner message="Getting your location..." />
        )}
        {lw instanceof LocalWeatherViewModelVariantFetchingWeather && (
          <Spinner message="Loading weather data..." />
        )}
        {lw instanceof LocalWeatherViewModelVariantFetched && (
          <WeatherDetail data={lw.value} />
        )}
        {lw instanceof LocalWeatherViewModelVariantFailed && (
          <StatusMessage
            icon={CloudSlash}
            message="Failed to load weather."
            tone="error"
          />
        )}
      </Card>
      {model.favorites.length > 0 && (
        <Card className="mb-4">
          <SectionTitle icon={Star} title="Favourites" />
          <div className="grid gap-2">
            {model.favorites.map((fav, i) => (
              <FavoriteWeatherCard key={i} fav={fav} />
            ))}
          </div>
        </Card>
      )}
      <div className="flex justify-center gap-2 mt-4">
        <Button
          label="Favourites"
          icon={Star}
          onClick={() =>
            dispatch(
              new EventVariantActive(
                new ActiveEventVariantHome(
                  new HomeEventVariantGoToFavorites(),
                ),
              ),
            )
          }
        />
        <Button
          label="Reset API Key"
          icon={Key}
          variant="secondary"
          onClick={() =>
            dispatch(
              new EventVariantActive(new ActiveEventVariantResetApiKey()),
            )
          }
        />
      </div>
    </>
  );
}
// ANCHOR_END: home_view
