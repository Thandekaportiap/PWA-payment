use std::sync::Arc;
use chrono::{Utc, Duration};
use uuid::Uuid;
use surrealdb::{Surreal, engine::remote::http::Client};
use crate::models::{
    user::{User, CreateUserDto},
    payment::{Payment, CreatePaymentDto, PaymentStatus, PaymentMethod},
    subscription::{Subscription, CreateSubscriptionDto, SubscriptionStatus},
    recurring_payment::{RecurringPayment, RecurringPaymentStatus},
};

#[derive(Clone)]
pub struct DatabaseService {
    pub db: Arc<Surreal<Client>>,
}

impl DatabaseService {
    pub async fn new() -> Result<Self, Box<dyn std::error::Error>> {
        // Connect to SurrealDB using HTTP client (not WebSocket)
        let db = Surreal::new::<surrealdb::engine::remote::http::Http>("127.0.0.1:8000").await?;
        
        // Sign in with root credentials
        db.signin(surrealdb::opt::auth::Root {
            username: "root",
            password: "root",
        }).await?;
        
        // Use namespace and database
        db.use_ns("payment_system").use_db("main").await?;
        
        // Initialize database schema
        Self::init_schema(&db).await?;
        
        Ok(Self {
            db: Arc::new(db),
        })
    }
    
    async fn init_schema(db: &Surreal<Client>) -> Result<(), Box<dyn std::error::Error>> {
        // Create tables and define schema
        let queries = vec![
            // Users table
            "DEFINE TABLE users SCHEMAFULL;",
            "DEFINE FIELD id ON users TYPE string;",
            "DEFINE FIELD email ON users TYPE string;",
            "DEFINE FIELD name ON users TYPE string;",
            "DEFINE FIELD created_at ON users TYPE datetime;",
            "DEFINE FIELD updated_at ON users TYPE datetime;",
            "DEFINE INDEX unique_email ON users COLUMNS email UNIQUE;",
            
            // Payments table
            "DEFINE TABLE payments SCHEMAFULL;",
            "DEFINE FIELD id ON payments TYPE string;",
            "DEFINE FIELD user_id ON payments TYPE string;",
            "DEFINE FIELD subscription_id ON payments TYPE option<string>;",
            "DEFINE FIELD amount ON payments TYPE number;",
            "DEFINE FIELD recurring_token ON payments TYPE option<string>;",
            "DEFINE FIELD status ON payments TYPE string;",
            "DEFINE FIELD payment_method ON payments TYPE string;",
            "DEFINE FIELD merchant_transaction_id ON payments TYPE string;",
            "DEFINE FIELD checkout_id ON payments TYPE option<string>;",
            "DEFINE FIELD created_at ON payments TYPE datetime;",
            "DEFINE FIELD updated_at ON payments TYPE datetime;",
            "DEFINE INDEX unique_merchant_txn ON payments COLUMNS merchant_transaction_id UNIQUE;",
            
            // Subscriptions table
            "DEFINE TABLE subscriptions SCHEMAFULL;",
            "DEFINE FIELD id ON subscriptions TYPE string;",
            "DEFINE FIELD user_id ON subscriptions TYPE string;",
            "DEFINE FIELD plan_name ON subscriptions TYPE string;",
            "DEFINE FIELD price ON subscriptions TYPE number;",
            "DEFINE FIELD status ON subscriptions TYPE string;",
            "DEFINE FIELD payment_method ON subscriptions TYPE option<string>;",
            "DEFINE FIELD payment_brand ON subscriptions TYPE option<string>;",
            "DEFINE FIELD start_date ON subscriptions TYPE option<datetime>;",
            "DEFINE FIELD end_date ON subscriptions TYPE option<datetime>;",
            "DEFINE FIELD created_at ON subscriptions TYPE datetime;",
            "DEFINE FIELD updated_at ON subscriptions TYPE datetime;",
            
            // Recurring payments table
            "DEFINE TABLE recurring_payments SCHEMAFULL;",
            "DEFINE FIELD id ON recurring_payments TYPE string;",
            "DEFINE FIELD user_id ON recurring_payments TYPE string;",
            "DEFINE FIELD subscription_id ON recurring_payments TYPE string;",
            "DEFINE FIELD recurring_token ON recurring_payments TYPE string;",
            "DEFINE FIELD card_last_four ON recurring_payments TYPE option<string>;",
            "DEFINE FIELD card_brand ON recurring_payments TYPE option<string>;",
            "DEFINE FIELD status ON recurring_payments TYPE string;",
            "DEFINE FIELD created_at ON recurring_payments TYPE datetime;",
            "DEFINE FIELD updated_at ON recurring_payments TYPE datetime;",
            
            // Notifications table
            "DEFINE TABLE notification SCHEMAFULL;",
            "DEFINE FIELD id ON notification TYPE string;",
            "DEFINE FIELD user_id ON notification TYPE string;",
            "DEFINE FIELD subscription_id ON notification TYPE string;",
            "DEFINE FIELD message ON notification TYPE string;",
            "DEFINE FIELD acknowledged ON notification TYPE bool;",
            "DEFINE FIELD created_at ON notification TYPE datetime;",
        ];
        
        for query in queries {
            let result = db.query(query).await;
            match result {
                Ok(_) => println!("✅ Executed: {}", query),
                Err(e) => println!("❌ Failed to execute {}: {}", query, e),
            }
        }
        
        println!("✅ Database schema initialization completed");
        Ok(())
    }

