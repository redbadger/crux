import { Location } from "shared_types/app";

/// `IsLocationEnabled` is answered with a bare `boolean` — that is the
/// operation's output type, so there is nothing to wrap or unwrap.
export async function isLocationEnabled(): Promise<boolean> {
  const enabled = "geolocation" in navigator;
  console.debug("location enabled:", enabled);
  return enabled;
}

/// `GetLocation` is answered with the coordinates, or `null` if we couldn't
/// get a fix.
export async function getLocation(): Promise<Location | null> {
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
    return new Location(position.coords.latitude, position.coords.longitude);
  } catch (e) {
    console.warn("geolocation failed:", e);
    return null;
  }
}
