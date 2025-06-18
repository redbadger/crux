use std::sync::LazyLock;

pub static API_KEY: LazyLock<String> = LazyLock::new(|| {
    #[cfg(test)]
    {
        "test_api_key".to_string()
    }
    #[cfg(not(test))]
    {
        use std::env;

        env::var("OPENWEATHER_API_KEY")
            .expect("OPENWEATHER_API_KEY must be set in .env or environment")
    }
});
