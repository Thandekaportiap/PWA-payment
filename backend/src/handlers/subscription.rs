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
        Err(e) => Ok(HttpResponse::BadRequest().json(serde_json::json!({
            "error": format!("Error creating subscription: {}", e)
        }))),
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
