use actix_web::{HttpResponse, Result, post, get};
use actix_web::web::{Data, Json, Path};
use uuid::Uuid;
use serde::Deserialize;
use crate::{
    models::{
        subscription::{CreateSubscriptionDto, SubscriptionStatus},
        payment::{PaymentMethod, CreatePaymentDto},
    },
    services::database::DatabaseService,
};

#[derive(Deserialize)]
pub struct VoucherRequest {
    pub user_id: Uuid,
    pub subscription_id: Uuid,
    pub voucher_code: String,
}

#[derive(Deserialize)]
pub struct ActivateSubscriptionRequest {
    pub subscription_id: Uuid,
    pub payment_method: Option<PaymentMethod>,
}

#[post("/create")]
pub async fn create_subscription(
    db: Data<DatabaseService>,
    payload: Json<CreateSubscriptionDto>,
) -> Result<HttpResponse> {
    match db.create_subscription(payload.into_inner()) {
        Ok(subscription) => Ok(HttpResponse::Created().json(subscription)),
        Err(e) => Ok(HttpResponse::BadRequest().json(format!("Error creating subscription: {}", e))),
    }
}

#[get("/{subscription_id}/status")]
pub async fn get_subscription_status(
    db: Data<DatabaseService>,
    path: Path<Uuid>,
) -> Result<HttpResponse> {
    let subscription_id = path.into_inner();
    match db.get_subscription(&subscription_id) {
        Some(subscription) => Ok(HttpResponse::Ok().json(subscription)),
        None => Ok(HttpResponse::NotFound().json("Subscription not found")),
    }
}

#[post("/voucher")]
pub async fn process_voucher(
    db: Data<DatabaseService>,
    payload: Json<VoucherRequest>,
) -> Result<HttpResponse> {
    // Validate voucher code (implement your voucher validation logic)
    if payload.voucher_code.is_empty() {
        return Ok(HttpResponse::BadRequest().json("Invalid voucher code"));
    }
    
    // Check if subscription exists
    let subscription = match db.get_subscription(&payload.subscription_id) {
        Some(sub) => sub,
        None => return Ok(HttpResponse::NotFound().json("Subscription not found")),
    };
    
    if subscription.status != SubscriptionStatus::Pending {
        return Ok(HttpResponse::BadRequest().json("Subscription is not pending"));
    }
    
    // Create a payment record for the voucher
    let payment_dto = CreatePaymentDto {
        user_id: payload.user_id,
        subscription_id: payload.subscription_id,
        amount: 0.0, // Voucher amount is 0
        payment_method: Some(PaymentMethod::Voucher),
    };
    
    match db.create_payment(payment_dto) {
        Ok(_payment) => {
            // Activate the subscription immediately for voucher payments
            match db.activate_subscription(&payload.subscription_id) {
                Ok(_) => Ok(HttpResponse::Ok().json(serde_json::json!({
                    "result": {
                        "code": "000.000.000",
                        "description": "Voucher applied successfully"
                    },
                    "subscription_id": payload.subscription_id,
                    "status": "active"
                }))),
                Err(e) => Ok(HttpResponse::InternalServerError().json(format!("Error activating subscription: {}", e))),
            }
        },
        Err(e) => Ok(HttpResponse::InternalServerError().json(format!("Error processing voucher: {}", e))),
    }
}

#[post("/activate")]
pub async fn activate_subscription(
    db: Data<DatabaseService>,
    payload: Json<ActivateSubscriptionRequest>,
) -> Result<HttpResponse> {
    match db.activate_subscription(&payload.subscription_id) {
        Ok(_) => Ok(HttpResponse::Ok().json(serde_json::json!({
            "subscription_id": payload.subscription_id,
            "status": "activated",
            "message": "Subscription activated successfully"
        }))),
        Err(e) => Ok(HttpResponse::BadRequest().json(format!("Error activating subscription: {}", e))),
    }
}
