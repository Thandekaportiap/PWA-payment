use std::sync::Mutex;
use chrono::{Utc, Duration};
use uuid::Uuid;
use crate::models::{
    user::{User, CreateUserDto},
    payment::{Payment, CreatePaymentDto, PaymentStatus, PaymentMethod},
    subscription::{Subscription, CreateSubscriptionDto, SubscriptionStatus},
};

#[derive(Clone)]
pub struct DatabaseService {
    pub users: std::sync::Arc<Mutex<Vec<User>>>,
    pub payments: std::sync::Arc<Mutex<Vec<Payment>>>,
    pub subscriptions: std::sync::Arc<Mutex<Vec<Subscription>>>,
}

impl DatabaseService {
    pub fn new() -> Self {
        Self {
            users: std::sync::Arc::new(Mutex::new(Vec::new())),
            payments: std::sync::Arc::new(Mutex::new(Vec::new())),
            subscriptions: std::sync::Arc::new(Mutex::new(Vec::new())),
        }
    }

    // User operations
    pub fn create_user(&self, user_dto: CreateUserDto) -> Result<User, String> {
        let mut users = self.users.lock().unwrap();
        
        // Check if user already exists
        if users.iter().any(|u| u.email == user_dto.email) {
            return Err("User with this email already exists".to_string());
        }

        let user = User {
            id: Uuid::new_v4(),
            email: user_dto.email,
            name: user_dto.name,
            created_at: Utc::now(),
            updated_at: Utc::now(),
        };

        users.push(user.clone());
        println!("✅ Created user: {} ({})", user.name, user.id);
        Ok(user)
    }

    pub fn get_user(&self, user_id: &Uuid) -> Option<User> {
        let users = self.users.lock().unwrap();
        users.iter().find(|u| u.id == *user_id).cloned()
    }

    pub fn get_user_by_email(&self, email: &str) -> Option<User> {
        let users = self.users.lock().unwrap();
        users.iter().find(|u| u.email == email).cloned()
    }

    // Payment operations
    pub fn create_payment(&self, payment_dto: CreatePaymentDto) -> Result<Payment, String> {
        let mut payments = self.payments.lock().unwrap();
        
        // Generate a unique merchant transaction ID
        let merchant_transaction_id = format!("TXN_{}", Uuid::new_v4().simple().to_string().to_uppercase()[..16].to_string());
        
        let payment = Payment {
            id: Uuid::new_v4(),
            user_id: payment_dto.user_id,
            subscription_id: Some(payment_dto.subscription_id),
            amount: payment_dto.amount,
            status: PaymentStatus::Pending,
            payment_method: payment_dto.payment_method.unwrap_or(PaymentMethod::Card),
            merchant_transaction_id: merchant_transaction_id.clone(),
            checkout_id: None,
            created_at: Utc::now(),
            updated_at: Utc::now(),
        };

        payments.push(payment.clone());
        println!("✅ Created payment: ID={}, MerchantTxnId={}, Amount={}", 
                payment.id, payment.merchant_transaction_id, payment.amount);
        Ok(payment)
    }

    pub fn get_payment(&self, payment_id: &Uuid) -> Option<Payment> {
        let payments = self.payments.lock().unwrap();
        payments.iter().find(|p| p.id == *payment_id).cloned()
    }

    pub fn get_payment_by_merchant_id(&self, merchant_transaction_id: &str) -> Option<Payment> {
        let payments = self.payments.lock().unwrap();
        let found = payments.iter().find(|p| p.merchant_transaction_id == merchant_transaction_id).cloned();
        
        if found.is_none() {
            println!("🔍 Payment not found for merchant_transaction_id: {}", merchant_transaction_id);
            println!("🔍 Available payments in database:");
            for payment in payments.iter().take(10) { // Show last 10 payments
                println!("  - ID: {}, MerchantTxnId: {}, Status: {:?}, Amount: {}", 
                        payment.id, payment.merchant_transaction_id, payment.status, payment.amount);
            }
            if payments.len() > 10 {
                println!("  ... and {} more payments", payments.len() - 10);
            }
        }
        
        found
    }

    pub fn update_payment_status(&self, merchant_transaction_id: &str, status: &PaymentStatus) -> Result<(), String> {
        let mut payments = self.payments.lock().unwrap();
        if let Some(payment) = payments.iter_mut().find(|p| p.merchant_transaction_id == merchant_transaction_id) {
            let old_status = payment.status.clone();
            payment.status = status.clone();
            payment.updated_at = Utc::now();
            println!("✅ Updated payment status: {} -> {:?} (MerchantTxnId: {})", 
                    format!("{:?}", old_status), status, merchant_transaction_id);
            Ok(())
        } else {
            let error_msg = format!("Payment not found for merchant_transaction_id: {}", merchant_transaction_id);
            println!("❌ {}", error_msg);
            Err(error_msg)
        }
    }

