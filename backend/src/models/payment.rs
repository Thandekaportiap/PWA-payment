use serde::{Deserialize, Serialize};
use uuid::Uuid;
use chrono::{DateTime, Utc};
use std::fmt;

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum PaymentStatus {
    Pending,
    Completed,
    Failed,
    Cancelled,
    Refunded,
}


#[derive(Debug, Deserialize, Serialize, Clone)]
pub enum PaymentMethod {
    Card,
    EFT,
    Voucher,
    ScanToPay,
}

impl fmt::Display for PaymentMethod {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        let s = match self {
            PaymentMethod::Card => "CARD",
            PaymentMethod::EFT => "EFT",
            PaymentMethod::Voucher => "VOUCHER",
            PaymentMethod::ScanToPay => "SCAN_TO_PAY",
        };
        write!(f, "{}", s)
    }
}

// New struct for storing payment method details from successful transactions
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PaymentMethodDetail {
    pub id: Uuid,
    pub user_id: Uuid,
    pub payment_method: PaymentMethod,
    pub peach_registration_id: Option<String>, // For recurring payments
    pub card_last_four: Option<String>,
    pub card_brand: Option<String>, // Visa, Mastercard, etc.
    pub expiry_month: Option<u8>,
    pub expiry_year: Option<u16>,
    pub bank_name: Option<String>, // For EFT
    pub is_default: bool,
    pub is_active: bool,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

// New struct for recurring payment requests
#[derive(Debug, Deserialize)]
pub struct CreateRecurringPaymentDto {
    pub user_id: Uuid,
    pub subscription_id: Uuid,
    pub amount: f64,
    pub payment_method_detail_id: Uuid, // Reference to stored payment method
}

// New struct for storing payment method from successful transaction
#[derive(Debug, Deserialize)]
pub struct StorePaymentMethodDto {
    pub payment_id: Uuid,
    pub set_as_default: Option<bool>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Payment {
    pub id: Uuid,
    pub user_id: Uuid,
    pub subscription_id: Option<Uuid>,
    pub amount: f64,
    pub status: PaymentStatus,
    pub payment_method: PaymentMethod,
    pub merchant_transaction_id: String,
    pub checkout_id: Option<String>,
    pub peach_payment_id: Option<String>, // ID from Peach after successful payment
    pub is_recurring: bool,
    pub parent_payment_id: Option<Uuid>, // For recurring payments, reference to original
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

#[derive(Debug, Deserialize)]
pub struct CreatePaymentDto {
    pub user_id: Uuid,
    pub subscription_id: Uuid,
    pub amount: f64,
    pub payment_method: Option<PaymentMethod>,
}

#[derive(Debug, Deserialize)]
pub struct PaymentCallbackDto {
    pub id: String,
    pub result: PaymentResult,
    #[serde(rename = "merchantTransactionId")]
    pub merchant_transaction_id: Option<String>,
}

#[derive(Debug, Deserialize)]
pub struct PaymentResult {
    pub code: String,
    pub description: String,
}
