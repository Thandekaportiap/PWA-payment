use serde::{Deserialize, Serialize};
use std::env;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Config {
    pub database_url: String,
    pub peach: PeachConfig,
    pub app: AppConfig,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PeachConfig {
    pub auth_service_url: String,
    pub checkout_endpoint: String,
    pub status_endpoint: String,
    pub client_id: String,
    pub client_secret: String,
    pub merchant_id: String,
    pub entity_id: String,
    pub webhook_secret: String,
    pub notification_url: String,
    pub shopper_result_url: String,
    pub origin_domain: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AppConfig {
    pub grace_period_days: u32,
    pub max_renewal_attempts: u32,
    pub notification_days: Vec<u32>, // Days before expiration to send notifications
}

impl Config {
    pub fn from_env() -> Result<Self, env::VarError> {
        Ok(Config {
            database_url: env::var("DATABASE_URL")
                .unwrap_or_else(|_| "file://subscription.db".to_string()),
            
            peach: PeachConfig {
                auth_service_url: env::var("PEACH_AUTH_SERVICE_URL")?,
                checkout_endpoint: env::var("PEACH_CHECKOUT_ENDPOINT")?,
                status_endpoint: env::var("PEACH_STATUS_ENDPOINT")?,
                client_id: env::var("PEACH_CLIENT_ID")?,
                client_secret: env::var("PEACH_CLIENT_SECRET")?,
                merchant_id: env::var("PEACH_MERCHANT_ID")?,
                entity_id: env::var("PEACH_ENTITY_ID")?,
                webhook_secret: env::var("PEACH_WEBHOOK_SECRET")?,
                notification_url: env::var("PEACH_NOTIFICATION_URL")?,
                shopper_result_url: env::var("PEACH_SHOPPER_RESULT_URL")?,
                origin_domain: env::var("PEACH_ORIGIN_DOMAIN")?,
            },
            
            app: AppConfig {
                grace_period_days: env::var("GRACE_PERIOD_DAYS")
                    .unwrap_or_else(|_| "7".to_string())
                    .parse()
                    .unwrap_or(7),
                max_renewal_attempts: env::var("MAX_RENEWAL_ATTEMPTS")
                    .unwrap_or_else(|_| "5".to_string())
                    .parse()
                    .unwrap_or(5),
                notification_days: env::var("NOTIFICATION_DAYS")
                    .unwrap_or_else(|_| "7,3,1".to_string())
                    .split(',')
                    .filter_map(|s| s.trim().parse().ok())
                    .collect(),
            },
        })
    }
}

impl Default for AppConfig {
    fn default() -> Self {
        Self {
            grace_period_days: 7,
            max_renewal_attempts: 5,
            notification_days: vec![7, 3, 1],
        }
    }
}