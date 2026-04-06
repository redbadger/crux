import type { LocationOperation, LocationResult } from "shared_types/app";
import {
  LocationOperationVariantIsLocationEnabled,
  LocationResultVariantEnabled,
  LocationResultVariantLocation,
  Location,
} from "shared_types/app";

export async function handle(
  operation: LocationOperation,
): Promise<LocationResult> {
  switch (operation.constructor) {
    case LocationOperationVariantIsLocationEnabled: {
      const enabled = "geolocation" in navigator;
      console.debug("location enabled:", enabled);
      return new LocationResultVariantEnabled(enabled);
    }
    default: {
      // GetLocation
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
        return new LocationResultVariantLocation(
          new Location(
            position.coords.latitude,
            position.coords.longitude,
          ),
        );
      } catch (e) {
        console.warn("geolocation failed:", e);
        return new LocationResultVariantLocation(null);
      }
    }
  }
}
