use std::sync::LazyLock;

pub static API_KEY: LazyLock<String> = LazyLock::new(|| {
    #[cfg(test)]
    {
        "test_api_key".to_string()
    }
    #[cfg(target_arch = "wasm32")]
    {
        #[allow(clippy::option_env_unwrap)]
        option_env!("OPENWEATHER_API_KEY")
            .expect("OPENWEATHER_API_KEY must be set at compile time for WASM builds")
            .to_string()
    }
    #[cfg(all(not(test), not(target_arch = "wasm32")))]
    {
        use std::env;

        env::var("OPENWEATHER_API_KEY")
            .expect("OPENWEATHER_API_KEY must be set in .env or environment")
    }
});