    // ---------------------
    // User operations
    // ---------------------
    
  pub async fn create_user(&self, user_dto: CreateUserDto) -> Result<User, String> {
    // Check if user already exists
    let existing: Vec<User> = self.db
        .query("SELECT * FROM users WHERE email = $email")
        .bind(("email", user_dto.email.clone()))
        .await
        .map_err(|e| format!("Database error: {}", e))?
        .take(0)
        .map_err(|e| format!("Query error: {}", e))?;
    
    if !existing.is_empty() {
        return Err("User with this email already exists".to_string());
    }


    let user_id = Uuid::new_v4().simple().to_string();
    
    // ✅ Don't set the id field in content - let SurrealDB handle it
    let user = User {
        id: user_id.clone(), 
        email: user_dto.email,
        name: user_dto.name,
        created_at: Utc::now(),
        updated_at: Utc::now(),
    };

    let created_user: User = self.db
        .create(("users", user_id.clone()))
        .content(user)
        .await
        .map_err(|e| format!("Failed to create user: {}", e))?
        .ok_or_else(|| "Failed to create user: no result returned".to_string())?;
    
    println!("✅ Created user: {} ({})", created_user.name, created_user.id);
    Ok(created_user)
}

    pub async fn get_user(&self, user_id: &str) -> Option<User> {
        // Extract the UUID part if it's in record ID format
        let id_part = if user_id.starts_with("users:") {
            user_id.strip_prefix("users:").unwrap_or(user_id)
        } else {
            user_id
        };

        let result: Result<Option<User>, _> = self.db
            .select(("users", id_part))
            .await;
        
        result.ok().flatten()
    }

    pub async fn get_user_by_email(&self, email: &str) -> Option<User> {
        let result: Result<Vec<User>, _> = self.db
            .query("SELECT * FROM users WHERE email = $email LIMIT 1")
            .bind(("email", email.to_string()))
            .await
            .and_then(|mut response| response.take(0));
        
        result.ok().and_then(|users| users.into_iter().next())
    }

    // ✅ Fixed: Changed parameter from &Uuid to &str
    pub async fn get_recurring_token_by_user(&self, user_id: &str) -> Option<String> {
        let result: Result<Vec<RecurringPayment>, _> = self.db
            .query("SELECT * FROM recurring_payments WHERE user_id = $user_id AND status = 'Active' LIMIT 1")
            .bind(("user_id", user_id.to_string()))
            .await
            .and_then(|mut response| response.take(0));
        
        result.ok()
            .and_then(|payments| payments.into_iter().next())
            .map(|payment| payment.recurring_token)
    }

    // ---------------------
    // Payment operations
    // ---------------------
    
