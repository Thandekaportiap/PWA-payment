use actix_web::{HttpResponse, Result, post, get, delete};
use actix_web::web::{Data, Json, Path, Query};
use bytes::Bytes;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use actix_web::HttpRequest;
use crate::services::peach::PeachPaymentService;
use actix_web::web; // Add this line
use crate::{
    models::{
        payment::{PaymentStatus, CreatePaymentDto, CreateRecurringPaymentDto, StorePaymentMethodDto, PaymentMethodDetail},
        subscription::SubscriptionStatus,
    },
    services::{database::DatabaseService},
};

#[derive(Debug, Serialize)]
pub struct ApiResponseError {
    pub message: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub details: Option<String>,
}

#[derive(Deserialize)]
pub struct PaymentCallbackQuery {
    pub id: Option<String>,
    #[serde(rename = "resourcePath")]
    pub resource_path: Option<String>,
}

// POST /initiate
#[post("/initiate")]
pub async fn initiate_payment(
    db: Data<DatabaseService>,
    peach_service: Data<PeachPaymentService>,
    payload: Json<CreatePaymentDto>,
) -> Result<HttpResponse> {
    let subscription_id = &payload.subscription_id;
    let subscription = match db.get_subscription(subscription_id) {
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

    let initial_payment_id = uuid::Uuid::new_v4(); // Generate a new UUID for the payment
    let merchant_transaction_id = format!("TXN_{}", initial_payment_id.simple()); // Prefix with TXN_ for internal tracking

    let payment_dto = CreatePaymentDto {
        user_id: payload.user_id.clone(),
        subscription_id: payload.subscription_id.clone(),
        amount: payload.amount,
        payment_method: None,
    };

    let payment_record = match db.create_payment(payment_dto) {
        Ok(payment) => payment,
        Err(e) => return Ok(HttpResponse::InternalServerError().json(ApiResponseError {
            message: "Error creating payment record".to_string(),
            details: Some(e.to_string()),
        })),
    };


     let merchant_transaction_id = payment_record.merchant_transaction_id.clone();
    let user_id_str = payload.user_id.to_string();
    let subscription_id_str = payload.subscription_id.to_string();


    match peach_service
         .initiate_checkout_api_v2(&user_id_str, &subscription_id_str, payload.amount, &payment_record.merchant_transaction_id)
        .await
    {
        Ok(peach_response) => {
           if let Some(checkout_id) = peach_response.get("checkoutId").and_then(|v| v.as_str()) {
    let _ = db.update_payment_checkout_id(&payment_record.merchant_transaction_id, checkout_id);

    Ok(HttpResponse::Ok().json(serde_json::json!({
        "checkoutId": checkout_id,
        "merchantTransactionId": payment_record.merchant_transaction_id
    })))
            } else {
                Ok(HttpResponse::InternalServerError().json(ApiResponseError {
                    message: "Peach Payments response missing 'checkoutId'".to_string(),
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

// GET /status/{merchant_transaction_id}
#[get("/status/{merchant_transaction_id}")]
pub async fn check_payment_status(
    db: Data<DatabaseService>,
    peach_service: Data<PeachPaymentService>,
    path: Path<String>,
) -> Result<HttpResponse> {
    let merchant_transaction_id = path.into_inner();
    let payment = match db.get_payment_by_merchant_id(&merchant_transaction_id) {
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
                "payment_details": payment
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
                let _ = db.update_payment_status(&merchant_transaction_id, &status);

                if status == PaymentStatus::Completed {
                    if let Some(subscription_id) = payment.subscription_id {
                        let _ = db.activate_subscription(&subscription_id);
                    }
                }
            }

            Ok(HttpResponse::Ok().json(serde_json::json!({
                "peach_response": status_response,
                "updated_status": new_status.map(|s| format!("{:?}", s)).unwrap_or("unknown".to_string())
            })))
        }
        Err(e) => Ok(HttpResponse::InternalServerError().json(ApiResponseError {
            message: "Error checking payment status".to_string(),
            details: Some(e.to_string()),
        })),
    }
}

// GET /callback (legacy Peach redirect fallback)
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
    // serde_urlencoded already decoded the values for us
    params
        .into_iter()
        .map(|(key, value)| format!("{}{}", key, value))
        .collect::<Vec<_>>()
        .join("")
}

// POST /callback (webhook)
#[post("/callback")]
pub async fn payment_callback(
    req: HttpRequest,
    body: web::Bytes,
    peach_service: web::Data<PeachPaymentService>,
    db: web::Data<DatabaseService>,
) -> HttpResponse {
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

    // 2. Parse form data to get all parameters
    let form_map: HashMap<String, String> = match serde_urlencoded::from_bytes(&body) {
        Ok(map) => map,
        Err(e) => {
            eprintln!("❌ Failed to parse form body: {}", e);
            return HttpResponse::BadRequest().body("Invalid form data");
        }
    };

    let provided_signature = form_map.get("signature")
        .map(|s| s.as_str())
        .unwrap_or("");

    if provided_signature.is_empty() {
        eprintln!("❌ No signature provided in webhook");
        return HttpResponse::BadRequest().body("Missing signature");
    }

    // 3. Create signature payload using Peach Payments format
    let signature_payload = create_signature_payload(&form_map);
    
    println!("🔍 Original body: {}", body_str);
    println!("🔍 Signature payload (Peach format): {}", signature_payload);
    println!("🔍 Provided signature: {}", provided_signature);

    // 4. Validate signature using the properly formatted payload
    if !peach_service.validate_webhook_signature(signature_payload.as_bytes(), provided_signature) {
        eprintln!("❌ Signature validation failed");
        eprintln!("Expected format: key1value1key2value2... (sorted alphabetically)");
        return HttpResponse::Unauthorized().body("Invalid signature");
    }

    println!("✅ Webhook signature validated successfully");

    // Extract important fields
    let status_code = form_map.get("result.code").cloned().unwrap_or_default();
    let merchant_transaction_id = form_map
        .get("merchantTransactionId")
        .cloned()
        .unwrap_or_default();
    
    // Handle both encoded and unencoded parameter names for subscription_id
    let subscription_id = form_map
        .get("customParameters[subscription_id]")
        .or_else(|| form_map.get("customParameters%5Bsubscription_id%5D"))
        .cloned();

    println!(
        "🧾 Parsed: result.code={}, transaction_id={}, subscription_id={:?}",
        status_code, merchant_transaction_id, subscription_id
    );

    // Process based on status code
    match status_code.as_str() {
        "000.000.000" | "000.100.110" => {
            println!("✅ Payment successful");

            // Fetch the payment record from DB
            if let Some(payment) = db.get_payment_by_merchant_id(&merchant_transaction_id) {
                let _ = db.update_payment_status(&merchant_transaction_id, &PaymentStatus::Completed);

                // Extract and store payment method details from webhook data
                let payment_id = form_map.get("id").cloned();
                if let Some(peach_payment_id) = payment_id {
                    let _ = db.update_payment_peach_id(&merchant_transaction_id, &peach_payment_id);
                    
                    // Auto-store payment method for future recurring payments
                    // This happens automatically for successful payments unless it's already a recurring payment
                    if !payment.is_recurring {
                        tokio::spawn({
                            let peach_service = peach_service.clone();
                            let db = db.clone();
                            let payment_clone = payment.clone();
                            let peach_payment_id_clone = peach_payment_id.clone();
                            
                            async move {
                                // Wait a bit for Peach to finalize the payment details
                                tokio::time::sleep(tokio::time::Duration::from_secs(2)).await;
                                
                                match peach_service.get_payment_details(&peach_payment_id_clone).await {
                                    Ok(payment_details) => {
                                        match peach_service.extract_payment_method_detail(&payment_details, payment_clone.user_id) {
                                            Ok(payment_method_detail) => {
                                                let store_dto = StorePaymentMethodDto {
                                                    payment_id: payment_clone.id,
                                                    set_as_default: Some(true), // Auto-set as default for convenience
                                                };
                                                
                                                match db.store_payment_method(store_dto, payment_method_detail) {
                                                    Ok(stored_method) => {
                                                        println!("✅ Auto-stored payment method: {:?}", stored_method.payment_method);
                                                    },
                                                    Err(e) => {
                                                        println!("⚠️ Failed to auto-store payment method: {}", e);
                                                    }
                                                }
                                            },
                                            Err(e) => {
                                                println!("⚠️ Failed to extract payment method details: {}", e);
                                            }
                                        }
                                    },
                                    Err(e) => {
                                        println!("⚠️ Failed to get payment details for auto-store: {}", e);
                                    }
                                }
                            }
                        });
                    }
                }

                if let Some(ref sub_id) = payment.subscription_id {
                    let _ = db.activate_subscription(sub_id);
                }
            } else {
                println!("⚠️ No payment found for merchantTransactionId: {}", merchant_transaction_id);
            }
        },
        "100.396.104" => {
            println!("⚠️ Payment uncertain/cancelled by user");
            let _ = db.update_payment_status(&merchant_transaction_id, &PaymentStatus::Failed);
        },
        "000.200.100" => {
            println!("ℹ️ Checkout created - no action needed");
        },
        "000.200.000" => {
            println!("ℹ️ Payment pending - no action needed");
        },
        _ => {
            println!("⚠️ Unhandled result.code: {}", status_code);
        }
    }

    HttpResponse::Ok().body("Webhook received")
}

// GET /payment-methods/{user_id}
#[get("/payment-methods/{user_id}")]
pub async fn get_user_payment_methods(
    db: Data<DatabaseService>,
    path: Path<String>,
) -> Result<HttpResponse> {
    let user_id = match uuid::Uuid::parse_str(&path.into_inner()) {
        Ok(id) => id,
        Err(_) => return Ok(HttpResponse::BadRequest().json(ApiResponseError {
            message: "Invalid user ID format".to_string(),
            details: None,
        })),
    };

    let payment_methods = db.get_user_payment_methods(&user_id);

    Ok(HttpResponse::Ok().json(serde_json::json!({
        "payment_methods": payment_methods,
        "count": payment_methods.len()
    })))
}

// POST /payment-methods/store
#[post("/payment-methods/store")]
pub async fn store_payment_method(
    db: Data<DatabaseService>,
    peach_service: Data<PeachPaymentService>,
    payload: Json<StorePaymentMethodDto>,
) -> Result<HttpResponse> {
    // Get the payment record to verify it's completed
    let payment = match db.get_payment(&payload.payment_id) {
        Some(p) => p,
        None => return Ok(HttpResponse::NotFound().json(ApiResponseError {
            message: "Payment not found".to_string(),
            details: None,
        })),
    };

    if payment.status != PaymentStatus::Completed {
        return Ok(HttpResponse::BadRequest().json(ApiResponseError {
            message: "Payment is not completed".to_string(),
            details: None,
        }));
    }

    // Get payment details from Peach to extract payment method information
    let payment_details = match &payment.peach_payment_id {
        Some(peach_id) => {
            match peach_service.get_payment_details(peach_id).await {
                Ok(details) => details,
                Err(e) => return Ok(HttpResponse::InternalServerError().json(ApiResponseError {
                    message: "Failed to retrieve payment details from Peach".to_string(),
                    details: Some(e.to_string()),
                })),
            }
        },
        None => {
            // Fallback: check payment status to get details
            match &payment.checkout_id {
                Some(checkout_id) => {
                    match peach_service.check_payment_status(checkout_id).await {
                        Ok(details) => details,
                        Err(e) => return Ok(HttpResponse::InternalServerError().json(ApiResponseError {
                            message: "Failed to retrieve payment details".to_string(),
                            details: Some(e.to_string()),
                        })),
                    }
                },
                None => return Ok(HttpResponse::BadRequest().json(ApiResponseError {
                    message: "No payment details available to extract payment method".to_string(),
                    details: None,
                })),
            }
        }
    };

    // Extract payment method detail from Peach response
    let payment_method_detail = match peach_service.extract_payment_method_detail(&payment_details, payment.user_id) {
        Ok(detail) => detail,
        Err(e) => return Ok(HttpResponse::InternalServerError().json(ApiResponseError {
            message: "Failed to extract payment method details".to_string(),
            details: Some(e.to_string()),
        })),
    };

    // Store the payment method in database
    match db.store_payment_method(payload.into_inner(), payment_method_detail) {
        Ok(stored_method) => Ok(HttpResponse::Ok().json(serde_json::json!({
            "message": "Payment method stored successfully",
            "payment_method": stored_method
        }))),
        Err(e) => Ok(HttpResponse::InternalServerError().json(ApiResponseError {
            message: "Failed to store payment method".to_string(),
            details: Some(e),
        })),
    }
}

// POST /recurring
#[post("/recurring")]
pub async fn create_recurring_payment(
    db: Data<DatabaseService>,
    peach_service: Data<PeachPaymentService>,
    payload: Json<CreateRecurringPaymentDto>,
) -> Result<HttpResponse> {
    let subscription_id = &payload.subscription_id;
    let subscription = match db.get_subscription(subscription_id) {
        Some(sub) => sub,
        None => return Ok(HttpResponse::NotFound().json(ApiResponseError {
            message: "Subscription not found".to_string(),
            details: None,
        })),
    };

    // Get the payment method details
    let payment_methods = db.get_user_payment_methods(&payload.user_id);
    let payment_method = payment_methods
        .iter()
        .find(|pm| pm.id == payload.payment_method_detail_id)
        .ok_or("Payment method not found")?;

    if !payment_method.is_active {
        return Ok(HttpResponse::BadRequest().json(ApiResponseError {
            message: "Payment method is not active".to_string(),
            details: None,
        }));
    }

    // Create recurring payment record
    let payment_record = match db.create_recurring_payment(payload.into_inner()) {
        Ok(payment) => payment,
        Err(e) => return Ok(HttpResponse::InternalServerError().json(ApiResponseError {
            message: "Error creating recurring payment record".to_string(),
            details: Some(e.to_string()),
        })),
    };

    let user_id_str = payment_record.user_id.to_string();
    let subscription_id_str = subscription_id.to_string();

    // Process recurring payment using stored registration ID
    if let Some(registration_id) = &payment_method.peach_registration_id {
        match peach_service
            .process_recurring_payment(
                registration_id,
                payload.amount,
                &payment_record.merchant_transaction_id,
                &user_id_str,
                &subscription_id_str,
            )
            .await
        {
            Ok(peach_response) => {
                // Update payment status based on response
                let result_code = peach_response
                    .get("result")
                    .and_then(|r| r.get("code"))
                    .and_then(|c| c.as_str())
                    .unwrap_or("");

                let status = if result_code.starts_with("000.000") || result_code.starts_with("000.100") {
                    PaymentStatus::Completed
                } else {
                    PaymentStatus::Failed
                };

                let _ = db.update_payment_status(&payment_record.merchant_transaction_id, &status);

                if status == PaymentStatus::Completed {
                    // Extend subscription
                    let _ = db.activate_subscription(&subscription_id);
                }

                Ok(HttpResponse::Ok().json(serde_json::json!({
                    "message": "Recurring payment processed",
                    "payment_id": payment_record.id,
                    "merchant_transaction_id": payment_record.merchant_transaction_id,
                    "status": format!("{:?}", status),
                    "peach_response": peach_response
                })))
            }
            Err(e) => Ok(HttpResponse::InternalServerError().json(ApiResponseError {
                message: "Failed to process recurring payment".to_string(),
                details: Some(e.to_string()),
            })),
        }
    } else {
        Ok(HttpResponse::BadRequest().json(ApiResponseError {
            message: "Payment method does not support recurring payments".to_string(),
            details: Some("No registration ID available".to_string()),
        }))
    }
}

// DELETE /payment-methods/{user_id}/{payment_method_id}
#[delete("/payment-methods/{user_id}/{payment_method_id}")]
pub async fn deactivate_payment_method(
    db: Data<DatabaseService>,
    path: Path<(String, String)>,
) -> Result<HttpResponse> {
    let (user_id_str, payment_method_id_str) = path.into_inner();
    
    let user_id = match uuid::Uuid::parse_str(&user_id_str) {
        Ok(id) => id,
        Err(_) => return Ok(HttpResponse::BadRequest().json(ApiResponseError {
            message: "Invalid user ID format".to_string(),
            details: None,
        })),
    };

    let payment_method_id = match uuid::Uuid::parse_str(&payment_method_id_str) {
        Ok(id) => id,
        Err(_) => return Ok(HttpResponse::BadRequest().json(ApiResponseError {
            message: "Invalid payment method ID format".to_string(),
            details: None,
        })),
    };

    match db.deactivate_payment_method(&user_id, &payment_method_id) {
        Ok(()) => Ok(HttpResponse::Ok().json(serde_json::json!({
            "message": "Payment method deactivated successfully"
        }))),
        Err(e) => Ok(HttpResponse::NotFound().json(ApiResponseError {
            message: "Failed to deactivate payment method".to_string(),
            details: Some(e),
        })),
    }
}