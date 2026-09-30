use axum::{
    extract::{Path, State},
    http::StatusCode,
    response::IntoResponse,
    Json,
};
use axum_extra::extract::cookie::CookieJar;
use jsonwebtoken::{decode, DecodingKey, Validation};

use crate::db::AppState;
use crate::models::auth::Claims;
use crate::models::quiz::{
    CreateQuestionPayload, CreateQuizPayload, Question, Quiz, QuizWithQuestions,
    UpdateQuestionPayload, UpdateQuizPayload,
};

// ─── Auth helper ─────────────────────────────────────────────────────────────

/// Validasi JWT dari cookie, kembalikan Claims atau error response.
async fn authenticate(
    state: &AppState,
    jar: &CookieJar,
) -> Result<Claims, (StatusCode, String)> {
    let token = jar
        .get("token")
        .map(|c| c.value().to_string())
        .ok_or((StatusCode::UNAUTHORIZED, "Missing token".to_string()))?;

    let token_data = decode::<Claims>(
        &token,
        &DecodingKey::from_secret(state.jwt_secret.as_bytes()),
        &Validation::default(),
    )
    .map_err(|e| (StatusCode::UNAUTHORIZED, format!("Invalid token: {}", e)))?;

    let is_blacklisted = sqlx::query_scalar::<_, bool>(
        "SELECT EXISTS(SELECT 1 FROM blacklisted_tokens WHERE token = $1)",
    )
    .bind(&token)
    .fetch_one(&state.db)
    .await
    .unwrap_or(false);

    if is_blacklisted {
        return Err((StatusCode::UNAUTHORIZED, "Token has been invalidated".to_string()));
    }

    Ok(token_data.claims)
}

// ═════════════════════════════════════════════════════════════════════════════
// QUIZ HANDLERS
// ═════════════════════════════════════════════════════════════════════════════

// GET /api/quizzes — Daftar semua quiz
pub async fn list_quizzes(
    State(state): State<AppState>,
) -> Result<impl IntoResponse, (StatusCode, String)> {
    let quizzes = sqlx::query_as::<_, Quiz>(
        "SELECT id, title, duration, created_at FROM quizzes ORDER BY created_at DESC",
    )
    .fetch_all(&state.db)
    .await
    .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, format!("Database error: {}", e)))?;

    Ok((StatusCode::OK, Json(quizzes)))
}

// GET /api/quizzes/:id — Detail quiz beserta semua soalnya
pub async fn get_quiz(
    State(state): State<AppState>,
    Path(quiz_id): Path<i32>,
) -> Result<impl IntoResponse, (StatusCode, String)> {
    // Ambil quiz
    let quiz = sqlx::query_as::<_, Quiz>(
        "SELECT id, title, duration, created_at FROM quizzes WHERE id = $1",
    )
    .bind(quiz_id)
    .fetch_optional(&state.db)
    .await
    .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, format!("Database error: {}", e)))?
    .ok_or((StatusCode::NOT_FOUND, "Quiz not found".to_string()))?;

    // Ambil semua soal
    let questions = sqlx::query_as::<_, Question>(
        r#"
        SELECT id, quiz_id, question_text, option_a, option_b, option_c, option_d,
               correct_option, order_index
        FROM questions
        WHERE quiz_id = $1
        ORDER BY order_index ASC, id ASC
        "#,
    )
    .bind(quiz_id)
    .fetch_all(&state.db)
    .await
    .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, format!("Database error: {}", e)))?;

    let response = QuizWithQuestions {
        id: quiz.id,
        title: quiz.title,
        duration: quiz.duration,
        created_at: quiz.created_at,
        questions,
    };

    Ok((StatusCode::OK, Json(response)))
}

// POST /api/quizzes — Buat quiz baru (protected)
pub async fn create_quiz(
    State(state): State<AppState>,
    jar: CookieJar,
    Json(payload): Json<CreateQuizPayload>,
) -> Result<impl IntoResponse, (StatusCode, String)> {
    authenticate(&state, &jar).await?;

    if payload.title.trim().is_empty() {
        return Err((StatusCode::BAD_REQUEST, "Title is required".to_string()));
    }

    let duration = payload.duration.unwrap_or(15).max(5).min(300);

    let quiz = sqlx::query_as::<_, Quiz>(
        r#"
        INSERT INTO quizzes (title, duration)
        VALUES ($1, $2)
        RETURNING id, title, duration, created_at
        "#,
    )
    .bind(payload.title.trim())
    .bind(duration)
    .fetch_one(&state.db)
    .await
    .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, format!("Database error: {}", e)))?;

    Ok((StatusCode::CREATED, Json(quiz)))
}

