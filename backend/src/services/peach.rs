use reqwest::Client;
use serde_json::{json, Value};
use std::collections::HashMap;
use uuid::Uuid;
use chrono::Utc;
use std::error::Error;
use hmac::{Hmac, Mac};
use sha2::Sha256;
use std::collections::BTreeMap;

use crate::models::payment::PaymentMethod;

#[derive(Clone)]
pub struct PeachPaymentService {
    client: Client,
    v1_base_url: String,
    v1_entity_id: String,
    v1_access_token: String,
    v1_secret_key: String,
    v2_auth_url: String,
    v2_checkout_url: String,
    v2_entity_id: String,
    client_id: String,
    client_secret: String,
    merchant_id: String,
    notification_url: String,
    shopper_result_url: String,
}

impl PeachPaymentService {
    pub fn new(
        v1_base_url: String,
        v1_entity_id: String,
        v1_access_token: String,
        v1_secret_key: String,
        v2_auth_url: String,
        v2_checkout_url: String,
        v2_entity_id: String,
        client_id: String,
        client_secret: String,
        merchant_id: String,
        notification_url: String,
        shopper_result_url: String,
    ) -> Self {
        Self {
            client: Client::new(),
            v1_base_url,
            v1_entity_id,
            v1_access_token,
            v1_secret_key,
            v2_auth_url,
            v2_checkout_url,
            v2_entity_id,
            client_id,
            client_secret,
            merchant_id,
            notification_url,
            shopper_result_url,
        }
    }

    pub async fn initiate_checkout_api_v2(
        &self,
        user_id: &str,
        subscription_id: &str,
        amount: f64,
    ) -> Result<Value, Box<dyn Error + Send + Sync>> {
        let token = self.get_oauth_token().await?;

        let transaction_id = Uuid::new_v4().to_string();
        let nonce = Uuid::new_v4().to_string();

        let payload = json!({
            "authentication": {
                "entityId": self.v2_entity_id,
            },
            "amount": amount,
            "currency": "ZAR",
            "merchantTransactionId": transaction_id,
            "paymentType": "DB",
            "nonce": nonce,
            "customer": {
                "merchantCustomerId": user_id
            },
            "customParameters": {
                "subscription_id": subscription_id
            },
            "notificationUrl": self.notification_url,
            "shopperResultUrl": self.shopper_result_url
        });

        println!("Initiate Checkout V2 Payload: {}", payload);

        let response = self.client
            .post(&self.v2_checkout_url)
            .header("content-type", "application/json")
            .header("Origin", "https://7a12-105-0-3-186.ngrok-free.app")
            .bearer_auth(token)
            .json(&payload)
            .send()
            .await?;

        let status = response.status();
        let body_text = response.text().await?;

        println!("Checkout API response status: {}", status);
        println!("Checkout API response body: {}", body_text);

        if !status.is_success() {
            return Err(format!("Checkout API error: Status {}, Body: {}", status, body_text).into());
        }

        let body: Value = serde_json::from_str(&body_text)?;
        let checkout_id = body["checkoutId"]
            .as_str()
            .ok_or("Peach Payments response missing 'checkoutId'")?;

        Ok(json!({ "checkoutId": checkout_id }))
    }

    pub async fn check_payment_status(&self, checkout_id: &str) -> Result<Value, Box<dyn Error + Send + Sync>> {
        let url = format!(
            "{}/v1/checkouts/{}/payment?entityId={}",
            self.v1_base_url, checkout_id, self.v1_entity_id
        );

        let response = self.client
            .get(&url)
            .bearer_auth(&self.v1_access_token)
            .send()
            .await?;

        let status = response.status();
        let body_text = response.text().await?;

        println!("Check Payment Status API response status: {}", status);
        println!("Check Payment Status API response body: {}", body_text);

        if !status.is_success() {
            return Err(format!("Check Payment Status API error: Status {}, Body: {}", status, body_text).into());
        }

        let body: Value = serde_json::from_str(&body_text)?;
        Ok(body)
    }

    pub async fn process_refund(&self, payment_id: &str, amount: &str) -> Result<Value, Box<dyn Error + Send + Sync>> {
        let url = format!("{}/v1/payments/{}/refund", self.v1_base_url, payment_id);

        let mut form = HashMap::new();
        form.insert("entityId", self.v1_entity_id.clone());
        form.insert("amount", amount.to_string());
        form.insert("currency", "ZAR".to_string());
        form.insert("paymentType", "RF".to_string());

        let response = self.client
            .post(&url)
            .bearer_auth(&self.v1_access_token)
            .form(&form)
            .send()
            .await?;

        let body = response.json::<Value>().await?;
        Ok(body)
    }

    pub async fn get_oauth_token(&self) -> Result<String, Box<dyn Error + Send + Sync>> {
        let payload = json!({
            "clientId": self.client_id,
            "clientSecret": self.client_secret,
            "merchantId": self.merchant_id
        });

        println!("OAuth request URL: {}", self.v2_auth_url);
        println!("OAuth payload: {}", payload);

        let response = self.client
            .post(&self.v2_auth_url)
            .header("Content-Type", "application/json")
            .json(&payload)
            .send()
            .await?;

        let status = response.status();
        let response_text = response.text().await?;

        println!("OAuth response status: {}", status);
        println!("OAuth response body: {}", response_text);

        if !status.is_success() {
            return Err(format!("OAuth failed: Status {}, Body: {}", status, response_text).into());
        }

        let body: Value = serde_json::from_str(&response_text)?;

        println!("Client ID length: {}", self.client_id.len());
        println!("Client Secret length: {}", self.client_secret.len());
        println!("Merchant ID length: {}", self.merchant_id.len());

        let token = body["access_token"]
            .as_str()
            .ok_or("No access_token in response")?
            .to_string();

        Ok(token)
    }

    /// Calculates the HMAC-SHA256 signature for webhook validation
    pub fn calculate_signature(&self, body: &[u8]) -> String {
        type HmacSha256 = Hmac<Sha256>;
        let key = self.v1_secret_key.as_bytes();
        
        let mut mac = HmacSha256::new_from_slice(key)
            .expect("HMAC can take key of any size");
        mac.update(body);
        hex::encode(mac.finalize().into_bytes())
    }

    /// Validates the webhook signature against the calculated signature
    pub fn validate_webhook_signature(&self, body: &[u8], signature: &str) -> bool {
        let calculated = self.calculate_signature(body);
        println!("Calculated signature: {}", calculated);
        println!("Provided signature:   {}", signature);
        calculated == signature
    }

    pub fn validate_config(&self) -> Result<(), Box<dyn Error + Send + Sync>> {
        if self.client_id.is_empty() {
            return Err("Client ID is empty".into());
        }
        if self.client_secret.is_empty() {
            return Err("Client Secret is empty".into());
        }
        if self.merchant_id.is_empty() {
            return Err("Merchant ID is empty".into());
        }
        if self.v2_auth_url.is_empty() {
            return Err("V2 Auth URL is empty".into());
        }
        if self.v2_checkout_url.is_empty() {
            return Err("V2 Checkout URL is empty".into());
        }
        if self.v2_entity_id.is_empty() {
            return Err("V2 Entity ID is empty".into());
        }
        if self.notification_url.is_empty() {
            return Err("Notification URL is empty".into());
        }
        if self.shopper_result_url.is_empty() {
            return Err("Shopper Result URL is empty".into());
        }

        println!("✓ Peach Payment Service configuration validated");
        Ok(())
    }
}