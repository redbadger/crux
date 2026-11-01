import { Location } from "shared_types/app";

/** After this time, the UI shows the "location disabled" panel, which has a retry. */
const TIMEOUT_MS = 10_000;

/**
 * `IsLocationEnabled` is answered with a bare `boolean` — that is the
 * operation's output type, so there is nothing to wrap or unwrap.
 *
 * If the user already refused the permission, this answers `false`. Where
 * `navigator.permissions` is not available, only the request itself can find
 * out.
 */
export async function isLocationEnabled(): Promise<boolean> {
  if (typeof navigator === "undefined" || !("geolocation" in navigator)) {
    console.debug("location enabled: false (no geolocation)");
    return false;
  }
  try {
    const permission = await navigator.permissions?.query({
      name: "geolocation",
    });
    if (permission?.state === "denied") {
      console.debug("location enabled: false (permission denied)");
      return false;
    }
  } catch (e) {
    console.debug("permissions query unavailable:", e);
  }
  console.debug("location enabled: true");
  return true;
}

/**
 * `GetLocation` is answered with the coordinates, or `null` if we couldn't
 * get a fix.
 *
 * The browser default has no timeout, so without one the core can wait forever.
 */
export async function getLocation(): Promise<Location | null> {
  try {
    const position = await new Promise<GeolocationPosition>(
      (resolve, reject) => {
        navigator.geolocation.getCurrentPosition(resolve, reject, {
          timeout: TIMEOUT_MS,
          maximumAge: 60_000,
        });
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
