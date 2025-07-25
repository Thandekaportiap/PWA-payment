use actix_web::{HttpResponse, Result, post, get};
use actix_web::web::{Data, Json, Path, Query};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use actix_web::HttpRequest;
use crate::services::peach::PeachPaymentService;
use actix_web::web;
use crate::{
    models::{
        payment::{PaymentStatus, CreatePaymentDto, PaymentMethod, InitiatePaymentResponse},
        subscription::SubscriptionStatus,
    },
    services::database::DatabaseService,
};

#[derive(Debug, Serialize)]
pub struct ApiResponseError {
    pub message: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub details: Option<String>,
}

#[derive(Deserialize)]
pub struct RecurringChargeRequest {
    pub user_id: String,
    pub amount: f64,
    pub initial_transaction_id: String,
}

#[derive(Debug, Deserialize)]
pub struct PaymentCallbackQuery {
    pub resource_path: Option<String>,
}

#[post("/initiate")]
pub async fn initiate_payment(
    db: Data<DatabaseService>,
    peach_service: Data<PeachPaymentService>,
    payload: Json<CreatePaymentDto>,
) -> Result<HttpResponse> {
    // 1. Validate subscription exists and is pending
    let subscription_id = &payload.subscription_id;
    let subscription = match db.get_subscription(subscription_id).await {
        Some(sub) => sub,
        None => return Ok(HttpResponse::NotFound().json(ApiResponseError {
            message: "Subscription not found".to_string(),
            details: None,
        })),
    };
    
    if subscription.status != SubscriptionStatus::Pending {
        return Ok(HttpResponse::BadRequest().json(ApiResponseError {
            message: "Subscription is not pending".to_string(),
            details: None,
        }));
    }

    // 2. Create payment record
    let payment_dto = CreatePaymentDto {
        user_id: payload.user_id.clone(),
        subscription_id: payload.subscription_id.clone(),
        amount: payload.amount,
        payment_method: payload.payment_method.clone(),
    };
    
    let payment_record = match db.create_payment(payment_dto).await {
        Ok(payment) => payment,
        Err(e) => return Ok(HttpResponse::InternalServerError().json(ApiResponseError {
            message: "Error creating payment record".to_string(),
            details: Some(e.to_string()),
        })),
    };

    // 3. Initiate Peach Payments checkout
    let user_id_str = payment_record.user_id.clone();
    let subscription_id_str = payment_record.subscription_id.clone().unwrap_or_default();
    
   match peach_service
        .initiate_checkout_api_v2_with_tokenization(
            &user_id_str,
            &subscription_id_str,
            payment_record.amount,
            &payment_record.merchant_transaction_id,
        )
        .await
    {
        Ok(peach_response) => {
            let checkout_id = peach_response
                .get("id")
                .and_then(|v| v.as_str())
                .or_else(|| peach_response.get("checkoutId").and_then(|v| v.as_str()));
            
            // Extract the redirect URL if it exists
            let redirect_url = peach_response
                .get("redirect")
                .and_then(|v| v.get("url"))
                .and_then(|v| v.as_str());
                
            if let Some(checkout_id) = checkout_id {
                // Update the database with the checkout ID
                let _ = db.update_payment_checkout_id(&payment_record.merchant_transaction_id, checkout_id).await;
                
                // Return a structured response with the redirect URL
                let response_dto = InitiatePaymentResponse {
                    checkout_id: checkout_id.to_string(),
                    merchant_transaction_id: payment_record.merchant_transaction_id,
                    redirect_url: redirect_url.map(|s| s.to_string()),
                };
                
                Ok(HttpResponse::Ok().json(response_dto))
            } else {
                Ok(HttpResponse::InternalServerError().json(ApiResponseError {
                    message: "Peach Payments response missing 'id' or 'checkoutId'".to_string(),
                    details: Some(format!("Full response: {:?}", peach_response)),
                }))
            }
        }
        Err(e) => Ok(HttpResponse::InternalServerError().json(ApiResponseError {
            message: "Failed to initiate payment with Peach Payments".to_string(),
            details: Some(e.to_string()),
        })),
    }
}

#[post("/charge-recurring")]
pub async fn charge_recurring_payment(
    db: Data<DatabaseService>,
    peach: Data<PeachPaymentService>,
    payload: Json<RecurringChargeRequest>,
) -> Result<HttpResponse> {
    let token = match db.get_recurring_token_by_user(&payload.user_id).await {  // ✅ Added .await
        Some(t) => t,
        None => {
            return Ok(HttpResponse::BadRequest().json(ApiResponseError {
                message: "No stored card token found for user".to_string(),
                details: None,
            }));
        }
    };
    
    match peach
        .execute_recurring_payment(&token, payload.amount, &payload.initial_transaction_id)
        .await
    {
        Ok(response) => Ok(HttpResponse::Ok().json(response)),
        Err(e) => Ok(HttpResponse::InternalServerError().json(ApiResponseError {
            message: "Failed to execute recurring payment".to_string(),
            details: Some(e.to_string()),
        })),
    }
}

