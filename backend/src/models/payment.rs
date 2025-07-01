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
}

impl fmt::Display for PaymentMethod {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        let s = match self {
            PaymentMethod::Card => "CARD",
            PaymentMethod::EFT => "EFT",
            PaymentMethod::Voucher => "VOUCHER",
        };
        write!(f, "{}", s)
    }
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
