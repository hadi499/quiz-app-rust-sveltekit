<script lang="ts">
  import { onMount } from 'svelte';
  import { goto } from '$app/navigation';

  // ─── Types ───────────────────────────────────────────────────────────────────
  interface Quiz {
    id: number;
    title: string;
    duration: number;
    created_at: string;
  }

  interface Question {
    id: number;
    quiz_id: number;
    question_text: string;
    option_a: string;
    option_b: string;
    option_c: string;
    option_d: string;
    correct_option: string;
    order_index: number;
  }

  interface QuizWithQuestions extends Quiz {
    questions: Question[];
  }

  interface QuizResultAdminView {
    id: number;
    quiz_id: number;
    quiz_title: string;
    user_id: number;
    username: string;
    score: number;
    correct_count: number;
    total_questions: number;
    completed_at: string;
  }

  // ─── Auth & Tabs state ────────────────────────────────────────────────────────
  let isLoading = $state(true);
  let activeTab = $state<'quizzes' | 'scores'>('quizzes');

  // ─── Quiz list state ──────────────────────────────────────────────────────────
  let quizzes = $state<Quiz[]>([]);
  let expandedQuizId = $state<number | null>(null);
  let questionsMap = $state<Record<number, Question[]>>({});
  let loadingQuestions = $state<Record<number, boolean>>({});

  // ─── Scores state ─────────────────────────────────────────────────────────────
  let allScores = $state<QuizResultAdminView[]>([]);
  let loadingScores = $state(false);
  let isDeletingAllScores = $state(false);
  let showDeleteAllScoresConfirm = $state(false);

  // ─── Quiz modal state ─────────────────────────────────────────────────────────
  let showQuizModal = $state(false);
  let quizModalMode = $state<'create' | 'edit'>('create');
  let editingQuiz = $state<Quiz | null>(null);
  let quizForm = $state({ title: '', duration: 15 });
  let quizSubmitting = $state(false);
  let quizError = $state('');

  // ─── Confirm delete state ─────────────────────────────────────────────────────
  let showDeleteQuizConfirm = $state<number | null>(null);
  let deletingQuizId = $state<number | null>(null);

  // ─── Question modal state ─────────────────────────────────────────────────────
  let showQuestionModal = $state(false);
  let questionModalMode = $state<'create' | 'edit'>('create');
  let questionModalQuizId = $state<number | null>(null);
  let editingQuestion = $state<Question | null>(null);
  let questionForm = $state({
    question_text: '',
    option_a: '',
    option_b: '',
    option_c: '',
    option_d: '',
    correct_option: 'A',
  });
  let questionSubmitting = $state(false);
  let questionError = $state('');
  let showDeleteQuestionConfirm = $state<number | null>(null);
  let deletingQuestionId = $state<number | null>(null);

  // ─── Toast ────────────────────────────────────────────────────────────────────
  let toast = $state<{ msg: string; type: 'success' | 'error' } | null>(null);
  let toastTimeout: ReturnType<typeof setTimeout> | null = null;

  function showToast(msg: string, type: 'success' | 'error' = 'success') {
    if (toastTimeout) clearTimeout(toastTimeout);
    toast = { msg, type };
    toastTimeout = setTimeout(() => { toast = null; }, 3000);
  }

  // ─── Init ─────────────────────────────────────────────────────────────────────
  onMount(async () => {
    try {
      const res = await fetch('/api/me', { credentials: 'include' });
      if (!res.ok) { goto('/login'); return; }
      const user = await res.json();
      if (user.role !== 'admin') {
        showToast('Akses ditolak: Hanya admin yang dapat mengelola quiz.', 'error');
        setTimeout(() => goto('/'), 1500);
        return;
      }
    } catch {
      goto('/login'); return;
    }

    const savedTab = sessionStorage.getItem('adminActiveTab');
    if (savedTab === 'scores' || savedTab === 'quizzes') {
      activeTab = savedTab;
    }

    await loadQuizzes();
    await loadAllScores();
    isLoading = false;
  });

  // ─── API helpers ──────────────────────────────────────────────────────────────
  async function loadQuizzes() {
    const res = await fetch('/api/quizzes', { credentials: 'include' });
    if (res.ok) quizzes = await res.json();
  }

  async function loadAllScores() {
    loadingScores = true;
    try {
      const res = await fetch('/api/results/all', { credentials: 'include' });
      if (res.ok) allScores = await res.json();
    } finally {
      loadingScores = false;
    }
  }

  async function deleteAllScores() {
    isDeletingAllScores = true;
    try {
      const res = await fetch('/api/results/all', {
        method: 'DELETE',
        credentials: 'include',
      });
      if (res.ok) {
        showToast('Semua skor berhasil dihapus!');
        await loadAllScores();
      } else {
        showToast('Gagal menghapus skor.', 'error');
      }
    } catch {
      showToast('Tidak dapat terhubung ke server.', 'error');
    } finally {
      isDeletingAllScores = false;
      showDeleteAllScoresConfirm = false;
    }
  }

  async function loadQuestions(quizId: number) {
    loadingQuestions = { ...loadingQuestions, [quizId]: true };
    try {
      const res = await fetch(`/api/quizzes/${quizId}/questions`, { credentials: 'include' });
      if (res.ok) {
        const data: Question[] = await res.json();
        questionsMap = { ...questionsMap, [quizId]: data };
      }
    } finally {
      loadingQuestions = { ...loadingQuestions, [quizId]: false };
    }
  }

  async function toggleExpand(quizId: number) {
    if (expandedQuizId === quizId) {
      expandedQuizId = null;
    } else {
      expandedQuizId = quizId;
      if (!questionsMap[quizId]) await loadQuestions(quizId);
    }
  }

  // ─── Quiz CRUD ────────────────────────────────────────────────────────────────
  function openCreateQuiz() {
    quizModalMode = 'create';
    editingQuiz = null;
    quizForm = { title: '', duration: 15 };
    quizError = '';
    showQuizModal = true;
  }

  function openEditQuiz(quiz: Quiz) {
    quizModalMode = 'edit';
    editingQuiz = quiz;
    quizForm = { title: quiz.title, duration: quiz.duration };
    quizError = '';
    showQuizModal = true;
  }

  function closeQuizModal() {
    showQuizModal = false;
    quizError = '';
  }

  async function submitQuizForm() {
    if (!quizForm.title.trim()) { quizError = 'Title wajib diisi.'; return; }
    quizSubmitting = true;
    quizError = '';
    try {
      let res: Response;
      if (quizModalMode === 'create') {
        res = await fetch('/api/quizzes', {
          method: 'POST',
          credentials: 'include',
          headers: { 'Content-Type': 'application/json' },
          body: JSON.stringify({ title: quizForm.title.trim(), duration: quizForm.duration }),
        });
      } else {
        res = await fetch(`/api/quizzes/${editingQuiz!.id}`, {
          method: 'PATCH',
          credentials: 'include',
          headers: { 'Content-Type': 'application/json' },
          body: JSON.stringify({ title: quizForm.title.trim(), duration: quizForm.duration }),
        });
      }
      if (res.ok) {
        await loadQuizzes();
        closeQuizModal();
        showToast(quizModalMode === 'create' ? 'Quiz berhasil dibuat!' : 'Quiz berhasil diperbarui!');
      } else {
        quizError = await res.text() || 'Terjadi kesalahan.';
      }
    } catch {
      quizError = 'Tidak dapat terhubung ke server.';
    } finally {
      quizSubmitting = false;
    }
  }

  async function deleteQuiz(quizId: number) {
    deletingQuizId = quizId;
    try {
      const res = await fetch(`/api/quizzes/${quizId}`, {
        method: 'DELETE',
        credentials: 'include',
      });
      if (res.ok) {
        await loadQuizzes();
        if (expandedQuizId === quizId) expandedQuizId = null;
        const newMap = { ...questionsMap };
        delete newMap[quizId];
        questionsMap = newMap;
        showToast('Quiz berhasil dihapus!');
      } else {
        showToast('Gagal menghapus quiz.', 'error');
      }
    } catch {
      showToast('Tidak dapat terhubung ke server.', 'error');
    } finally {
      deletingQuizId = null;
      showDeleteQuizConfirm = null;
    }
  }

  // ─── Question CRUD ────────────────────────────────────────────────────────────
  function openCreateQuestion(quizId: number) {
    questionModalMode = 'create';
    questionModalQuizId = quizId;
    editingQuestion = null;
    questionForm = { question_text: '', option_a: '', option_b: '', option_c: '', option_d: '', correct_option: 'A' };
    questionError = '';
    showQuestionModal = true;
  }

  function openEditQuestion(q: Question) {
    questionModalMode = 'edit';
    questionModalQuizId = q.quiz_id;
    editingQuestion = q;
    questionForm = {
      question_text: q.question_text,
      option_a: q.option_a,
      option_b: q.option_b,
      option_c: q.option_c,
      option_d: q.option_d,
      correct_option: q.correct_option,
    };
    questionError = '';
    showQuestionModal = true;
  }

  function closeQuestionModal() {
    showQuestionModal = false;
    questionError = '';
  }

  async function submitQuestionForm() {
    if (!questionForm.question_text.trim()) { questionError = 'Pertanyaan wajib diisi.'; return; }
    if (!questionForm.option_a.trim() || !questionForm.option_b.trim() ||
        !questionForm.option_c.trim() || !questionForm.option_d.trim()) {
      questionError = 'Semua pilihan jawaban wajib diisi.'; return;
    }
    questionSubmitting = true;
    questionError = '';
    try {
      let res: Response;
      if (questionModalMode === 'create') {
        res = await fetch(`/api/quizzes/${questionModalQuizId}/questions`, {
          method: 'POST',
          credentials: 'include',
          headers: { 'Content-Type': 'application/json' },
          body: JSON.stringify(questionForm),
        });
      } else {
        res = await fetch(`/api/questions/${editingQuestion!.id}`, {
          method: 'PATCH',
          credentials: 'include',
          headers: { 'Content-Type': 'application/json' },
          body: JSON.stringify(questionForm),
        });
      }
      if (res.ok) {
        await loadQuestions(questionModalQuizId!);
        closeQuestionModal();
        showToast(questionModalMode === 'create' ? 'Soal berhasil ditambahkan!' : 'Soal berhasil diperbarui!');
      } else {
        questionError = await res.text() || 'Terjadi kesalahan.';
      }
    } catch {
      questionError = 'Tidak dapat terhubung ke server.';
    } finally {
      questionSubmitting = false;
    }
  }

  async function deleteQuestion(questionId: number, quizId: number) {
    deletingQuestionId = questionId;
    try {
      const res = await fetch(`/api/questions/${questionId}`, {
        method: 'DELETE',
        credentials: 'include',
      });
      if (res.ok) {
        await loadQuestions(quizId);
        showToast('Soal berhasil dihapus!');
      } else {
        showToast('Gagal menghapus soal.', 'error');
      }
    } catch {
      showToast('Tidak dapat terhubung ke server.', 'error');
    } finally {
      deletingQuestionId = null;
      showDeleteQuestionConfirm = null;
    }
  }

  const optionLetters = ['A', 'B', 'C', 'D'];
  const optionKeys = ['option_a', 'option_b', 'option_c', 'option_d'] as const;