// PATCH /api/quizzes/:id — Update quiz (protected)
pub async fn update_quiz(
    State(state): State<AppState>,
    jar: CookieJar,
    Path(quiz_id): Path<i32>,
    Json(payload): Json<UpdateQuizPayload>,
) -> Result<impl IntoResponse, (StatusCode, String)> {
    authenticate(&state, &jar).await?;

    // Pastikan quiz ada
    let exists = sqlx::query_scalar::<_, bool>("SELECT EXISTS(SELECT 1 FROM quizzes WHERE id = $1)")
        .bind(quiz_id)
        .fetch_one(&state.db)
        .await
        .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, format!("Database error: {}", e)))?;

    if !exists {
        return Err((StatusCode::NOT_FOUND, "Quiz not found".to_string()));
    }

    let quiz = sqlx::query_as::<_, Quiz>(
        r#"
        UPDATE quizzes
        SET
            title    = COALESCE($1, title),
            duration = COALESCE($2, duration)
        WHERE id = $3
        RETURNING id, title, duration, created_at
        "#,
    )
    .bind(payload.title.as_deref().map(str::trim))
    .bind(payload.duration)
    .bind(quiz_id)
    .fetch_one(&state.db)
    .await
    .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, format!("Database error: {}", e)))?;

    Ok((StatusCode::OK, Json(quiz)))
}

// DELETE /api/quizzes/:id — Hapus quiz (protected, cascade hapus questions)
pub async fn delete_quiz(
    State(state): State<AppState>,
    jar: CookieJar,
    Path(quiz_id): Path<i32>,
) -> Result<impl IntoResponse, (StatusCode, String)> {
    authenticate(&state, &jar).await?;

    let result = sqlx::query("DELETE FROM quizzes WHERE id = $1")
        .bind(quiz_id)
        .execute(&state.db)
        .await
        .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, format!("Database error: {}", e)))?;

    if result.rows_affected() == 0 {
        return Err((StatusCode::NOT_FOUND, "Quiz not found".to_string()));
    }

    Ok((StatusCode::OK, Json(serde_json::json!({ "message": "Quiz deleted" }))))
}

// ═════════════════════════════════════════════════════════════════════════════
// QUESTION HANDLERS
// ═════════════════════════════════════════════════════════════════════════════

// GET /api/quizzes/:id/questions — Semua soal dari quiz tertentu
pub async fn list_questions(
    State(state): State<AppState>,
    Path(quiz_id): Path<i32>,
) -> Result<impl IntoResponse, (StatusCode, String)> {
    // Pastikan quiz ada
    let exists = sqlx::query_scalar::<_, bool>("SELECT EXISTS(SELECT 1 FROM quizzes WHERE id = $1)")
        .bind(quiz_id)
        .fetch_one(&state.db)
        .await
        .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, format!("Database error: {}", e)))?;

    if !exists {
        return Err((StatusCode::NOT_FOUND, "Quiz not found".to_string()));
    }

    let questions = sqlx::query_as::<_, Question>(
        r#"
        SELECT id, quiz_id, question_text, option_a, option_b, option_c, option_d,
               correct_option, order_index
        FROM questions
        WHERE quiz_id = $1
        ORDER BY order_index ASC, id ASC
        "#,
    )
    .bind(quiz_id)
    .fetch_all(&state.db)
    .await
    .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, format!("Database error: {}", e)))?;

    Ok((StatusCode::OK, Json(questions)))
}