   // Fix the create_payment method around line 219
pub async fn create_payment(&self, payment_dto: CreatePaymentDto) -> Result<Payment, String> {
    let merchant_transaction_id = format!(
        "TXN_{}",
        Uuid::new_v4()
            .simple()
            .to_string()
            .to_uppercase()
            .get(..16)
            .unwrap_or("0000000000000000")
    );

    let payment_id = Uuid::new_v4().simple().to_string();
    
    // ✅ Don't set the id field in content
    let payment = Payment {
        id: String::new(), // Will be set by SurrealDB
        user_id: payment_dto.user_id,
        subscription_id: Some(payment_dto.subscription_id),
        amount: payment_dto.amount,
        recurring_token: None,
        status: PaymentStatus::Pending,
        payment_method: payment_dto.payment_method.unwrap_or(PaymentMethod::Card),
        merchant_transaction_id,
        checkout_id: None,
        created_at: Utc::now(),
        updated_at: Utc::now(),
    };

    let created_payment: Payment = self.db
        .create(("payments", payment_id.clone()))
        .content(payment)
        .await
        .map_err(|e| format!("Failed to create payment: {}", e))?
        .ok_or_else(|| "Failed to create payment: no result returned".to_string())?;
    
    println!(
        "✅ Created payment: ID={}, MerchantTxnId={}, Amount={}",
        created_payment.id, created_payment.merchant_transaction_id, created_payment.amount
    );
    Ok(created_payment)
}
    // ✅ Fixed: Changed parameter from &Uuid to &str
    pub async fn get_payment(&self, payment_id: &str) -> Option<Payment> {
        let id_part = if payment_id.starts_with("payments:") {
            payment_id.strip_prefix("payments:").unwrap_or(payment_id)
        } else {
            payment_id
        };

        let result: Result<Option<Payment>, _> = self.db
            .select(("payments", id_part))
            .await;
        
        result.ok().flatten()
    }

    pub async fn get_payment_by_merchant_id(&self, merchant_transaction_id: &str) -> Option<Payment> {
        let result: Result<Vec<Payment>, _> = self.db
            .query("SELECT * FROM payments WHERE merchant_transaction_id = $merchant_id LIMIT 1")
            .bind(("merchant_id", merchant_transaction_id.to_string()))
            .await
            .and_then(|mut response| response.take(0));
        
        let found = result.ok().and_then(|payments| payments.into_iter().next());
        
        if found.is_none() {
            println!("🔍 Payment not found for merchant_transaction_id: {}", merchant_transaction_id);
        }
        
        found
    }

    pub async fn update_payment_status(&self, merchant_transaction_id: &str, status: &PaymentStatus) -> Result<(), String> {
        let status_str = format!("{:?}", status);
        let result: Result<Vec<Payment>, _> = self.db
            .query("UPDATE payments SET status = $status, updated_at = $now WHERE merchant_transaction_id = $merchant_id RETURN AFTER")
            .bind(("status", status_str))
            .bind(("now", Utc::now()))
            .bind(("merchant_id", merchant_transaction_id.to_string()))
            .await
            .and_then(|mut response| response.take(0));
        
        match result {
            Ok(payments) if !payments.is_empty() => {
                println!("✅ Updated payment status: {:?} (MerchantTxnId: {})", status, merchant_transaction_id);
                Ok(())
            }
            Ok(_) => Err(format!("Payment not found for merchant_transaction_id: {}", merchant_transaction_id)),
            Err(e) => Err(format!("Database error: {}", e)),
        }
    }

    pub async fn update_payment_checkout_id(&self, merchant_transaction_id: &str, checkout_id: &str) -> Result<(), String> {
        let result: Result<Vec<Payment>, _> = self.db
            .query("UPDATE payments SET checkout_id = $checkout_id, updated_at = $now WHERE merchant_transaction_id = $merchant_id RETURN AFTER")
            .bind(("checkout_id", checkout_id.to_string()))
            .bind(("now", Utc::now()))
            .bind(("merchant_id", merchant_transaction_id.to_string()))
            .await
            .and_then(|mut response| response.take(0));
        
        match result {
            Ok(payments) if !payments.is_empty() => {
                println!("✅ Updated payment checkout_id: {} (MerchantTxnId: {})", checkout_id, merchant_transaction_id);
                Ok(())
            }
            Ok(_) => Err(format!("Payment not found for merchant_transaction_id: {}", merchant_transaction_id)),
            Err(e) => Err(format!("Database error: {}", e)),
        }
    }

