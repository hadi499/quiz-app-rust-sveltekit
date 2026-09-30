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
use crate::models::result::{
    QuizResult, QuizResultAnswer, QuizResultWithTitle, SubmitResultPayload, SubmitResultResponse,
};

// ─── Auth helper (optional) ────────────────────────────────────────────────────

/// Coba decode token — kembalikan user_id jika valid, None jika tidak ada/invalid.
/// Tidak return error agar endpoint tetap bisa dipakai tanpa login.
async fn try_authenticate(state: &AppState, jar: &CookieJar) -> Option<i32> {
    let token = jar.get("token")?.value().to_string();

    let token_data = decode::<Claims>(
        &token,
        &DecodingKey::from_secret(state.jwt_secret.as_bytes()),
        &Validation::default(),
    )
    .ok()?;

    let is_blacklisted = sqlx::query_scalar::<_, bool>(
        "SELECT EXISTS(SELECT 1 FROM blacklisted_tokens WHERE token = $1)",
    )
    .bind(&token)
    .fetch_one(&state.db)
    .await
    .unwrap_or(false);

    if is_blacklisted { return None; }

    Some(token_data.claims.user_id)
}

/// Wajib login — kembalikan user_id atau error.
async fn authenticate(state: &AppState, jar: &CookieJar) -> Result<i32, (StatusCode, String)> {
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

    Ok(token_data.claims.user_id)
}

// ═════════════════════════════════════════════════════════════════════════════
// POST /api/results — Simpan hasil quiz
// ═════════════════════════════════════════════════════════════════════════════
pub async fn submit_result(
    State(state): State<AppState>,
    jar: CookieJar,
    Json(payload): Json<SubmitResultPayload>,
) -> Result<impl IntoResponse, (StatusCode, String)> {
    // User login opsional — jika ada token valid, simpan user_id
    let user_id = try_authenticate(&state, &jar).await;

    // Validasi: quiz harus ada
    let quiz_exists = sqlx::query_scalar::<_, bool>(
        "SELECT EXISTS(SELECT 1 FROM quizzes WHERE id = $1)",
    )
    .bind(payload.quiz_id)
    .fetch_one(&state.db)
    .await
    .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, format!("Database error: {}", e)))?;

    if !quiz_exists {
        return Err((StatusCode::NOT_FOUND, "Quiz not found".to_string()));
    }

    // Insert ke quiz_results
    let result = sqlx::query_as::<_, QuizResult>(
        r#"
        INSERT INTO quiz_results (user_id, quiz_id, score, correct_count, total_questions)
        VALUES ($1, $2, $3, $4, $5)
        RETURNING id, user_id, quiz_id, score, correct_count, total_questions, completed_at
        "#,
    )
    .bind(user_id)
    .bind(payload.quiz_id)
    .bind(payload.score)
    .bind(payload.correct_count)
    .bind(payload.total_questions)
    .fetch_one(&state.db)
    .await
    .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, format!("Database error: {}", e)))?;

    // Insert semua jawaban per soal
    let mut saved_answers: Vec<QuizResultAnswer> = Vec::new();

    for ans in &payload.answers {
        let saved = sqlx::query_as::<_, QuizResultAnswer>(
            r#"
            INSERT INTO quiz_result_answers
                (result_id, question_id, selected_option, is_correct, time_left)
            VALUES ($1, $2, $3, $4, $5)
            RETURNING id, result_id, question_id, selected_option, is_correct, time_left
            "#,
        )
        .bind(result.id)
        .bind(ans.question_id)
        .bind(&ans.selected_option)
        .bind(ans.is_correct)
        .bind(ans.time_left)
        .fetch_one(&state.db)
        .await
        .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, format!("Database error: {}", e)))?;

        saved_answers.push(saved);
    }

    let response = SubmitResultResponse {
        result,
        answers: saved_answers,
    };

    Ok((StatusCode::CREATED, Json(response)))
}