</script>

<svelte:head>
  <title>Kelola Quiz — QuizApp</title>
</svelte:head>

<!-- ── TOAST ─────────────────────────────────────────────────────────────────── -->
{#if toast}
  <div class="fixed top-6 right-6 z-[100] px-5 py-3.5 rounded-2xl shadow-2xl flex items-center gap-3 text-sm font-semibold transition-all
    {toast.type === 'success'
      ? 'bg-emerald-500/20 border border-emerald-500/40 text-emerald-300 shadow-emerald-500/20'
      : 'bg-red-500/20 border border-red-500/40 text-red-300 shadow-red-500/20'}">
    {#if toast.type === 'success'}
      <svg class="w-4 h-4 shrink-0" fill="none" stroke="currentColor" viewBox="0 0 24 24">
        <path stroke-linecap="round" stroke-linejoin="round" stroke-width="2.5" d="M5 13l4 4L19 7"/>
      </svg>
    {:else}
      <svg class="w-4 h-4 shrink-0" fill="none" stroke="currentColor" viewBox="0 0 24 24">
        <path stroke-linecap="round" stroke-linejoin="round" stroke-width="2.5" d="M6 18L18 6M6 6l12 12"/>
      </svg>
    {/if}
    {toast.msg}
  </div>
{/if}

<!-- ── QUIZ MODAL ─────────────────────────────────────────────────────────────── -->
{#if showQuizModal}
  <!-- svelte-ignore a11y_click_events_have_key_events a11y_no_static_element_interactions -->
  <div class="fixed inset-0 z-50 flex items-center justify-center p-4 bg-slate-950/80 backdrop-blur-sm" onclick={(e) => e.target === e.currentTarget && closeQuizModal()}>
    <div class="relative w-full max-w-md bg-slate-900 border border-slate-700 rounded-3xl shadow-2xl overflow-hidden">
      <div class="absolute -top-16 -right-16 w-40 h-40 bg-cyan-500/10 rounded-full blur-3xl pointer-events-none"></div>
      <div class="absolute -bottom-16 -left-16 w-40 h-40 bg-violet-500/10 rounded-full blur-3xl pointer-events-none"></div>
      <div class="relative z-10 p-8">
        <div class="flex items-center justify-between mb-6">
          <h2 class="text-xl font-bold text-slate-100">
            {quizModalMode === 'create' ? '✨ Buat Quiz Baru' : '✏️ Edit Quiz'}
          </h2>
          <button onclick={closeQuizModal} class="w-8 h-8 flex items-center justify-center rounded-full bg-slate-800 hover:bg-slate-700 text-slate-400 hover:text-slate-100 transition-colors">
            <svg class="w-4 h-4" fill="none" stroke="currentColor" viewBox="0 0 24 24"><path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M6 18L18 6M6 6l12 12"/></svg>
          </button>
        </div>

        <div class="flex flex-col gap-4">
          <!-- Title -->
          <div>
            <label for="quiz-title" class="block text-xs font-semibold text-slate-400 uppercase tracking-wide mb-1.5">Judul Quiz</label>
            <input
              id="quiz-title"
              type="text"
              bind:value={quizForm.title}
              placeholder="Contoh: Quiz Teknologi Web"
              class="w-full px-4 py-3 bg-slate-800 border border-slate-700 rounded-xl text-slate-100 placeholder-slate-500 focus:outline-none focus:border-cyan-500/60 focus:ring-2 focus:ring-cyan-500/20 transition-all"
            />
          </div>

          <!-- Duration -->
          <div>
            <label for="quiz-duration" class="block text-xs font-semibold text-slate-400 uppercase tracking-wide mb-1.5">
              Durasi per Soal: <span class="text-cyan-400">{quizForm.duration} detik</span>
            </label>
            <input
              id="quiz-duration"
              type="range"
              min="5" max="120" step="5"
              bind:value={quizForm.duration}
              class="w-full accent-cyan-500"
            />
            <div class="flex justify-between text-xs text-slate-600 mt-1"><span>5s</span><span>120s</span></div>
          </div>

          {#if quizError}
            <div class="p-3 rounded-xl bg-red-500/10 border border-red-500/20 text-red-400 text-sm">{quizError}</div>
          {/if}

          <button
            onclick={submitQuizForm}
            disabled={quizSubmitting}
            class="w-full py-3 bg-gradient-to-r from-cyan-500 to-violet-500 hover:from-cyan-400 hover:to-violet-400 text-white font-bold rounded-xl shadow-lg shadow-cyan-500/25 transition-all hover:-translate-y-0.5 active:scale-95 disabled:opacity-50 disabled:cursor-not-allowed disabled:transform-none flex items-center justify-center gap-2"
          >
            {#if quizSubmitting}
              <svg class="animate-spin h-4 w-4" fill="none" viewBox="0 0 24 24"><circle class="opacity-25" cx="12" cy="12" r="10" stroke="currentColor" stroke-width="4"/><path class="opacity-75" fill="currentColor" d="M4 12a8 8 0 018-8V0C5.373 0 0 5.373 0 12h4z"/></svg>
              Menyimpan...
            {:else}
              {quizModalMode === 'create' ? 'Buat Quiz' : 'Simpan Perubahan'}
            {/if}
          </button>
        </div>
      </div>
    </div>
  </div>
{/if}

<!-- ── QUESTION MODAL ──────────────────────────────────────────────────────────── -->
{#if showQuestionModal}
  <!-- svelte-ignore a11y_click_events_have_key_events a11y_no_static_element_interactions -->
  <div class="fixed inset-0 z-50 flex items-center justify-center p-4 bg-slate-950/80 backdrop-blur-sm" onclick={(e) => e.target === e.currentTarget && closeQuestionModal()}>
    <div class="relative w-full max-w-lg bg-slate-900 border border-slate-700 rounded-3xl shadow-2xl overflow-hidden max-h-[90vh] overflow-y-auto">
      <div class="absolute -top-16 -right-16 w-40 h-40 bg-violet-500/10 rounded-full blur-3xl pointer-events-none"></div>
      <div class="relative z-10 p-8">
        <div class="flex items-center justify-between mb-6">
          <h2 class="text-xl font-bold text-slate-100">
            {questionModalMode === 'create' ? '➕ Tambah Soal' : '✏️ Edit Soal'}
          </h2>
          <button onclick={closeQuestionModal} class="w-8 h-8 flex items-center justify-center rounded-full bg-slate-800 hover:bg-slate-700 text-slate-400 hover:text-slate-100 transition-colors">
            <svg class="w-4 h-4" fill="none" stroke="currentColor" viewBox="0 0 24 24"><path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M6 18L18 6M6 6l12 12"/></svg>
          </button>
        </div>

        <div class="flex flex-col gap-4">
          <!-- Question text -->
          <div>
            <label for="q-text" class="block text-xs font-semibold text-slate-400 uppercase tracking-wide mb-1.5">Teks Pertanyaan</label>
            <textarea
              id="q-text"
              bind:value={questionForm.question_text}
              rows="3"
              placeholder="Tulis pertanyaan di sini..."
              class="w-full px-4 py-3 bg-slate-800 border border-slate-700 rounded-xl text-slate-100 placeholder-slate-500 focus:outline-none focus:border-violet-500/60 focus:ring-2 focus:ring-violet-500/20 transition-all resize-none"
            ></textarea>
          </div>

          <!-- Options -->
          <div>
            <p class="text-xs font-semibold text-slate-400 uppercase tracking-wide mb-2">Pilihan Jawaban</p>
            <div class="flex flex-col gap-2">
              {#each optionLetters as letter, i}
                <div class="flex items-center gap-3">
                  <span class="shrink-0 w-8 h-8 rounded-lg flex items-center justify-center text-xs font-bold
                    {questionForm.correct_option === letter
                      ? 'bg-emerald-500/30 text-emerald-400'
                      : 'bg-slate-700 text-slate-400'}">
                    {letter}
                  </span>
                  <input
                    id="option-{letter}"
                    type="text"
                    bind:value={questionForm[optionKeys[i]]}
                    placeholder="Pilihan {letter}..."
                    class="flex-1 px-3 py-2.5 bg-slate-800 border rounded-xl text-sm text-slate-100 placeholder-slate-500 focus:outline-none transition-all
                      {questionForm.correct_option === letter
                        ? 'border-emerald-500/50 focus:border-emerald-500/70 focus:ring-2 focus:ring-emerald-500/15'
                        : 'border-slate-700 focus:border-violet-500/60 focus:ring-2 focus:ring-violet-500/20'}"
                  />
                </div>
              {/each}
            </div>
          </div>

          <!-- Correct option -->
          <div>
            <p class="text-xs font-semibold text-slate-400 uppercase tracking-wide mb-2">Jawaban Benar</p>
            <div class="grid grid-cols-4 gap-2">
              {#each optionLetters as letter}
                <button
                  onclick={() => questionForm.correct_option = letter}
                  class="py-2.5 rounded-xl text-sm font-bold border transition-all
                    {questionForm.correct_option === letter
                      ? 'bg-emerald-500/20 border-emerald-500/50 text-emerald-400'
                      : 'bg-slate-800 border-slate-700 text-slate-500 hover:border-slate-600 hover:text-slate-300'}"
                >
                  {letter}
                </button>
              {/each}
            </div>
          </div>

          {#if questionError}
            <div class="p-3 rounded-xl bg-red-500/10 border border-red-500/20 text-red-400 text-sm">{questionError}</div>
          {/if}

          <button
            onclick={submitQuestionForm}
            disabled={questionSubmitting}
            class="w-full py-3 bg-gradient-to-r from-violet-500 to-cyan-500 hover:from-violet-400 hover:to-cyan-400 text-white font-bold rounded-xl shadow-lg shadow-violet-500/25 transition-all hover:-translate-y-0.5 active:scale-95 disabled:opacity-50 disabled:cursor-not-allowed disabled:transform-none flex items-center justify-center gap-2"
          >
            {#if questionSubmitting}
              <svg class="animate-spin h-4 w-4" fill="none" viewBox="0 0 24 24"><circle class="opacity-25" cx="12" cy="12" r="10" stroke="currentColor" stroke-width="4"/><path class="opacity-75" fill="currentColor" d="M4 12a8 8 0 018-8V0C5.373 0 0 5.373 0 12h4z"/></svg>
              Menyimpan...
            {:else}
              {questionModalMode === 'create' ? 'Tambah Soal' : 'Simpan Perubahan'}
            {/if}
          </button>
        </div>
      </div>
    </div>
  </div>
{/if}

<!-- ── MAIN LAYOUT ────────────────────────────────────────────────────────────── -->
<div class="min-h-screen p-6 md:p-10">

  <!-- Navbar -->
  <nav class="max-w-5xl mx-auto flex items-center justify-between mb-10 pb-6 border-b border-slate-800">
    <div class="flex items-center gap-3">
      <div class="w-10 h-10 rounded-xl bg-gradient-to-br from-cyan-500 to-violet-500 flex items-center justify-center shadow-lg shadow-cyan-500/20">
        <span class="font-bold text-white text-xl">Q</span>
      </div>
      <div>
        <h1 class="text-xl font-bold tracking-tight leading-none">QuizApp</h1>
        <p class="text-xs text-slate-500 mt-0.5">Manajemen Quiz</p>
      </div>
    </div>
    <a href="/" class="px-4 py-2 bg-slate-800 hover:bg-slate-700 text-slate-300 hover:text-white rounded-xl text-sm font-medium border border-slate-700 hover:border-slate-600 transition-all flex items-center gap-2">
      <svg class="w-4 h-4" fill="none" stroke="currentColor" viewBox="0 0 24 24">
        <path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M10 19l-7-7m0 0l7-7m-7 7h18"/>
      </svg>
      Dashboard
    </a>
  </nav>

  <div class="max-w-5xl mx-auto">

    {#if isLoading}
      <!-- Loading -->
      <div class="flex h-[60vh] items-center justify-center">
        <div class="flex flex-col items-center gap-4">
          <svg class="animate-spin h-10 w-10 text-cyan-500" fill="none" viewBox="0 0 24 24">
            <circle class="opacity-25" cx="12" cy="12" r="10" stroke="currentColor" stroke-width="4"/>
            <path class="opacity-75" fill="currentColor" d="M4 12a8 8 0 018-8V0C5.373 0 0 5.373 0 12h4z"/>
          </svg>
          <p class="text-slate-400 animate-pulse">Memuat data...</p>
        </div>
      </div>

    {:else}

      <!-- Tabs -->
      <div class="flex items-center gap-4 mb-8 border-b border-slate-800 pb-2">
        <button
          onclick={() => { activeTab = 'quizzes'; sessionStorage.setItem('adminActiveTab', 'quizzes'); }}
          class="pb-2 text-sm font-bold border-b-2 transition-all {activeTab === 'quizzes' ? 'border-cyan-500 text-cyan-400' : 'border-transparent text-slate-500 hover:text-slate-300'}"
        >
          Kelola Quiz
        </button>
        <button
          onclick={() => { activeTab = 'scores'; sessionStorage.setItem('adminActiveTab', 'scores'); }}
          class="pb-2 text-sm font-bold border-b-2 transition-all {activeTab === 'scores' ? 'border-cyan-500 text-cyan-400' : 'border-transparent text-slate-500 hover:text-slate-300'}"
        >
          Skor Pengguna
        </button>
      </div>

      {#if activeTab === 'quizzes'}
        <!-- Header -->
        <div class="flex items-center justify-between mb-8">
        <div>
          <h2 class="text-3xl font-extrabold text-slate-100">Daftar Quiz</h2>
          <p class="text-slate-500 text-sm mt-1">{quizzes.length} quiz tersedia</p>
        </div>
        <button
          id="create-quiz-btn"
          onclick={openCreateQuiz}
          class="flex items-center gap-2 px-5 py-3 bg-gradient-to-r from-cyan-500 to-violet-500 hover:from-cyan-400 hover:to-violet-400 text-white font-bold rounded-2xl shadow-lg shadow-cyan-500/25 transition-all transform hover:-translate-y-0.5 active:scale-95"
        >
          <svg class="w-5 h-5" fill="none" stroke="currentColor" viewBox="0 0 24 24">
            <path stroke-linecap="round" stroke-linejoin="round" stroke-width="2.5" d="M12 4v16m8-8H4"/>
          </svg>
          Buat Quiz
        </button>
      </div>

      <!-- Quiz List -->
      {#if quizzes.length === 0}
        <div class="flex flex-col items-center justify-center py-24 text-center">
          <div class="w-20 h-20 bg-slate-800 rounded-3xl flex items-center justify-center mb-6">
            <svg class="w-10 h-10 text-slate-600" fill="none" stroke="currentColor" viewBox="0 0 24 24">
              <path stroke-linecap="round" stroke-linejoin="round" stroke-width="1.5" d="M9 5H7a2 2 0 00-2 2v12a2 2 0 002 2h10a2 2 0 002-2V7a2 2 0 00-2-2h-2M9 5a2 2 0 002 2h2a2 2 0 002-2M9 5a2 2 0 012-2h2a2 2 0 012 2"/>
            </svg>
          </div>
          <h3 class="text-xl font-bold text-slate-400 mb-2">Belum ada quiz</h3>
          <p class="text-slate-600 text-sm mb-6">Mulai dengan membuat quiz pertamamu</p>
          <button
            onclick={openCreateQuiz}
            class="px-6 py-3 bg-gradient-to-r from-cyan-500 to-violet-500 text-white font-bold rounded-2xl shadow-lg shadow-cyan-500/25 transition-all hover:-translate-y-0.5"
          >
            ✨ Buat Quiz Pertama
          </button>
        </div>

      {:else}
        <div class="flex flex-col gap-4">
          {#each quizzes as quiz (quiz.id)}
            <div class="bg-slate-900/60 backdrop-blur border border-slate-800 rounded-2xl overflow-hidden transition-all">

              <!-- Quiz header row -->
              <div class="flex items-center gap-4 p-5">
                <!-- Expand toggle -->
                <button
                  onclick={() => toggleExpand(quiz.id)}
                  class="shrink-0 w-9 h-9 rounded-xl bg-slate-800 hover:bg-slate-700 flex items-center justify-center text-slate-400 hover:text-slate-100 transition-all"
                >
                  <svg class="w-4 h-4 transition-transform duration-200 {expandedQuizId === quiz.id ? 'rotate-90' : ''}" fill="none" stroke="currentColor" viewBox="0 0 24 24">
                    <path stroke-linecap="round" stroke-linejoin="round" stroke-width="2.5" d="M9 5l7 7-7 7"/>
                  </svg>
                </button>

                <!-- Info -->
                <div class="flex-1 min-w-0">
                  <h3 class="font-bold text-slate-100 truncate">{quiz.title}</h3>
                  <div class="flex items-center gap-3 mt-1">
                    <span class="text-xs text-slate-500 flex items-center gap-1">
                      <svg class="w-3 h-3" fill="none" stroke="currentColor" viewBox="0 0 24 24"><path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M12 8v4l3 3m6-3a9 9 0 11-18 0 9 9 0 0118 0z"/></svg>
                      {quiz.duration}s/soal
                    </span>
                    <span class="text-xs text-slate-600">•</span>
                    <span class="text-xs text-slate-500">
                      {new Date(quiz.created_at).toLocaleDateString('id-ID', { day: 'numeric', month: 'short', year: 'numeric' })}
                    </span>
                    {#if questionsMap[quiz.id]}
                      <span class="text-xs text-slate-600">•</span>
                      <span class="text-xs text-cyan-500/80">{questionsMap[quiz.id].length} soal</span>
                    {/if}
                  </div>
                </div>

                <!-- Actions -->
                <div class="flex items-center gap-2 shrink-0">
                  <button
                    onclick={() => openEditQuiz(quiz)}
                    title="Edit quiz"
                    class="w-8 h-8 rounded-lg bg-slate-800 hover:bg-cyan-500/20 hover:text-cyan-400 text-slate-400 flex items-center justify-center transition-all border border-transparent hover:border-cyan-500/30"
                  >
                    <svg class="w-4 h-4" fill="none" stroke="currentColor" viewBox="0 0 24 24">
                      <path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M15.232 5.232l3.536 3.536m-2.036-5.036a2.5 2.5 0 113.536 3.536L6.5 21.036H3v-3.572L16.732 3.732z"/>
                    </svg>
                  </button>

                  {#if showDeleteQuizConfirm === quiz.id}
                    <div class="flex items-center gap-1.5 px-3 py-1.5 bg-red-500/10 border border-red-500/30 rounded-xl">
                      <span class="text-xs text-red-400 font-medium">Hapus?</span>
                      <button
                        onclick={() => deleteQuiz(quiz.id)}
                        disabled={deletingQuizId === quiz.id}
                        class="text-xs font-bold text-red-400 hover:text-red-300 transition-colors disabled:opacity-50"
                      >
                        {deletingQuizId === quiz.id ? '...' : 'Ya'}
                      </button>
                      <span class="text-slate-600 text-xs">·</span>
                      <button onclick={() => showDeleteQuizConfirm = null} class="text-xs text-slate-500 hover:text-slate-300 transition-colors">
                        Batal
                      </button>
                    </div>
                  {:else}
                    <button
                      onclick={() => showDeleteQuizConfirm = quiz.id}
                      title="Hapus quiz"
                      class="w-8 h-8 rounded-lg bg-slate-800 hover:bg-red-500/20 hover:text-red-400 text-slate-400 flex items-center justify-center transition-all border border-transparent hover:border-red-500/30"
                    >
                      <svg class="w-4 h-4" fill="none" stroke="currentColor" viewBox="0 0 24 24">
                        <path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M19 7l-.867 12.142A2 2 0 0116.138 21H7.862a2 2 0 01-1.995-1.858L5 7m5 4v6m4-6v6m1-10V4a1 1 0 00-1-1h-4a1 1 0 00-1 1v3M4 7h16"/>
                      </svg>
                    </button>
                  {/if}
                </div>
              </div>

              <!-- Expanded: Questions -->
              {#if expandedQuizId === quiz.id}
                <div class="border-t border-slate-800 bg-slate-950/40">
                  <div class="p-5">
                    <div class="flex items-center justify-between mb-4">
                      <h4 class="text-sm font-semibold text-slate-400 uppercase tracking-wide">
                        Daftar Soal
                      </h4>
                      <button
                        onclick={() => openCreateQuestion(quiz.id)}
                        class="flex items-center gap-1.5 px-3 py-1.5 bg-violet-500/15 hover:bg-violet-500/25 text-violet-400 hover:text-violet-300 rounded-xl text-xs font-semibold border border-violet-500/25 hover:border-violet-500/40 transition-all"
                      >
                        <svg class="w-3.5 h-3.5" fill="none" stroke="currentColor" viewBox="0 0 24 24">
                          <path stroke-linecap="round" stroke-linejoin="round" stroke-width="2.5" d="M12 4v16m8-8H4"/>
                        </svg>
                        Tambah Soal
                      </button>
                    </div>

                    {#if loadingQuestions[quiz.id]}
                      <div class="flex items-center justify-center py-8 gap-3 text-slate-500">
                        <svg class="animate-spin h-5 w-5" fill="none" viewBox="0 0 24 24">
                          <circle class="opacity-25" cx="12" cy="12" r="10" stroke="currentColor" stroke-width="4"/>
                          <path class="opacity-75" fill="currentColor" d="M4 12a8 8 0 018-8V0C5.373 0 0 5.373 0 12h4z"/>
                        </svg>
                        <span class="text-sm">Memuat soal...</span>
                      </div>

                    {:else if !questionsMap[quiz.id] || questionsMap[quiz.id].length === 0}
                      <div class="text-center py-10">
                        <div class="w-12 h-12 bg-slate-800 rounded-2xl flex items-center justify-center mx-auto mb-3">
                          <svg class="w-6 h-6 text-slate-600" fill="none" stroke="currentColor" viewBox="0 0 24 24">
                            <path stroke-linecap="round" stroke-linejoin="round" stroke-width="1.5" d="M8.228 9c.549-1.165 2.03-2 3.772-2 2.21 0 4 1.343 4 3 0 1.4-1.278 2.575-3.006 2.907-.542.104-.994.54-.994 1.093m0 3h.01M21 12a9 9 0 11-18 0 9 9 0 0118 0z"/>
                          </svg>
                        </div>
                        <p class="text-slate-500 text-sm">Belum ada soal. Tambahkan soal pertama!</p>
                      </div>

                    {:else}
                      <div class="flex flex-col gap-2">
                        {#each questionsMap[quiz.id] as q, qi (q.id)}
                          <div class="flex items-start gap-3 p-4 bg-slate-900/60 rounded-xl border border-slate-800/80 hover:border-slate-700/80 transition-colors group">
                            <!-- Number -->
                            <span class="shrink-0 w-7 h-7 rounded-lg bg-slate-800 text-slate-500 text-xs font-bold flex items-center justify-center mt-0.5">
                              {qi + 1}
                            </span>

                            <!-- Content -->
                            <div class="flex-1 min-w-0">
                              <p class="text-slate-200 text-sm font-medium leading-snug mb-2">{q.question_text}</p>
                              <div class="grid grid-cols-2 gap-1.5">
                                {#each optionLetters as letter, li}
                                  <div class="flex items-center gap-1.5 text-xs
                                    {q.correct_option === letter ? 'text-emerald-400' : 'text-slate-500'}">
                                    <span class="shrink-0 w-5 h-5 rounded flex items-center justify-center font-bold text-[10px]
                                      {q.correct_option === letter ? 'bg-emerald-500/20 text-emerald-400' : 'bg-slate-800 text-slate-600'}">
                                      {letter}
                                    </span>
                                    <span class="truncate">{q[optionKeys[li]]}</span>
                                  </div>
                                {/each}
                              </div>
                            </div>

                            <!-- Actions -->
                            <div class="flex items-center gap-1.5 shrink-0 opacity-0 group-hover:opacity-100 transition-opacity">
                              <button
                                onclick={() => openEditQuestion(q)}
                                title="Edit soal"
                                class="w-7 h-7 rounded-lg bg-slate-800 hover:bg-cyan-500/20 hover:text-cyan-400 text-slate-500 flex items-center justify-center transition-all"
                              >
                                <svg class="w-3.5 h-3.5" fill="none" stroke="currentColor" viewBox="0 0 24 24">
                                  <path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M15.232 5.232l3.536 3.536m-2.036-5.036a2.5 2.5 0 113.536 3.536L6.5 21.036H3v-3.572L16.732 3.732z"/>
                                </svg>
                              </button>

                              {#if showDeleteQuestionConfirm === q.id}
                                <div class="flex items-center gap-1 px-2 py-1 bg-red-500/10 border border-red-500/30 rounded-lg">
                                  <button
                                    onclick={() => deleteQuestion(q.id, quiz.id)}
                                    disabled={deletingQuestionId === q.id}
                                    class="text-xs font-bold text-red-400 hover:text-red-300 disabled:opacity-50"
                                  >
                                    {deletingQuestionId === q.id ? '...' : 'Hapus'}
                                  </button>
                                  <span class="text-slate-600 text-xs">·</span>
                                  <button onclick={() => showDeleteQuestionConfirm = null} class="text-xs text-slate-500 hover:text-slate-300">
                                    Batal
                                  </button>
                                </div>
                              {:else}
                                <button
                                  onclick={() => showDeleteQuestionConfirm = q.id}
                                  title="Hapus soal"
                                  class="w-7 h-7 rounded-lg bg-slate-800 hover:bg-red-500/20 hover:text-red-400 text-slate-500 flex items-center justify-center transition-all"
                                >
                                  <svg class="w-3.5 h-3.5" fill="none" stroke="currentColor" viewBox="0 0 24 24">
                                    <path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M19 7l-.867 12.142A2 2 0 0116.138 21H7.862a2 2 0 01-1.995-1.858L5 7m5 4v6m4-6v6m1-10V4a1 1 0 00-1-1h-4a1 1 0 00-1 1v3M4 7h16"/>
                                  </svg>
                                </button>
                              {/if}
                            </div>
                          </div>
                        {/each}
                      </div>
                    {/if}
                  </div>
                </div>
              {/if}

            </div>
          {/each}
        </div>
      {/if}
      {/if}

      {#if activeTab === 'scores'}
        <div class="flex items-center justify-between mb-8">
          <div>
            <h2 class="text-3xl font-extrabold text-slate-100">Semua Skor Pengguna</h2>
            <p class="text-slate-500 text-sm mt-1">{allScores.length} riwayat quiz</p>
          </div>
          <div class="flex items-center gap-3">
            {#if allScores.length > 0}
              {#if showDeleteAllScoresConfirm}
                <div class="flex items-center gap-2 px-3 py-1.5 bg-red-500/10 border border-red-500/30 rounded-xl">
                  <span class="text-sm text-red-400 font-medium">Hapus Semua?</span>
                  <button
                    onclick={deleteAllScores}
                    disabled={isDeletingAllScores}
                    class="text-sm font-bold text-red-400 hover:text-red-300 disabled:opacity-50 transition-colors"
                  >
                    {isDeletingAllScores ? '...' : 'Ya'}
                  </button>
                  <span class="text-slate-600 text-sm">·</span>
                  <button onclick={() => showDeleteAllScoresConfirm = false} class="text-sm text-slate-500 hover:text-slate-300 transition-colors">
                    Batal
                  </button>
                </div>
              {:else}
                <button
                  onclick={() => showDeleteAllScoresConfirm = true}
                  class="flex items-center gap-2 px-4 py-2 bg-red-500/10 hover:bg-red-500/20 text-red-400 border border-red-500/20 hover:border-red-500/40 rounded-xl transition-all font-semibold"
                >
                  <svg class="w-4 h-4" fill="none" stroke="currentColor" viewBox="0 0 24 24"><path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M19 7l-.867 12.142A2 2 0 0116.138 21H7.862a2 2 0 01-1.995-1.858L5 7m5 4v6m4-6v6m1-10V4a1 1 0 00-1-1h-4a1 1 0 00-1 1v3M4 7h16"/></svg>
                  Hapus Semua
                </button>
              {/if}
            {/if}
            <button onclick={loadAllScores} class="flex items-center gap-2 px-4 py-2 bg-slate-800 hover:bg-slate-700 text-slate-300 rounded-xl transition-all font-semibold border border-slate-700 hover:border-slate-600">
              <svg class="w-4 h-4" fill="none" stroke="currentColor" viewBox="0 0 24 24"><path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M4 4v5h.582m15.356 2A8.001 8.001 0 004.582 9m0 0H9m11 11v-5h-.581m0 0a8.003 8.003 0 01-15.357-2m15.357 2H15"/></svg>
              Refresh
            </button>
          </div>
        </div>

        {#if loadingScores}
          <div class="flex justify-center py-20"><svg class="animate-spin h-8 w-8 text-cyan-500" fill="none" viewBox="0 0 24 24"><circle class="opacity-25" cx="12" cy="12" r="10" stroke="currentColor" stroke-width="4"/><path class="opacity-75" fill="currentColor" d="M4 12a8 8 0 018-8V0C5.373 0 0 5.373 0 12h4z"/></svg></div>
        {:else if allScores.length === 0}
          <div class="flex flex-col items-center justify-center py-24 text-center">
            <div class="w-20 h-20 bg-slate-800 rounded-3xl flex items-center justify-center mb-6">
              <svg class="w-10 h-10 text-slate-600" fill="none" stroke="currentColor" viewBox="0 0 24 24">
                <path stroke-linecap="round" stroke-linejoin="round" stroke-width="1.5" d="M9 5H7a2 2 0 00-2 2v12a2 2 0 002 2h10a2 2 0 002-2V7a2 2 0 00-2-2h-2M9 5a2 2 0 002 2h2a2 2 0 002-2M9 5a2 2 0 012-2h2a2 2 0 012 2"/>
              </svg>
            </div>
            <h3 class="text-xl font-bold text-slate-400 mb-2">Belum ada skor</h3>
            <p class="text-slate-600 text-sm mb-6">Belum ada user yang menyelesaikan quiz.</p>
          </div>
        {:else}
          <div class="bg-slate-900/60 border border-slate-800 rounded-2xl overflow-hidden backdrop-blur">
            <div class="hidden md:grid grid-cols-[1.5fr_1.5fr_100px_120px_150px] gap-4 px-6 py-4 bg-slate-950/50 border-b border-slate-800 text-xs font-semibold text-slate-500 uppercase tracking-wide">
              <span>Pengguna</span>
              <span>Quiz</span>
              <span class="text-center">Skor</span>
              <span class="text-center">Akurasi</span>
              <span class="text-right">Selesai Pada</span>
            </div>
            <div class="divide-y divide-slate-800/60">
              {#each allScores as score (score.id)}
                {@const accuracy = Math.round((score.correct_count / score.total_questions) * 100)}
                <div class="grid grid-cols-1 md:grid-cols-[1.5fr_1.5fr_100px_120px_150px] gap-2 md:gap-4 px-6 py-4 hover:bg-slate-800/40 transition-colors items-center">
                  <div class="flex items-center gap-3">
                    <div class="w-8 h-8 rounded-full bg-slate-800 flex items-center justify-center font-bold text-slate-300 text-xs shadow-inner shadow-slate-900">
                      {score.username.substring(0,2).toUpperCase()}
                    </div>
                    <div class="font-bold text-slate-200">{score.username}</div>
                  </div>
                  <div class="text-sm text-slate-300 truncate font-medium">{score.quiz_title}</div>
                  <div class="text-center">
                    <span class="font-bold text-sm {accuracy >= 80 ? 'text-emerald-400' : accuracy >= 60 ? 'text-cyan-400' : accuracy >= 40 ? 'text-amber-400' : 'text-red-400'}">{score.score}</span>
                    <span class="text-xs text-slate-600">/{score.total_questions * 10}</span>
                  </div>
                  <div class="flex items-center gap-2 justify-center">
                    <div class="flex-1 h-1.5 bg-slate-800 rounded-full max-w-[50px]">
                      <div class="h-full rounded-full {accuracy >= 80 ? 'bg-emerald-400' : accuracy >= 60 ? 'bg-cyan-400' : accuracy >= 40 ? 'bg-amber-400' : 'bg-red-400'}" style="width: {accuracy}%"></div>
                    </div>
                    <span class="text-xs font-bold {accuracy >= 80 ? 'text-emerald-400' : accuracy >= 60 ? 'text-cyan-400' : accuracy >= 40 ? 'text-amber-400' : 'text-red-400'}">{accuracy}%</span>
                  </div>
                  <div class="text-right text-xs text-slate-500 hidden md:block">
                    {new Date(score.completed_at).toLocaleString('id-ID', { day: 'numeric', month: 'short', year: 'numeric', hour: '2-digit', minute: '2-digit' })}
                  </div>
                </div>
              {/each}
            </div>
          </div>
        {/if}
      {/if}

    {/if}
  </div>
</div>
