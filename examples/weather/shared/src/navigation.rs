#![allow(clippy::unused_self)]
use std::marker::PhantomData;

use crate::app::Workflow;

#[derive(Debug, Default, Clone)]
pub struct Home;
#[derive(Debug, Default, Clone)]
pub struct Favorites;
#[derive(Debug, Default, Clone)]
pub struct AddFavorite;

#[derive(Debug, Default, Clone)]
pub struct Page<P> {
    _state: PhantomData<P>,
}

impl Default for CurrentPage {
    fn default() -> Self {
        Self::Home(Page::default())
    }
}

impl Page<Home> {
    pub fn favorites(self) -> Page<Favorites> {
        Page::default()
    }
}

impl Page<Favorites> {
    pub fn add_favorite(self) -> Page<AddFavorite> {
        Page::default()
    }
}

impl<T> Page<T> {
    pub fn home(self) -> Page<Home> {
        Page::default()
    }
}

#[derive(Debug, Clone)]
pub enum CurrentPage {
    Home(Page<Home>),
    Favorites(Page<Favorites>),
    AddFavorite(Page<AddFavorite>),
}

pub fn navigate(page: &mut CurrentPage, next: &Workflow) {
    let current = page.clone();
    let next = match next {
        Workflow::Home => match current {
            CurrentPage::Home(p) => CurrentPage::Home(p),
            CurrentPage::Favorites(p) => CurrentPage::Home(p.home()),
            CurrentPage::AddFavorite(p) => CurrentPage::Home(p.home()),
        },
        Workflow::Favorites(_) => match current {
            CurrentPage::Home(p) => CurrentPage::Favorites(p.favorites()),
            CurrentPage::Favorites(p) => CurrentPage::Favorites(p),
            CurrentPage::AddFavorite(_p) => unimplemented!(),
        },
        Workflow::AddFavorite => match current {
            CurrentPage::Home(_p) => unimplemented!(),
            CurrentPage::Favorites(p) => CurrentPage::AddFavorite(p.add_favorite()),
            CurrentPage::AddFavorite(p) => CurrentPage::AddFavorite(p),
        },
    };
    *page = next;
}