    // ✅ Fixed: Changed parameter from &Uuid to &str
    pub async fn get_payments_by_user(&self, user_id: &str) -> Vec<Payment> {
        let result: Result<Vec<Payment>, _> = self.db
            .query("SELECT * FROM payments WHERE user_id = $user_id")
            .bind(("user_id", user_id.to_string()))
            .await
            .and_then(|mut response| response.take(0));
        
        result.unwrap_or_default()
    }

    // ---------------------
    // Subscription operations
    // ---------------------
    
 pub async fn create_subscription(&self, dto: CreateSubscriptionDto) -> Result<Subscription, String> {
    let subscription_id = Uuid::new_v4().simple().to_string();
    
    // ✅ Don't set the id field in content
    let subscription = Subscription {
        id: String::new(), // Will be set by SurrealDB
        user_id: dto.user_id,
        plan_name: dto.plan_name,
        price: dto.price,
        status: SubscriptionStatus::Pending,
        payment_method: dto.payment_method,
        payment_brand: None,
        start_date: None,
        end_date: None,
        created_at: Utc::now(),
        updated_at: Utc::now(),
    };

    let created_subscription: Subscription = self.db
        .create(("subscriptions", subscription_id.clone()))
        .content(subscription)
        .await
        .map_err(|e| format!("Failed to create subscription: {}", e))?
        .ok_or_else(|| "Failed to create subscription: no result returned".to_string())?;
    
    println!("✅ Created subscription: {} ({})", created_subscription.plan_name, created_subscription.id);
    Ok(created_subscription)
}
        
      pub async fn get_subscription(&self, subscription_id: &str) -> Option<Subscription> {
        let id_part = if subscription_id.starts_with("subscriptions:") {
            subscription_id.strip_prefix("subscriptions:").unwrap_or(subscription_id)
        } else {
            subscription_id
        };

        let result: Result<Option<Subscription>, _> = self.db
            .select(("subscriptions", id_part))
            .await;
        
        result.ok().flatten()
    }

    // ✅ Fixed: Changed parameter from &Uuid to &str
    pub async fn get_subscriptions_by_user(&self, user_id: &str) -> Vec<Subscription> {
        let result: Result<Vec<Subscription>, _> = self.db
            .query("SELECT * FROM subscriptions WHERE user_id = $user_id")
            .bind(("user_id", user_id.to_string()))
            .await
            .and_then(|mut response| response.take(0));
        
        result.unwrap_or_default()
    }

    // ✅ Fixed: Changed parameter from &Uuid to &str
    pub async fn activate_subscription(&self, subscription_id: &str) -> Result<(), String> {
        let now = Utc::now();
        let end_date = now + Duration::days(1);
        
        let id_part = if subscription_id.starts_with("subscriptions:") {
            subscription_id.strip_prefix("subscriptions:").unwrap_or(subscription_id)
        } else {
            subscription_id
        };

        let result: Result<Vec<Subscription>, _> = self.db
            .query("UPDATE subscriptions SET status = 'Active', start_date = $start, end_date = $end, updated_at = $now WHERE id = $id RETURN AFTER")
            .bind(("start", now))
            .bind(("end", end_date))
            .bind(("now", now))
            .bind(("id", format!("subscriptions:{}", id_part)))
            .await
            .and_then(|mut response| response.take(0));
        
        match result {
            Ok(subscriptions) if !subscriptions.is_empty() => {
                println!("✅ Activated subscription: Active (ID: {})", subscription_id);
                Ok(())
            }
            Ok(_) => Err(format!("Subscription not found: {}", subscription_id)),
            Err(e) => Err(format!("Database error: {}", e)),
        }
    }

