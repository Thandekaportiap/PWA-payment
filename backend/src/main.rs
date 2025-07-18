mod models;
mod handlers;
mod services;
mod tasks;

use actix_web::{web, App, HttpServer, middleware::Logger, HttpResponse, Result};
use actix_web::web::Data;
use std::env;
use std::sync::Arc;
use dotenv::dotenv;
use actix_cors::Cors;
use services::{
    database::DatabaseService,
    peach::PeachPaymentService,
};

// Simple health check endpoint
async fn health_check() -> Result<HttpResponse> {
    Ok(HttpResponse::Ok().json(serde_json::json!({
        "status": "healthy",
        "service": "payment_system",
        "timestamp": chrono::Utc::now()
    })))
}

#[actix_web::main]
async fn main() -> std::io::Result<()> {
    // Load .env
    dotenv().ok();
    env_logger::init();

    // Initialize database service (now async)
    let database_service = DatabaseService::new().await
        .expect("Failed to initialize database service");

    // Load Peach Payments configuration from .env
    let webhook_secret_key = env::var("PEACH_SECRET_KEY")
        .expect("PEACH_SECRET_KEY must be set in .env");
    
    let peach_service = PeachPaymentService::new(
        env::var("PEACH_AUTH_SERVICE_URL").expect("PEACH_AUTH_SERVICE_URL must be set"),
        env::var("PEACH_CHECKOUT_V2_ENDPOINT").expect("PEACH_CHECKOUT_V2_ENDPOINT must be set"),
        env::var("PEACH_ENTITY_ID_V2").expect("PEACH_ENTITY_ID_V2 must be set"),
        env::var("PEACH_CLIENT_ID").expect("PEACH_CLIENT_ID must be set"),
        env::var("PEACH_CLIENT_SECRET").expect("PEACH_CLIENT_SECRET must be set"),
        env::var("PEACH_MERCHANT_ID").expect("PEACH_MERCHANT_ID must be set"),
        env::var("PEACH_NOTIFICATION_URL").expect("PEACH_NOTIFICATION_URL must be set"),
        env::var("PEACH_SHOPPER_RESULT_URL").expect("PEACH_SHOPPER_RESULT_URL must be set"),
        webhook_secret_key,
    );

    // ✅ Spawn the renewal task after both services are available
    let db = Arc::new(database_service.clone());
    let peach = Arc::new(peach_service.clone());
    actix_rt::spawn(tasks::renewal_task::start_renewal_task(db, peach));

    // Start web server
    let port = env::var("PORT").unwrap_or_else(|_| "8080".to_string());
    let bind_address = format!("0.0.0.0:{}", port);

        println!("🚀 Starting server on {}", bind_address);

    HttpServer::new(move || {
        App::new()
            .wrap(Logger::default())
            .wrap(
                Cors::default()
                    .allow_any_origin()
                    .allow_any_method()
                    .allow_any_header()
                    .supports_credentials()
            )
            .app_data(Data::new(database_service.clone()))
            .app_data(Data::new(peach_service.clone()))
            .route("/health", web::get().to(health_check))
            .service(
                web::scope("/api/v1")
                    .service(
                        web::scope("/users")
                              .service(handlers::user::register_user)
                                .service(handlers::user::get_user_by_email)
                            .service(handlers::user::get_user)
                    )
                    .service(
                        web::scope("/payments")
                            .service(handlers::payment::initiate_payment)
                            .service(handlers::payment::check_payment_status)
                            .service(handlers::payment::handle_payment_callback_get)
                            .service(handlers::payment::payment_callback)
                            .service(handlers::payment::charge_recurring_payment)
                    )
                    .service(
                        web::scope("/subscriptions")
                        .service(handlers::subscription::create_subscription)
                            .service(handlers::subscription::get_subscription)
                            .service(handlers::subscription::renew_subscription)
                    )
                       .service(
                        web::scope("/notifications")
                            .service(handlers::notification::get_notifications)
                            .service(handlers::notification::mark_notification_read)
                    )
            )
    })
    .bind(&bind_address)?
    .run()
    .await
}