#[get("/status/{merchant_transaction_id}")]
pub async fn check_payment_status(
    db: Data<DatabaseService>,
    peach_service: Data<PeachPaymentService>,
    path: Path<String>,
) -> Result<HttpResponse> {
    let merchant_transaction_id = path.into_inner();
    
    let payment = match db.get_payment_by_merchant_id(&merchant_transaction_id).await {  // ✅ Added .await
        Some(p) => p,
        None => {
            return Ok(HttpResponse::NotFound().json(ApiResponseError {
                message: "Payment not found".to_string(),
                details: Some(merchant_transaction_id),
            }));
        }
    };
    
    let checkout_id = match &payment.checkout_id {
        Some(id) => id,
        None => {
            return Ok(HttpResponse::Ok().json(serde_json::json!({
                "status": "info",
                "message": "No checkout ID available",
                "payment_id": payment.id,
                "merchant_transaction_id": payment.merchant_transaction_id,
                "payment_method": format!("{:?}", payment.payment_method),
                "status": format!("{:?}", payment.status),
                "amount": payment.amount
            })));
        }
    };
    
    match peach_service.check_payment_status(checkout_id).await {
        Ok(status_response) => {
            let new_status = status_response
                .get("result")
                .and_then(|r| r.get("code"))
                .and_then(|c| c.as_str())
                .map(|code| {
                    if code.starts_with("000.000") || code.starts_with("000.100") {
                        PaymentStatus::Completed
                    } else if code.starts_with("000.200") {
                        PaymentStatus::Pending
                    } else {
                        PaymentStatus::Failed
                    }
                });
            
            if let Some(status) = new_status.clone() {
                let _ = db.update_payment_status(&merchant_transaction_id, &status).await;  // ✅ Added .await
                
                if status == PaymentStatus::Completed {
                    if let Some(subscription_id) = payment.subscription_id {
                        let _ = db.activate_subscription(&subscription_id).await;  // ✅ Added .await
                    }
                }
            }
            
            Ok(HttpResponse::Ok().json(serde_json::json!({
                "peach_response": status_response,
                "updated_status": new_status.map(|s| format!("{:?}", s)).unwrap_or("unknown".to_string()),
                "payment_id": payment.id,
                "merchant_transaction_id": payment.merchant_transaction_id,
                "payment_method": format!("{:?}", payment.payment_method),
                "amount": payment.amount
            })))
        }
        Err(e) => Ok(HttpResponse::InternalServerError().json(ApiResponseError {
            message: "Error checking payment status".to_string(),
            details: Some(e.to_string()),
        })),
    }
}

#[get("/checkout-status/{checkout_id}")]
pub async fn get_checkout_status_and_store(
    peach_service: Data<PeachPaymentService>,
    db: Data<DatabaseService>,
    path: Path<String>,
) -> Result<HttpResponse> {
    let checkout_id = path.into_inner();
    
    match peach_service.get_checkout_status(&checkout_id).await {
        Ok(status_response) => {
            let merchant_txn_id = status_response
                .get("merchantTransactionId")
                .and_then(|v| v.as_str())
                .map(|s| s.to_string());
            
            let result_code = status_response
                .get("result")
                .and_then(|r| r.get("code"))
                .and_then(|c| c.as_str())
                .unwrap_or_default();
            
            let payment_brand = status_response
                .get("paymentBrand")
                .and_then(|v| v.as_str())
                .map(|s| s.to_string());
            
            let payment_status = if result_code.starts_with("000.000") || result_code.starts_with("000.100") {
                PaymentStatus::Completed
            } else if result_code.starts_with("000.200") {
                PaymentStatus::Pending
            } else {
                PaymentStatus::Failed
            };
            
            if let Some(ref txn_id) = merchant_txn_id {
                let _ = db.update_payment_status(txn_id, &payment_status).await;  // ✅ Added .await
                
                if payment_status == PaymentStatus::Completed {
                    if let Some(payment) = db.get_payment_by_merchant_id(txn_id).await {  // ✅ Added .await
                        if let Some(subscription_id) = payment.subscription_id {
                            let _ = db.activate_subscription(&subscription_id).await;  // ✅ Added .await
                            
                            if let Some(brand_str) = payment_brand.clone() {
                                let method = match brand_str.to_lowercase().as_str() {
                                    "visa" | "mastercard" | "amex" => PaymentMethod::Card,
                                                                        "eft" => PaymentMethod::EFT,
                                    "1voucher" => PaymentMethod::Voucher,
                                    "scan_to_pay" => PaymentMethod::ScanToPay,
                                    _ => PaymentMethod::Card,
                                };
                                
                                let _ = db.update_subscription_payment_details(&subscription_id, method, Some(brand_str)).await;  // ✅ Added .await
                            }
                        }
                    }
                }
            }
            
            Ok(HttpResponse::Ok().json(serde_json::json!({
                "checkout_id": checkout_id,
                "merchant_transaction_id": merchant_txn_id,
                "result_code": result_code,
                "payment_brand": payment_brand,
                "updated_status": format!("{:?}", payment_status),
                "raw_response": status_response
            })))
        }
        Err(e) => Ok(HttpResponse::InternalServerError().json(ApiResponseError {
            message: "Failed to get checkout status".to_string(),
            details: Some(e.to_string()),
        })),
    }
}