    // ✅ Fixed: Changed parameter from &Uuid to &str
    pub async fn update_subscription_status(&self, subscription_id: &str, status: SubscriptionStatus) -> Result<(), String> {
        let status_str = format!("{:?}", status);
        let id_part = if subscription_id.starts_with("subscriptions:") {
            subscription_id.strip_prefix("subscriptions:").unwrap_or(subscription_id)
        } else {
            subscription_id
        };

        let result: Result<Vec<Subscription>, _> = self.db
            .query("UPDATE subscriptions SET status = $status, updated_at = $now WHERE id = $id RETURN AFTER")
            .bind(("status", status_str))
            .bind(("now", Utc::now()))
            .bind(("id", format!("subscriptions:{}", id_part)))
            .await
            .and_then(|mut response| response.take(0));
        
        match result {
            Ok(subscriptions) if !subscriptions.is_empty() => {
                println!("✅ Updated subscription status: {:?} (ID: {})", status, subscription_id);
                Ok(())
            }
            Ok(_) => Err(format!("Subscription not found: {}", subscription_id)),
            Err(e) => Err(format!("Database error: {}", e)),
        }
    }

    // ✅ Fixed: Changed parameter from &Uuid to &str
    pub async fn update_subscription_payment_details(
        &self,
        subscription_id: &str,
        method: PaymentMethod,
        brand: Option<String>,
    ) -> Result<(), String> {
        let method_str = format!("{:?}", method);
        let id_part = if subscription_id.starts_with("subscriptions:") {
            subscription_id.strip_prefix("subscriptions:").unwrap_or(subscription_id)
        } else {
            subscription_id
        };

        let result: Result<Vec<Subscription>, _> = self.db
            .query("UPDATE subscriptions SET payment_method = $method, payment_brand = $brand, updated_at = $now WHERE id = $id RETURN AFTER")
            .bind(("method", method_str))
            .bind(("brand", brand.clone()))
            .bind(("now", Utc::now()))
            .bind(("id", format!("subscriptions:{}", id_part)))
            .await
            .and_then(|mut response| response.take(0));
        
        match result {
            Ok(subscriptions) if !subscriptions.is_empty() => {
                println!("✅ Updated subscription payment: {:?}, brand: {:?} (Subscription ID: {})", method, brand, subscription_id);
                Ok(())
            }
            Ok(_) => Err(format!("Subscription not found: {}", subscription_id)),
            Err(e) => Err(format!("Database error: {}", e)),
        }
    }

    // ---------------------
    // Recurring Payment operations
    // ---------------------
    
    // ✅ Fixed: Changed parameters from Uuid to String
    pub async fn create_recurring_payment(
        &self,
        user_id: String,
        subscription_id: String,
        token: String,
        card_last_four: Option<String>,
        card_brand: Option<String>,
    ) -> RecurringPayment {
        let rec_payment_id = Uuid::new_v4().simple().to_string();
        let rec_payment = RecurringPayment {
            id: format!("recurring_payments:{}", rec_payment_id),
            user_id,
            subscription_id,
            recurring_token: token,
            card_last_four,
            card_brand,
            status: RecurringPaymentStatus::Active,
            created_at: Utc::now(),
            updated_at: Utc::now(),
        };

        let _: Result<Vec<RecurringPayment>, _> = self.db
            .query("CREATE recurring_payments:$record_id CONTENT $rec_payment")
            .bind(("record_id", rec_payment_id.clone()))
            .bind(("rec_payment", rec_payment.clone()))
            .await
            .and_then(|mut response| response.take(0));
        
        println!("✅ Created recurring payment: {}", rec_payment.id);
        rec_payment
    }

    pub async fn get_active_recurring_payments(&self) -> Vec<RecurringPayment> {
        let result: Result<Vec<RecurringPayment>, _> = self.db
            .query("SELECT * FROM recurring_payments WHERE status = 'Active'")
            .await
            .and_then(|mut response| response.take(0));
        
        result.unwrap_or_default()
    }

    pub async fn update_payment_recurring_token(
        &self,
        merchant_transaction_id: &str,
        token: &str,
    ) -> Result<(), String> {
        let result: Result<Vec<Payment>, _> = self.db
            .query("UPDATE payments SET recurring_token = $token, updated_at = $now WHERE merchant_transaction_id = $merchant_id RETURN AFTER")
            .bind(("token", token.to_string()))
            .bind(("now", Utc::now()))
            .bind(("merchant_id", merchant_transaction_id.to_string()))
            .await
            .and_then(|mut response| response.take(0));
        
        match result {
            Ok(payments) if !payments.is_empty() => {
                println!("✅ Updated payment recurring_token (TxnId: {}, Token: {})", merchant_transaction_id, token);
                Ok(())
            }
            Ok(_) => Err(format!("Payment not found for merchant_transaction_id: {}", merchant_transaction_id)),
            Err(e) => Err(format!("Database error: {}", e)),
        }
    }

