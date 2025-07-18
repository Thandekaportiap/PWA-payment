// main.rs
mod models;
mod handlers;
mod services;

use actix_web::{web, App, HttpServer, middleware::Logger};
use actix_web::web::Data;
use std::env;
use dotenv::dotenv;
use actix_cors::Cors;

use services::{
    database::DatabaseService,
    peach::PeachPaymentService,
};

#[actix_web::main]
async fn main() -> std::io::Result<()> {
    dotenv().ok();
    env_logger::init();

    let database_service = DatabaseService::new();

    // V1 related URLs and credentials (if still needed for other V1 routes)
    let v1_base_url = env::var("PEACH_V1_BASE_URL")
        .expect("PEACH_V1_BASE_URL must be set");
    let v1_entity_id = env::var("PEACH_ENTITY_ID")
        .expect("PEACH_ENTITY_ID must be set");
    let v1_access_token = env::var("PEACH_ACCESS_TOKEN")
        .expect("PEACH_ACCESS_TOKEN must be set");
    let v1_secret_key = env::var("PEACH_SECRET_KEY")
        .expect("PEACH_SECRET_KEY must be set"); // Used for webhook signature verification too

    // V2 Embedded Checkout related URLs and credentials
    let v2_auth_service_url = env::var("PEACH_AUTH_SERVICE_URL")
        .expect("PEACH_AUTH_SERVICE_URL must be set");
    let v2_checkout_endpoint = env::var("PEACH_CHECKOUT_V2_ENDPOINT")
        .expect("PEACH_CHECKOUT_V2_ENDPOINT must be set");
    let client_id = env::var("PEACH_CLIENT_ID")
        .expect("PEACH_CLIENT_ID must be set");
    let client_secret = env::var("PEACH_CLIENT_SECRET")
        .expect("PEACH_CLIENT_SECRET must be set");
    let merchant_id = env::var("PEACH_MERCHANT_ID")
        .expect("PEACH_MERCHANT_ID must be set");
    let v2_entity_id = env::var("PEACH_ENTITY_ID_V2") // Correctly load V2 specific entity ID
        .expect("PEACH_ENTITY_ID_V2 must be set");
    let notification_url = env::var("PEACH_NOTIFICATION_URL")
        .expect("PEACH_NOTIFICATION_URL must be set");
    let shopper_result_url = env::var("PEACH_SHOPPER_RESULT_URL") // New env var for shopperResultUrl
        .expect("PEACH_SHOPPER_RESULT_URL must be set");


    let peach_service = PeachPaymentService::new(
        v1_base_url,
        v1_entity_id,
        v1_access_token,
        v1_secret_key,
        v2_auth_service_url,
        v2_checkout_endpoint,
        v2_entity_id, // Pass V2 specific entity ID
        client_id,
        client_secret,
        merchant_id,
        notification_url,
        shopper_result_url,
    );

    let port = env::var("PORT").unwrap_or_else(|_| "8080".to_string());
    let bind_address = format!("0.0.0.0:{}", port);

    HttpServer::new(move || {
        App::new()
            .wrap(Logger::default())
            .wrap(
                Cors::default()
                    .allow_any_origin() // Consider restricting this in production
                    .allow_any_method()
                    .allow_any_header()
                    .supports_credentials()
            )
            .app_data(Data::new(database_service.clone()))
            .app_data(Data::new(peach_service.clone()))
            .service(
                web::scope("/api/v1")
                    .service(
                        web::scope("/users")
                            .service(handlers::user::register_user)
                            .service(handlers::user::get_user)
                    )
                    .service(
                        web::scope("/payments")
                            .service(handlers::payment::initiate_payment)
                            .service(handlers::payment::check_payment_status)
                            .service(handlers::payment::handle_payment_callback_get)
                            .service(handlers::payment::payment_callback)
                            .service(handlers::payment::get_user_payment_methods)
                            .service(handlers::payment::store_payment_method)
                            .service(handlers::payment::create_recurring_payment)
                            .service(handlers::payment::deactivate_payment_method)
                    )
                    .service(
                        web::scope("/subscriptions")
                            .service(handlers::subscription::create_subscription)
                            .service(handlers::subscription::get_subscription_status)
                            .service(handlers::subscription::activate_subscription)
                    )
            )
    })
    .bind(&bind_address)?
    .run()
    .await
}