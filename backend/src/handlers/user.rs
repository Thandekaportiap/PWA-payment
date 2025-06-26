use actix_web::{HttpResponse, Result, get, post};
use actix_web::web::{Data, Json, Path};
use uuid::Uuid;
use crate::services::database::DatabaseService;
use crate::models::user::CreateUserDto;

#[post("/register")]
pub async fn register_user(
    db: Data<DatabaseService>,
    payload: Json<CreateUserDto>,
) -> Result<HttpResponse> {
    match db.create_user(payload.into_inner()) {
        Ok(user) => Ok(HttpResponse::Created().json(user)),
        Err(e) => Ok(HttpResponse::BadRequest().json(format!("Error creating user: {}", e))),
    }
}

#[get("/{user_id}")]
pub async fn get_user(
    db: Data<DatabaseService>,
    path: Path<Uuid>,
) -> Result<HttpResponse> {
    let user_id = path.into_inner();
    match db.get_user(&user_id) {
        Some(user) => Ok(HttpResponse::Ok().json(user)),
        None => Ok(HttpResponse::NotFound().json("User not found")),
    }
}