// ═════════════════════════════════════════════════════════════════════════════
// GET /api/results/me — Riwayat hasil quiz milik user yang sedang login
// ═════════════════════════════════════════════════════════════════════════════
pub async fn my_results(
    State(state): State<AppState>,
    jar: CookieJar,
) -> Result<impl IntoResponse, (StatusCode, String)> {
    let user_id = authenticate(&state, &jar).await?;

    let results = sqlx::query_as::<_, QuizResultWithTitle>(
        r#"
        SELECT
            qr.id,
            qr.quiz_id,
            q.title AS quiz_title,
            qr.score,
            qr.correct_count,
            qr.total_questions,
            qr.completed_at
        FROM quiz_results qr
        JOIN quizzes q ON q.id = qr.quiz_id
        WHERE qr.user_id = $1
        ORDER BY qr.completed_at DESC
        "#,
    )
    .bind(user_id)
    .fetch_all(&state.db)
    .await
    .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, format!("Database error: {}", e)))?;

    Ok((StatusCode::OK, Json(results)))
}

// ═════════════════════════════════════════════════════════════════════════════
// GET /api/results/:id — Detail satu hasil quiz (beserta jawaban)
// ═════════════════════════════════════════════════════════════════════════════
pub async fn get_result(
    State(state): State<AppState>,
    jar: CookieJar,
    Path(result_id): Path<i32>,
) -> Result<impl IntoResponse, (StatusCode, String)> {
    let user_id = authenticate(&state, &jar).await?;

    // Ambil result, pastikan milik user ini
    let result = sqlx::query_as::<_, QuizResult>(
        "SELECT id, user_id, quiz_id, score, correct_count, total_questions, completed_at
         FROM quiz_results WHERE id = $1 AND user_id = $2",
    )
    .bind(result_id)
    .bind(user_id)
    .fetch_optional(&state.db)
    .await
    .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, format!("Database error: {}", e)))?
    .ok_or((StatusCode::NOT_FOUND, "Result not found".to_string()))?;

    let answers = sqlx::query_as::<_, QuizResultAnswer>(
        "SELECT id, result_id, question_id, selected_option, is_correct, time_left
         FROM quiz_result_answers WHERE result_id = $1 ORDER BY id",
    )
    .bind(result_id)
    .fetch_all(&state.db)
    .await
    .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, format!("Database error: {}", e)))?;

    Ok((
        StatusCode::OK,
        Json(SubmitResultResponse { result, answers }),
    ))
}

// ═════════════════════════════════════════════════════════════════════════════
// GET /api/results/all — Semua hasil quiz dari seluruh user (Admin Only)
// ═════════════════════════════════════════════════════════════════════════════
pub async fn get_all_results(
    State(state): State<AppState>,
    jar: CookieJar,
) -> Result<impl IntoResponse, (StatusCode, String)> {
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

    if token_data.claims.role != "admin" {
        return Err((StatusCode::FORBIDDEN, "Admin only".to_string()));
    }

    let results = sqlx::query_as::<_, crate::models::result::QuizResultAdminView>(
        r#"
        SELECT
            qr.id,
            qr.quiz_id,
            q.title AS quiz_title,
            qr.user_id,
            u.username,
            qr.score,
            qr.correct_count,
            qr.total_questions,
            qr.completed_at
        FROM quiz_results qr
        JOIN quizzes q ON q.id = qr.quiz_id
        JOIN users u ON u.id = qr.user_id
        ORDER BY qr.completed_at DESC
        "#,
    )
    .fetch_all(&state.db)
    .await
    .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, format!("Database error: {}", e)))?;

    Ok((StatusCode::OK, Json(results)))
}

// ═════════════════════════════════════════════════════════════════════════════
// DELETE /api/results/all — Hapus semua hasil quiz (Admin Only)
// ═════════════════════════════════════════════════════════════════════════════
pub async fn delete_all_results(
    State(state): State<AppState>,
    jar: CookieJar,
) -> Result<impl IntoResponse, (StatusCode, String)> {
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

    if token_data.claims.role != "admin" {
        return Err((StatusCode::FORBIDDEN, "Admin only".to_string()));
    }
    
    // First delete from quiz_result_answers, then quiz_results
    let _ = sqlx::query("DELETE FROM quiz_result_answers").execute(&state.db).await
        .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, format!("Failed to delete answers: {}", e)))?;
        
    let _ = sqlx::query("DELETE FROM quiz_results").execute(&state.db).await
        .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, format!("Failed to delete results: {}", e)))?;

    Ok((StatusCode::OK, "All scores deleted successfully".to_string()))
}
