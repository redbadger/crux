mod core;
mod http;
mod kv;
mod location;

use leptos::prelude::*;

use shared::{
    Event, WorkflowViewModel,
    favorites::{events::FavoritesEvent, model::FavoritesState},
    location::model::geocoding_response::GeocodingResponse,
    weather::events::WeatherEvent,
};

#[component]
fn root_component() -> impl IntoView {
    let core = core::new();
    let (view, render) = signal(core.view());
    let (event, set_event) = signal(Event::Home(Box::new(WeatherEvent::Show)));

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
                    let v = view.get();
                    match v.workflow {
                        WorkflowViewModel::Home { weather_data, favorites } => {
                            let set_event = set_event;
                            view! {
                                <HomeView
                                    weather_data=weather_data
                                    favorites=favorites
                                    set_event=set_event
                                />
                            }.into_any()
                        }
                        WorkflowViewModel::Favorites { favorites, delete_confirmation } => {
                            let set_event = set_event;
                            view! {
                                <FavoritesView
                                    favorites=favorites
                                    delete_confirmation=delete_confirmation
                                    set_event=set_event
                                />
                            }.into_any()
                        }
                        WorkflowViewModel::AddFavorite { search_results } => {
                            let set_event = set_event;
                            view! {
                                <AddFavoriteView
                                    search_results=search_results
                                    set_event=set_event
                                />
                            }.into_any()
                        }
                    }
                }}
            </section>
        </>
    }
}

#[component]
fn home_view(
    weather_data: Box<shared::weather::model::current_response::CurrentWeatherResponse>,
    favorites: Vec<shared::FavoriteView>,
    set_event: WriteSignal<Event>,
) -> impl IntoView {
    let wd = *weather_data;
    let has_data = wd.cod == 200;

    view! {
        <div class="box">
            {if has_data {
                let name = wd.name.clone();
                let desc = wd.weather.first().map(|w| w.description.clone());
                view! {
                    <div class="has-text-centered">
                        <h2 class="title is-4">{name}</h2>
                        <p class="is-size-1 has-text-weight-bold">
                            {format!("{:.1}°", wd.main.temp)}
                        </p>
                        {desc.map(|d| view! {
                            <p class="is-size-5">{d}</p>
                        })}
                        <div class="columns is-multiline is-centered mt-4">
                            <div class="column is-one-third">
                                <p class="heading">{"Feels Like"}</p>
                                <p>{format!("{:.1}°", wd.main.feels_like)}</p>
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
            } else {
                view! {
                    <p class="has-text-centered">{"Loading weather data..."}</p>
                }.into_any()
            }}
        </div>
        {if !favorites.is_empty() {
            view! {
                <div class="box">
                    <h3 class="title is-5">{"Favorites"}</h3>
                    {favorites.into_iter().map(|fav| {
                        let name = fav.name.clone();
                        view! {
                            <div class="box">
                                <strong>{name}</strong>
                            </div>
                        }
                    }).collect::<Vec<_>>()}
                </div>
            }.into_any()
        } else {
            view! { <div></div> }.into_any()
        }}
        <div class="buttons is-centered mt-4">
            <button class="button is-info"
                on:click=move |_| set_event.set(Event::Navigate(
                    Box::new(shared::Workflow::Favorites(FavoritesState::Idle))
                ))
            >
                {"Favorites"}
            </button>
        </div>
    }
}

#[component]
fn favorites_view(
    favorites: Vec<shared::FavoriteView>,
    delete_confirmation: Option<shared::location::Location>,
    set_event: WriteSignal<Event>,
) -> impl IntoView {
    view! {
        <div class="box">
            <h2 class="title is-4">{"Favorites"}</h2>
            {if favorites.is_empty() {
                view! { <p>{"No favorites yet"}</p> }.into_any()
            } else {
                view! {
                    <div>
                        {favorites.into_iter().map(|fav| {
                            let loc = fav.location;
                            let name = fav.name.clone();
                            view! {
                                <div class="box level">
                                    <div class="level-left">
                                        <strong>{name}</strong>
                                    </div>
                                    <div class="level-right">
                                        <button class="button is-danger is-small"
                                            on:click=move |_| set_event.set(Event::Favorites(
                                                Box::new(FavoritesEvent::DeletePressed(loc))
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
                                on:click=move |_| set_event.set(Event::Favorites(
                                    Box::new(FavoritesEvent::DeleteCancelled)
                                ))
                            >
                                {"Cancel"}
                            </button>
                            <button class="button is-danger"
                                on:click=move |_| set_event.set(Event::Favorites(
                                    Box::new(FavoritesEvent::DeleteConfirmed)
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
                on:click=move |_| set_event.set(Event::Navigate(
                    Box::new(shared::Workflow::Home)
                ))
            >
                {"Back"}
            </button>
            <button class="button is-primary"
                on:click=move |_| set_event.set(Event::Navigate(
                    Box::new(shared::Workflow::AddFavorite)
                ))
            >
                {"Add Favorite"}
            </button>
        </div>
    }
}

#[component]
fn add_favorite_view(
    search_results: Option<Vec<GeocodingResponse>>,
    set_event: WriteSignal<Event>,
) -> impl IntoView {
    let (search_text, set_search_text) = signal(String::new());

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
                                set_event.set(Event::Favorites(
                                    Box::new(FavoritesEvent::Search(val))
                                ));
                            }
                        }
                    />
                </div>
            </div>
            {move || {
                search_results.clone().map(|results| {
                    if results.is_empty() {
                        view! { <p>{"No results found"}</p> }.into_any()
                    } else {
                        view! {
                            <div>
                                {results.into_iter().map(|result| {
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
                                                            set_event.set(Event::Favorites(
                                                                Box::new(FavoritesEvent::Submit(Box::new(r)))
                                                            ));
                                                        }
                                                    >
                                                        {"Add"}
                                                    </button>
                                                </div>
                                            </div>
                                        </div>
                                    }
                                }).collect::<Vec<_>>()}
                            </div>
                        }.into_any()
                    }
                })
            }}
        </div>
        <div class="buttons is-centered mt-4">
            <button class="button"
                on:click=move |_| set_event.set(Event::Navigate(
                    Box::new(shared::Workflow::Home)
                ))
            >
                {"Cancel"}
            </button>
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
