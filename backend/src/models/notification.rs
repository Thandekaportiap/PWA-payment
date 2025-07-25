use serde::{Deserialize, Serialize};
use chrono::{DateTime, Utc};
use surrealdb::sql::Thing;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Notification {
    pub id: Thing,
    pub user_id: String,
    pub subscription_id: String,
    pub message: String,
    pub acknowledged: bool,
    pub created_at: DateTime<Utc>,
}

#[derive(Debug, Deserialize)]
pub struct CreateNotificationDto {
    pub user_id: String,
    pub subscription_id: String,
    pub message: String,
}

#[derive(Debug, Deserialize)]
pub struct UpdateNotificationDto {
    pub acknowledged: Option<bool>,
    pub message: Option<String>,
}
