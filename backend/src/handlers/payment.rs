use actix_web::{HttpResponse, Result, post, get};
use actix_web::web::{Data, Json, Path, Query};
use uuid::Uuid;
use serde::Deserialize;
use hmac::{Hmac, Mac};
use sha2::Sha256;
use crate::{
    models::{
        payment::{PaymentStatus, PaymentMethod, CreatePaymentDto},
        subscription::SubscriptionStatus,
    },
    services::{database::DatabaseService, peach::PeachPaymentService},
};

type HmacSha256 = Hmac<Sha256>;

#[derive(Deserialize)]
pub struct PaymentCallbackQuery {
    pub id: Option<String>,
    #[serde(rename = "resourcePath")]
    pub resource_path: Option<String>,
}

#[post("/initiate")]
pub async fn initiate_payment(
    db: Data<DatabaseService>,
    peach_service: Data<PeachPaymentService>,
    payload: Json<CreatePaymentDto>,
) -> Result<HttpResponse> {
    // Validate subscription exists and is pending
    let subscription_id = payload.subscription_id;
    let subscription = match db.get_subscription(&subscription_id) {
        Some(sub) => sub,
        None => return Ok(HttpResponse::NotFound().json("Subscription not found")),
    };

    if subscription.status != SubscriptionStatus::Pending {
        return Ok(HttpResponse::BadRequest().json("Subscription is not pending"));
    }

    // Create payment record
    let payment_dto = CreatePaymentDto {
        user_id: payload.user_id,
        subscription_id: payload.subscription_id,
        amount: payload.amount,
        payment_method: payload.payment_method.clone(),
    };

    let payment_record = match db.create_payment(payment_dto) {
        Ok(payment) => payment,
        Err(e) => return Ok(HttpResponse::InternalServerError().json(format!("Error creating payment: {}", e))),
    };

    // Generate signature for Peach API
    let mut params_for_signature = std::collections::HashMap::new();
    params_for_signature.insert("authentication.entityId".to_string(), peach_service.get_entity_id().clone());
    params_for_signature.insert("amount".to_string(), payload.amount.to_string());
    params_for_signature.insert("currency".to_string(), "ZAR".to_string());
    params_for_signature.insert("paymentType".to_string(), "DB".to_string());
    params_for_signature.insert("merchantTransactionId".to_string(), payment_record.merchant_transaction_id.clone());

    // Create signature
    let mut data_to_sign = String::new();
    for (key, value) in &params_for_signature {
        data_to_sign.push_str(&format!("{}={}&", key, value));
    }
    data_to_sign.push_str(&peach_service.get_secret_key());
    
    let mut mac = HmacSha256::new_from_slice(peach_service.get_secret_key().as_bytes())
        .expect("HMAC can take key of any size");
    mac.update(data_to_sign.as_bytes());
    let signature = hex::encode(mac.finalize().into_bytes());

    // Initiate checkout with Peach
    let payment_method = payload.payment_method.clone().unwrap_or(PaymentMethod::CreditCard);
    match peach_service.initiate_checkout_api(
        &payload.user_id,
        &payload.subscription_id,
        &payload.amount,
        &payment_method,
    ).await {
        Ok(checkout_response) => {
            // Update payment with checkout ID
            if let Some(checkout_id) = checkout_response.get("id").and_then(|v| v.as_str()) {
                let _ = db.update_payment_checkout_id(&payment_record.merchant_transaction_id, checkout_id);
            }
            
            Ok(HttpResponse::Ok().json(serde_json::json!({
                "payment_id": payment_record.id,
                "merchant_transaction_id": payment_record.merchant_transaction_id,
                "checkout_response": checkout_response,
                "signature": signature
            })))
        },
        Err(e) => Ok(HttpResponse::InternalServerError().json(format!("Error initiating payment: {}", e))),
    }
}