pub async fn handle_payment_callback(query: Query<PaymentCallbackQuery>) -> HttpResponse {
    let query = query.into_inner();
    if let Some(resource_path) = query.resource_path {
        println!("Resource path: {}", resource_path);
        // Process further...
    } else {
        println!("No resource path provided");
    }
    HttpResponse::Ok().finish()
}

#[get("/callback")]
pub async fn handle_payment_callback_get(
    query: Query<PaymentCallbackQuery>,
    peach_service: Data<PeachPaymentService>,
) -> Result<HttpResponse> {
    if let Some(resource_path) = &query.resource_path {
        let parts: Vec<&str> = resource_path.trim_start_matches('/').split('/').collect();
        if parts.len() >= 2 && parts[0] == "checkouts" {
            let checkout_id = parts[1];
            match peach_service.check_payment_status(checkout_id).await {
                Ok(status_response) => {
                    let status_param = if status_response
                        .get("result")
                        .and_then(|r| r.get("code"))
                        .and_then(|c| c.as_str())
                        .map_or(false, |code| code.starts_with("000."))
                    {
                        "success"
                    } else {
                        "failure"
                    };
                    
                    return Ok(HttpResponse::Found()
                        .insert_header((
                            "Location",
                            format!(
                                "/payment-result.html?id={}&resourcePath={}",
                                status_response
                                    .get("merchantTransactionId")
                                    .and_then(|v| v.as_str())
                                    .unwrap_or("unknown"),
                                resource_path
                            ),
                        ))
                        .finish());
                }
                Err(e) => {
                    eprintln!("❌ Error processing GET callback: {}", e);
                    return Ok(HttpResponse::Found()
                        .insert_header(("Location", "/payment-result.html?status=error"))
                        .finish());
                }
            }
        }
    }
    
    Ok(HttpResponse::BadRequest().json(serde_json::json!({
        "status": "error",
        "message": "Invalid or missing resource path"
    })))
}

// Helper function to create signature payload in the correct format for Peach Payments
fn create_signature_payload(form_data: &HashMap<String, String>) -> String {
    // Get all parameters except signature
    let mut params: Vec<(&String, &String)> = form_data
        .iter()
        .filter(|(key, _)| *key != "signature")
        .collect();
    
    // Sort alphabetically by key
    params.sort_by(|a, b| a.0.cmp(b.0));
    
    // Concatenate key+value pairs (no separators)
    params
        .into_iter()
        .map(|(key, value)| format!("{}{}", key, value))
        .collect::<Vec<_>>()
        .join("")
}

