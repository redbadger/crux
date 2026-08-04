import { CloudSlash, Key, MapPinLine, Star } from "@phosphor-icons/react";

import type { HomeViewModel } from "shared_types/app";
import {
  matchLocalWeatherViewModel,
  eventActive,
  activeEventHome,
  activeEventResetApiKey,
  homeEventGoToFavorites,
} from "shared_types/app";

import { useDispatch } from "../../lib/core/provider";
import { Button, Card, SectionTitle, Spinner, StatusMessage } from "./common";
import { FavoriteWeatherCard } from "./FavoriteWeatherCard";
import { WeatherDetail } from "./WeatherDetail";

// ANCHOR: home_view
export function HomeView({ model }: { model: HomeViewModel }) {
  const dispatch = useDispatch();
  const lw = model.local_weather;

  return (
    <>
      <Card className="mb-4">
        {matchLocalWeatherViewModel(lw, {
          CheckingPermission: () => (
            <StatusMessage
              icon={MapPinLine}
              message="Checking location permission..."
            />
          ),
          LocationDisabled: () => (
            <StatusMessage
              icon={MapPinLine}
              message="Location is disabled. Enable location access to see local weather."
            />
          ),
          FetchingLocation: () => (
            <Spinner message="Getting your location..." />
          ),
          FetchingWeather: () => <Spinner message="Loading weather data..." />,
          Fetched: (v) => <WeatherDetail data={v.value} />,
          Failed: () => (
            <StatusMessage
              icon={CloudSlash}
              message="Failed to load weather."
              tone="error"
            />
          ),
        })}
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
            dispatch(eventActive(activeEventHome(homeEventGoToFavorites())))
          }
        />
        <Button
          label="Reset API Key"
          icon={Key}
          variant="secondary"
          onClick={() => dispatch(eventActive(activeEventResetApiKey()))}
        />
      </div>
    </>
  );
}
// ANCHOR_END: home_view
