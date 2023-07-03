use axum::{response::IntoResponse, Json};
use rand::seq::IteratorRandom;
use serde::Serialize;

const CATS: &str = include_str!("./cat_urls.txt");

#[derive(Serialize)]
struct CatImage {
    pub href: String,
}

pub(crate) async fn get() -> impl IntoResponse {
    Json(CatImage {
        href: CATS
            .lines()
            .choose(&mut rand::thread_rng())
            .unwrap_or_default()
            .to_string(),
    })
}
