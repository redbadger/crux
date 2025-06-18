use once_cell::sync::Lazy;

pub static API_KEY: Lazy<String> = Lazy::new(|| {
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
