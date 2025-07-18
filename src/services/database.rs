use surrealdb::{Surreal, engine::local::{Db, File, Mem}, sql::Thing, Response};
use serde::{Deserialize, Serialize};
use uuid::Uuid;
use anyhow::{Result, anyhow};
use chrono::{DateTime, Utc};

use crate::models::{
    user::{User, CreateUserRequest},
    payment::{Payment, InitiatePaymentRequest, PaymentStatus},
    subscription::{Subscription, CreateSubscriptionRequest, SubscriptionStatus},
    common::{PaginationQuery, PaginatedResponse},
};

#[derive(Clone)]
pub struct DatabaseService {
    db: Surreal<Db>,
}

impl DatabaseService {
    pub async fn new(database_url: &str) -> Result<Self> {
        let db = if database_url.starts_with("memory://") {
            Surreal::new::<Mem>(()).await?
        } else if database_url.starts_with("file://") {
            let path = database_url.strip_prefix("file://").unwrap_or("subscription.db");
            Surreal::new::<File>(path).await?
        } else {
            return Err(anyhow!("Unsupported database URL: {}", database_url));
        };

        // Use namespace and database
        db.use_ns("subscription_app").use_db("main").await?;

        let service = Self { db };
        service.initialize_schema().await?;
        
        Ok(service)
    }

