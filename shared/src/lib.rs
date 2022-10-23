use anyhow::Result;
use futures::executor::block_on;
use serde::Deserialize;
use thiserror::Error;
use uniffi::UnexpectedUniFFICallbackError;

const API_URL: &str = "https://catfact.ninja/fact";

#[derive(Error, Debug)]
pub enum PlatformError {
    #[error("InternalPlatformError")]
    InternalPlatformError,
}

impl From<UnexpectedUniFFICallbackError> for PlatformError {
    fn from(_: UnexpectedUniFFICallbackError) -> Self {
        Self::InternalPlatformError
    }
}

pub trait Platform: Send + Sync {
    fn get(&self) -> Result<String, PlatformError>;
}

pub fn add_for_platform(
    left: u32,
    right: u32,
    platform: Box<dyn Platform>,
) -> Result<String, PlatformError> {
    Ok(format!(
        "Platform {}, {left} + {right} = {}",
        platform.get()?,
        left + right
    ))
}

#[derive(Deserialize, Default)]
struct Cat {
    fact: String,
}

#[cfg(target_family = "wasm")]
pub async fn cat_fact_async() -> String {
    let cat = cat_fact_impl().await.unwrap();
    cat.fact
}

#[cfg(target_family = "wasm")]
async fn cat_fact_impl() -> Result<Cat> {
    Ok(gloo_net::http::Request::get(API_URL)
        .send()
        .await?
        .json::<Cat>()
        .await?)
}

pub fn cat_fact() -> String {
    let cat = block_on(async { cat_fact_impl().await }).unwrap();
    cat.fact
}

#[cfg(not(target_family = "wasm"))]
async fn cat_fact_impl() -> Result<Cat> {
    Ok(ureq::get(API_URL).call()?.into_json::<Cat>()?)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn add_for_platform_works_for_ios() {
        struct IosPlatform;
        impl Platform for IosPlatform {
            fn get(&self) -> Result<String, PlatformError> {
                Ok("iOS".to_string())
            }
        }
        let result = add_for_platform(2, 2, Box::new(IosPlatform {})).unwrap();
        assert_eq!(result, "Platform iOS, 2 + 2 = 4");
    }
}

uniffi_macros::include_scaffolding!("shared");
