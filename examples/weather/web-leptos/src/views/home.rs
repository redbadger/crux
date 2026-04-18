use leptos::prelude::*;

use shared::{
    Event,
    model::active::{ActiveEvent, home::HomeEvent},
    view::active::home::{
        FavoriteWeatherStateViewModel, FavoriteWeatherViewModel, HomeViewModel,
        LocalWeatherViewModel,
    },
};

// ANCHOR: home_view
#[component]
#[allow(clippy::too_many_lines)]
pub fn home_view(model: HomeViewModel, set_event: WriteSignal<Event>) -> impl IntoView {
    view! {
        <div class="card">
            {match model.local_weather {
                LocalWeatherViewModel::CheckingPermission => {
                    view! {
                        <div class="status-message">
                            <i class="ph ph-map-pin-line"></i>
                            <p>{"Checking location permission..."}</p>
                        </div>
                    }.into_any()
                }
                LocalWeatherViewModel::LocationDisabled => {
                    view! {
                        <div class="status-message">
                            <i class="ph ph-map-pin-line"></i>
                            <p>{"Location is disabled. Enable location access to see local weather."}</p>
                        </div>
                    }.into_any()
                }
                LocalWeatherViewModel::FetchingLocation => {
                    view! {
                        <div class="status-message">
                            <i class="ph ph-gps"></i>
                            <p>{"Getting your location..."}</p>
                        </div>
                    }.into_any()
                }
                LocalWeatherViewModel::FetchingWeather => {
                    view! {
                        <div class="status-message">
                            <i class="ph ph-cloud"></i>
                            <p>{"Loading weather data..."}</p>
                        </div>
                    }.into_any()
                }
                LocalWeatherViewModel::Fetched(wd) => {
                    let name = wd.name.clone();
                    let desc = wd.weather.first().map(|w| w.description.clone());
                    view! {
                        <div style="text-align: center;">
                            <p style="font-weight: 600; color: #6b7280; font-size: 0.9rem; text-transform: uppercase; letter-spacing: 0.05em;">
                                <i class="ph ph-map-pin" style="margin-right: 0.25rem;"></i>
                                {name}
                            </p>
                            <p class="temp-large">{format!("{:.1}\u{00b0}", wd.main.temp)}</p>
                            {desc.map(|d| view! { <p class="weather-desc">{d}</p> })}
                            <div class="stat-grid">
                                <div>
                                    <p class="stat-label">
                                        <i class="ph ph-thermometer" style="margin-right: 0.2rem;"></i>
                                        {"Feels Like"}
                                    </p>
                                    <p class="stat-value">{format!("{:.1}\u{00b0}", wd.main.feels_like)}</p>
                                </div>
                                <div>
                                    <p class="stat-label">
                                        <i class="ph ph-drop" style="margin-right: 0.2rem;"></i>
                                        {"Humidity"}
                                    </p>
                                    <p class="stat-value">{format!("{}%", wd.main.humidity)}</p>
                                </div>
                                <div>
                                    <p class="stat-label">
                                        <i class="ph ph-wind" style="margin-right: 0.2rem;"></i>
                                        {"Wind"}
                                    </p>
                                    <p class="stat-value">{format!("{:.1} m/s", wd.wind.speed)}</p>
                                </div>
                                <div>
                                    <p class="stat-label">
                                        <i class="ph ph-gauge" style="margin-right: 0.2rem;"></i>
                                        {"Pressure"}
                                    </p>
                                    <p class="stat-value">{format!("{} hPa", wd.main.pressure)}</p>
                                </div>
                                <div>
                                    <p class="stat-label">
                                        <i class="ph ph-cloud" style="margin-right: 0.2rem;"></i>
                                        {"Clouds"}
                                    </p>
                                    <p class="stat-value">{format!("{}%", wd.clouds.all)}</p>
                                </div>
                                <div>
                                    <p class="stat-label">
                                        <i class="ph ph-eye" style="margin-right: 0.2rem;"></i>
                                        {"Visibility"}
                                    </p>
                                    <p class="stat-value">{format!("{} km", wd.visibility / 1000)}</p>
                                </div>
                            </div>
                        </div>
                    }.into_any()
                }
                LocalWeatherViewModel::Failed => {
                    view! {
                        <div class="status-message">
                            <i class="ph ph-cloud-slash" style="color: #ef4444;"></i>
                            <p style="color: #ef4444;">{"Failed to load weather."}</p>
                        </div>
                    }.into_any()
                }
            }}
        </div>
        {if model.favorites.is_empty() {
            view! { <div></div> }.into_any()
        } else {
            view! {
                <div class="card">
                    <p class="section-title">
                        <i class="ph ph-star"></i>
                        {"Favorites"}
                    </p>
                    {model.favorites.into_iter().map(|fav| {
                        view! { <FavoriteWeatherCard fav=fav /> }
                    }).collect::<Vec<_>>()}
                </div>
            }.into_any()
        }}
        <div class="buttons is-centered" style="margin-top: 1rem;">
            <button class="button is-info btn"
                on:click=move |_| set_event.set(
                    Event::Active(ActiveEvent::home(HomeEvent::GoToFavorites))
                )
            >
                <span class="icon"><i class="ph ph-star"></i></span>
                <span>{"Favorites"}</span>
            </button>
            <button class="button is-light btn"
                on:click=move |_| set_event.set(
                    Event::Active(ActiveEvent::ResetApiKey)
                )
            >
                <span class="icon"><i class="ph ph-key"></i></span>
                <span>{"Reset API Key"}</span>
            </button>
        </div>
    }
}
// ANCHOR_END: home_view

#[component]
fn favorite_weather_card(fav: FavoriteWeatherViewModel) -> impl IntoView {
    let name = fav.name.clone();
    view! {
        <div class="fav-card">
            <span class="fav-name">{name}</span>
            {match fav.weather {
                FavoriteWeatherStateViewModel::Fetching => {
                    view! { <span class="fav-detail">{"Loading..."}</span> }.into_any()
                }
                FavoriteWeatherStateViewModel::Fetched(w) => {
                    let desc = w.weather.first().map(|wd| wd.description.clone());
                    view! {
                        <>
                            <span class="temp-medium" style="font-size: 1.5rem;">
                                {format!("{:.1}\u{00b0}", w.main.temp)}
                            </span>
                            {desc.map(|d| view! { <span class="fav-detail">{d}</span> })}
                            <span class="fav-detail">
                                <i class="ph ph-drop" style="margin-right: 0.2rem;"></i>
                                {format!("{}%", w.main.humidity)}
                            </span>
                        </>
                    }.into_any()
                }
                FavoriteWeatherStateViewModel::Failed => {
                    view! {
                        <span style="color: #ef4444;">
                            <i class="ph ph-warning"></i>
                            {" Failed"}
                        </span>
                    }.into_any()
                }
            }}
        </div>
    }
}
