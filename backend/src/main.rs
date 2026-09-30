mod controllers;
mod db;
mod models;
mod routes;

use dotenvy::dotenv;
use std::env;
use tower_http::cors::CorsLayer;
use tower_http::services::ServeDir;
use axum::http::{Method, header::{AUTHORIZATION, ACCEPT, CONTENT_TYPE}};

use crate::db::{init_db, AppState};
use crate::routes::init_routes;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Load .env file
    dotenv().ok();

    // Initialize database
    let pool = init_db().await?;
    
    // Get JWT Secret
    let jwt_secret = env::var("JWT_SECRET").expect("JWT_SECRET must be set in .env");

    let app_state = AppState { db: pool, jwt_secret };

    let cors = CorsLayer::new()
        .allow_origin([
            "http://localhost:5173".parse().unwrap(),
            "http://localhost:5174".parse().unwrap(),
        ])
        .allow_methods([Method::GET, Method::POST, Method::DELETE, Method::OPTIONS])
        .allow_headers([AUTHORIZATION, ACCEPT, CONTENT_TYPE])
        .allow_credentials(true);

    // Set up routes with static file serving for uploaded avatars
    let app = init_routes()
        .with_state(app_state)
        .layer(cors)
        .nest_service("/uploads", ServeDir::new("./uploads"));

    // Ensure uploads directory exists on startup
    tokio::fs::create_dir_all("./uploads/avatars").await?;

    // Run the server
    let listener = tokio::net::TcpListener::bind("0.0.0.0:3000").await?;
    println!("Server running on http://0.0.0.0:3000");
    axum::serve(listener, app).await?;

    Ok(())
}
