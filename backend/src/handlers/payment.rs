use actix_web::{HttpResponse, Result, post, get};
use actix_web::web::{Data, Json, Path, Query};
use bytes::Bytes;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use actix_web::HttpRequest;
use crate::services::peach::PeachPaymentService;
use actix_web::web; // Add this line
use crate::{
    models::{
        payment::{PaymentStatus, CreatePaymentDto},
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

    let user_id_str = payload.user_id.to_string();
    let subscription_id_str = payload.subscription_id.to_string();

    match peach_service
        .initiate_checkout_api_v2(&user_id_str, &subscription_id_str, payload.amount)
        .await
    {
        Ok(peach_response) => {
            if let Some(checkout_id) = peach_response.get("checkoutId").and_then(|v| v.as_str()) {
                let _ = db.update_payment_checkout_id(&payment_record.merchant_transaction_id, checkout_id);

                Ok(HttpResponse::Ok().json(serde_json::json!({
                    "checkoutId": checkout_id
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

// POST /callback (webhook)
#[post("/callback")]
pub async fn payment_callback(
    req: HttpRequest,
    body: web::Bytes,
    peach_service: web::Data<PeachPaymentService>,
    db: web::Data<DatabaseService>,
) -> HttpResponse {
     // Log raw body for debugging
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

    // 2. Parse form data to get the signature
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

    // 3. Debug output before validation
  
    println!("Provided signature: {}", provided_signature);

    // 4. Validate signature with EXACT body bytes
    if !peach_service.validate_webhook_signature(&body, provided_signature) {
        // Calculate what we expected for debugging
        let calculated = peach_service.calculate_signature(&body);
        eprintln!("❌ Signature validation failed");
        eprintln!("Calculated signature: {}", calculated);
        eprintln!("Provided signature:   {}", provided_signature);
        eprintln!("Body content: {}", body_str);
        return HttpResponse::Unauthorized().body("Invalid signature");
    }

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

    match status_code.as_str() {
        "000.000.000" | "000.100.110" => {
            println!("✅ Payment successful");

            // Fetch the payment record from DB
            if let Some(payment) = db.get_payment_by_merchant_id(&merchant_transaction_id) {
                let _ = db.update_payment_status(&merchant_transaction_id, &PaymentStatus::Completed);

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