// POST /api/quizzes/:id/questions — Tambah soal ke quiz (protected)
pub async fn create_question(
    State(state): State<AppState>,
    jar: CookieJar,
    Path(quiz_id): Path<i32>,
    Json(payload): Json<CreateQuestionPayload>,
) -> Result<impl IntoResponse, (StatusCode, String)> {
    authenticate(&state, &jar).await?;

    // Validasi correct_option
    let correct = payload.correct_option.to_uppercase();
    if !["A", "B", "C", "D"].contains(&correct.as_str()) {
        return Err((StatusCode::BAD_REQUEST, "correct_option must be A, B, C, or D".to_string()));
    }

    // Pastikan quiz ada
    let exists = sqlx::query_scalar::<_, bool>("SELECT EXISTS(SELECT 1 FROM quizzes WHERE id = $1)")
        .bind(quiz_id)
        .fetch_one(&state.db)
        .await
        .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, format!("Database error: {}", e)))?;

    if !exists {
        return Err((StatusCode::NOT_FOUND, "Quiz not found".to_string()));
    }

    // Auto order_index jika tidak diisi
    let order_index = match payload.order_index {
        Some(o) => o,
        None => {
            sqlx::query_scalar::<_, i32>(
                "SELECT COALESCE(MAX(order_index), -1) + 1 FROM questions WHERE quiz_id = $1",
            )
            .bind(quiz_id)
            .fetch_one(&state.db)
            .await
            .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, format!("Database error: {}", e)))?
        }
    };

    let question = sqlx::query_as::<_, Question>(
        r#"
        INSERT INTO questions
            (quiz_id, question_text, option_a, option_b, option_c, option_d, correct_option, order_index)
        VALUES ($1, $2, $3, $4, $5, $6, $7, $8)
        RETURNING id, quiz_id, question_text, option_a, option_b, option_c, option_d,
                  correct_option, order_index
        "#,
    )
    .bind(quiz_id)
    .bind(payload.question_text.trim())
    .bind(payload.option_a.trim())
    .bind(payload.option_b.trim())
    .bind(payload.option_c.trim())
    .bind(payload.option_d.trim())
    .bind(&correct)
    .bind(order_index)
    .fetch_one(&state.db)
    .await
    .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, format!("Database error: {}", e)))?;

    Ok((StatusCode::CREATED, Json(question)))
}

// PATCH /api/questions/:id — Update soal (protected)
pub async fn update_question(
    State(state): State<AppState>,
    jar: CookieJar,
    Path(question_id): Path<i32>,
    Json(payload): Json<UpdateQuestionPayload>,
) -> Result<impl IntoResponse, (StatusCode, String)> {
    authenticate(&state, &jar).await?;

    // Validasi correct_option jika diisi
    if let Some(ref c) = payload.correct_option {
        if !["A", "B", "C", "D"].contains(&c.to_uppercase().as_str()) {
            return Err((StatusCode::BAD_REQUEST, "correct_option must be A, B, C, or D".to_string()));
        }
    }

    let question = sqlx::query_as::<_, Question>(
        r#"
        UPDATE questions
        SET
            question_text  = COALESCE($1, question_text),
            option_a       = COALESCE($2, option_a),
            option_b       = COALESCE($3, option_b),
            option_c       = COALESCE($4, option_c),
            option_d       = COALESCE($5, option_d),
            correct_option = COALESCE($6, correct_option),
            order_index    = COALESCE($7, order_index)
        WHERE id = $8
        RETURNING id, quiz_id, question_text, option_a, option_b, option_c, option_d,
                  correct_option, order_index
        "#,
    )
    .bind(payload.question_text.as_deref().map(str::trim))
    .bind(payload.option_a.as_deref().map(str::trim))
    .bind(payload.option_b.as_deref().map(str::trim))
    .bind(payload.option_c.as_deref().map(str::trim))
    .bind(payload.option_d.as_deref().map(str::trim))
    .bind(payload.correct_option.as_deref().map(|c| c.to_uppercase()))
    .bind(payload.order_index)
    .bind(question_id)
    .fetch_optional(&state.db)
    .await
    .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, format!("Database error: {}", e)))?
    .ok_or((StatusCode::NOT_FOUND, "Question not found".to_string()))?;

    Ok((StatusCode::OK, Json(question)))
}

// DELETE /api/questions/:id — Hapus soal (protected)
pub async fn delete_question(
    State(state): State<AppState>,
    jar: CookieJar,
    Path(question_id): Path<i32>,
) -> Result<impl IntoResponse, (StatusCode, String)> {
    authenticate(&state, &jar).await?;

    let result = sqlx::query("DELETE FROM questions WHERE id = $1")
        .bind(question_id)
        .execute(&state.db)
        .await
        .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, format!("Database error: {}", e)))?;

    if result.rows_affected() == 0 {
        return Err((StatusCode::NOT_FOUND, "Question not found".to_string()));
    }

    Ok((StatusCode::OK, Json(serde_json::json!({ "message": "Question deleted" }))))
}
