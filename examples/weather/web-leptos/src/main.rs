mod core;

use leptos::prelude::*;

use shared::{
    Event, ViewModel,
    effects::http::location::GeocodingResponse,
    model::{
        OnboardEvent,
        active::{
            ActiveEvent,
            favorites::{
                FavoritesScreenEvent, add::AddFavoriteEvent, confirm_delete::ConfirmDeleteEvent,
            },
            home::HomeEvent,
        },
    },
    view::{
        active::{
            ActiveViewModel,
            favorites::{FavoritesViewModel, FavoritesWorkflowViewModel},
            home::{
                FavoriteWeatherStateViewModel, FavoriteWeatherViewModel, HomeViewModel,
                LocalWeatherViewModel,
            },
        },
        onboard::{OnboardStateViewModel, OnboardViewModel},
    },
};

// ANCHOR: content_view
#[component]
fn root_component() -> impl IntoView {
    let core = core::new();
    let (view, render) = signal(core.view());
    let (event, set_event) = signal(Event::Start);

    Effect::new(move |_| {
        core::update(&core, event.get(), render);
    });

    view! {
        <>
            <section class="section has-text-centered">
                <p class="title">{"Crux Weather Example"}</p>
                <p class="is-size-5">{"Rust Core, Rust Shell (Leptos)"}</p>
            </section>
            <section class="container">
                {move || {
                    match view.get() {
                        ViewModel::Loading => {
                            view! { <p class="has-text-centered">{"Loading..."}</p> }.into_any()
                        }
                        ViewModel::Onboard(onboard) => {
                            view! { <OnboardView model=onboard set_event=set_event /> }.into_any()
                        }
                        ViewModel::Active(active) => match active {
                            ActiveViewModel::Home(home) => {
                                view! { <HomeView model=home set_event=set_event /> }.into_any()
                            }
                            ActiveViewModel::Favorites(favorites) => {
                                view! { <FavoritesView model=favorites set_event=set_event /> }.into_any()
                            }
                        }
                        ViewModel::Failed { message } => {
                            view! {
                                <div class="notification is-danger">
                                    <p class="has-text-centered">{message}</p>
                                </div>
                            }.into_any()
                        }
                    }
                }}
            </section>
        </>
    }
}
// ANCHOR_END: content_view

#[component]
fn onboard_view(model: OnboardViewModel, set_event: WriteSignal<Event>) -> impl IntoView {
    let reason_text = match model.reason {
        shared::model::onboard::OnboardReason::Welcome => {
            "Welcome! Enter your OpenWeather API key to get started."
        }
        shared::model::onboard::OnboardReason::Unauthorized => {
            "Your API key was rejected. Please enter a valid key."
        }
        shared::model::onboard::OnboardReason::Reset => "Enter a new API key.",
    };

    match model.state {
        OnboardStateViewModel::Input {
            api_key,
            can_submit,
        } => view! {
            <div class="box">
                <h2 class="title is-4">{"Setup"}</h2>
                <p class="mb-4">{reason_text}</p>
                <div class="field">
                    <div class="control">
                        <input
                            class="input"
                            type="text"
                            placeholder="API Key"
                            prop:value=api_key
                            on:input=move |ev| {
                                set_event.set(Event::Onboard(OnboardEvent::ApiKey(
                                    event_target_value(&ev),
                                )));
                            }
                        />
                    </div>
                </div>
                <button
                    class="button is-primary"
                    disabled=move || !can_submit
                    on:click=move |_| set_event.set(Event::Onboard(OnboardEvent::Submit))
                >
                    {"Submit"}
                </button>
            </div>
        }
        .into_any(),
        OnboardStateViewModel::Saving => view! {
            <div class="box">
                <p class="has-text-centered">{"Saving..."}</p>
            </div>
        }
        .into_any(),
    }
}

