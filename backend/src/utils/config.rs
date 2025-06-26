use serde::Deserialize;

#[derive(Debug, Clone, Deserialize)]
pub struct AppConfig {
    pub app_base_url: String,
    pub peach_entity_id: String,
    pub peach_secret_key: String,
    pub peach_access_token: String,
    pub peach_api_url: String,
    pub peach_checkout_type: String,
    pub peach_region: String,
}

impl AppConfig {
    pub fn from_env() -> Result<Self, Box<dyn std::error::Error>> {
        Ok(AppConfig {
            app_base_url: std::env::var("APP_BASE_URL")
                .unwrap_or_else(|_| "http://localhost:8080".to_string()),
            peach_entity_id: std::env::var("PEACH_ENTITY_ID")?,
            peach_secret_key: std::env::var("PEACH_SECRET_KEY")?,
            peach_access_token: std::env::var("PEACH_ACCESS_TOKEN")?,
            peach_api_url: std::env::var("PEACH_API_URL")?,
            peach_checkout_type: std::env::var("PEACH_CHECKOUT_TYPE")
                .unwrap_or_else(|_| "hosted".to_string()),
            peach_region: std::env::var("PEACH_REGION")
                .unwrap_or_else(|_| "ZA".to_string()),
        })
    }
}