    pub fn update_payment_checkout_id(&self, merchant_transaction_id: &str, checkout_id: &str) -> Result<(), String> {
        let mut payments = self.payments.lock().unwrap();
        if let Some(payment) = payments.iter_mut().find(|p| p.merchant_transaction_id == merchant_transaction_id) {
            payment.checkout_id = Some(checkout_id.to_string());
            payment.updated_at = Utc::now();
            println!("✅ Updated payment checkout_id: {} (MerchantTxnId: {})", 
                    checkout_id, merchant_transaction_id);
            Ok(())
        } else {
            let error_msg = format!("Payment not found for merchant_transaction_id: {}", merchant_transaction_id);
            println!("❌ {}", error_msg);
            Err(error_msg)
        }
    }

    pub fn get_payments_by_user(&self, user_id: &Uuid) -> Vec<Payment> {
        let payments = self.payments.lock().unwrap();
        payments.iter().filter(|p| p.user_id == *user_id).cloned().collect()
    }

    // Subscription operations
    pub fn create_subscription(&self, subscription_dto: CreateSubscriptionDto) -> Result<Subscription, String> {
        let mut subscriptions = self.subscriptions.lock().unwrap();
        
        let subscription = Subscription {
            id: Uuid::new_v4(),
            user_id: subscription_dto.user_id,
            plan_name: subscription_dto.plan_name.clone(),
            price: subscription_dto.price,
            status: SubscriptionStatus::Pending,
            start_date: None,
            end_date: None,
            created_at: Utc::now(),
            updated_at: Utc::now(),
        };

        subscriptions.push(subscription.clone());
        println!("✅ Created subscription: ID={}, Plan={}, Price={}", 
                subscription.id, subscription.plan_name, subscription.price);
        Ok(subscription)
    }

    pub fn get_subscription(&self, subscription_id: &Uuid) -> Option<Subscription> {
        let subscriptions = self.subscriptions.lock().unwrap();
        subscriptions.iter().find(|s| s.id == *subscription_id).cloned()
    }

    pub fn activate_subscription(&self, subscription_id: &Uuid) -> Result<(), String> {
        let mut subscriptions = self.subscriptions.lock().unwrap();
        if let Some(subscription) = subscriptions.iter_mut().find(|s| s.id == *subscription_id) {
            let old_status = subscription.status.clone();
            subscription.status = SubscriptionStatus::Active;
            subscription.start_date = Some(Utc::now());
            subscription.end_date = Some(Utc::now() + Duration::days(30)); // 30-day subscription
            subscription.updated_at = Utc::now();
            println!("✅ Activated subscription: {} -> Active (ID: {})", 
                    format!("{:?}", old_status), subscription_id);
            Ok(())
        } else {
            let error_msg = format!("Subscription not found: {}", subscription_id);
            println!("❌ {}", error_msg);
            Err(error_msg)
        }
    }

    pub fn get_subscriptions_by_user(&self, user_id: &Uuid) -> Vec<Subscription> {
        let subscriptions = self.subscriptions.lock().unwrap();
        subscriptions.iter().filter(|s| s.user_id == *user_id).cloned().collect()
    }

    pub fn update_subscription_status(&self, subscription_id: &Uuid, status: SubscriptionStatus) -> Result<(), String> {
        let mut subscriptions = self.subscriptions.lock().unwrap();
        if let Some(subscription) = subscriptions.iter_mut().find(|s| s.id == *subscription_id) {
            let old_status = subscription.status.clone();
            subscription.status = status.clone();
            subscription.updated_at = Utc::now();
            println!("✅ Updated subscription status: {:?} -> {:?} (ID: {})", 
                    old_status, status, subscription_id);
            Ok(())
        } else {
            let error_msg = format!("Subscription not found: {}", subscription_id);
            println!("❌ {}", error_msg);
            Err(error_msg)
        }
    }

    // Debug methods
    pub fn debug_list_payments(&self) -> Vec<Payment> {
        let payments = self.payments.lock().unwrap();
        payments.clone()
    }

    pub fn debug_list_subscriptions(&self) -> Vec<Subscription> {
        let subscriptions = self.subscriptions.lock().unwrap();
        subscriptions.clone()
    }

    pub fn debug_print_all_payments(&self) {
        let payments = self.payments.lock().unwrap();
        println!("🔍 All payments in database ({} total):", payments.len());
        for (i, payment) in payments.iter().enumerate() {
            println!("  {}. ID: {}, MerchantTxnId: {}, Status: {:?}, Amount: {}, CheckoutId: {:?}", 
                    i + 1, payment.id, payment.merchant_transaction_id, payment.status, 
                    payment.amount, payment.checkout_id);
        }
    }

    pub fn get_payment_count(&self) -> usize {
        let payments = self.payments.lock().unwrap();
        payments.len()
    }

    pub fn get_subscription_count(&self) -> usize {
        let subscriptions = self.subscriptions.lock().unwrap();
        subscriptions.len()
    }
}

impl Default for DatabaseService {
    fn default() -> Self {
        Self::new()
    }
}