use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

// ─── Quiz ─────────────────────────────────────────────────────────────────────

/// Row dari tabel `quizzes`
#[derive(Debug, Serialize, sqlx::FromRow)]
pub struct Quiz {
    pub id: i32,
    pub title: String,
    pub duration: i32, // detik per soal
    pub created_at: DateTime<Utc>,
}

/// Payload untuk membuat quiz baru
#[derive(Debug, Deserialize)]
pub struct CreateQuizPayload {
    pub title: String,
    pub duration: Option<i32>, // default 15 jika tidak diisi
}

/// Payload untuk update quiz
#[derive(Debug, Deserialize)]
pub struct UpdateQuizPayload {
    pub title: Option<String>,
    pub duration: Option<i32>,
}

// ─── Question ─────────────────────────────────────────────────────────────────

/// Row dari tabel `questions`
#[derive(Debug, Serialize, sqlx::FromRow)]
pub struct Question {
    pub id: i32,
    pub quiz_id: i32,
    pub question_text: String,
    pub option_a: String,
    pub option_b: String,
    pub option_c: String,
    pub option_d: String,
    pub correct_option: String, // "A" | "B" | "C" | "D"
    pub order_index: i32,
}

/// Payload untuk membuat soal baru
#[derive(Debug, Deserialize)]
pub struct CreateQuestionPayload {
    pub question_text: String,
    pub option_a: String,
    pub option_b: String,
    pub option_c: String,
    pub option_d: String,
    pub correct_option: String, // "A" | "B" | "C" | "D"
    pub order_index: Option<i32>,
}

/// Payload untuk update soal
#[derive(Debug, Deserialize)]
pub struct UpdateQuestionPayload {
    pub question_text: Option<String>,
    pub option_a: Option<String>,
    pub option_b: Option<String>,
    pub option_c: Option<String>,
    pub option_d: Option<String>,
    pub correct_option: Option<String>,
    pub order_index: Option<i32>,
}

// ─── Response ─────────────────────────────────────────────────────────────────

/// Quiz beserta semua soalnya (dipakai untuk GET /api/quizzes/:id)
#[derive(Debug, Serialize)]
pub struct QuizWithQuestions {
    pub id: i32,
    pub title: String,
    pub duration: i32,
    pub created_at: DateTime<Utc>,
    pub questions: Vec<Question>,
}
