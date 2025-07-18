mod config;
mod models;
mod handlers;
mod services;
mod utils;

use actix_web::{web, App, HttpServer, middleware::Logger};
use actix_cors::Cors;
use dotenv::dotenv;
use std::env;

use services::{
    database::DatabaseService,
    peach::PeachPaymentService,
};

#[actix_web::main]
async fn main() -> std::io::Result<()> {
    dotenv().ok();
    env_logger::init();

    let config = config::Config::from_env().expect("Failed to load configuration");
    
    // Initialize database service
    let database_service = DatabaseService::new(&config.database_url).await
        .expect("Failed to initialize database");

    // Initialize Peach payment service
    let peach_service = PeachPaymentService::new(config.peach.clone());

    let port = env::var("PORT").unwrap_or_else(|_| "8080".to_string());
    let bind_address = format!("0.0.0.0:{}", port);

    println!("🚀 Starting Subscription Management Server on {}", bind_address);

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
            .app_data(web::Data::new(database_service.clone()))
            .app_data(web::Data::new(peach_service.clone()))
            .service(
                web::scope("/api/v1")
                    // User Management
                    .service(
                        web::scope("/users")
                            .service(handlers::users::register_user)
                            .service(handlers::users::get_user)
                    )
                    // Payment Handling
                    .service(
                        web::scope("/payments")
                            .service(handlers::payments::initiate_payment)
                            .service(handlers::payments::check_payment_status)
                            .service(handlers::payments::payment_callback_post)
                            .service(handlers::payments::payment_callback_get)
                    )
                    // Subscription Management
                    .service(
                        web::scope("/subscriptions")
                            .service(handlers::subscriptions::create_subscription)
                            .service(handlers::subscriptions::get_subscription_status)
                            .service(handlers::subscriptions::cancel_subscription)
                            .service(handlers::subscriptions::manual_renewal)
                            .service(handlers::subscriptions::change_plan)
                            .service(handlers::subscriptions::pause_subscription)
                            .service(handlers::subscriptions::resume_subscription)
                            .service(handlers::subscriptions::get_renewal_info)
                            .service(handlers::subscriptions::change_billing_date)
                    )
                    // Health check
                    .route("/health", web::get().to(handlers::health::health_check))
            )
    })
    .bind(&bind_address)?
    .run()
    .await
}