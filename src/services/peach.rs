use reqwest::Client;
use serde_json::{json, Value};
use uuid::Uuid;
use anyhow::{Result, anyhow};
use chrono::{Utc, Duration};
use std::sync::Arc;
use tokio::sync::RwLock;
use hmac::{Hmac, Mac};
use sha2::Sha256;

use crate::config::PeachConfig;
use crate::models::{
    payment::{InitiatePaymentRequest, InitiatePaymentResponse, EmbedConfig, PaymentWebhookPayload},
    common::PaymentMethod,
};

type HmacSha256 = Hmac<Sha256>;

#[derive(Debug, Clone)]
struct AccessToken {
    token: String,
    expires_at: chrono::DateTime<Utc>,
}

#[derive(Clone)]
pub struct PeachPaymentService {
    client: Client,
    config: PeachConfig,
    access_token: Arc<RwLock<Option<AccessToken>>>,
}

impl PeachPaymentService {
    pub fn new(config: PeachConfig) -> Self {
        Self {
            client: Client::new(),
            config,
            access_token: Arc::new(RwLock::new(None)),
        }
    }

    /// Generate an access token using OAuth
    pub async fn authenticate(&self) -> Result<String> {
        // Check if we have a valid token
        {
            let token_guard = self.access_token.read().await;
            if let Some(token) = &*token_guard {
                if token.expires_at > Utc::now() + Duration::minutes(5) {
                    return Ok(token.token.clone());
                }
            }
        }

        // Generate new token
        let payload = json!({
            "clientId": self.config.client_id,
            "clientSecret": self.config.client_secret,
            "merchantId": self.config.merchant_id
        });

        log::info!("Requesting new Peach access token");

        let response = self.client
            .post(&format!("{}/api/oauth/token", self.config.auth_service_url))
            .header("content-type", "application/json")
            .json(&payload)
            .send()
            .await?;

        if !response.status().is_success() {
            let error_text = response.text().await?;
            return Err(anyhow!("Authentication failed: {}", error_text));
        }

        let auth_response: Value = response.json().await?;
        
        let access_token = auth_response["access_token"]
            .as_str()
            .ok_or_else(|| anyhow!("No access_token in response"))?;

        let expires_in = auth_response["expires_in"]
            .as_u64()
            .unwrap_or(3600); // Default to 1 hour

        let expires_at = Utc::now() + Duration::seconds(expires_in as i64);

        // Store the new token
        {
            let mut token_guard = self.access_token.write().await;
            *token_guard = Some(AccessToken {
                token: access_token.to_string(),
                expires_at,
            });
        }

        log::info!("Successfully obtained Peach access token");
        Ok(access_token.to_string())
    }

