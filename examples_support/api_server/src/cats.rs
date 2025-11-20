use axum::{Json, response::IntoResponse};
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
            .choose(&mut rand::rng())
            .unwrap_or_default()
            .to_string(),
    })
}
