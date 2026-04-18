use leptos::prelude::*;

use shared::{
    Event,
    effects::http::location::GeocodingResponse,
    model::active::{
        ActiveEvent,
        favorites::{
            FavoritesScreenEvent, add::AddFavoriteEvent, confirm_delete::ConfirmDeleteEvent,
        },
    },
    view::active::favorites::{FavoritesViewModel, FavoritesWorkflowViewModel},
};

#[component]
#[allow(clippy::too_many_lines)]
pub fn favorites_view(model: FavoritesViewModel, set_event: WriteSignal<Event>) -> impl IntoView {
    if let Some(FavoritesWorkflowViewModel::Add(add)) = model.workflow {
        view! {
            <AddFavoriteView model=add set_event=set_event />
        }
        .into_any()
    } else {
        let delete_confirmation = match model.workflow {
            Some(FavoritesWorkflowViewModel::ConfirmDelete { location }) => Some(location),
            _ => None,
        };
        view! {
            <div class="card">
                <p class="section-title">
                    <i class="ph ph-star"></i>
                    {"Favorites"}
                </p>
                {if model.favorites.is_empty() {
                    view! {
                        <div class="status-message">
                            <i class="ph ph-star"></i>
                            <p>{"No favorites yet"}</p>
                        </div>
                    }.into_any()
                } else {
                    view! {
                        <div>
                            {model.favorites.into_iter().map(|fav| {
                                let loc = fav.location;
                                let name = fav.name.clone();
                                view! {
                                    <div class="fav-card" style="justify-content: space-between;">
                                        <span class="fav-name">
                                            <i class="ph ph-map-pin" style="margin-right: 0.25rem;"></i>
                                            {name}
                                        </span>
                                        <button class="button is-danger is-small btn"
                                            on:click=move |_| set_event.set(Event::Active(
                                                ActiveEvent::favorites(
                                                    FavoritesScreenEvent::RequestDelete(loc)
                                                )
                                            ))
                                        >
                                            <span class="icon is-small"><i class="ph ph-trash"></i></span>
                                        </button>
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
                        <div class="card" style="text-align: center;">
                            <i class="ph ph-warning" style="font-size: 2.5rem; color: #ef4444;"></i>
                            <p style="font-weight: 600; font-size: 1.1rem; margin: 0.75rem 0;">{"Delete this favorite?"}</p>
                            <div class="buttons is-centered">
                                <button class="button btn"
                                    on:click=move |_| set_event.set(Event::Active(
                                        ActiveEvent::favorites(
                                            FavoritesScreenEvent::confirm_delete(ConfirmDeleteEvent::Cancelled)
                                        )
                                    ))
                                >
                                    {"Cancel"}
                                </button>
                                <button class="button is-danger btn"
                                    on:click=move |_| set_event.set(Event::Active(
                                        ActiveEvent::favorites(
                                            FavoritesScreenEvent::confirm_delete(ConfirmDeleteEvent::Confirmed)
                                        )
                                    ))
                                >
                                    <span class="icon"><i class="ph ph-trash"></i></span>
                                    <span>{"Delete"}</span>
                                </button>
                            </div>
                        </div>
                    </div>
                </div>
            })}
            <div class="buttons is-centered" style="margin-top: 1rem;">
                <button class="button btn"
                    on:click=move |_| set_event.set(Event::Active(
                        ActiveEvent::favorites(FavoritesScreenEvent::GoToHome)
                    ))
                >
                    <span class="icon"><i class="ph ph-arrow-left"></i></span>
                    <span>{"Back"}</span>
                </button>
                <button class="button is-primary btn"
                    on:click=move |_| set_event.set(Event::Active(
                        ActiveEvent::favorites(FavoritesScreenEvent::RequestAddFavorite)
                    ))
                >
                    <span class="icon"><i class="ph ph-plus"></i></span>
                    <span>{"Add Favorite"}</span>
                </button>
            </div>
        }.into_any()
    }
}

#[component]
fn add_favorite_view(
    model: shared::view::active::favorites::AddFavoriteViewModel,
    set_event: WriteSignal<Event>,
) -> impl IntoView {
    let (search_text, set_search_text) = signal(model.search_input.clone());

    view! {
        <div class="card">
            <p class="section-title">
                <i class="ph ph-magnifying-glass"></i>
                {"Add Favorite"}
            </p>
            <div class="field">
                <div class="control has-icons-left">
                    <input
                        class="input"
                        type="text"
                        placeholder="Search for a city..."
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
                    <span class="icon is-left">
                        <i class="ph ph-magnifying-glass"></i>
                    </span>
                </div>
            </div>
            {move || {
                model.search_results.clone().map(|results| {
                    if results.is_empty() {
                        view! {
                            <div class="status-message">
                                <i class="ph ph-map-pin-line"></i>
                                <p>{"No results found"}</p>
                            </div>
                        }.into_any()
                    } else {
                        view! {
                            <div>
                                {results.into_iter().map(|result| {
                                    search_result_item(&result, set_event)
                                }).collect::<Vec<_>>()}
                            </div>
                        }.into_any()
                    }
                })
            }}
        </div>
        <div class="buttons is-centered" style="margin-top: 1rem;">
            <button class="button btn"
                on:click=move |_| set_event.set(Event::Active(
                    ActiveEvent::favorites(
                        FavoritesScreenEvent::add(AddFavoriteEvent::Cancel)
                    )
                ))
            >
                <span class="icon"><i class="ph ph-arrow-left"></i></span>
                <span>{"Cancel"}</span>
            </button>
        </div>
    }
}

fn search_result_item(
    result: &GeocodingResponse,
    set_event: WriteSignal<Event>,
) -> impl IntoView + use<> {
    let name = result.name.clone();
    let country = result.country.clone();
    let state = result.state.clone();
    let r = result.clone();
    view! {
        <div class="fav-card" style="justify-content: space-between;">
            <div>
                <span class="fav-name">
                    <i class="ph ph-map-pin" style="margin-right: 0.25rem;"></i>
                    {name}
                </span>
                <br/>
                <small style="color: #9ca3af;">{
                    state.map(|s| format!("{s}, {country}"))
                        .unwrap_or(country)
                }</small>
            </div>
            <button class="button is-primary is-small btn"
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
                <span class="icon is-small"><i class="ph ph-plus"></i></span>
                <span>{"Add"}</span>
            </button>
        </div>
    }
}
