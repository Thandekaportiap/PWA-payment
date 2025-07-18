use reqwest::Client;
use serde_json::{json, Value};
use std::error::Error;
use hmac::{Hmac, Mac};
use sha2::Sha256;
use uuid::Uuid;

#[derive(Clone)]
pub struct PeachPaymentService {
    client: Client,
    v2_auth_url: String,
    v2_checkout_url: String,
    v2_entity_id: String,
    client_id: String,
    client_secret: String,
    merchant_id: String,
    notification_url: String,
    shopper_result_url: String,
    webhook_secret_key: String,
}

impl PeachPaymentService {
    pub fn new(
        v2_auth_url: String,
        v2_checkout_url: String,
        v2_entity_id: String,
        client_id: String,
        client_secret: String,
        merchant_id: String,
        notification_url: String,
        shopper_result_url: String,
        webhook_secret_key: String, // For webhook HMAC
    ) -> Self {
        Self {
            client: Client::new(),
            v2_auth_url,
            v2_checkout_url,
            v2_entity_id,
            client_id,
            client_secret,
            merchant_id,
            notification_url,
            shopper_result_url,
            webhook_secret_key,
        }
    }

    pub async fn initiate_checkout_api_v2_with_tokenization(
        &self,
        user_id: &str,
        subscription_id: &str,
        amount: f64,
        merchant_transaction_id: &str,
    ) -> Result<Value, Box<dyn std::error::Error + Send + Sync>> {
        let token = self.get_oauth_token().await?;

        let nonce = Uuid::new_v4().to_string();
let payload = json!({
    "authentication": {
        "entityId": self.v2_entity_id,
    },
    "amount": amount,
    "currency": "ZAR",
    "merchantTransactionId": merchant_transaction_id,
    "paymentType": "DB",
    "nonce": nonce,
    "customer": {
        "merchantCustomerId": user_id
    },
    "createRegistration": true,
    "customParameters": {
        "subscription_id": subscription_id,
        "user_id": user_id
    },
    "notificationUrl": self.notification_url,
    "shopperResultUrl": self.shopper_result_url
});
        println!("Initiate Checkout V2 Payload: {}", payload);

        let response = self.client
            .post(&self.v2_checkout_url)
            .header("Content-Type", "application/json")
            .header("Origin", "http://127.0.0.1:8001")

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
        Ok(body)
    }

    pub async fn execute_recurring_payment(
        &self,
        registration_id: &str,
        amount: f64,
        initial_transaction_id: &str,
    ) -> Result<Value, Box<dyn std::error::Error + Send + Sync>> {
        let url = format!("{}/registrations/{}/payments", self.v2_checkout_url, registration_id);

        let payload = [
            ("entityId", self.v2_entity_id.as_str()),
            ("amount", &amount.to_string()),
            ("currency", "ZAR"),
            ("paymentType", "PA"),
            ("standingInstruction.mode", "REPEATED"),
            ("standingInstruction.type", "RECURRING"),
            ("standingInstruction.source", "MIT"),
            ("standingInstruction.initialTransactionId", initial_transaction_id),
        ];

        let response = self.client
            .post(&url)
            .form(&payload)
            .send()
            .await?
            .json::<Value>()
            .await?;

        Ok(response)
    }

    pub async fn get_oauth_token(&self) -> Result<String, Box<dyn std::error::Error + Send + Sync>> {
        let payload = json!({
            "clientId": self.client_id,
            "clientSecret": self.client_secret,
            "merchantId": self.merchant_id
        });

        let response = self.client
            .post(&self.v2_auth_url)
            .header("Content-Type", "application/json")
            .json(&payload)
            .send()
            .await?;

        let status = response.status();
        let response_text = response.text().await?;

        if !status.is_success() {
            return Err(format!("OAuth failed: Status {}, Body: {}", status, response_text).into());
        }

        let body: Value = serde_json::from_str(&response_text)?;
        let token = body["access_token"]
            .as_str()
            .ok_or("No access_token in response")?
            .to_string();

        Ok(token)
    }

    /// Calculates the HMAC-SHA256 signature for webhook validation
    pub fn calculate_signature(&self, body: &[u8]) -> String {
        type HmacSha256 = Hmac<Sha256>;
        let key = self.webhook_secret_key.as_bytes();

        let mut mac = HmacSha256::new_from_slice(key)
            .expect("HMAC can take key of any size");
        mac.update(body);
        hex::encode(mac.finalize().into_bytes())
    }

    /// Validates the webhook signature
    pub fn validate_webhook_signature(&self, body: &[u8], signature: &str) -> bool {
        let calculated = self.calculate_signature(body);
        calculated == signature
    }

    /// Optional: Validate required config fields
    pub fn validate_config(&self) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
        if self.client_id.is_empty()
            || self.client_secret.is_empty()
            || self.merchant_id.is_empty()
            || self.v2_auth_url.is_empty()
            || self.v2_checkout_url.is_empty()
            || self.v2_entity_id.is_empty()
            || self.notification_url.is_empty()
            || self.shopper_result_url.is_empty()
        {
            return Err("Missing Peach config values".into());
        }

        println!("✓ PeachPaymentService config validated");
        Ok(())
    }

    pub async fn check_payment_status(&self, checkout_id: &str) -> Result<Value, Box<dyn std::error::Error + Send + Sync>> {
        // TODO: Implement querying Peach Payments status endpoint
        unimplemented!()
    }

    pub async fn get_checkout_status(&self, checkout_id: &str) -> Result<serde_json::Value, Box<dyn std::error::Error + Send + Sync>> {
    let token = self.get_oauth_token().await?;
    let url = format!("{}/checkouts/{}", self.v2_checkout_url, checkout_id);

    let response = self.client
        .get(&url)
        .bearer_auth(token)
        .send()
        .await?;

    let status = response.status();
    let body_text = response.text().await?;

    if !status.is_success() {
        return Err(format!("Checkout status API error: Status {}, Body: {}", status, body_text).into());
    }

    let body: serde_json::Value = serde_json::from_str(&body_text)?;
    Ok(body)
}

}
