use axum::{
    routing::{get, post},
    Router,
};

use crate::controllers::result::{get_result, my_results, submit_result, get_all_results, delete_all_results};
use crate::db::AppState;

pub fn result_routes() -> Router<AppState> {
    Router::new()
        .route("/results", post(submit_result))
        .route("/results/all", get(get_all_results).delete(delete_all_results))
        .route("/results/me", get(my_results))
        .route("/results/:id", get(get_result))
}