    pub async fn get_due_subscriptions(&self) -> Result<Vec<crate::models::subscription::Subscription>, String> {
        let now = Utc::now();
        let result: Result<Vec<crate::models::subscription::Subscription>, _> = self.db
            .query("SELECT * FROM subscriptions WHERE status = 'Active' AND end_date <= $now")
            .bind(("now", now))
            .await
            .and_then(|mut response| response.take(0));
        
        result.map_err(|e| format!("Database error: {}", e))
    }

    pub async fn get_expired_unpaid_subscriptions(&self) -> Result<Vec<crate::models::subscription::Subscription>, String> {
        let cutoff_date = Utc::now() - chrono::Duration::days(1);
        let result: Result<Vec<crate::models::subscription::Subscription>, _> = self.db
            .query("SELECT * FROM subscriptions WHERE status = 'Active' AND end_date < $cutoff")
            .bind(("cutoff", cutoff_date))
            .await
            .and_then(|mut response| response.take(0));
        
        result.map_err(|e| format!("Database error: {}", e))
    }

    // ✅ Fixed: Changed parameter from &uuid::Uuid to &str
    pub async fn mark_subscription_renewed(&self, subscription_id: &str) -> Result<(), String> {
        let now = Utc::now();
        let end_date = now + chrono::Duration::days(30);
        
        let id_part = if subscription_id.starts_with("subscriptions:") {
            subscription_id.strip_prefix("subscriptions:").unwrap_or(subscription_id)
        } else {
            subscription_id
        };

        let result: Result<Vec<crate::models::subscription::Subscription>, _> = self.db
            .query("UPDATE subscriptions SET start_date = $start, end_date = $end, updated_at = $now, status = 'Active' WHERE id = $id RETURN AFTER")
            .bind(("start", now))
            .bind(("end", end_date))
            .bind(("now", now))
            .bind(("id", format!("subscriptions:{}", id_part)))
            .await
            .and_then(|mut response| response.take(0));
        
        match result {
            Ok(subscriptions) if !subscriptions.is_empty() => {
                println!("🔁 Subscription {} renewed successfully", subscription_id);
                Ok(())
            }
            Ok(_) => Err(format!("Sub not found {}", subscription_id)),
            Err(e) => Err(format!("Database error: {}", e)),
        }
    }

    // ✅ Fixed: Changed parameter from &uuid::Uuid to &str
    pub async fn suspend_subscription(&self, subscription_id: &str) -> Result<(), String> {
        let id_part = if subscription_id.starts_with("subscriptions:") {
            subscription_id.strip_prefix("subscriptions:").unwrap_or(subscription_id)
        } else {
            subscription_id
        };

        let result: Result<Vec<crate::models::subscription::Subscription>, _> = self.db
            .query("UPDATE subscriptions SET status = 'Suspended', updated_at = $now WHERE id = $id RETURN AFTER")
            .bind(("now", Utc::now()))
            .bind(("id", format!("subscriptions:{}", id_part)))
            .await
            .and_then(|mut response| response.take(0));
        
        match result {
            Ok(subscriptions) if !subscriptions.is_empty() => {
                println!("🛑 Subscription {} suspended", subscription_id);
                Ok(())
            }
            Ok(_) => Err(format!("Sub not found {}", subscription_id)),
            Err(e) => Err(format!("Database error: {}", e)),
        }
    }

    // ✅ Fixed: Changed parameters from Uuid to String
    pub async fn create_manual_renewal_notification(
        &self,
        user_id: String,
        subscription_id: String,
    ) -> Result<(), String> {
        let notification_id = Uuid::new_v4().simple().to_string();
        let message = format!("Your subscription {} is due for renewal", subscription_id);
        let now = Utc::now();

        let query = r#"
            CREATE notification:$record_id SET
                user_id = $user_id,
                subscription_id = $subscription_id,
                message = $message,
                acknowledged = false,
                created_at = $created_at
        "#;

        self.db
            .query(query)
            .bind(("record_id", notification_id))
            .bind(("user_id", user_id.clone()))
            .bind(("subscription_id", subscription_id.clone()))
            .bind(("message", message.clone()))
            .bind(("created_at", now))
            .await
            .map_err(|e| e.to_string())?;
        
        println!("🔔 Notification created for user {} to manually renew subscription {}", user_id, subscription_id);
        Ok(())
    }

