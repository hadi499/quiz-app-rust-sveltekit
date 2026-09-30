pub mod auth;
pub mod quiz;
pub mod result;

use axum::Router;
use crate::db::AppState;
use auth::auth_routes;
use quiz::quiz_routes;
use result::result_routes;

pub fn init_routes() -> Router<AppState> {
    Router::new()
        .merge(auth_routes())
        .merge(quiz_routes())
        .merge(result_routes())
}
