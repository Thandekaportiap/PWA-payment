mod models;
mod handlers;
mod services;

use actix_web::{web, App, HttpServer, middleware::Logger};
use actix_web::web::Data;
use std::env;
use dotenv::dotenv;

use services::{
    database::DatabaseService,
    peach::PeachPaymentService,
};


#[actix_web::main]
async fn main() -> std::io::Result<()> {
    // Load environment variables from .env file
    // This must be called early to ensure env vars are available
    dotenv().ok();
    
    // Initialize logger
    env_logger::init();

    // Initialize services
    let database_service = DatabaseService::new();
    
    // Get environment variables. Using .expect() to ensure they are set.
    // If these environment variables are not found, the application will panic with a clear error message.
    // This forces the use of the correct credentials from your .env file.
    let peach_base_url = env::var("PEACH_BASE_URL")
        .unwrap_or_else(|_| "https://testsecure.peachpayments.com".to_string()); // Default for convenience

    let peach_entity_id = env::var("PEACH_ENTITY_ID")
        .expect("PEACH_ENTITY_ID must be set in the environment or .env file.");
    
    let peach_access_token = env::var("PEACH_ACCESS_TOKEN")
        .expect("PEACH_ACCESS_TOKEN must be set in the environment or .env file.");
    
    let peach_secret_key = env::var("PEACH_SECRET_KEY")
        .expect("PEACH_SECRET_KEY must be set in the environment or .env file.");

    let peach_service = PeachPaymentService::new(
        peach_base_url,
        peach_entity_id,
        peach_access_token,
        peach_secret_key,
    );

    let port = env::var("PORT").unwrap_or_else(|_| "8080".to_string());
    let bind_address = format!("0.0.0.0:{}", port);

    println!("Starting server at http://{}", bind_address);
    println!("API Documentation:");
    println!("  POST /api/v1/users/register - Register a new user");
    println!("  GET  /api/v1/users/{{user_id}} - Get user details");
    println!("  POST /api/v1/subscriptions/create - Create subscription");
    println!("  GET  /api/v1/subscriptions/{{id}}/status - Get subscription status");
    println!("  POST /api/v1/subscriptions/voucher - Apply voucher code");
    println!("  POST /api/v1/payments/initiate - Initiate payment");
    println!("  GET  /api/v1/payments/status/{{id}} - Check payment status");

    HttpServer::new(move || {
        App::new()
            .app_data(Data::new(database_service.clone()))
            .app_data(Data::new(peach_service.clone()))
            .wrap(Logger::default())
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
                            .service(handlers::payment::handle_payment_callback_post)
                    )
                    .service(
                        web::scope("/subscriptions")
                            .service(handlers::subscription::create_subscription)
                            .service(handlers::subscription::get_subscription_status)
                            .service(handlers::subscription::process_voucher)
                            .service(handlers::subscription::activate_subscription)
                    )
            )
    })
    .bind(&bind_address)?
    .run()
    .await
}