import type { LocationOperation, LocationResult } from "shared_types/app";
import {
  matchLocationOperation,
  locationResultEnabled,
  locationResultLocation,
  Location,
} from "shared_types/app";

export async function handle(
  operation: LocationOperation,
): Promise<LocationResult> {
  return matchLocationOperation<Promise<LocationResult>>(operation, {
    IsLocationEnabled: async () => {
      const enabled = "geolocation" in navigator;
      console.debug("location enabled:", enabled);
      return locationResultEnabled(enabled);
    },
    GetLocation: async () => {
      try {
        const position = await new Promise<GeolocationPosition>(
          (resolve, reject) => {
            navigator.geolocation.getCurrentPosition(resolve, reject);
          },
        );
        console.debug(
          "location fetched:",
          position.coords.latitude,
          position.coords.longitude,
        );
        return locationResultLocation(
          new Location(position.coords.latitude, position.coords.longitude),
        );
      } catch (e) {
        console.warn("geolocation failed:", e);
        return locationResultLocation(null);
      }
    },
  });
}
