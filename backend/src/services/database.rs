use std::sync::Arc;
use chrono::{Utc, Duration};
use uuid::Uuid;
use surrealdb::sql::Thing;
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

    // Add this in your init or setup code
// db.query("REMOVE TABLE users;").await?;
// db.query("REMOVE TABLE payments;").await?;
// db.query("REMOVE TABLE subscriptions;").await?;
// db.query("REMOVE TABLE recurring_payments;").await?;
// db.query("REMOVE TABLE notification;").await?;

    // Create tables and define schema WITHOUT timestamp fields
    let queries = vec![

        
        // Users table - no timestamps
        "DEFINE TABLE users SCHEMAFULL;",
        "DEFINE FIELD id ON users TYPE string;",
        "DEFINE FIELD email ON users TYPE string;",
        "DEFINE FIELD name ON users TYPE string;",
        "DEFINE INDEX unique_email ON users COLUMNS email UNIQUE;",
        
        // Payments table - no timestamps
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
        "DEFINE INDEX unique_merchant_txn ON payments COLUMNS merchant_transaction_id UNIQUE;",
        
        // Subscriptions table - no timestamps
        "DEFINE TABLE subscriptions SCHEMAFULL;",
        "DEFINE FIELD id ON subscriptions TYPE string;",
        "DEFINE FIELD user_id ON subscriptions TYPE string;",
        "DEFINE FIELD plan_name ON subscriptions TYPE string;",
        "DEFINE FIELD price ON subscriptions TYPE number;",
        "DEFINE FIELD status ON subscriptions TYPE string;",
        "DEFINE FIELD payment_method ON subscriptions TYPE option<string>;",
        "DEFINE FIELD payment_brand ON subscriptions TYPE option<string>;",
        "DEFINE FIELD start_date ON subscriptions TYPE option<string>;", 
        "DEFINE FIELD end_date ON subscriptions TYPE option<string>;",   
        
        // Recurring payments table - 
        "DEFINE TABLE recurring_payments SCHEMAFULL;",
        "DEFINE FIELD id ON recurring_payments TYPE string;",
        "DEFINE FIELD user_id ON recurring_payments TYPE string;",
        "DEFINE FIELD subscription_id ON recurring_payments TYPE string;",
        "DEFINE FIELD recurring_token ON recurring_payments TYPE string;",
        "DEFINE FIELD card_last_four ON recurring_payments TYPE option<string>;",
        "DEFINE FIELD card_brand ON recurring_payments TYPE option<string>;",
        "DEFINE FIELD status ON recurring_payments TYPE string;",
        
        // Notifications table 
       "DEFINE TABLE notification SCHEMAFULL;",
        "DEFINE FIELD id ON notification TYPE record;", 
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
    let user_key = Thing::from(("users", user_id.clone().as_str()));

    let mut result = self.db
        .query("CREATE $user_key SET email = $email, name = $name")
        .bind(("user_key", user_key))
        .bind(("email", user_dto.email))
        .bind(("name", user_dto.name))
        .await
        .map_err(|e| format!("Failed to create user: {}", e))?;

    let created_user: Option<User> = result.take(0)
        .map_err(|e| format!("Query error: {}", e))?;

    let created_user = created_user.ok_or_else(|| "Failed to create user: no result returned".to_string())?;

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
    
   // Create payment without timestamps
pub async fn create_payment(&self, dto: CreatePaymentDto) -> Result<Payment, String> {
    let payment_id = Uuid::new_v4().simple().to_string();
    let payment_key = Thing::from(("payments", payment_id.clone().as_str()));
    
    let merchant_transaction_id = format!(
        "TXN_{}",
        Uuid::new_v4()
            .simple()
            .to_string()
            .to_uppercase()
            .get(..16)
            .unwrap_or("0000000000000000")
    );

    let mut result = self.db
        .query("CREATE $payment_key SET user_id = $user_id, subscription_id = $subscription_id, amount = $amount, status = $status, payment_method = $payment_method, merchant_transaction_id = $merchant_transaction_id, checkout_id = $checkout_id, recurring_token = $recurring_token")
        .bind(("payment_key", payment_key))
        .bind(("user_id", dto.user_id))
        .bind(("subscription_id", dto.subscription_id))
        .bind(("amount", dto.amount))
        .bind(("status", PaymentStatus::Pending))
        .bind(("payment_method", dto.payment_method.unwrap_or(PaymentMethod::Card)))
        .bind(("merchant_transaction_id", merchant_transaction_id))
        .bind(("checkout_id", None::<String>))
        .bind(("recurring_token", None::<String>))
        .await
        .map_err(|e| format!("Failed to create payment: {}", e))?;

    let created_payment: Option<Payment> = result.take(0)
        .map_err(|e| format!("Query error: {}", e))?;

    let created_payment = created_payment.ok_or_else(|| "Failed to create payment: no result returned".to_string())?;

    println!("✅ Created payment: {} ({})", created_payment.merchant_transaction_id, created_payment.id);
    Ok(created_payment)
}



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
        .query("UPDATE payments SET status = $status WHERE merchant_transaction_id = $merchant_id RETURN AFTER")
        .bind(("status", status_str))
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
            .query("UPDATE payments SET checkout_id = $checkout_id WHERE merchant_transaction_id = $merchant_id RETURN AFTER")
            .bind(("checkout_id", checkout_id.to_string()))
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
    let thing_id = Thing::from(("subscriptions", subscription_id.clone().as_str()));

    let subscription = Subscription {
        id: thing_id,// set id explicitly
        user_id: dto.user_id,
        plan_name: dto.plan_name,
        price: dto.price,
        status: SubscriptionStatus::Pending,
        payment_method: dto.payment_method,
        payment_brand: None,
        start_date: None,
        end_date: None,
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

    pub async fn get_subscriptions_by_user(&self, user_id: &str) -> Vec<Subscription> {
        let result: Result<Vec<Subscription>, _> = self.db
            .query("SELECT * FROM subscriptions WHERE user_id = $user_id")
            .bind(("user_id", user_id.to_string()))
            .await
            .and_then(|mut response| response.take(0));
        
        result.unwrap_or_default()
    }

 pub async fn activate_subscription(&self, subscription_id: &str) -> Result<(), String> {
    let now = Utc::now();
    let end_date = now + Duration::days(1);

    let id_part = if subscription_id.starts_with("subscriptions:") {
        subscription_id.strip_prefix("subscriptions:").unwrap_or(subscription_id)
    } else {
        subscription_id
    };

    let record_id = format!("subscriptions:{}", id_part);

    let query = format!(
        "UPDATE {} SET status = 'Active', start_date = $start, end_date = $end RETURN AFTER",
        record_id
    );

    let result: Result<Vec<Subscription>, _> = self.db
        .query(&query)
        .bind(("start", now))
        .bind(("end", end_date))
        .await
        .and_then(|mut response| response.take(0));

    match result {
        Ok(subscriptions) if !subscriptions.is_empty() => {
            println!("✅ Activated subscription: Active (ID: {})", record_id);
            Ok(())
        }
        Ok(_) => Err(format!("Subscription not found: {}", record_id)),
        Err(e) => Err(format!("Database error: {}", e)),
    }
}



    pub async fn update_subscription_status(&self, subscription_id: &str, status: SubscriptionStatus) -> Result<(), String> {
        let status_str = format!("{:?}", status);
        let id_part = if subscription_id.starts_with("subscriptions:") {
            subscription_id.strip_prefix("subscriptions:").unwrap_or(subscription_id)
        } else {
            subscription_id
        };

        let result: Result<Vec<Subscription>, _> = self.db
            .query("UPDATE subscriptions SET status = $status WHERE id = $id RETURN AFTER")
            .bind(("status", status_str))
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
            .query("UPDATE subscriptions SET payment_method = $method, payment_brand = $brand WHERE id = $id RETURN AFTER")
            .bind(("method", method_str))
            .bind(("brand", brand.clone()))
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
    
    pub async fn create_recurring_payment(
        &self,
        user_id: String,
        subscription_id: String,
        token: String,
        card_last_four: Option<String>,
        card_brand: Option<String>,
    ) -> Result<RecurringPayment, String> {
        let rec_payment_id = Uuid::new_v4().simple().to_string();
        let rec_payment = RecurringPayment {
            id: String::new(), // Will be set by SurrealDB
            user_id,
            subscription_id,
            recurring_token: token,
            card_last_four,
            card_brand,
            status: RecurringPaymentStatus::Active,
        };

        let created_payment: RecurringPayment = self.db
            .create(("recurring_payments", rec_payment_id.clone()))
            .content(rec_payment)
            .await
            .map_err(|e| format!("Failed to create recurring payment: {}", e))?
            .ok_or_else(|| "Failed to create recurring payment: no result returned".to_string())?;
        
        println!("✅ Created recurring payment: {}", created_payment.id);
        Ok(created_payment)
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
            .query("UPDATE payments SET recurring_token = $token WHERE merchant_transaction_id = $merchant_id RETURN AFTER")
            .bind(("token", token.to_string()))
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
        let now = Utc::now().to_rfc3339();
        let result: Result<Vec<crate::models::subscription::Subscription>, _> = self.db
            .query("SELECT * FROM subscriptions WHERE status = 'Active' AND end_date <= $now")
            .bind(("now", now))
            .await
            .and_then(|mut response| response.take(0));
        
        result.map_err(|e| format!("Database error: {}", e))
    }

    pub async fn get_expired_unpaid_subscriptions(&self) -> Result<Vec<crate::models::subscription::Subscription>, String> {
        let cutoff_date = (Utc::now() - chrono::Duration::days(1)).to_rfc3339();
        let result: Result<Vec<crate::models::subscription::Subscription>, _> = self.db
            .query("SELECT * FROM subscriptions WHERE status = 'Active' AND end_date < $cutoff")
            .bind(("cutoff", cutoff_date))
            .await
            .and_then(|mut response| response.take(0));
        
        result.map_err(|e| format!("Database error: {}", e))
    }

    pub async fn mark_subscription_renewed(&self, subscription_id: &str) -> Result<(), String> {
        let now = Utc::now().to_rfc3339();
        let end_date = (Utc::now() + chrono::Duration::days(30)).to_rfc3339();
        
        let id_part = if subscription_id.starts_with("subscriptions:") {
            subscription_id.strip_prefix("subscriptions:").unwrap_or(subscription_id)
        } else {
            subscription_id
        };

        let result: Result<Vec<crate::models::subscription::Subscription>, _> = self.db
            .query("UPDATE subscriptions SET start_date = $start, end_date = $end, status = 'Active' WHERE id = $id RETURN AFTER")
            .bind(("start", now))
            .bind(("end", end_date))
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

    pub async fn suspend_subscription(&self, subscription_id: &str) -> Result<(), String> {
        let id_part = if subscription_id.starts_with("subscriptions:") {
            subscription_id.strip_prefix("subscriptions:").unwrap_or(subscription_id)
        } else {
            subscription_id
        };

        let result: Result<Vec<crate::models::subscription::Subscription>, _> = self.db
            .query("UPDATE subscriptions SET status = 'Suspended' WHERE id = $id RETURN AFTER")
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

    pub async fn create_manual_renewal_notification(
        &self,
        user_id: String,
        subscription_id: String,
    ) -> Result<(), String> {
        
        let notification_id = format!("notification:{}", Uuid::new_v4().simple());

        let message = format!("Your subscription {} is due for renewal", subscription_id);

        let query = r#"
            CREATE notification:$record_id SET
                user_id = $user_id,
                subscription_id = $subscription_id,
                message = $message,
                acknowledged = false
        "#;

        self.db
            .query(query)
            .bind(("record_id", notification_id))
            .bind(("user_id", user_id.clone()))
            .bind(("subscription_id", subscription_id.clone()))
            .bind(("message", message.clone()))
            .await
            .map_err(|e| e.to_string())?;
        
        println!("🔔 Notification created for user {} to manually renew subscription {}", user_id, subscription_id);
        Ok(())
    }

    pub async fn get_user_notifications(
        &self,
        user_id: &str,
    ) -> Result<Vec<crate::models::notification::Notification>, String> {
        let query = "SELECT * FROM notification WHERE user_id = $user_id";
        
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
        let sql = "UPDATE $notification_id SET acknowledged = true";

        
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

        let query = r#"
            CREATE notification SET
                id = $record_id,
                user_id = $user_id,
                subscription_id = "test-subscription",
                message = $message,
                acknowledged = false
        "#;

        match self.db
            .query(query)
            .bind(("record_id", notification_id.clone()))
            .bind(("user_id", user_id.clone()))
            .bind(("message", message.clone()))
            .await 
        {
            Ok(_) => {
                println!("📝 Test notification created for user {}: {}", user_id, message);
                Ok(())
            }
            Err(e) => {
                eprintln!("❌ Database error creating notification: {}", e);
                Err(format!("Database error: {}", e))
            }
        }
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
