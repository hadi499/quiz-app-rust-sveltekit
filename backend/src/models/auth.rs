use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

// Model for incoming request data
#[derive(Deserialize)]
pub struct AuthPayload {
    pub username: String,
    pub password: String,
}

// Model for user representation in response
#[derive(Serialize)]
pub struct UserResponse {
    pub id: i32,
    pub username: String,
    pub role: String,
    pub avatar_url: Option<String>,
    pub created_at: DateTime<Utc>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct Claims {
    pub sub: String,
    pub user_id: i32,
    pub role: String,
    pub exp: usize,
}