#[get("/status/{merchant_transaction_id}")]
pub async fn check_payment_status(
    db: Data<DatabaseService>,
    peach_service: Data<PeachPaymentService>,
    path: Path<String>,
) -> Result<HttpResponse> {
    let merchant_transaction_id = path.into_inner();
    
    // Get payment from database
    let payment = match db.get_payment_by_merchant_id(&merchant_transaction_id) {
        Some(payment) => payment,
        None => return Ok(HttpResponse::NotFound().json("Payment not found")),
    };

    // Check status with Peach if we have a checkout ID
    if let Some(checkout_id) = &payment.checkout_id {
        match peach_service.check_payment_status(checkout_id).await {
            Ok(status_response) => {
                // Update payment status based on response
                if let Some(result) = status_response.get("result") {
                    if let Some(code) = result.get("code").and_then(|v| v.as_str()) {
                        let new_status = if code.starts_with("000.000") || code.starts_with("000.100") {
                            PaymentStatus::Completed
                        } else if code.starts_with("000.200") {
                            PaymentStatus::Pending
                        } else {
                            PaymentStatus::Failed
                        };
                        
                        let _ = db.update_payment_status(&merchant_transaction_id, &new_status);
                        
                        // If payment is completed, activate subscription
                        if new_status == PaymentStatus::Completed {
                            if let Some(subscription_id) = payment.subscription_id {
                                let _ = db.activate_subscription(&subscription_id);
                            }
                        }
                    }
                }
                
                Ok(HttpResponse::Ok().json(status_response))
            },
            Err(e) => Ok(HttpResponse::InternalServerError().json(format!("Error checking payment status: {}", e))),
        }
    } else {
        Ok(HttpResponse::Ok().json(serde_json::json!({
            "payment": payment,
            "message": "No checkout ID available"
        })))
    }
}

#[get("/callback")]
pub async fn handle_payment_callback_get(
    query: Query<PaymentCallbackQuery>,
    _db: Data<DatabaseService>,
    peach_service: Data<PeachPaymentService>,
) -> Result<HttpResponse> {
    if let Some(resource_path) = &query.resource_path {
        // Extract checkout ID from resource path
        let checkout_id = resource_path.trim_start_matches('/').split('/').next().unwrap_or("");
        
        if !checkout_id.is_empty() {
            match peach_service.check_payment_status(checkout_id).await {
                Ok(status_response) => {
                    // Process the callback
                    return Ok(HttpResponse::Ok().json(serde_json::json!({
                        "status": "success",
                        "data": status_response
                    })));
                },
                Err(e) => {
                    return Ok(HttpResponse::InternalServerError().json(format!("Error processing callback: {}", e)));
                }
            }
        }
    }
    
    Ok(HttpResponse::Ok().json(serde_json::json!({
        "status": "received",
        "message": "Callback received but no valid resource path"
    })))
}

#[post("/callback")]
pub async fn handle_payment_callback_post(
    payload: Json<serde_json::Value>,
    db: Data<DatabaseService>,
) -> Result<HttpResponse> {
    // Log the callback for debugging
    println!("Payment callback received: {:?}", payload);
    
    // Extract relevant information from the callback
    if let Some(merchant_transaction_id) = payload.get("merchantTransactionId").and_then(|v| v.as_str()) {
        if let Some(result) = payload.get("result") {
            if let Some(code) = result.get("code").and_then(|v| v.as_str()) {
                let new_status = if code.starts_with("000.000") || code.starts_with("000.100") {
                    PaymentStatus::Completed
                } else if code.starts_with("000.200") {
                    PaymentStatus::Pending
                } else {
                    PaymentStatus::Failed
                };
                
                let _ = db.update_payment_status(merchant_transaction_id, &new_status);
                
                // If payment is completed, activate subscription
                if new_status == PaymentStatus::Completed {
                    if let Some(payment) = db.get_payment_by_merchant_id(merchant_transaction_id) {
                        if let Some(subscription_id) = payment.subscription_id {
                            let _ = db.activate_subscription(&subscription_id);
                        }
                    }
                }
            }
        }
    }
    
    Ok(HttpResponse::Ok().json(serde_json::json!({
        "status": "processed"
    })))
}