// ANCHOR: home_view
#[component]
fn home_view(model: HomeViewModel, set_event: WriteSignal<Event>) -> impl IntoView {
    view! {
        <div class="box">
            {match model.local_weather {
                LocalWeatherViewModel::CheckingPermission => {
                    view! { <p class="has-text-centered">{"Checking location permission..."}</p> }.into_any()
                }
                LocalWeatherViewModel::LocationDisabled => {
                    view! { <p class="has-text-centered">{"Location is disabled. Enable location access to see local weather."}</p> }.into_any()
                }
                LocalWeatherViewModel::FetchingLocation => {
                    view! { <p class="has-text-centered">{"Getting your location..."}</p> }.into_any()
                }
                LocalWeatherViewModel::FetchingWeather => {
                    view! { <p class="has-text-centered">{"Loading weather data..."}</p> }.into_any()
                }
                LocalWeatherViewModel::Fetched(wd) => {
                    let name = wd.name.clone();
                    let desc = wd.weather.first().map(|w| w.description.clone());
                    view! {
                        <div class="has-text-centered">
                            <h2 class="title is-4">{name}</h2>
                            <p class="is-size-1 has-text-weight-bold">
                                {format!("{:.1}\u{00b0}", wd.main.temp)}
                            </p>
                            {desc.map(|d| view! { <p class="is-size-5">{d}</p> })}
                            <div class="columns is-multiline is-centered mt-4">
                                <div class="column is-one-third">
                                    <p class="heading">{"Feels Like"}</p>
                                    <p>{format!("{:.1}\u{00b0}", wd.main.feels_like)}</p>
                                </div>
                                <div class="column is-one-third">
                                    <p class="heading">{"Humidity"}</p>
                                    <p>{format!("{}%", wd.main.humidity)}</p>
                                </div>
                                <div class="column is-one-third">
                                    <p class="heading">{"Wind"}</p>
                                    <p>{format!("{:.1} m/s", wd.wind.speed)}</p>
                                </div>
                                <div class="column is-one-third">
                                    <p class="heading">{"Pressure"}</p>
                                    <p>{format!("{} hPa", wd.main.pressure)}</p>
                                </div>
                                <div class="column is-one-third">
                                    <p class="heading">{"Clouds"}</p>
                                    <p>{format!("{}%", wd.clouds.all)}</p>
                                </div>
                                <div class="column is-one-third">
                                    <p class="heading">{"Visibility"}</p>
                                    <p>{format!("{} km", wd.visibility / 1000)}</p>
                                </div>
                            </div>
                        </div>
                    }.into_any()
                }
                LocalWeatherViewModel::Failed => {
                    view! { <p class="has-text-centered has-text-danger">{"Failed to load weather."}</p> }.into_any()
                }
            }}
        </div>
        {if model.favorites.is_empty() {
            view! { <div></div> }.into_any()
        } else {
            view! {
                <div class="box">
                    <h3 class="title is-5">{"Favorites"}</h3>
                    {model.favorites.into_iter().map(|fav| {
                        view! { <FavoriteWeatherCard fav=fav /> }
                    }).collect::<Vec<_>>()}
                </div>
            }.into_any()
        }}
        <div class="buttons is-centered mt-4">
            <button class="button is-info"
                on:click=move |_| set_event.set(
                    Event::Active(ActiveEvent::home(HomeEvent::GoToFavorites))
                )
            >
                {"Favorites"}
            </button>
            <button class="button is-light"
                on:click=move |_| set_event.set(
                    Event::Active(ActiveEvent::ResetApiKey)
                )
            >
                {"Reset API Key"}
            </button>
        </div>
    }
}
// ANCHOR_END: home_view

#[component]
fn favorite_weather_card(fav: FavoriteWeatherViewModel) -> impl IntoView {
    let name = fav.name.clone();
    view! {
        <div class="box">
            <strong>{name}</strong>
            {match fav.weather {
                FavoriteWeatherStateViewModel::Fetching => {
                    view! { <p class="has-text-grey">{"Loading..."}</p> }.into_any()
                }
                FavoriteWeatherStateViewModel::Fetched(w) => {
                    view! {
                        <div class="columns is-multiline mt-2">
                            <div class="column is-one-third">
                                <p class="is-size-3 has-text-weight-bold">
                                    {format!("{:.1}\u{00b0}", w.main.temp)}
                                </p>
                            </div>
                            <div class="column is-one-third">
                                {w.weather.first().map(|wd| view! {
                                    <p>{wd.description.clone()}</p>
                                })}
                            </div>
                            <div class="column is-one-third">
                                <p>{format!("Humidity: {}%", w.main.humidity)}</p>
                            </div>
                        </div>
                    }.into_any()
                }
                FavoriteWeatherStateViewModel::Failed => {
                    view! { <p class="has-text-danger">{"Failed to load"}</p> }.into_any()
                }
            }}
        </div>
    }
}

