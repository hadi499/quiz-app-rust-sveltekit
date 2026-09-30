use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

// ─── DB Row ───────────────────────────────────────────────────────────────────

#[derive(Debug, Serialize, sqlx::FromRow)]
pub struct QuizResult {
    pub id: i32,
    pub user_id: Option<i32>,
    pub quiz_id: i32,
    pub score: i32,
    pub correct_count: i32,
    pub total_questions: i32,
    pub completed_at: DateTime<Utc>,
}

#[derive(Debug, Serialize, sqlx::FromRow)]
pub struct QuizResultAnswer {
    pub id: i32,
    pub result_id: i32,
    pub question_id: i32,
    pub selected_option: Option<String>,
    pub is_correct: bool,
    pub time_left: i32,
}

// ─── Request payload ──────────────────────────────────────────────────────────

/// Payload jawaban per soal dikirim dari frontend
#[derive(Debug, Deserialize)]
pub struct AnswerPayload {
    pub question_id: i32,
    pub selected_option: Option<String>, // None = waktu habis
    pub is_correct: bool,
    pub time_left: i32,
}

/// Payload utama yang dikirim frontend setelah quiz selesai
#[derive(Debug, Deserialize)]
pub struct SubmitResultPayload {
    pub quiz_id: i32,
    pub score: i32,
    pub correct_count: i32,
    pub total_questions: i32,
    pub answers: Vec<AnswerPayload>,
}

// ─── Response ─────────────────────────────────────────────────────────────────

#[derive(Debug, Serialize)]
pub struct SubmitResultResponse {
    pub result: QuizResult,
    pub answers: Vec<QuizResultAnswer>,
}

/// Riwayat hasil quiz (untuk GET /api/results/me)
#[derive(Debug, Serialize, sqlx::FromRow)]
pub struct QuizResultWithTitle {
    pub id: i32,
    pub quiz_id: i32,
    pub quiz_title: String,
    pub score: i32,
    pub correct_count: i32,
    pub total_questions: i32,
    pub completed_at: DateTime<Utc>,
}

/// Riwayat hasil quiz untuk semua user (untuk GET /api/results/all)
#[derive(Debug, Serialize, sqlx::FromRow)]
pub struct QuizResultAdminView {
    pub id: i32,
    pub quiz_id: i32,
    pub quiz_title: String,
    pub user_id: i32,
    pub username: String,
    pub score: i32,
    pub correct_count: i32,
    pub total_questions: i32,
    pub completed_at: DateTime<Utc>,
}
