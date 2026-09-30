CREATE TABLE IF NOT EXISTS quiz_results (
    id               SERIAL PRIMARY KEY,
    user_id          INTEGER REFERENCES users(id) ON DELETE SET NULL,
    quiz_id          INTEGER NOT NULL REFERENCES quizzes(id) ON DELETE CASCADE,
    score            INTEGER NOT NULL DEFAULT 0,
    correct_count    INTEGER NOT NULL DEFAULT 0,
    total_questions  INTEGER NOT NULL DEFAULT 0,
    completed_at     TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

CREATE TABLE IF NOT EXISTS quiz_result_answers (
    id               SERIAL PRIMARY KEY,
    result_id        INTEGER NOT NULL REFERENCES quiz_results(id) ON DELETE CASCADE,
    question_id      INTEGER NOT NULL REFERENCES questions(id) ON DELETE CASCADE,
    selected_option  CHAR(1),         -- NULL jika waktu habis
    is_correct       BOOLEAN NOT NULL DEFAULT FALSE,
    time_left        INTEGER NOT NULL DEFAULT 0
);