    /// Create a checkout instance for embedded checkout
    pub async fn create_checkout(&self, request: &InitiatePaymentRequest) -> Result<InitiatePaymentResponse> {
        let access_token = self.authenticate().await?;
        let nonce = Uuid::new_v4().to_string();
        let merchant_transaction_id = format!("TXN_{}", Uuid::new_v4().simple());

        let mut checkout_payload = json!({
            "entityId": self.config.entity_id,
            "amount": request.amount.to_string(),
            "currency": request.currency.as_ref().unwrap_or(&"ZAR".to_string()),
            "paymentType": "DB", // Debit transaction
            "merchantTransactionId": merchant_transaction_id,
            "nonce": nonce,
            "notificationUrl": self.config.notification_url,
            "shopperResultUrl": self.config.shopper_result_url,
        });

        // Add payment method specific parameters
        match request.payment_method {
            PaymentMethod::Card => {
                if request.enable_recurring.unwrap_or(false) {
                    checkout_payload["createRegistration"] = json!(true);
                }
            },
            PaymentMethod::Eft => {
                checkout_payload["paymentBrand"] = json!("EFT");
            },
            PaymentMethod::OneVoucher => {
                checkout_payload["paymentBrand"] = json!("1VOUCHER");
            },
            PaymentMethod::ScanToPay => {
                checkout_payload["paymentBrand"] = json!("SCAN_TO_PAY");
            },
        }

        // Add custom parameters
        if let Some(metadata) = &request.metadata {
            checkout_payload["customParameters"] = metadata.clone();
        }

        log::info!("Creating checkout with payload: {}", checkout_payload);

        let response = self.client
            .post(&format!("{}/v2/checkout", self.config.checkout_endpoint))
            .header("Authorization", format!("Bearer {}", access_token))
            .header("Origin", &self.config.origin_domain)
            .header("Content-Type", "application/json")
            .json(&checkout_payload)
            .send()
            .await?;

        if !response.status().is_success() {
            let error_text = response.text().await?;
            return Err(anyhow!("Checkout creation failed: {}", error_text));
        }

        let checkout_response: Value = response.json().await?;
        
        let checkout_id = checkout_response["checkoutId"]
            .as_str()
            .ok_or_else(|| anyhow!("No checkoutId in response"))?;

        log::info!("Successfully created checkout: {}", checkout_id);

        Ok(InitiatePaymentResponse {
            payment_id: request.user_id, // This should be replaced with actual payment ID from DB
            merchant_transaction_id,
            checkout_id: checkout_id.to_string(),
            checkout_url: Some(format!("{}/v2/checkout/{}", self.config.checkout_endpoint, checkout_id)),
            embed_config: Some(EmbedConfig {
                entity_id: self.config.entity_id.clone(),
                checkout_id: checkout_id.to_string(),
                script_url: "https://sandbox-checkout.peachpayments.com/js/checkout.js".to_string(),
            }),
        })
    }

    /// Check the status of a checkout
    pub async fn get_checkout_status(&self, checkout_id: &str) -> Result<Value> {
        let url = format!("{}/v2/checkout/{}/status", self.config.status_endpoint, checkout_id);

        log::info!("Checking checkout status for: {}", checkout_id);

        let response = self.client
            .get(&url)
            .header("Accept", "application/json")
            .send()
            .await?;

        if !response.status().is_success() {
            let error_text = response.text().await?;
            return Err(anyhow!("Status check failed: {}", error_text));
        }

        let status_response: Value = response.json().await?;
        log::info!("Checkout status response: {}", status_response);
        
        Ok(status_response)
    }

    /// Process a recurring payment using a stored registration token
    pub async fn process_recurring_payment(
        &self,
        registration_id: &str,
        amount: rust_decimal::Decimal,
        merchant_transaction_id: &str,
        user_id: &str,
    ) -> Result<Value> {
        let access_token = self.authenticate().await?;
        let nonce = Uuid::new_v4().to_string();

        let payload = json!({
            "entityId": self.config.entity_id,
            "amount": amount.to_string(),
            "currency": "ZAR",
            "paymentType": "DB",
            "merchantTransactionId": merchant_transaction_id,
            "nonce": nonce,
            "registrationId": registration_id,
            "customer": {
                "merchantCustomerId": user_id
            },
            "notificationUrl": self.config.notification_url,
            "customParameters": {
                "paymentType": "recurring"
            }
        });

        log::info!("Processing recurring payment with registration: {}", registration_id);

        let response = self.client
            .post(&format!("{}/v2/payments", self.config.checkout_endpoint))
            .header("Authorization", format!("Bearer {}", access_token))
            .header("Origin", &self.config.origin_domain)
            .header("Content-Type", "application/json")
            .json(&payload)
            .send()
            .await?;

        if !response.status().is_success() {
            let error_text = response.text().await?;
            return Err(anyhow!("Recurring payment failed: {}", error_text));
        }

        let payment_response: Value = response.json().await?;
        log::info!("Recurring payment response: {}", payment_response);
        
        Ok(payment_response)
    }

    /// Validate webhook signature
    pub fn validate_webhook_signature(&self, payload: &[u8], signature: &str) -> bool {
        let key = self.config.webhook_secret.as_bytes();
        
        let mut mac = match HmacSha256::new_from_slice(key) {
            Ok(mac) => mac,
            Err(_) => return false,
        };
        
        mac.update(payload);
        let calculated_signature = hex::encode(mac.finalize().into_bytes());
        
        log::debug!("Calculated signature: {}", calculated_signature);
        log::debug!("Provided signature: {}", signature);
        
        calculated_signature == signature
    }

