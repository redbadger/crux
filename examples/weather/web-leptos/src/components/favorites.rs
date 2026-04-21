use leptos::callback::UnsyncCallback;
use leptos::prelude::*;
use phosphor_leptos::{
    ARROW_LEFT, Icon, MAGNIFYING_GLASS, MAP_PIN, MAP_PIN_LINE, PLUS, STAR, TRASH, WARNING,
};

use shared::{
    Event,
    effects::http::location::GeocodingResponse,
    model::active::{
        ActiveEvent,
        favorites::{
            FavoritesScreenEvent, add::AddFavoriteEvent, confirm_delete::ConfirmDeleteEvent,
        },
    },
    view::active::favorites::{
        AddFavoriteViewModel, FavoritesViewModel, FavoritesWorkflowViewModel,
    },
};

use super::{
    SendEvent,
    common::{
        Button, ButtonVariant, Card, IconButton, IconButtonVariant, Modal, SectionTitle,
        StatusMessage, TextField,
    },
    use_dispatch,
};

#[component]
pub fn favorites_view(#[prop(into)] vm: Signal<FavoritesViewModel>) -> impl IntoView {
    // Project the workflow slice — `Some(Add)` drives the add sub-screen,
    // `Some(ConfirmDelete)` drives the modal, `None` renders the list.
    let workflow = Memo::new(move |_| vm.with(|v| v.workflow.clone()));
    let add_workflow = Memo::new(move |_| {
        workflow.with(|w| match w {
            Some(FavoritesWorkflowViewModel::Add(add)) => add.clone(),
            _ => AddFavoriteViewModel::default(),
        })
    });

    view! {
        <Show when=move || matches!(workflow.read().as_ref(), Some(FavoritesWorkflowViewModel::Add(_)))>
            <AddFavoriteView vm=add_workflow />
        </Show>
        <Show when=move || !matches!(workflow.read().as_ref(), Some(FavoritesWorkflowViewModel::Add(_)))>
            <FavoritesList vm=vm />
        </Show>
    }
}

#[component]
fn favorites_list(#[prop(into)] vm: Signal<FavoritesViewModel>) -> impl IntoView {
    let dispatch = use_dispatch();

    view! {
        <Card class="mb-4">
            <SectionTitle icon=STAR title="Favourites" />
            {move || {
                let favorites = vm.with(|v| v.favorites.clone());
                if favorites.is_empty() {
                    view! {
                        <StatusMessage icon=STAR message="No favourites yet" />
                    }.into_any()
                } else {
                    view! {
                        <div class="grid gap-2">
                            {favorites.into_iter().map(|fav| {
                                let loc = fav.location;
                                let name = fav.name.clone();
                                view! {
                                    <div class="bg-slate-50 rounded-xl px-4 py-3 flex items-center justify-between gap-4">
                                        <span class="font-semibold text-slate-900 flex items-center gap-1">
                                            <Icon icon=MAP_PIN size="16px" />
                                            {name}
                                        </span>
                                        <IconButton
                                            icon=TRASH
                                            variant=IconButtonVariant::Danger
                                            aria_label="Delete favourite"
                                            on_click=UnsyncCallback::new(move |()| {
                                                dispatch.run(Event::Active(
                                                    ActiveEvent::favorites(
                                                        FavoritesScreenEvent::RequestDelete(loc),
                                                    ),
                                                ));
                                            })
                                        />
                                    </div>
                                }
                            }).collect::<Vec<_>>()}
                        </div>
                    }.into_any()
                }
            }}
        </Card>
        <Show when=move || matches!(vm.read().workflow.as_ref(), Some(FavoritesWorkflowViewModel::ConfirmDelete { .. }))>
            <Modal>
                <Card>
                    <div class="flex flex-col items-center text-center gap-4">
                        <span class="text-red-500">
                            <Icon icon=WARNING size="40px" />
                        </span>
                        <p class="text-slate-900 font-semibold text-lg">"Delete this favourite?"</p>
                        <div class="flex gap-2">
                            <Button
                                label="Cancel"
                                variant=ButtonVariant::Secondary
                                on_click=UnsyncCallback::new(move |()| {
                                    dispatch.run(Event::Active(
                                        ActiveEvent::favorites(
                                            FavoritesScreenEvent::confirm_delete(ConfirmDeleteEvent::Cancelled),
                                        ),
                                    ));
                                })
                            />
                            <Button
                                label="Delete"
                                icon=TRASH
                                variant=ButtonVariant::Danger
                                on_click=UnsyncCallback::new(move |()| {
                                    dispatch.run(Event::Active(
                                        ActiveEvent::favorites(
                                            FavoritesScreenEvent::confirm_delete(ConfirmDeleteEvent::Confirmed),
                                        ),
                                    ));
                                })
                            />
                        </div>
                    </div>
                </Card>
            </Modal>
        </Show>
        <div class="flex justify-center gap-2 mt-4">
            <Button
                label="Back"
                icon=ARROW_LEFT
                variant=ButtonVariant::Secondary
                on_click=UnsyncCallback::new(move |()| {
                    dispatch.run(Event::Active(
                        ActiveEvent::favorites(FavoritesScreenEvent::GoToHome),
                    ));
                })
            />
            <Button
                label="Add Favourite"
                icon=PLUS
                on_click=UnsyncCallback::new(move |()| {
                    dispatch.run(Event::Active(
                        ActiveEvent::favorites(FavoritesScreenEvent::RequestAddFavorite),
                    ));
                })
            />
        </div>
    }
}