    pub async fn get_user_notifications(
        &self,
        user_id: &str,
    ) -> Result<Vec<crate::models::notification::Notification>, String> {
        let query = "SELECT * FROM notification WHERE user_id = $user_id ORDER BY created_at DESC";
        
        match self.db.query(query).bind(("user_id", user_id.to_string())).await {
            Ok(mut result) => {
                let notifications: Vec<crate::models::notification::Notification> = result
                    .take(0)
                    .map_err(|e| format!("Failed to extract notifications: {}", e))?;
                Ok(notifications)
            }
            Err(e) => Err(format!("Database error: {}", e)),
        }
    }

    pub async fn acknowledge_notification(&self, notification_id: &str) -> Result<(), String> {
        let query = "UPDATE notification SET acknowledged = true WHERE id = $notification_id";
        
        match self.db.query(query).bind(("notification_id", notification_id.to_string())).await {
            Ok(_) => {
                println!("✅ Notification {} marked as acknowledged", notification_id);
                Ok(())
            }
            Err(e) => Err(format!("Database error: {}", e)),
        }
    }

    pub async fn create_test_notification(&self, user_id: String, message: String) -> Result<(), String> {
        let notification_id = Uuid::new_v4().simple().to_string();
        let now = Utc::now();

        let query = r#"
            CREATE notification:$record_id SET
                user_id = $user_id,
                subscription_id = "test-subscription",
                message = $message,
                acknowledged = false,
                created_at = $created_at
        "#;

        self.db
            .query(query)
            .bind(("record_id", notification_id))
            .bind(("user_id", user_id.clone()))
            .bind(("message", message.clone()))
            .bind(("created_at", now))
            .await
            .map_err(|e| e.to_string())?;
        
        println!("📝 Test notification created for user {}: {}", user_id, message);
        Ok(())
    }

        
    // ---------------------
    // Debug utilities (converted to async)
    // ---------------------
    pub async fn debug_list_payments(&self) -> Vec<Payment> {
        let result: Result<Vec<Payment>, _> = self.db
            .query("SELECT * FROM payments")
            .await
            .and_then(|mut response| response.take(0));
                    
        result.unwrap_or_default()
    }

    pub async fn debug_list_subscriptions(&self) -> Vec<Subscription> {
        let result: Result<Vec<Subscription>, _> = self.db
            .query("SELECT * FROM subscriptions")
            .await
            .and_then(|mut response| response.take(0));
                    
        result.unwrap_or_default()
    }

    pub async fn debug_print_all_payments(&self) {
        let payments = self.debug_list_payments().await;
        println!("🔍 All payments ({} total):", payments.len());
        for (i, payment) in payments.iter().enumerate() {
            println!(
                "{}. ID: {}, MerchantTxnId: {}, Status: {:?}, Amount: {}, CheckoutId: {:?}",
                i + 1,
                payment.id,
                payment.merchant_transaction_id,
                payment.status,
                payment.amount,
                payment.checkout_id
            );
        }
    }

    pub async fn get_payment_count(&self) -> usize {
        let result: Result<Vec<serde_json::Value>, _> = self.db
            .query("SELECT count() FROM payments GROUP ALL")
            .await
            .and_then(|mut response| response.take(0));
                    
        result.unwrap_or_default().len()
    }

    pub async fn get_subscription_count(&self) -> usize {
        let result: Result<Vec<serde_json::Value>, _> = self.db
            .query("SELECT count() FROM subscriptions GROUP ALL")
            .await
            .and_then(|mut response| response.take(0));
                    
        result.unwrap_or_default().len()
    }
}

impl Default for DatabaseService {
    fn default() -> Self {
        // Note: This will panic if called synchronously.         
        // Consider removing Default implementation or using a different approach        
        panic!("Use DatabaseService::new().await instead of default()")
    }
}
