use reqwest::Client;
use serde_json::{json, Value};
use std::collections::HashMap;
use uuid::Uuid;
use chrono::Utc;
use hmac::{Hmac, Mac};
use sha2::Sha256;

use crate::models::payment::PaymentMethod;

type HmacSha256 = Hmac<Sha256>;

#[derive(Clone)]
pub struct PeachPaymentService {
    client: Client,
    base_url: String,
    entity_id: String,
    access_token: String,
    secret_key: String, // Use as raw string
}

impl PeachPaymentService {
    pub fn new(base_url: String, entity_id: String, access_token: String, secret_key: String) -> Self {
        Self {
            client: Client::new(),
            base_url,
            entity_id,
            access_token,
            secret_key,
        }
    }

    pub fn get_entity_id(&self) -> &String {
        &self.entity_id
    }

    pub fn get_secret_key(&self) -> &String {
        &self.secret_key
    }

    pub async fn initiate_checkout_api(
    &self,
    user_id: &Uuid,
    subscription_id: &Uuid,
    amount: &f64,
    payment_method: &PaymentMethod,
) -> Result<Value, Box<dyn std::error::Error + Send + Sync>> {
    let merchant_transaction_id = format!(
        "TXN_{}_{}",
        user_id.to_string().replace("-", "")[..8].to_string(),
        Utc::now().timestamp_millis()
    );

    let nonce = Uuid::new_v4().to_string();

    let default_payment_method_str: Option<String> = match payment_method {
        PaymentMethod::BankTransfer => None, // Don't set for EFT
        PaymentMethod::CreditCard | PaymentMethod::DebitCard => Some("CARD".to_string()),
        PaymentMethod::Voucher => {
            return Ok(json!({
                "result": {
                    "code": "000.000.000",
                    "description": "Voucher applied"
                }
            }));
        }
    };

    let mut params_for_signature = HashMap::new();
    params_for_signature.insert("amount".to_string(), format!("{:.2}", amount));
    params_for_signature.insert("authentication.entityId".to_string(), self.entity_id.clone());
    params_for_signature.insert("currency".to_string(), "ZAR".to_string());
    params_for_signature.insert("merchantTransactionId".to_string(), merchant_transaction_id.clone());
    params_for_signature.insert("nonce".to_string(), nonce.clone());
    params_for_signature.insert("notificationUrl".to_string(), "".to_string());
    params_for_signature.insert("paymentType".to_string(), "DB".to_string());
    params_for_signature.insert(
        "shopperResultUrl".to_string(),
        format!("{}/api/v1/payments/callback", self.base_url),
    );

    // ✅ Only include test mode parameter for BankTransfer
    if let PaymentMethod::BankTransfer = payment_method {
        params_for_signature.insert("customParameters[enableTestMode]".to_string(), "true".to_string());
    }

    if let Some(method) = &default_payment_method_str {
        params_for_signature.insert("defaultPaymentMethod".to_string(), method.clone());
    }

    // Generate signature
    let mut sorted_params: Vec<(&String, &String)> = params_for_signature.iter().collect();
    sorted_params.sort_by_key(|(k, _)| *k);

    let mut data_to_sign = String::new();
    for (key, value) in sorted_params {
        data_to_sign.push_str(key);
        data_to_sign.push_str(value);
    }

    println!("DEBUG: Data to sign for Peach Payments: '{}'", data_to_sign);

    let mut mac = HmacSha256::new_from_slice(self.secret_key.as_bytes())
        .expect("HMAC can take key of any size");
    mac.update(data_to_sign.as_bytes());
    let generated_signature = hex::encode(mac.finalize().into_bytes());

    println!("DEBUG: Generated Signature: '{}'", generated_signature);

    let mut peach_params = HashMap::new();
    peach_params.insert("amount", format!("{:.2}", amount));
    peach_params.insert("authentication.entityId", self.entity_id.clone());
    peach_params.insert("currency", "ZAR".to_string());
    peach_params.insert("merchantTransactionId", merchant_transaction_id);
    peach_params.insert("nonce", nonce);
    peach_params.insert("notificationUrl", "".to_string());
    peach_params.insert("paymentType", "DB".to_string());
    peach_params.insert(
        "shopperResultUrl",
        format!("{}/api/v1/payments/callback", self.base_url),
    );
    peach_params.insert("signature", generated_signature);

    // ✅ Also insert the test mode flag into the actual request if EFT
    if let PaymentMethod::BankTransfer = payment_method {
        peach_params.insert("customParameters[enableTestMode]", "true".to_string());
    }

    if let Some(method) = default_payment_method_str {
        peach_params.insert("defaultPaymentMethod", method);
    }

    println!("DEBUG: JSON Payload being sent to Peach Payments: {}", json!(peach_params).to_string());

    let url = format!("{}/checkout/initiate", self.base_url);

    let response = self.client
        .post(&url)
        .form(&peach_params)
        .send()
        .await?;

    if response.status().is_success() {
        let json_response: Value = response.json().await?;
        Ok(json_response)
    } else {
        let error_text = response.text().await?;
        Err(format!("Peach API error: {}", error_text).into())
    }
}


    pub async fn check_payment_status(&self, checkout_id: &str) -> Result<Value, Box<dyn std::error::Error + Send + Sync>> {
        let url = format!("{}/v1/checkouts/{}/payment", self.base_url, checkout_id);

        let mut params = HashMap::new();
        params.insert("authentication.entityId", &self.entity_id);
        params.insert("authentication.accessToken", &self.access_token);

        let response = self.client
            .get(&url)
            .query(&params)
            .send()
            .await?;

        if response.status().is_success() {
            let json_response: Value = response.json().await?;
            Ok(json_response)
        } else {
            let error_text = response.text().await?;
            Err(format!("Peach API error: {}", error_text).into())
        }
    }

    pub async fn process_refund(&self, payment_id: &str, amount: f64) -> Result<Value, Box<dyn std::error::Error + Send + Sync>> {
        let url = format!("{}/v1/payments/{}", self.base_url, payment_id);

        let mut params = HashMap::new();
        params.insert("authentication.entityId", self.entity_id.clone());
        params.insert("authentication.accessToken", self.access_token.clone());
        params.insert("amount", amount.to_string());
        params.insert("currency", "ZAR".to_string());
        params.insert("paymentType", "RF".to_string());

        let response = self.client
            .post(&url)
            .form(&params)
            .send()
            .await?;

        if response.status().is_success() {
            let json_response: Value = response.json().await?;
            Ok(json_response)
        } else {
            let error_text = response.text().await?;
            Err(format!("Peach API error: {}", error_text).into())
        }
    }
}