#[post("/callback")]
pub async fn payment_callback(
    _req: HttpRequest,
    body: web::Bytes,
    peach_service: web::Data<PeachPaymentService>,
    db: web::Data<DatabaseService>,
) -> HttpResponse {
    println!("🔔 Webhook received at /callback");
    
    // 1. Log raw incoming data
    let body_str = match std::str::from_utf8(&body) {
        Ok(s) => s,
        Err(e) => {
            eprintln!("❌ Invalid UTF-8 body: {}", e);
            return HttpResponse::BadRequest().body("Invalid UTF-8");
        }
    };
    
    println!("📩 Raw webhook body: {}", body_str);
    println!("Body length: {} bytes", body.len());
    
    // 2. Parse form data
    let form_map: HashMap<String, String> = match serde_urlencoded::from_bytes(&body) {
        Ok(map) => map,
        Err(e) => {
            eprintln!("❌ Failed to parse form body: {}", e);
            return HttpResponse::BadRequest().body("Invalid form data");
        }
    };
    
    let provided_signature = form_map.get("signature").map(|s| s.as_str()).unwrap_or("");
    if provided_signature.is_empty() {
        eprintln!("❌ No signature provided in webhook");
        return HttpResponse::BadRequest().body("Missing signature");
    }
    
    // 3. Create and validate signature
    let signature_payload = create_signature_payload(&form_map);
    println!("🔍 Signature payload: {}", signature_payload);
    println!("🔍 Provided signature: {}", provided_signature);
    
    if !peach_service.validate_webhook_signature(signature_payload.as_bytes(), provided_signature) {
        eprintln!("❌ Signature validation failed");
        return HttpResponse::Unauthorized().body("Invalid signature");
    }
    
    println!("✅ Webhook signature validated successfully");
    
    // 4. Extract fields
    let status_code = form_map.get("result.code").cloned().unwrap_or_default();
    let merchant_transaction_id = form_map
        .get("merchantTransactionId")
        .cloned()
        .unwrap_or_default();
    let subscription_id = form_map
        .get("customParameters[subscription_id]")
        .or_else(|| form_map.get("customParameters%5Bsubscription_id%5D"))
        .cloned();
    
    println!(
        "🧾 Parsed: result.code={}, transaction_id={}, subscription_id={:?}",
        status_code, merchant_transaction_id, subscription_id
    );
    
    // 5. Process based on status code
 match status_code.as_str() {
        "000.000.000" | "000.100.110" => {
            println!("✅ Payment successful");
            
            // First, find the payment record
            if let Some(payment) = db.get_payment_by_merchant_id(&merchant_transaction_id).await {
                // Update payment status
                let _ = db.update_payment_status(&merchant_transaction_id, &PaymentStatus::Completed).await;
                println!("✅ Updated payment status: Completed (MerchantTxnId: {})", merchant_transaction_id);

                // Now, handle the subscription
                if let Some(ref sub_id) = payment.subscription_id {
                    // Check if the subscription exists *before* trying to activate it
                    if let None = db.get_subscription(sub_id).await {
                         eprintln!("❌ Failed to find subscription record {} linked to payment {}. This payment will not be linked to a subscription.", sub_id, merchant_transaction_id);
                         // Return early, as we cannot proceed without a subscription record
                         return HttpResponse::Ok().body("Webhook received, but subscription not found for activation.");
                    }

                    // Subscription exists, proceed with activation
                    match db.activate_subscription(sub_id).await {
                        Ok(_) => {
                            println!("✅ Subscription activated successfully (ID: {})", sub_id);
                        }
                        Err(e) => {
                            // This error will still be logged, but the above check makes it less likely
                            // It handles the edge case where the record might be deleted between checks.
                            eprintln!("❌ Failed to activate subscription {}: {}", sub_id, e);
                        }
                    }
                    
                    // Update payment brand and method
                    if let Some(payment_brand_str) = form_map.get("paymentBrand").cloned() {
                        let brand_lc = payment_brand_str.to_lowercase();
                        let method = match brand_lc.as_str() {
                            "visa" | "mastercard" | "amex" => PaymentMethod::Card,
                            "eft" | "ozow" => PaymentMethod::EFT,
                            "1voucher" | "1foryou" => PaymentMethod::Voucher,
                            "scan_to_pay" | "scantopay" | "masterpass" => PaymentMethod::ScanToPay,
                            _ => {
                                eprintln!("⚠️ Unknown paymentBrand: '{}', defaulting to Card", brand_lc);
                                PaymentMethod::Card
                            }
                        };
                        
                        let _ = db.update_subscription_payment_details(
                            sub_id,
                            method.clone(),
                            Some(payment_brand_str.clone()),
                        ).await;
                        
                        println!(
                            "🔄 Updated subscription {} with payment method {:?} and brand {}",
                            sub_id, method, payment_brand_str
                        );
                    } else {
                        println!("ℹ️ No paymentBrand found in webhook for subscription {}", sub_id);
                    }
                } else {
                    println!("ℹ️ Payment {} has no subscription ID linked. No subscription to activate.", merchant_transaction_id);
                }
            } else {
                println!("⚠️ No payment found for merchantTransactionId: {}", merchant_transaction_id);
            }
        }
        "100.396.104" => {
            println!("⚠️ Payment uncertain/cancelled by user");
            let _ = db.update_payment_status(&merchant_transaction_id, &PaymentStatus::Failed).await;
        }
        "000.200.100" => {
            println!("ℹ️ Checkout created - no action needed");
        }
        "000.200.000" => {
            println!("ℹ️ Payment pending - no action needed");
        }
        _ => {
            println!("⚠️ Unhandled result.code: {}", status_code);
        }
    }
    
    HttpResponse::Ok().body("Webhook received")
}
