use serde::{Deserialize, Serialize};
use chrono::{DateTime, Utc};
use uuid::Uuid;
use rust_decimal::Decimal;
use validator::Validate;

use crate::models::common::PaymentMethod;

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum PaymentStatus {
    Pending,
    Processing,
    Completed,
    Failed,
    Cancelled,
    Refunded,
    PartiallyRefunded,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum PaymentType {
    OneTime,
    Recurring,
    Registration, // For tokenization
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Payment {
    pub id: Uuid,
    pub user_id: Uuid,
    pub subscription_id: Option<Uuid>,
    pub merchant_transaction_id: String,
    pub peach_checkout_id: Option<String>,
    pub peach_payment_id: Option<String>,
    pub amount: Decimal,
    pub currency: String,
    pub payment_method: PaymentMethod,
    pub payment_type: PaymentType,
    pub status: PaymentStatus,
    pub failure_reason: Option<String>,
    pub recurring_token: Option<String>,
    pub enable_recurring: bool,
    pub retry_count: u32,
    pub metadata: Option<serde_json::Value>,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
    pub completed_at: Option<DateTime<Utc>>,
}

#[derive(Debug, Deserialize, Validate)]
pub struct InitiatePaymentRequest {
    pub user_id: Uuid,
    pub subscription_id: Uuid,
    
    #[validate(range(min = 0.01, message = "Amount must be greater than 0"))]
    pub amount: Decimal,
    
    pub currency: Option<String>,
    pub payment_method: PaymentMethod,
    pub enable_recurring: Option<bool>,
    pub return_url: Option<String>,
    pub webhook_url: Option<String>,
    pub metadata: Option<serde_json::Value>,
}

#[derive(Debug, Serialize)]
pub struct InitiatePaymentResponse {
    pub payment_id: Uuid,
    pub merchant_transaction_id: String,
    pub checkout_id: String,
    pub checkout_url: Option<String>,
    pub embed_config: Option<EmbedConfig>,
}

#[derive(Debug, Serialize)]
pub struct EmbedConfig {
    pub entity_id: String,
    pub checkout_id: String,
    pub script_url: String,
}

#[derive(Debug, Deserialize)]
pub struct PaymentWebhookPayload {
    pub id: String,
    pub merchant_transaction_id: String,
    pub result: WebhookResult,
    pub amount: String,
    pub currency: String,
    pub payment_type: String,
    pub registration_id: Option<String>,
    pub timestamp: String,
    pub ndc: Option<String>,
    pub checkout_id: Option<String>,
    pub custom_parameters: Option<serde_json::Value>,
}

#[derive(Debug, Deserialize)]
pub struct WebhookResult {
    pub code: String,
    pub description: String,
}

#[derive(Debug, Serialize)]
pub struct PaymentStatusResponse {
    pub payment_id: Uuid,
    pub status: PaymentStatus,
    pub amount: Decimal,
    pub currency: String,
    pub payment_method: PaymentMethod,
    pub created_at: DateTime<Utc>,
    pub completed_at: Option<DateTime<Utc>>,
    pub failure_reason: Option<String>,
    pub recurring_available: bool,
}

impl Payment {
    pub fn new(request: InitiatePaymentRequest, merchant_transaction_id: String) -> Self {
        let now = Utc::now();
        Self {
            id: Uuid::new_v4(),
            user_id: request.user_id,
            subscription_id: Some(request.subscription_id),
            merchant_transaction_id,
            peach_checkout_id: None,
            peach_payment_id: None,
            amount: request.amount,
            currency: request.currency.unwrap_or_else(|| "ZAR".to_string()),
            payment_method: request.payment_method,
            payment_type: PaymentType::OneTime,
            status: PaymentStatus::Pending,
            failure_reason: None,
            recurring_token: None,
            enable_recurring: request.enable_recurring.unwrap_or(false),
            retry_count: 0,
            metadata: request.metadata,
            created_at: now,
            updated_at: now,
            completed_at: None,
        }
    }

    pub fn update_status(&mut self, status: PaymentStatus, failure_reason: Option<String>) {
        self.status = status.clone();
        self.failure_reason = failure_reason;
        self.updated_at = Utc::now();
        
        if matches!(status, PaymentStatus::Completed | PaymentStatus::Failed | PaymentStatus::Cancelled) {
            self.completed_at = Some(Utc::now());
        }
    }

    pub fn set_peach_ids(&mut self, checkout_id: Option<String>, payment_id: Option<String>) {
        self.peach_checkout_id = checkout_id;
        self.peach_payment_id = payment_id;
        self.updated_at = Utc::now();
    }

    pub fn set_recurring_token(&mut self, token: String) {
        self.recurring_token = Some(token);
        self.payment_type = PaymentType::Recurring;
        self.updated_at = Utc::now();
    }

    pub fn increment_retry(&mut self) {
        self.retry_count += 1;
        self.updated_at = Utc::now();
    }

    pub fn is_final_status(&self) -> bool {
        matches!(
            self.status,
            PaymentStatus::Completed | PaymentStatus::Failed | PaymentStatus::Cancelled | PaymentStatus::Refunded
        )
    }

    pub fn can_retry(&self) -> bool {
        !self.is_final_status() && self.retry_count < 3
    }
}

impl PaymentStatus {
    pub fn from_peach_code(code: &str) -> Self {
        match code {
            // Successful codes
            code if code.starts_with("000.000") => PaymentStatus::Completed,
            code if code.starts_with("000.100") => PaymentStatus::Completed,
            
            // Pending codes
            code if code.starts_with("000.200") => PaymentStatus::Processing,
            
            // Cancelled by user
            "100.396.104" => PaymentStatus::Cancelled,
            
            // Failed codes
            _ => PaymentStatus::Failed,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_payment_status_from_peach_code() {
        assert_eq!(PaymentStatus::from_peach_code("000.000.000"), PaymentStatus::Completed);
        assert_eq!(PaymentStatus::from_peach_code("000.100.110"), PaymentStatus::Completed);
        assert_eq!(PaymentStatus::from_peach_code("000.200.100"), PaymentStatus::Processing);
        assert_eq!(PaymentStatus::from_peach_code("100.396.104"), PaymentStatus::Cancelled);
        assert_eq!(PaymentStatus::from_peach_code("800.100.100"), PaymentStatus::Failed);
    }

    #[test]
    fn test_payment_retry_logic() {
        let request = InitiatePaymentRequest {
            user_id: Uuid::new_v4(),
            subscription_id: Uuid::new_v4(),
            amount: Decimal::new(10000, 2),
            currency: Some("ZAR".to_string()),
            payment_method: PaymentMethod::Card,
            enable_recurring: Some(false),
            return_url: None,
            webhook_url: None,
            metadata: None,
        };

        let mut payment = Payment::new(request, "TXN_123".to_string());
        
        assert!(payment.can_retry());
        
        payment.increment_retry();
        payment.increment_retry();
        payment.increment_retry();
        
        assert!(!payment.can_retry());
    }
}