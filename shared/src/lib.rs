mod cat_fact;

use anyhow::Result;
pub use cat_fact::*;
use thiserror::Error;
use uniffi::UnexpectedUniFFICallbackError;

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
