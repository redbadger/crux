import { Location } from "shared_types/app";

/// How long to wait for a fix before giving up and showing the user the
/// "location disabled" panel, which has a retry.
const TIMEOUT_MS = 10_000;

/// `IsLocationEnabled` is answered with a bare `boolean` — that is the
/// operation's output type, so there is nothing to wrap or unwrap.
///
/// The browser having the API is not the same as this page being allowed to
/// use it, so a permission the user has already refused is answered here
/// rather than by asking for a fix we will not get. `navigator.permissions`
/// is not everywhere, and where it is missing the request itself is the only
/// way to find out.
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

/// `GetLocation` is answered with the coordinates, or `null` if we couldn't
/// get a fix.
///
/// The request carries a timeout because the default is not to have one: a
/// prompt the user never answers, or a device with nothing to locate itself
/// by, would otherwise leave this promise pending forever and the core
/// waiting on an effect that never resolves.
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
