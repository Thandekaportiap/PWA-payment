use actix_web::{HttpResponse, Result, get, post};
use actix_web::web::{Data, Json, Path};
use serde::{Deserialize, Serialize};
use crate::services::database::DatabaseService;
use crate::models::subscription::CreateSubscriptionDto;

#[derive(Deserialize)]
pub struct CreateSubscriptionRequest {
    pub user_id: String,
    pub plan_name: String,
    pub price: f64,
}

#[derive(Serialize)]
pub struct SubscriptionResponse {
    pub id: String,
    pub user_id: String,
    pub plan_name: String,
    pub price: f64,
    pub status: String,
}

#[post("/create")]
pub async fn create_subscription(
    db: Data<DatabaseService>,
    payload: Json<CreateSubscriptionRequest>,
) -> Result<HttpResponse> {
    let dto = CreateSubscriptionDto {
        user_id: payload.user_id.clone(),
        plan_name: payload.plan_name.clone(),
        price: payload.price,
        payment_method: None, // Will be set during payment
    };

    match db.create_subscription(dto).await {
        Ok(subscription) => Ok(HttpResponse::Ok().json(SubscriptionResponse {
            id: subscription.id,
            user_id: subscription.user_id,
            plan_name: subscription.plan_name,
            price: subscription.price,
            status: format!("{:?}", subscription.status),
        })),
        Err(e) => Ok(HttpResponse::BadRequest().json(serde_json::json!({
            "error": e
        }))),
    }
}

#[get("/{subscription_id}")]
pub async fn get_subscription(
    db: Data<DatabaseService>,
    path: Path<String>,
) -> Result<HttpResponse> {
    let subscription_id = path.into_inner();
    
    match db.get_subscription(&subscription_id).await {
        Some(subscription) => Ok(HttpResponse::Ok().json(SubscriptionResponse {
            id: subscription.id,
            user_id: subscription.user_id,
            plan_name: subscription.plan_name,
            price: subscription.price,
            status: format!("{:?}", subscription.status),
        })),
        None => Ok(HttpResponse::NotFound().json(serde_json::json!({
            "error": "Subscription not found"
        }))),
    }
}

#[post("/{subscription_id}/renew")]
pub async fn renew_subscription(
    db: Data<DatabaseService>,
    path: Path<String>,
) -> Result<HttpResponse> {
    let subscription_id = path.into_inner();
    
    match db.activate_subscription(&subscription_id).await {
        Ok(_) => Ok(HttpResponse::Ok().json(serde_json::json!({
            "message": "Subscription renewed successfully",
            "status": "Active"
        }))),
        Err(e) => Ok(HttpResponse::BadRequest().json(serde_json::json!({
            "error": e
        }))),
    }
}
