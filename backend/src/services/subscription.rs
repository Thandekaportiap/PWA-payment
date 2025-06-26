use serde::{Deserialize, Serialize};
use uuid::Uuid;
use chrono::{DateTime, Utc};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Subscription {
    pub id: Uuid,
    pub user_id: Uuid,
    pub plan_type: String,
    pub status: SubscriptionStatus,
    pub amount: f64,
    pub currency: String,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
    pub activated_at: Option<DateTime<Utc>>,
    pub expires_at: Option<DateTime<Utc>>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum SubscriptionStatus {
    Pending,
    Active,
    Expired,
    Cancelled,
}

impl Subscription {
    pub fn new(user_id: Uuid, plan_type: String, amount: f64, currency: String) -> Self {
        let now = Utc::now();
        Self {
            id: Uuid::new_v4(),
            user_id,
            plan_type,
            status: SubscriptionStatus::Pending,
            amount,
            currency,
            created_at: now,
            updated_at: now,
            activated_at: None,
            expires_at: None,
        }
    }

    pub fn activate(&mut self) {
        self.status = SubscriptionStatus::Active;
        let now = Utc::now();
        self.activated_at = Some(now);
        self.expires_at = Some(now + chrono::Duration::days(30)); // 30 days from activation
        self.updated_at = now;
    }

    pub fn cancel(&mut self) {
        self.status = SubscriptionStatus::Cancelled;
        self.updated_at = Utc::now();
    }

    pub fn expire(&mut self) {
        self.status = SubscriptionStatus::Expired;
        self.updated_at = Utc::now();
    }
}
