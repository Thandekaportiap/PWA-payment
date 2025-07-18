use serde::{Deserialize, Serialize};
use chrono::{DateTime, Utc};
use uuid::Uuid;

#[derive(Debug, Serialize)]
pub struct ApiResponse<T> {
    pub success: bool,
    pub data: Option<T>,
    pub message: Option<String>,
    pub error: Option<String>,
}

impl<T> ApiResponse<T> {
    pub fn success(data: T) -> Self {
        Self {
            success: true,
            data: Some(data),
            message: None,
            error: None,
        }
    }

    pub fn success_with_message(data: T, message: String) -> Self {
        Self {
            success: true,
            data: Some(data),
            message: Some(message),
            error: None,
        }
    }

    pub fn error(error: String) -> Self {
        Self {
            success: false,
            data: None,
            message: None,
            error: Some(error),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum PaymentMethod {
    Card,
    Eft,
    OneVoucher,
    ScanToPay,
}

impl std::fmt::Display for PaymentMethod {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            PaymentMethod::Card => write!(f, "CARD"),
            PaymentMethod::Eft => write!(f, "EFT"),
            PaymentMethod::OneVoucher => write!(f, "1VOUCHER"),
            PaymentMethod::ScanToPay => write!(f, "SCAN_TO_PAY"),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum SubscriptionPlan {
    Monthly,
    Annual,
}

impl SubscriptionPlan {
    pub fn price(&self) -> rust_decimal::Decimal {
        match self {
            SubscriptionPlan::Monthly => rust_decimal::Decimal::new(100_00, 2), // R100.00
            SubscriptionPlan::Annual => rust_decimal::Decimal::new(1000_00, 2), // R1000.00
        }
    }

    pub fn duration_days(&self) -> i64 {
        match self {
            SubscriptionPlan::Monthly => 30,
            SubscriptionPlan::Annual => 365,
        }
    }
}

impl std::fmt::Display for SubscriptionPlan {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            SubscriptionPlan::Monthly => write!(f, "monthly"),
            SubscriptionPlan::Annual => write!(f, "annual"),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PaginationQuery {
    pub page: Option<u32>,
    pub limit: Option<u32>,
}

impl Default for PaginationQuery {
    fn default() -> Self {
        Self {
            page: Some(1),
            limit: Some(20),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PaginatedResponse<T> {
    pub data: Vec<T>,
    pub total: u32,
    pub page: u32,
    pub limit: u32,
    pub total_pages: u32,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AuditLog {
    pub id: Uuid,
    pub entity_type: String, // "user", "payment", "subscription"
    pub entity_id: Uuid,
    pub action: String,
    pub old_value: Option<serde_json::Value>,
    pub new_value: Option<serde_json::Value>,
    pub user_id: Option<Uuid>,
    pub ip_address: Option<String>,
    pub user_agent: Option<String>,
    pub created_at: DateTime<Utc>,
}