use leptos::callback::UnsyncCallback;
use leptos::prelude::*;
use phosphor_leptos::{
    CLOUD, CLOUD_SLASH, DROP, EYE, GAUGE, Icon, KEY, MAP_PIN, MAP_PIN_LINE, STAR, THERMOMETER, WIND,
};

use shared::{
    Event,
    model::active::{ActiveEvent, home::HomeEvent},
    view::active::home::{
        FavoriteWeatherStateViewModel, FavoriteWeatherViewModel, HomeViewModel,
        LocalWeatherViewModel,
    },
};

use super::{
    common::{Button, ButtonVariant, Card, SectionTitle, Spinner, StatusMessage, StatusTone},
    use_dispatch,
};

// ANCHOR: home_view
#[component]
pub fn home_view(#[prop(into)] vm: Signal<HomeViewModel>) -> impl IntoView {
    let dispatch = use_dispatch();

    view! {
        <Card class="mb-4">
            {move || {
                // Read the current `local_weather` slice. `.read()` returns
                // a guard that derefs to `&HomeViewModel`; clone the inner
                // variant so we can match it outside the borrow.
                match vm.read().local_weather.clone() {
                    LocalWeatherViewModel::CheckingPermission => view! {
                        <StatusMessage icon=MAP_PIN_LINE message="Checking location permission..." />
                    }.into_any(),
                    LocalWeatherViewModel::LocationDisabled => view! {
                        <StatusMessage
                            icon=MAP_PIN_LINE
                            message="Location is disabled. Enable location access to see local weather."
                        />
                    }.into_any(),
                    LocalWeatherViewModel::FetchingLocation => view! {
                        <Spinner message="Getting your location..." />
                    }.into_any(),
                    LocalWeatherViewModel::FetchingWeather => view! {
                        <Spinner message="Loading weather data..." />
                    }.into_any(),
                    LocalWeatherViewModel::Fetched(wd) => view! { <CurrentWeather data=*wd /> }.into_any(),
                    LocalWeatherViewModel::Failed => view! {
                        <StatusMessage
                            icon=CLOUD_SLASH
                            message="Failed to load weather."
                            tone=StatusTone::Error
                        />
                    }.into_any(),
                }
            }}
        </Card>
        {move || {
            // `.with(|v| ...)` is the closure form — borrow, project,
            // return whatever the closure returns. Here: the favourites
            // vector (cloned once per render).
            let favorites = vm.with(|v| v.favorites.clone());
            (!favorites.is_empty()).then(|| view! {
                <Card class="mb-4">
                    <SectionTitle icon=STAR title="Favourites" />
                    <div class="grid gap-2">
                        {favorites.into_iter().map(|fav| view! {
                            <FavoriteWeatherCard fav=fav />
                        }).collect::<Vec<_>>()}
                    </div>
                </Card>
            })
        }}
        <div class="flex justify-center gap-2 mt-4">
            <Button
                label="Favourites"
                icon=STAR
                on_click=UnsyncCallback::new(move |()| {
                    dispatch.run(Event::Active(ActiveEvent::home(HomeEvent::GoToFavorites)));
                })
            />
            <Button
                label="Reset API Key"
                icon=KEY
                variant=ButtonVariant::Secondary
                on_click=UnsyncCallback::new(move |()| {
                    dispatch.run(Event::Active(ActiveEvent::ResetApiKey));
                })
            />
        </div>
    }
}
// ANCHOR_END: home_view

#[component]
fn current_weather(
    data: shared::effects::http::weather::model::current_response::CurrentWeatherResponse,
) -> impl IntoView {
    let name = data.name.clone();
    let desc = data.weather.first().map(|w| w.description.clone());

    view! {
        <div class="text-center">
            <p class="uppercase tracking-wide text-xs text-slate-500 font-semibold flex items-center justify-center gap-1">
                <Icon icon=MAP_PIN size="14px" />
                {name}
            </p>
            <p class="text-6xl font-bold text-slate-900 mt-2 leading-none">
                {format!("{:.1}\u{00b0}", data.main.temp)}
            </p>
            {desc.map(|d| view! { <p class="text-slate-500 mt-1 capitalize">{d}</p> })}
            <div class="grid grid-cols-3 gap-3 mt-6 text-center">
                <Stat icon=THERMOMETER label="Feels Like" value=format!("{:.1}\u{00b0}", data.main.feels_like) />
                <Stat icon=DROP label="Humidity" value=format!("{}%", data.main.humidity) />
                <Stat icon=WIND label="Wind" value=format!("{:.1} m/s", data.wind.speed) />
                <Stat icon=GAUGE label="Pressure" value=format!("{} hPa", data.main.pressure) />
                <Stat icon=CLOUD label="Clouds" value=format!("{}%", data.clouds.all) />
                <Stat icon=EYE label="Visibility" value=format!("{} km", data.visibility / 1000) />
            </div>
        </div>
    }
}

#[component]
fn stat(
    icon: phosphor_leptos::IconData,
    #[prop(into)] label: String,
    #[prop(into)] value: String,
) -> impl IntoView {
    view! {
        <div>
            <p class="text-xs uppercase tracking-wide text-slate-400 font-semibold flex items-center justify-center gap-1">
                <Icon icon=icon size="14px" />
                {label}
            </p>
            <p class="text-sm font-semibold text-slate-700 mt-1">{value}</p>
        </div>
    }
}

#[component]
fn favorite_weather_card(fav: FavoriteWeatherViewModel) -> impl IntoView {
    let name = fav.name.clone();
    view! {
        <div class="bg-slate-50 rounded-xl px-4 py-3 flex items-center justify-between gap-4">
            <span class="font-semibold text-slate-900">{name}</span>
            {match fav.weather {
                FavoriteWeatherStateViewModel::Fetching => view! {
                    <span class="text-sm text-slate-500">"Loading..."</span>
                }.into_any(),
                FavoriteWeatherStateViewModel::Fetched(w) => {
                    let desc = w.weather.first().map(|wd| wd.description.clone());
                    view! {
                        <div class="flex items-center gap-3 text-sm text-slate-600">
                            <span class="text-2xl font-bold text-slate-900">
                                {format!("{:.1}\u{00b0}", w.main.temp)}
                            </span>
                            {desc.map(|d| view! { <span class="capitalize">{d}</span> })}
                            <span class="flex items-center gap-1">
                                <Icon icon=DROP size="14px" />
                                {format!("{}%", w.main.humidity)}
                            </span>
                        </div>
                    }.into_any()
                }
                FavoriteWeatherStateViewModel::Failed => view! {
                    <span class="text-red-500 text-sm">"Failed"</span>
                }.into_any(),
            }}
        </div>
    }
}
