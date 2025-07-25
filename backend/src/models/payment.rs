use serde::{Deserialize, Serialize};
use chrono::{DateTime, Utc};
use surrealdb::sql::Thing;
use std::fmt;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Payment {
    pub id: Thing,
    pub user_id: String,
    pub subscription_id: Option<String>,
    pub amount: f64,
    pub status: PaymentStatus,
    pub payment_method: PaymentMethod,
    pub recurring_token: Option<String>,
    pub merchant_transaction_id: String,
    pub checkout_id: Option<String>,
    pub created_at: Option<DateTime<Utc>>,
    pub updated_at: Option<DateTime<Utc>>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum PaymentStatus {
    Pending,
    Completed,
    Failed,
    Cancelled,
    Refunded,
}

#[derive(Debug, Deserialize, Serialize, Clone, PartialEq)]
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

#[derive(Debug, Deserialize)]
pub struct CreatePaymentDto {
    pub user_id: String,
    pub subscription_id: String,
    pub amount: f64,
    pub payment_method: Option<PaymentMethod>,
}

#[derive(Debug, Deserialize)]
pub struct UpdatePaymentDto {
    pub status: Option<PaymentStatus>,
    pub checkout_id: Option<String>,
    pub recurring_token: Option<String>,
}

#[derive(Debug, Deserialize)]
pub struct PaymentCallbackDto {
    pub id: String,
    pub result: PaymentResult,
    #[serde(rename = "merchantTransactionId")]
    pub merchant_transaction_id: Option<String>,
}


// ... (existing structs)

#[derive(Debug, Serialize)]
pub struct InitiatePaymentResponse {
    #[serde(rename = "checkoutId")]
    pub checkout_id: String,
    #[serde(rename = "merchantTransactionId")]
    pub merchant_transaction_id: String,
    #[serde(rename = "redirectUrl", skip_serializing_if = "Option::is_none")]
    pub redirect_url: Option<String>,
}

#[derive(Debug, Deserialize)]
pub struct PaymentResult {
    pub code: String,
    pub description: String,
}
