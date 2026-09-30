use axum::{
    routing::{delete, get, patch, post},
    Router,
};

use crate::controllers::quiz::{
    create_question, create_quiz, delete_question, delete_quiz, get_quiz, list_questions,
    list_quizzes, update_question, update_quiz,
};
use crate::db::AppState;

pub fn quiz_routes() -> Router<AppState> {
    Router::new()
        // Quiz CRUD
        .route("/quizzes", get(list_quizzes))
        .route("/quizzes", post(create_quiz))
        .route("/quizzes/:id", get(get_quiz))
        .route("/quizzes/:id", patch(update_quiz))
        .route("/quizzes/:id", delete(delete_quiz))
        // Question CRUD (nested di bawah quiz untuk create & list)
        .route("/quizzes/:id/questions", get(list_questions))
        .route("/quizzes/:id/questions", post(create_question))
        // Question update & delete langsung via question id
        .route("/questions/:id", patch(update_question))
        .route("/questions/:id", delete(delete_question))
}