#[component]
fn add_favorite_view(#[prop(into)] vm: Signal<AddFavoriteViewModel>) -> impl IntoView {
    let dispatch = use_dispatch();
    let (search_text, set_search_text) = signal(vm.with(|v| v.search_input.clone()));

    view! {
        <Card class="mb-4">
            <SectionTitle icon=MAGNIFYING_GLASS title="Add Favourite" />
            <div class="mb-4">
                <TextField
                    value=search_text
                    placeholder="Search for a city..."
                    icon=MAGNIFYING_GLASS
                    on_input=UnsyncCallback::new(move |val: String| {
                        set_search_text.set(val.clone());
                        if !val.is_empty() {
                            dispatch.run(Event::Active(
                                ActiveEvent::favorites(
                                    FavoritesScreenEvent::add(AddFavoriteEvent::Search(val)),
                                ),
                            ));
                        }
                    })
                />
            </div>
            {move || {
                let results = vm.with(|v| v.search_results.clone());
                results.map(|results| {
                    if results.is_empty() {
                        view! {
                            <StatusMessage icon=MAP_PIN_LINE message="No results found" />
                        }.into_any()
                    } else {
                        view! {
                            <div class="grid gap-2">
                                {results.into_iter().map(|result| {
                                    search_result_item(&result, dispatch)
                                }).collect::<Vec<_>>()}
                            </div>
                        }.into_any()
                    }
                })
            }}
        </Card>
        <div class="flex justify-center gap-2 mt-4">
            <Button
                label="Cancel"
                icon=ARROW_LEFT
                variant=ButtonVariant::Secondary
                on_click=UnsyncCallback::new(move |()| {
                    dispatch.run(Event::Active(
                        ActiveEvent::favorites(
                            FavoritesScreenEvent::add(AddFavoriteEvent::Cancel),
                        ),
                    ));
                })
            />
        </div>
    }
}

fn search_result_item(result: &GeocodingResponse, dispatch: SendEvent) -> impl IntoView + use<> {
    let name = result.name.clone();
    let country = result.country.clone();
    let state = result.state.clone();
    let r = result.clone();
    view! {
        <div class="bg-slate-50 rounded-xl px-4 py-3 flex items-center justify-between gap-4">
            <div>
                <div class="font-semibold text-slate-900 flex items-center gap-1">
                    <Icon icon=MAP_PIN size="16px" />
                    {name}
                </div>
                <small class="text-slate-500">{
                    state.map(|s| format!("{s}, {country}"))
                        .unwrap_or(country)
                }</small>
            </div>
            <Button
                label="Add"
                icon=PLUS
                on_click=UnsyncCallback::new(move |()| {
                    let r = r.clone();
                    dispatch.run(Event::Active(
                        ActiveEvent::favorites(
                            FavoritesScreenEvent::add(
                                AddFavoriteEvent::Submit(Box::new(r)),
                            ),
                        ),
                    ));
                })
            />
        </div>
    }
}