    /// Parse webhook payload
    pub fn parse_webhook(&self, payload: &str) -> Result<PaymentWebhookPayload> {
        let webhook: PaymentWebhookPayload = serde_json::from_str(payload)?;
        Ok(webhook)
    }

    /// Get payment details by payment ID
    pub async fn get_payment_details(&self, payment_id: &str) -> Result<Value> {
        let access_token = self.authenticate().await?;
        let url = format!("{}/v1/payments/{}", self.config.checkout_endpoint, payment_id);

        log::info!("Getting payment details for: {}", payment_id);

        let response = self.client
            .get(&url)
            .header("Authorization", format!("Bearer {}", access_token))
            .header("Accept", "application/json")
            .send()
            .await?;

        if !response.status().is_success() {
            let error_text = response.text().await?;
            return Err(anyhow!("Payment details fetch failed: {}", error_text));
        }

        let payment_details: Value = response.json().await?;
        Ok(payment_details)
    }

    /// Extract registration token from payment response for future recurring payments
    pub fn extract_registration_token(&self, payment_response: &Value) -> Option<String> {
        payment_response.get("registrationId")
            .and_then(|v| v.as_str())
            .map(|s| s.to_string())
    }

    /// Create a test checkout for development/testing
    pub async fn create_test_checkout(&self, amount: rust_decimal::Decimal) -> Result<InitiatePaymentResponse> {
        let test_request = InitiatePaymentRequest {
            user_id: Uuid::new_v4(),
            subscription_id: Uuid::new_v4(),
            amount,
            currency: Some("ZAR".to_string()),
            payment_method: PaymentMethod::Card,
            enable_recurring: Some(true),
            return_url: None,
            webhook_url: None,
            metadata: Some(json!({
                "test": true,
                "environment": "development"
            })),
        };

        self.create_checkout(&test_request).await
    }

    /// Health check for Peach Payments service
    pub async fn health_check(&self) -> Result<()> {
        // Try to authenticate to verify service is working
        self.authenticate().await?;
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::models::common::PaymentMethod;

    fn create_test_config() -> PeachConfig {
        PeachConfig {
            auth_service_url: "https://test-auth.peach.com".to_string(),
            checkout_endpoint: "https://test-checkout.peach.com".to_string(),
            status_endpoint: "https://test-status.peach.com".to_string(),
            client_id: "test_client".to_string(),
            client_secret: "test_secret".to_string(),
            merchant_id: "test_merchant".to_string(),
            entity_id: "test_entity".to_string(),
            webhook_secret: "test_webhook_secret".to_string(),
            notification_url: "https://example.com/webhook".to_string(),
            shopper_result_url: "https://example.com/result".to_string(),
            origin_domain: "https://example.com".to_string(),
        }
    }

    #[test]
    fn test_webhook_signature_validation() {
        let config = create_test_config();
        let service = PeachPaymentService::new(config);
        
        let payload = b"test payload";
        let key = b"test_webhook_secret";
        
        let mut mac = HmacSha256::new_from_slice(key).unwrap();
        mac.update(payload);
        let valid_signature = hex::encode(mac.finalize().into_bytes());
        
        assert!(service.validate_webhook_signature(payload, &valid_signature));
        assert!(!service.validate_webhook_signature(payload, "invalid_signature"));
    }

    #[test]
    fn test_registration_token_extraction() {
        let config = create_test_config();
        let service = PeachPaymentService::new(config);
        
        let payment_response = json!({
            "id": "payment_123",
            "registrationId": "reg_456",
            "status": "completed"
        });
        
        let token = service.extract_registration_token(&payment_response);
        assert_eq!(token, Some("reg_456".to_string()));
        
        let response_without_token = json!({
            "id": "payment_123",
            "status": "completed"
        });
        
        let no_token = service.extract_registration_token(&response_without_token);
        assert_eq!(no_token, None);
    }
}