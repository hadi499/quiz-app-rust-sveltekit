use axum::{routing::{delete, get, post}, Router};

use crate::controllers::auth::{delete_avatar, login, logout, me, register, upload_avatar};
use crate::db::AppState;

pub fn auth_routes() -> Router<AppState> {
    Router::new()
        .route("/register", post(register))
        .route("/login", post(login))
        .route("/me", get(me))
        .route("/logout", post(logout))
        .route("/avatar", post(upload_avatar))
        .route("/avatar", delete(delete_avatar))
}