    async fn initialize_schema(&self) -> Result<()> {
        // Create users table with unique email index
        self.db.query("
            DEFINE TABLE users SCHEMAFULL;
            DEFINE FIELD id ON users TYPE record(users);
            DEFINE FIELD name ON users TYPE string;
            DEFINE FIELD email ON users TYPE string;
            DEFINE FIELD is_active ON users TYPE bool DEFAULT true;
            DEFINE FIELD email_verified ON users TYPE bool DEFAULT false;
            DEFINE FIELD created_at ON users TYPE datetime;
            DEFINE FIELD updated_at ON users TYPE datetime;
            DEFINE INDEX unique_email ON users COLUMNS email UNIQUE;
        ").await?;

        // Create payments table
        self.db.query("
            DEFINE TABLE payments SCHEMAFULL;
            DEFINE FIELD id ON payments TYPE record(payments);
            DEFINE FIELD user_id ON payments TYPE record(users);
            DEFINE FIELD subscription_id ON payments TYPE option<record(subscriptions)>;
            DEFINE FIELD merchant_transaction_id ON payments TYPE string;
            DEFINE FIELD peach_checkout_id ON payments TYPE option<string>;
            DEFINE FIELD peach_payment_id ON payments TYPE option<string>;
            DEFINE FIELD amount ON payments TYPE decimal;
            DEFINE FIELD currency ON payments TYPE string;
            DEFINE FIELD payment_method ON payments TYPE string;
            DEFINE FIELD payment_type ON payments TYPE string;
            DEFINE FIELD status ON payments TYPE string;
            DEFINE FIELD failure_reason ON payments TYPE option<string>;
            DEFINE FIELD recurring_token ON payments TYPE option<string>;
            DEFINE FIELD enable_recurring ON payments TYPE bool DEFAULT false;
            DEFINE FIELD retry_count ON payments TYPE int DEFAULT 0;
            DEFINE FIELD metadata ON payments TYPE option<object>;
            DEFINE FIELD created_at ON payments TYPE datetime;
            DEFINE FIELD updated_at ON payments TYPE datetime;
            DEFINE FIELD completed_at ON payments TYPE option<datetime>;
            DEFINE INDEX unique_merchant_txn ON payments COLUMNS merchant_transaction_id UNIQUE;
        ").await?;

        // Create subscriptions table
        self.db.query("
            DEFINE TABLE subscriptions SCHEMAFULL;
            DEFINE FIELD id ON subscriptions TYPE record(subscriptions);
            DEFINE FIELD user_id ON subscriptions TYPE record(users);
            DEFINE FIELD plan ON subscriptions TYPE string;
            DEFINE FIELD status ON subscriptions TYPE string;
            DEFINE FIELD price ON subscriptions TYPE decimal;
            DEFINE FIELD currency ON subscriptions TYPE string;
            DEFINE FIELD start_date ON subscriptions TYPE option<datetime>;
            DEFINE FIELD end_date ON subscriptions TYPE option<datetime>;
            DEFINE FIELD grace_end_date ON subscriptions TYPE option<datetime>;
            DEFINE FIELD billing_cycle_anchor ON subscriptions TYPE option<datetime>;
            DEFINE FIELD renewal_attempts ON subscriptions TYPE int DEFAULT 0;
            DEFINE FIELD max_renewal_attempts ON subscriptions TYPE int DEFAULT 5;
            DEFINE FIELD auto_renew ON subscriptions TYPE bool DEFAULT true;
            DEFINE FIELD paused_at ON subscriptions TYPE option<datetime>;
            DEFINE FIELD metadata ON subscriptions TYPE option<object>;
            DEFINE FIELD created_at ON subscriptions TYPE datetime;
            DEFINE FIELD updated_at ON subscriptions TYPE datetime;
        ").await?;

        log::info!("Database schema initialized successfully");
        Ok(())
    }

    // User operations
    pub async fn create_user(&self, request: CreateUserRequest) -> Result<User> {
        // Check if user with email already exists
        let existing: Option<User> = self.db
            .query("SELECT * FROM users WHERE email = $email")
            .bind(("email", &request.email))
            .await?
            .take(0)?;

        if existing.is_some() {
            return Err(anyhow!("User with email {} already exists", request.email));
        }

        let user = User::new(request.name, request.email);
        let user_id = format!("users:{}", user.id);
        
        let created: Option<User> = self.db
            .create(&user_id)
            .content(&user)
            .await?;

        created.ok_or_else(|| anyhow!("Failed to create user"))
    }

    pub async fn get_user(&self, user_id: &Uuid) -> Result<Option<User>> {
        let user_id = format!("users:{}", user_id);
        let user: Option<User> = self.db.select(&user_id).await?;
        Ok(user)
    }

    pub async fn get_user_by_email(&self, email: &str) -> Result<Option<User>> {
        let user: Option<User> = self.db
            .query("SELECT * FROM users WHERE email = $email")
            .bind(("email", email))
            .await?
            .take(0)?;
        Ok(user)
    }

    pub async fn update_user(&self, user_id: &Uuid, user: &User) -> Result<User> {
        let user_id = format!("users:{}", user_id);
        let updated: Option<User> = self.db
            .update(&user_id)
            .content(user)
            .await?;

        updated.ok_or_else(|| anyhow!("Failed to update user"))
    }

    // Payment operations
    pub async fn create_payment(&self, request: InitiatePaymentRequest, merchant_transaction_id: String) -> Result<Payment> {
        let payment = Payment::new(request, merchant_transaction_id);
        let payment_id = format!("payments:{}", payment.id);
        
        let created: Option<Payment> = self.db
            .create(&payment_id)
            .content(&payment)
            .await?;

        created.ok_or_else(|| anyhow!("Failed to create payment"))
    }

    pub async fn get_payment(&self, payment_id: &Uuid) -> Result<Option<Payment>> {
        let payment_id = format!("payments:{}", payment_id);
        let payment: Option<Payment> = self.db.select(&payment_id).await?;
        Ok(payment)
    }

    pub async fn get_payment_by_merchant_id(&self, merchant_transaction_id: &str) -> Result<Option<Payment>> {
        let payment: Option<Payment> = self.db
            .query("SELECT * FROM payments WHERE merchant_transaction_id = $merchant_id")
            .bind(("merchant_id", merchant_transaction_id))
            .await?
            .take(0)?;
        Ok(payment)
    }

    pub async fn update_payment(&self, payment_id: &Uuid, payment: &Payment) -> Result<Payment> {
        let payment_id = format!("payments:{}", payment_id);
        let updated: Option<Payment> = self.db
            .update(&payment_id)
            .content(payment)
            .await?;

        updated.ok_or_else(|| anyhow!("Failed to update payment"))
    }

    pub async fn update_payment_status(&self, payment_id: &Uuid, status: PaymentStatus, failure_reason: Option<String>) -> Result<()> {
        let payment_id = format!("payments:{}", payment_id);
        let now = Utc::now();
        
        let mut query = "UPDATE $payment_id SET status = $status, updated_at = $updated_at".to_string();
        
        if let Some(reason) = &failure_reason {
            query.push_str(", failure_reason = $failure_reason");
        }
        
        if matches!(status, PaymentStatus::Completed | PaymentStatus::Failed | PaymentStatus::Cancelled) {
            query.push_str(", completed_at = $completed_at");
        }

        let mut db_query = self.db
            .query(&query)
            .bind(("payment_id", payment_id))
            .bind(("status", format!("{:?}", status)))
            .bind(("updated_at", now));
            
        if let Some(reason) = failure_reason {
            db_query = db_query.bind(("failure_reason", reason));
        }
        
        if matches!(status, PaymentStatus::Completed | PaymentStatus::Failed | PaymentStatus::Cancelled) {
            db_query = db_query.bind(("completed_at", now));
        }

        db_query.await?;
        Ok(())
    }

    pub async fn get_payments_by_user(&self, user_id: &Uuid, pagination: Option<PaginationQuery>) -> Result<PaginatedResponse<Payment>> {
        let pagination = pagination.unwrap_or_default();
        let page = pagination.page.unwrap_or(1);
        let limit = pagination.limit.unwrap_or(20);
        let offset = (page - 1) * limit;

        let user_id = format!("users:{}", user_id);
        
        // Get total count
        let total_result: Vec<serde_json::Value> = self.db
            .query("SELECT count() FROM payments WHERE user_id = $user_id GROUP ALL")
            .bind(("user_id", &user_id))
            .await?
            .take(0)?;
        
        let total = total_result.first()
            .and_then(|v| v.get("count"))
            .and_then(|v| v.as_u64())
            .unwrap_or(0) as u32;

        // Get paginated data
        let payments: Vec<Payment> = self.db
            .query("SELECT * FROM payments WHERE user_id = $user_id ORDER BY created_at DESC LIMIT $limit START $offset")
            .bind(("user_id", user_id))
            .bind(("limit", limit))
            .bind(("offset", offset))
            .await?
            .take(0)?;

        Ok(PaginatedResponse {
            data: payments,
            total,
            page,
            limit,
            total_pages: (total + limit - 1) / limit,
        })
    }

    // Subscription operations
    pub async fn create_subscription(&self, request: CreateSubscriptionRequest) -> Result<Subscription> {
        let subscription = Subscription::new(request);
        let subscription_id = format!("subscriptions:{}", subscription.id);
        
        let created: Option<Subscription> = self.db
            .create(&subscription_id)
            .content(&subscription)
            .await?;

        created.ok_or_else(|| anyhow!("Failed to create subscription"))
    }

    pub async fn get_subscription(&self, subscription_id: &Uuid) -> Result<Option<Subscription>> {
        let subscription_id = format!("subscriptions:{}", subscription_id);
        let subscription: Option<Subscription> = self.db.select(&subscription_id).await?;
        Ok(subscription)
    }

    pub async fn get_subscriptions_by_user(&self, user_id: &Uuid) -> Result<Vec<Subscription>> {
        let user_id = format!("users:{}", user_id);
        let subscriptions: Vec<Subscription> = self.db
            .query("SELECT * FROM subscriptions WHERE user_id = $user_id ORDER BY created_at DESC")
            .bind(("user_id", user_id))
            .await?
            .take(0)?;
        Ok(subscriptions)
    }

    pub async fn get_active_subscription_by_user(&self, user_id: &Uuid) -> Result<Option<Subscription>> {
        let user_id = format!("users:{}", user_id);
        let subscription: Option<Subscription> = self.db
            .query("SELECT * FROM subscriptions WHERE user_id = $user_id AND status IN ['Active', 'Grace'] ORDER BY created_at DESC LIMIT 1")
            .bind(("user_id", user_id))
            .await?
            .take(0)?;
        Ok(subscription)
    }

    pub async fn update_subscription(&self, subscription_id: &Uuid, subscription: &Subscription) -> Result<Subscription> {
        let subscription_id = format!("subscriptions:{}", subscription_id);
        let updated: Option<Subscription> = self.db
            .update(&subscription_id)
            .content(subscription)
            .await?;

        updated.ok_or_else(|| anyhow!("Failed to update subscription"))
    }

    pub async fn get_expiring_subscriptions(&self, days_ahead: i64) -> Result<Vec<Subscription>> {
        let target_date = Utc::now() + chrono::Duration::days(days_ahead);
        
        let subscriptions: Vec<Subscription> = self.db
            .query("SELECT * FROM subscriptions WHERE status = 'Active' AND end_date <= $target_date AND auto_renew = true")
            .bind(("target_date", target_date))
            .await?
            .take(0)?;
        
        Ok(subscriptions)
    }

    pub async fn get_grace_period_subscriptions(&self) -> Result<Vec<Subscription>> {
        let subscriptions: Vec<Subscription> = self.db
            .query("SELECT * FROM subscriptions WHERE status = 'Grace'")
            .await?
            .take(0)?;
        
        Ok(subscriptions)
    }

    // Utility methods
    pub async fn health_check(&self) -> Result<()> {
        self.db.health().await?;
        Ok(())
    }

    pub async fn get_statistics(&self) -> Result<DatabaseStats> {
        let user_count: Vec<serde_json::Value> = self.db
            .query("SELECT count() FROM users GROUP ALL")
            .await?
            .take(0)?;
        
        let payment_count: Vec<serde_json::Value> = self.db
            .query("SELECT count() FROM payments GROUP ALL")
            .await?
            .take(0)?;
        
        let subscription_count: Vec<serde_json::Value> = self.db
            .query("SELECT count() FROM subscriptions GROUP ALL")
            .await?
            .take(0)?;
        
        let active_subscriptions: Vec<serde_json::Value> = self.db
            .query("SELECT count() FROM subscriptions WHERE status = 'Active' GROUP ALL")
            .await?
            .take(0)?;

        Ok(DatabaseStats {
            total_users: extract_count(&user_count),
            total_payments: extract_count(&payment_count),
            total_subscriptions: extract_count(&subscription_count),
            active_subscriptions: extract_count(&active_subscriptions),
        })
    }
}

#[derive(Debug, Serialize)]
pub struct DatabaseStats {
    pub total_users: u64,
    pub total_payments: u64,
    pub total_subscriptions: u64,
    pub active_subscriptions: u64,
}

fn extract_count(result: &[serde_json::Value]) -> u64 {
    result.first()
        .and_then(|v| v.get("count"))
        .and_then(|v| v.as_u64())
        .unwrap_or(0)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::models::common::SubscriptionPlan;

    #[tokio::test]
    async fn test_user_operations() {
        let db = DatabaseService::new("memory://").await.unwrap();
        
        let request = CreateUserRequest {
            name: "John Doe".to_string(),
            email: "john@example.com".to_string(),
        };
        
        let user = db.create_user(request).await.unwrap();
        assert_eq!(user.name, "John Doe");
        assert_eq!(user.email, "john@example.com");
        
        let retrieved = db.get_user(&user.id).await.unwrap();
        assert!(retrieved.is_some());
        assert_eq!(retrieved.unwrap().email, "john@example.com");
    }

    #[tokio::test]
    async fn test_subscription_operations() {
        let db = DatabaseService::new("memory://").await.unwrap();
        
        // Create a user first
        let user_request = CreateUserRequest {
            name: "Jane Doe".to_string(),
            email: "jane@example.com".to_string(),
        };
        let user = db.create_user(user_request).await.unwrap();
        
        // Create subscription
        let sub_request = CreateSubscriptionRequest {
            user_id: user.id,
            plan: SubscriptionPlan::Monthly,
            auto_renew: Some(true),
            billing_cycle_anchor: None,
            metadata: None,
        };
        
        let subscription = db.create_subscription(sub_request).await.unwrap();
        assert_eq!(subscription.user_id, user.id);
        assert_eq!(subscription.plan, SubscriptionPlan::Monthly);
        
        let retrieved = db.get_subscription(&subscription.id).await.unwrap();
        assert!(retrieved.is_some());
    }
}