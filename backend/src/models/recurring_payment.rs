use chrono::{DateTime, Utc};
use serde::{Serialize, Deserialize}; // Import Serialize and Deserialize

#[derive(Clone, Debug, Serialize, Deserialize)] // Added Serialize and Deserialize derives
pub struct RecurringPayment {
    pub id: String,
    pub user_id: String,
    pub subscription_id: String,
    pub recurring_token: String, // token from payment provider to charge future payments
    pub card_last_four: Option<String>,
    pub card_brand: Option<String>,
    pub status: RecurringPaymentStatus,
   
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)] // Added Serialize and Deserialize derives
pub enum RecurringPaymentStatus {
    Active,
    Cancelled,
    Failed,
}