#[component]
fn favorites_view(model: FavoritesViewModel, set_event: WriteSignal<Event>) -> impl IntoView {
    match model.workflow {
        Some(FavoritesWorkflowViewModel::Add(add)) => view! {
            <AddFavoriteView model=add set_event=set_event />
        }
        .into_any(),
        _ => {
            let delete_confirmation = match model.workflow {
                Some(FavoritesWorkflowViewModel::ConfirmDelete { location }) => Some(location),
                _ => None,
            };
            view! {
                <div class="box">
                    <h2 class="title is-4">{"Favorites"}</h2>
                    {if model.favorites.is_empty() {
                        view! { <p>{"No favorites yet"}</p> }.into_any()
                    } else {
                        view! {
                            <div>
                                {model.favorites.into_iter().map(|fav| {
                                    let loc = fav.location;
                                    let name = fav.name.clone();
                                    view! {
                                        <div class="box level">
                                            <div class="level-left">
                                                <strong>{name}</strong>
                                            </div>
                                            <div class="level-right">
                                                <button class="button is-danger is-small"
                                                    on:click=move |_| set_event.set(Event::Active(
                                                        ActiveEvent::favorites(
                                                            FavoritesScreenEvent::RequestDelete(loc)
                                                        )
                                                    ))
                                                >
                                                    {"Delete"}
                                                </button>
                                            </div>
                                        </div>
                                    }
                                }).collect::<Vec<_>>()}
                            </div>
                        }.into_any()
                    }}
                </div>
                {delete_confirmation.map(|_loc| view! {
                    <div class="modal is-active">
                        <div class="modal-background"></div>
                        <div class="modal-content">
                            <div class="box has-text-centered">
                                <p class="title is-5">{"Delete Favorite?"}</p>
                                <div class="buttons is-centered">
                                    <button class="button"
                                        on:click=move |_| set_event.set(Event::Active(
                                            ActiveEvent::favorites(
                                                FavoritesScreenEvent::confirm_delete(ConfirmDeleteEvent::Cancelled)
                                            )
                                        ))
                                    >
                                        {"Cancel"}
                                    </button>
                                    <button class="button is-danger"
                                        on:click=move |_| set_event.set(Event::Active(
                                            ActiveEvent::favorites(
                                                FavoritesScreenEvent::confirm_delete(ConfirmDeleteEvent::Confirmed)
                                            )
                                        ))
                                    >
                                        {"Delete"}
                                    </button>
                                </div>
                            </div>
                        </div>
                    </div>
                })}
                <div class="buttons is-centered mt-4">
                    <button class="button"
                        on:click=move |_| set_event.set(Event::Active(
                            ActiveEvent::favorites(FavoritesScreenEvent::GoToHome)
                        ))
                    >
                        {"Back"}
                    </button>
                    <button class="button is-primary"
                        on:click=move |_| set_event.set(Event::Active(
                            ActiveEvent::favorites(FavoritesScreenEvent::RequestAddFavorite)
                        ))
                    >
                        {"Add Favorite"}
                    </button>
                </div>
            }.into_any()
        }
    }
}

#[component]
fn add_favorite_view(
    model: shared::view::active::favorites::AddFavoriteViewModel,
    set_event: WriteSignal<Event>,
) -> impl IntoView {
    let (search_text, set_search_text) = signal(model.search_input.clone());

    view! {
        <div class="box">
            <h2 class="title is-4">{"Add Favorite"}</h2>
            <div class="field has-addons">
                <div class="control is-expanded">
                    <input
                        class="input"
                        type="text"
                        placeholder="Search location..."
                        prop:value=move || search_text.get()
                        on:input=move |ev| {
                            let val = event_target_value(&ev);
                            set_search_text.set(val.clone());
                            if !val.is_empty() {
                                set_event.set(Event::Active(
                                    ActiveEvent::favorites(
                                        FavoritesScreenEvent::add(AddFavoriteEvent::Search(val))
                                    )
                                ));
                            }
                        }
                    />
                </div>
            </div>
            {move || {
                model.search_results.clone().map(|results| {
                    if results.is_empty() {
                        view! { <p>{"No results found"}</p> }.into_any()
                    } else {
                        view! {
                            <div>
                                {results.into_iter().map(|result| {
                                    search_result_item(result, set_event)
                                }).collect::<Vec<_>>()}
                            </div>
                        }.into_any()
                    }
                })
            }}
        </div>
        <div class="buttons is-centered mt-4">
            <button class="button"
                on:click=move |_| set_event.set(Event::Active(
                    ActiveEvent::favorites(
                        FavoritesScreenEvent::add(AddFavoriteEvent::Cancel)
                    )
                ))
            >
                {"Cancel"}
            </button>
        </div>
    }
}

fn search_result_item(result: GeocodingResponse, set_event: WriteSignal<Event>) -> impl IntoView {
    let name = result.name.clone();
    let country = result.country.clone();
    let state = result.state.clone();
    let r = result.clone();
    view! {
        <div class="box">
            <div class="level">
                <div class="level-left">
                    <div>
                        <strong>{name}</strong>
                        <br/>
                        <small>{
                            state.map(|s| format!("{s}, {country}"))
                                .unwrap_or(country)
                        }</small>
                    </div>
                </div>
                <div class="level-right">
                    <button class="button is-primary is-small"
                        on:click=move |_| {
                            let r = r.clone();
                            set_event.set(Event::Active(
                                ActiveEvent::favorites(
                                    FavoritesScreenEvent::add(
                                        AddFavoriteEvent::Submit(Box::new(r))
                                    )
                                )
                            ));
                        }
                    >
                        {"Add"}
                    </button>
                </div>
            </div>
        </div>
    }
}

fn main() {
    _ = console_log::init_with_level(log::Level::Debug);
    console_error_panic_hook::set_once();

    leptos::mount::mount_to_body(|| {
        view! { <RootComponent /> }
    });
}
