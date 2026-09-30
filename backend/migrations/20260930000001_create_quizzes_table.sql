CREATE TABLE IF NOT EXISTS quizzes (
    id         SERIAL PRIMARY KEY,
    title      VARCHAR(255) NOT NULL,
    duration   INTEGER NOT NULL DEFAULT 15, -- waktu per soal dalam detik
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW()
);
