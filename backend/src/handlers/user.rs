use actix_web::{HttpResponse, Result, get, post, web};
use actix_web::web::{Data, Json, Path};
use serde::{Deserialize, Serialize};
use surrealdb::sql::Thing;
use crate::services::database::DatabaseService;
use crate::models::user::CreateUserDto;

#[derive(Deserialize, Debug)]
pub struct RegisterUserRequest {
    pub email: String,
    pub name: String,
}

#[derive(Serialize)]
pub struct UserResponse {
    pub id: String,
    pub email: String,
    pub name: String,
}

#[derive(Serialize)]
pub struct ErrorResponse {
    pub error: String,
}

#[post("/register")]
pub async fn register_user(
    db: Data<DatabaseService>,
    payload: Json<RegisterUserRequest>,
) -> Result<HttpResponse> {
    println!("📝 Register request received: {:?}", payload);

    if payload.email.is_empty() || payload.name.is_empty() {
        return Ok(HttpResponse::BadRequest().json(ErrorResponse {
            error: "Email and name are required".to_string(),
        }));
    }

    let dto = CreateUserDto {
        email: payload.email.clone(),
        name: payload.name.clone(),
    };

    match db.create_user(dto).await {
        Ok(user) => {
            println!("✅ User created successfully: {}", user.email);
            Ok(HttpResponse::Ok().json(UserResponse {
                id: user.id.id.to_string(), // Extract UUID from Thing
                email: user.email,
                name: user.name,
            }))
        }
        Err(e) => {
            println!("❌ Failed to create user: {}", e);
            Ok(HttpResponse::BadRequest().json(ErrorResponse {
                error: format!("Failed to create user: {}", e),
            }))
        }
    }
}

#[get("/email/{email}")]
pub async fn get_user_by_email(
    db: Data<DatabaseService>,
    path: Path<String>,
) -> Result<HttpResponse> {
    let email = path.into_inner();
    println!("🔍 Looking up user by email: {}", email);

    match db.get_user_by_email(&email).await {
        Some(user) => Ok(HttpResponse::Ok().json(UserResponse {
            id: user.id.id.to_string(),
            email: user.email,
            name: user.name,
        })),
        None => Ok(HttpResponse::NotFound().json(ErrorResponse {
            error: "User not found".to_string(),
        })),
    }
}

#[get("/{user_id}")]
pub async fn get_user(
    db: Data<DatabaseService>,
    path: Path<String>,
) -> Result<HttpResponse> {
    let user_id = path.into_inner();
    println!("🔍 Looking up user by ID: {}", user_id);

    match db.get_user(&user_id).await {
        Some(user) => Ok(HttpResponse::Ok().json(UserResponse {
            id: user.id.id.to_string(),
            email: user.email,
            name: user.name,
        })),
        None => Ok(HttpResponse::NotFound().json(ErrorResponse {
            error: "User not found".to_string(),
        })),
    }
}
