use actix_web::{HttpResponse, Result, get, post};
use actix_web::web::{Data, Path, Json};
use serde::{Serialize, Deserialize};
use crate::services::database::DatabaseService;

#[derive(Serialize)]
pub struct NotificationResponse {
    pub id: String,
    pub user_id: String,
    pub subscription_id: String,
    pub message: String,
    pub acknowledged: bool,
    pub created_at: String,
}

#[get("/user/{user_id}")]
pub async fn get_notifications(
    db: Data<DatabaseService>,
    path: Path<String>,
) -> Result<HttpResponse> {
    let user_id = path.into_inner();
    
    match db.get_user_notifications(user_id).await {
        Ok(notifications) => {
            let response: Vec<NotificationResponse> = notifications
                .into_iter()
                .map(|n| NotificationResponse {
                    id: n.id,
                    user_id: n.user_id,
                    subscription_id: n.subscription_id,
                    message: n.message,
                    acknowledged: n.acknowledged,
                    created_at: n.created_at.to_rfc3339(),
                })
                .collect();
            
            Ok(HttpResponse::Ok().json(response))
        }
        Err(e) => {
            eprintln!("Error fetching notifications: {}", e);
            Ok(HttpResponse::InternalServerError().json(serde_json::json!({
                "error": "Failed to fetch notifications"
            })))
        }
    }
}

#[post("/{notification_id}/acknowledge")]
pub async fn mark_notification_read(
    db: Data<DatabaseService>,
    path: Path<String>,
) -> Result<HttpResponse> {
    let notification_id = path.into_inner();
    
    match db.acknowledge_notification(notification_id).await {
        Ok(_) => Ok(HttpResponse::Ok().json(serde_json::json!({
            "message": "Notification marked as read"
        }))),
        Err(e) => {
            eprintln!("Error acknowledging notification: {}", e);
            Ok(HttpResponse::InternalServerError().json(serde_json::json!({
                "error": "Failed to acknowledge notification"
            })))
        }
    }
}

#[derive(Deserialize)]
pub struct TestNotificationRequest {
    pub user_id: String,
    pub message: String,
}

#[post("/test")]
pub async fn create_test_notification(
    db: Data<DatabaseService>,
    payload: Json<TestNotificationRequest>,
) -> Result<HttpResponse> {
    match db.create_test_notification(payload.user_id.clone(), payload.message.clone()).await {
        Ok(_) => Ok(HttpResponse::Ok().json(serde_json::json!({
            "message": "Test notification created successfully"
        }))),
        Err(e) => {
            eprintln!("Error creating test notification: {}", e);
            Ok(HttpResponse::InternalServerError().json(serde_json::json!({
                "error": "Failed to create test notification"
            })))
        }
    }
}
