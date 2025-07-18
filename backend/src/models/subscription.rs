use serde::{Deserialize, Serialize};
use chrono::{DateTime, Utc};
use crate::models::payment::PaymentMethod; 

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CreateSubscriptionDto {
    pub user_id: String,
    pub plan_name: String,
    pub price: f64,
    pub payment_method: Option<PaymentMethod>, 
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Subscription {
     pub id: String,  // Changed from Uuid to String
    pub user_id: String,
    pub plan_name: String,
    pub price: f64,
    pub status: SubscriptionStatus,
     pub payment_method: Option<PaymentMethod>, // ✅ Add this
      pub payment_brand: Option<String>,
    pub start_date: Option<DateTime<Utc>>,
    pub end_date: Option<DateTime<Utc>>,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum SubscriptionStatus {
    Pending,
    Active,
    Expired,
    Cancelled,
    Suspended,
}
