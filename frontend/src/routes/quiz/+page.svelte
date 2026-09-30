<script lang="ts">
  import { onMount, onDestroy } from 'svelte';

  // ─── Types ───────────────────────────────────────────────────────────────────
  interface QuizSummary {
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
    correct_option: string; // "A" | "B" | "C" | "D"
    order_index: number;
  }

  interface UserAnswer {
    questionId: number;
    selectedLetter: string | null; // null = timeout
    isCorrect: boolean;
    timeLeft: number;
  }

  // ─── Phase ───────────────────────────────────────────────────────────────────
  type Phase = 'list' | 'intro' | 'quiz' | 'result';

  let phase = $state<Phase>('list');

  // ─── List state ──────────────────────────────────────────────────────────────
  let quizList = $state<QuizSummary[]>([]);
  let listLoading = $state(true);
  let listError = $state('');

  // ─── Active quiz state ────────────────────────────────────────────────────────
  let activeQuiz = $state<QuizSummary | null>(null);
  let questions = $state<Question[]>([]);
  let quizLoading = $state(false);
  let quizLoadError = $state('');

  // ─── Quiz play state ──────────────────────────────────────────────────────────
  let currentIndex = $state(0);
  let isAnswered = $state(false);
  let timeLeft = $state(15);
  let userAnswers = $state<UserAnswer[]>([]);

  let timerInterval: ReturnType<typeof setInterval> | null = null;

  // ─── Submit state ─────────────────────────────────────────────────────────────
  let submitStatus = $state<'idle' | 'saving' | 'saved' | 'error'>('idle');
  let savedResultId = $state<number | null>(null);

  let currentQuestion = $derived(questions[currentIndex]);
  let totalScore = $derived(userAnswers.filter((a) => a.isCorrect).length * 10);
  let correctCount = $derived(userAnswers.filter((a) => a.isCorrect).length);
  let progressPct = $derived(questions.length > 0 ? ((currentIndex + 1) / questions.length) * 100 : 0);
  let timerPct = $derived(activeQuiz ? (timeLeft / activeQuiz.duration) * 100 : 100);
  let maxScore = $derived(questions.length * 10);

  const optionLetters = ['A', 'B', 'C', 'D'] as const;
  const optionKeys = ['option_a', 'option_b', 'option_c', 'option_d'] as const;

  // ─── Init ─────────────────────────────────────────────────────────────────────
  onMount(async () => {
    await loadQuizList();
  });

  onDestroy(() => clearTimer());

  // ─── API ─────────────────────────────────────────────────────────────────────
  async function loadQuizList() {
    listLoading = true;
    listError = '';
    try {
      const res = await fetch('/api/quizzes');
      if (res.ok) {
        quizList = await res.json();
      } else {
        listError = 'Gagal memuat daftar quiz.';
      }
    } catch {
      listError = 'Tidak dapat terhubung ke server.';
    } finally {
      listLoading = false;
    }
  }

  async function selectQuiz(quiz: QuizSummary) {
    quizLoading = true;
    quizLoadError = '';
    activeQuiz = quiz;
    phase = 'intro';
    try {
      const res = await fetch(`/api/quizzes/${quiz.id}`);
      if (res.ok) {
        const data = await res.json();
        questions = data.questions ?? [];
        if (questions.length === 0) {
          quizLoadError = 'Quiz ini belum memiliki soal.';
        }
      } else {
        quizLoadError = 'Gagal memuat soal quiz.';
      }
    } catch {
      quizLoadError = 'Tidak dapat terhubung ke server.';
    } finally {
      quizLoading = false;
    }
  }

  // ─── Timer ────────────────────────────────────────────────────────────────────
  function clearTimer() {
    if (timerInterval) { clearInterval(timerInterval); timerInterval = null; }
  }

  function startTimer() {
    clearTimer();
    timeLeft = activeQuiz!.duration;
    timerInterval = setInterval(() => {
      timeLeft -= 1;
      if (timeLeft <= 0) {
        timeLeft = 0;
        handleTimeout();
      }
    }, 1000);
  }

  function handleTimeout() {
    clearTimer();
    userAnswers = [
      ...userAnswers,
      { questionId: currentQuestion.id, selectedLetter: null, isCorrect: false, timeLeft: 0 }
    ];
    goToNext();
  }

  function goToNext() {
    if (currentIndex + 1 < questions.length) {
      currentIndex += 1;
      isAnswered = false;
      startTimer();
    } else {
      phase = 'result';
      clearTimer();
      submitResult();
    }
  }

  // ─── Actions ──────────────────────────────────────────────────────────────────
  function startQuiz() {
    currentIndex = 0;
    userAnswers = [];
    isAnswered = false;
    phase = 'quiz';
    startTimer();
  }

  function selectAnswer(letter: string) {
    if (isAnswered) return;
    clearTimer();
    isAnswered = true;
    const isCorrect = letter === currentQuestion.correct_option;
    userAnswers = [
      ...userAnswers,
      { questionId: currentQuestion.id, selectedLetter: letter, isCorrect, timeLeft }
    ];
    goToNext();
  }

  function backToList() {
    clearTimer();
    phase = 'list';
    activeQuiz = null;
    questions = [];
    userAnswers = [];
    currentIndex = 0;
    isAnswered = false;
    quizLoadError = '';
    submitStatus = 'idle';
    savedResultId = null;
  }

  function retryQuiz() {
    currentIndex = 0;
    userAnswers = [];
    isAnswered = false;
    quizLoadError = '';
    submitStatus = 'idle';
    savedResultId = null;
    phase = 'intro';
  }

  // ─── Submit result ke backend ─────────────────────────────────────────────────
  async function submitResult() {
    if (!activeQuiz) return;
    submitStatus = 'saving';
    try {
      const payload = {
        quiz_id: activeQuiz.id,
        score: userAnswers.filter((a) => a.isCorrect).length * 10,
        correct_count: userAnswers.filter((a) => a.isCorrect).length,
        total_questions: questions.length,
        answers: userAnswers.map((a) => ({
          question_id: a.questionId,
          selected_option: a.selectedLetter,
          is_correct: a.isCorrect,
          time_left: a.timeLeft,
        })),
      };
      const res = await fetch('/api/results', {
        method: 'POST',
        credentials: 'include',
        headers: { 'Content-Type': 'application/json' },
        body: JSON.stringify(payload),
      });
      if (res.ok) {
        const data = await res.json();
        savedResultId = data.result.id;
        submitStatus = 'saved';
      } else {
        submitStatus = 'error';
      }
    } catch {
      submitStatus = 'error';
    }
  }

  // ─── Helpers ─────────────────────────────────────────────────────────────────
  function getOptionText(q: Question, letter: string): string {
    const map: Record<string, string> = {
      A: q.option_a, B: q.option_b, C: q.option_c, D: q.option_d
    };
    return map[letter] ?? '';
  }

  function getScoreGrade(): { label: string; color: string; emoji: string } {
    if (questions.length === 0) return { label: '', color: '', emoji: '' };
    const pct = (correctCount / questions.length) * 100;
    if (pct === 100) return { label: 'Sempurna!', color: 'text-amber-400', emoji: '🏆' };
    if (pct >= 80)  return { label: 'Luar Biasa!', color: 'text-cyan-400', emoji: '🌟' };
    if (pct >= 60)  return { label: 'Bagus!', color: 'text-emerald-400', emoji: '👍' };
    if (pct >= 40)  return { label: 'Cukup', color: 'text-violet-400', emoji: '📚' };
    return { label: 'Perlu Belajar Lagi', color: 'text-red-400', emoji: '💪' };
  }

  function formatDate(str: string) {
    return new Date(str).toLocaleDateString('id-ID', { day: 'numeric', month: 'short', year: 'numeric' });
  }
</script>

<svelte:head>
  <title>{activeQuiz ? activeQuiz.title + ' — QuizApp' : 'Pilih Quiz — QuizApp'}</title>
</svelte:head>

<div class="min-h-screen p-4 md:p-8 flex flex-col">

  <!-- NAVBAR -->
  <nav class="max-w-3xl mx-auto w-full flex items-center justify-between mb-8">
    <div class="flex items-center gap-3">
      <div class="w-10 h-10 rounded-xl bg-gradient-to-br from-cyan-500 to-violet-500 flex items-center justify-center shadow-lg shadow-cyan-500/20">
        <span class="font-bold text-white text-xl">Q</span>
      </div>
      <span class="text-xl font-bold tracking-tight">QuizApp</span>
    </div>
    {#if phase !== 'list'}
      <button
        onclick={backToList}
        class="px-4 py-2 bg-slate-800 hover:bg-slate-700 text-slate-300 hover:text-white rounded-xl text-sm font-medium border border-slate-700 hover:border-slate-600 transition-all flex items-center gap-2"
      >
        <svg class="w-4 h-4" fill="none" stroke="currentColor" viewBox="0 0 24 24">
          <path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M10 19l-7-7m0 0l7-7m-7 7h18"/>
        </svg>
        Pilih Quiz Lain
      </button>
    {:else}
      <a
        href="/"
        class="px-4 py-2 bg-slate-800 hover:bg-slate-700 text-slate-300 hover:text-white rounded-xl text-sm font-medium border border-slate-700 hover:border-slate-600 transition-all flex items-center gap-2"
      >
        <svg class="w-4 h-4" fill="none" stroke="currentColor" viewBox="0 0 24 24">
          <path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M10 19l-7-7m0 0l7-7m-7 7h18"/>
        </svg>
        Dashboard
      </a>
    {/if}
  </nav>

  <!-- ══════════ LIST SCREEN ══════════ -->
  {#if phase === 'list'}
    <div class="max-w-3xl mx-auto w-full flex-1 flex flex-col">

      <!-- Header -->
      <div class="mb-8">
        <h1 class="text-4xl font-extrabold bg-clip-text text-transparent bg-gradient-to-r from-cyan-400 via-violet-400 to-pink-400 mb-2">
          Pilih Quiz
        </h1>
        <p class="text-slate-400">Pilih quiz yang ingin kamu kerjakan</p>
      </div>

      {#if listLoading}
        <div class="flex-1 flex items-center justify-center">
          <div class="flex flex-col items-center gap-4">
            <svg class="animate-spin h-10 w-10 text-cyan-500" fill="none" viewBox="0 0 24 24">
              <circle class="opacity-25" cx="12" cy="12" r="10" stroke="currentColor" stroke-width="4"/>
              <path class="opacity-75" fill="currentColor" d="M4 12a8 8 0 018-8V0C5.373 0 0 5.373 0 12h4z"/>
            </svg>
            <p class="text-slate-400 animate-pulse">Memuat daftar quiz...</p>
          </div>
        </div>

      {:else if listError}
        <div class="flex-1 flex flex-col items-center justify-center gap-4">
          <div class="p-6 bg-red-500/10 border border-red-500/20 rounded-2xl text-center max-w-sm">
            <p class="text-red-400 mb-4">{listError}</p>
            <button onclick={loadQuizList} class="px-5 py-2 bg-red-500/20 hover:bg-red-500/30 text-red-300 rounded-xl transition-colors text-sm font-medium">
              Coba Lagi
            </button>
          </div>
        </div>

      {:else if quizList.length === 0}
        <div class="flex-1 flex flex-col items-center justify-center gap-4 text-center">
          <div class="w-20 h-20 bg-slate-800 rounded-3xl flex items-center justify-center mb-2">
            <svg class="w-10 h-10 text-slate-600" fill="none" stroke="currentColor" viewBox="0 0 24 24">
              <path stroke-linecap="round" stroke-linejoin="round" stroke-width="1.5" d="M9.663 17h4.673M12 3v1m6.364 1.636l-.707.707M21 12h-1M4 12H3m3.343-5.657l-.707-.707m2.828 9.9a5 5 0 117.072 0l-.548.547A3.374 3.374 0 0014 18.469V19a2 2 0 11-4 0v-.531c0-.895-.356-1.754-.988-2.386l-.548-.547z"/>
            </svg>
          </div>
          <h2 class="text-xl font-bold text-slate-400">Belum ada quiz tersedia</h2>
          <p class="text-slate-600 text-sm">Admin belum menambahkan quiz apapun.</p>
          <a href="/manage" class="mt-2 px-5 py-2.5 bg-gradient-to-r from-cyan-500 to-violet-500 text-white text-sm font-bold rounded-xl shadow-lg shadow-cyan-500/25 transition-all hover:-translate-y-0.5">
            ⚙️ Buat Quiz
          </a>
        </div>

      {:else}
        <div class="grid grid-cols-1 sm:grid-cols-2 gap-4">
          {#each quizList as quiz (quiz.id)}
            <button
              onclick={() => selectQuiz(quiz)}
              class="text-left relative bg-slate-900/60 backdrop-blur border border-slate-800 hover:border-cyan-500/40 rounded-2xl p-6 overflow-hidden transition-all duration-200 hover:-translate-y-1 hover:shadow-xl hover:shadow-cyan-500/10 group active:scale-[0.98]"
            >
              <!-- Hover glow -->
              <div class="absolute inset-0 bg-gradient-to-br from-cyan-500/5 to-violet-500/5 opacity-0 group-hover:opacity-100 transition-opacity pointer-events-none"></div>

              <div class="relative z-10">
                <!-- Icon -->
                <div class="w-12 h-12 bg-gradient-to-br from-cyan-500/20 to-violet-500/20 rounded-xl flex items-center justify-center mb-4 group-hover:from-cyan-500/30 group-hover:to-violet-500/30 transition-all">
                  <svg class="w-6 h-6 text-cyan-400" fill="none" stroke="currentColor" viewBox="0 0 24 24">
                    <path stroke-linecap="round" stroke-linejoin="round" stroke-width="1.5" d="M9.663 17h4.673M12 3v1m6.364 1.636l-.707.707M21 12h-1M4 12H3m3.343-5.657l-.707-.707m2.828 9.9a5 5 0 117.072 0l-.548.547A3.374 3.374 0 0014 18.469V19a2 2 0 11-4 0v-.531c0-.895-.356-1.754-.988-2.386l-.548-.547z"/>
                  </svg>
                </div>

                <!-- Title -->
                <h3 class="font-bold text-slate-100 text-lg leading-snug mb-3 group-hover:text-white transition-colors">
                  {quiz.title}
                </h3>

                <!-- Meta -->
                <div class="flex items-center gap-3 flex-wrap">
                  <span class="flex items-center gap-1.5 text-xs text-slate-500">
                    <svg class="w-3.5 h-3.5" fill="none" stroke="currentColor" viewBox="0 0 24 24">
                      <path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M12 8v4l3 3m6-3a9 9 0 11-18 0 9 9 0 0118 0z"/>
                    </svg>
                    {quiz.duration}s / soal
                  </span>
                  <span class="text-slate-700">•</span>
                  <span class="text-xs text-slate-600">{formatDate(quiz.created_at)}</span>
                </div>

                <!-- Arrow -->
                <div class="absolute top-5 right-5 w-8 h-8 rounded-xl bg-slate-800 group-hover:bg-cyan-500/20 flex items-center justify-center transition-all">
                  <svg class="w-4 h-4 text-slate-500 group-hover:text-cyan-400 transition-colors" fill="none" stroke="currentColor" viewBox="0 0 24 24">
                    <path stroke-linecap="round" stroke-linejoin="round" stroke-width="2.5" d="M9 5l7 7-7 7"/>
                  </svg>
                </div>
              </div>
            </button>
          {/each}
        </div>
      {/if}

    </div>

  <!-- ══════════ INTRO SCREEN ══════════ -->
  {:else if phase === 'intro'}
    <div class="max-w-2xl mx-auto w-full flex-1 flex flex-col items-center justify-center gap-8">

      {#if quizLoading}
        <div class="flex flex-col items-center gap-4">
          <svg class="animate-spin h-10 w-10 text-cyan-500" fill="none" viewBox="0 0 24 24">
            <circle class="opacity-25" cx="12" cy="12" r="10" stroke="currentColor" stroke-width="4"/>
            <path class="opacity-75" fill="currentColor" d="M4 12a8 8 0 018-8V0C5.373 0 0 5.373 0 12h4z"/>
          </svg>
          <p class="text-slate-400 animate-pulse">Memuat soal...</p>
        </div>

      {:else if quizLoadError}
        <div class="p-6 bg-red-500/10 border border-red-500/20 rounded-2xl text-center max-w-sm">
          <p class="text-red-400 mb-4">{quizLoadError}</p>
          <button onclick={backToList} class="px-5 py-2 bg-slate-800 text-slate-300 rounded-xl text-sm font-medium">
            Kembali ke Daftar
          </button>
        </div>

      {:else}
        <!-- Icon -->
        <div class="relative">
          <div class="w-28 h-28 rounded-3xl bg-gradient-to-br from-cyan-500 to-violet-500 flex items-center justify-center shadow-2xl shadow-cyan-500/30 animate-pulse">
            <svg class="w-14 h-14 text-white" fill="none" stroke="currentColor" viewBox="0 0 24 24">
              <path stroke-linecap="round" stroke-linejoin="round" stroke-width="1.5"
                d="M9.663 17h4.673M12 3v1m6.364 1.636l-.707.707M21 12h-1M4 12H3m3.343-5.657l-.707-.707m2.828 9.9a5 5 0 117.072 0l-.548.547A3.374 3.374 0 0014 18.469V19a2 2 0 11-4 0v-.531c0-.895-.356-1.754-.988-2.386l-.548-.547z"/>
            </svg>
          </div>
          <div class="absolute -top-2 -right-2 w-8 h-8 bg-amber-400 rounded-xl flex items-center justify-center shadow-lg">
            <span class="text-sm font-bold text-slate-900">{questions.length}</span>
          </div>
        </div>

        <div class="text-center">
          <h1 class="text-4xl md:text-5xl font-extrabold mb-3 bg-clip-text text-transparent bg-gradient-to-r from-cyan-400 via-violet-400 to-pink-400">
            {activeQuiz?.title}
          </h1>
          <p class="text-slate-400 text-lg">Siap untuk mengerjakan quiz ini?</p>
        </div>

        <!-- Rules -->
        <div class="w-full bg-slate-900/60 backdrop-blur-sm border border-slate-800 rounded-2xl p-6 grid grid-cols-1 sm:grid-cols-3 gap-4">
          {#each [
            { icon: '❓', label: `${questions.length} Pertanyaan`, sub: 'Pilihan ganda' },
            { icon: '⏱️', label: `${activeQuiz?.duration} Detik`, sub: 'Per soal' },
            { icon: '🏆', label: '10 Poin', sub: 'Per jawaban benar' }
          ] as rule}
            <div class="flex items-center gap-3 p-3 bg-slate-800/50 rounded-xl">
              <span class="text-2xl">{rule.icon}</span>
              <div>
                <p class="font-semibold text-slate-100 text-sm">{rule.label}</p>
                <p class="text-xs text-slate-500">{rule.sub}</p>
              </div>
            </div>
          {/each}
        </div>

        <button
          id="start-quiz-btn"
          onclick={startQuiz}
          disabled={questions.length === 0}
          class="px-12 py-5 text-lg font-bold text-white rounded-2xl
                 bg-gradient-to-r from-cyan-500 to-violet-500
                 hover:from-cyan-400 hover:to-violet-400
                 shadow-2xl shadow-cyan-500/30
                 transform hover:-translate-y-1 active:scale-95
                 transition-all duration-200
                 disabled:opacity-40 disabled:cursor-not-allowed disabled:transform-none"
        >
          Mulai Quiz 🚀
        </button>
      {/if}
    </div>

  <!-- ══════════ QUIZ SCREEN ══════════ -->
  {:else if phase === 'quiz' && currentQuestion}
    <div class="max-w-3xl mx-auto w-full flex-1 flex flex-col gap-6">

      <!-- Progress + Timer -->
      <div class="flex items-center gap-4">
        <span class="shrink-0 text-sm font-semibold text-slate-400 tabular-nums">
          {currentIndex + 1} <span class="text-slate-600">/</span> {questions.length}
        </span>

        <div class="flex-1 h-2 bg-slate-800 rounded-full overflow-hidden">
          <div
            class="h-full bg-gradient-to-r from-cyan-500 to-violet-500 rounded-full transition-all duration-500"
            style="width: {progressPct}%"
          ></div>
        </div>

        <!-- Circular timer -->
        <div class="shrink-0 relative w-12 h-12">
          <svg class="w-12 h-12 -rotate-90" viewBox="0 0 48 48">
            <circle cx="24" cy="24" r="20" fill="none" stroke="#1e293b" stroke-width="4"/>
            <circle
              cx="24" cy="24" r="20"
              fill="none"
              stroke-width="4"
              stroke-linecap="round"
              stroke-dasharray={2 * Math.PI * 20}
              stroke-dashoffset={2 * Math.PI * 20 * (1 - timerPct / 100)}
              class="transition-all duration-1000 {timerPct > 60 ? 'stroke-emerald-400' : timerPct > 30 ? 'stroke-amber-400' : 'stroke-red-500'}"
            />
          </svg>
          <span class="absolute inset-0 flex items-center justify-center text-sm font-bold tabular-nums
            {timerPct > 60 ? 'text-emerald-400' : timerPct > 30 ? 'text-amber-400' : 'text-red-400'}">
            {timeLeft}
          </span>
        </div>
      </div>

      <!-- Question Card -->
      <div class="relative bg-slate-900/70 backdrop-blur-xl border border-slate-700/60 rounded-3xl p-8 shadow-2xl overflow-hidden">
        <div class="absolute -top-20 -right-20 w-48 h-48 bg-cyan-500/8 rounded-full blur-3xl pointer-events-none"></div>
        <div class="absolute -bottom-20 -left-20 w-48 h-48 bg-violet-500/8 rounded-full blur-3xl pointer-events-none"></div>

        <div class="relative z-10 mb-6">
          <span class="px-3 py-1 bg-cyan-500/15 text-cyan-400 text-xs font-semibold rounded-lg border border-cyan-500/20 uppercase tracking-wide">
            Pertanyaan {currentIndex + 1}
          </span>
        </div>

        <h2 class="relative z-10 text-2xl font-bold text-slate-100 leading-snug mb-8">
          {currentQuestion.question_text}
        </h2>

        <div class="relative z-10 grid grid-cols-1 gap-3">
          {#each optionLetters as letter, i}
            <button
              id="option-{letter}"
              onclick={() => selectAnswer(letter)}
              disabled={isAnswered}
              class="w-full text-left px-5 py-4 rounded-2xl border transition-all duration-200 flex items-center gap-4 group
                     border-slate-700/60 bg-slate-800/40 hover:bg-slate-700/60 hover:border-cyan-500/40
                     cursor-pointer hover:-translate-y-0.5 active:scale-[0.98]
                     disabled:cursor-not-allowed"
            >
              <span class="shrink-0 w-9 h-9 rounded-xl flex items-center justify-center text-sm font-bold
                bg-slate-700 text-slate-300 group-hover:bg-cyan-500/20 group-hover:text-cyan-300 transition-colors">
                {letter}
              </span>
              <span class="font-medium text-sm leading-snug">{getOptionText(currentQuestion, letter)}</span>
            </button>
          {/each}
        </div>
      </div>

      <!-- Dot progress -->
      <div class="flex items-center justify-center gap-1.5 pb-4">
        {#each questions as _, i}
          <div class="rounded-full transition-all duration-300 {
            i < userAnswers.length
              ? (userAnswers[i]?.isCorrect ? 'w-2.5 h-2.5 bg-emerald-400' : 'w-2.5 h-2.5 bg-red-400')
              : i === currentIndex
              ? 'w-6 h-2.5 bg-cyan-400'
              : 'w-2.5 h-2.5 bg-slate-700'
          }"></div>
        {/each}
      </div>
    </div>

  <!-- ══════════ RESULT SCREEN ══════════ -->
  {:else if phase === 'result'}
    {@const grade = getScoreGrade()}
    <div class="max-w-3xl mx-auto w-full flex-1 flex flex-col gap-8">

      <!-- Score Card -->
      <div class="relative bg-slate-900/70 backdrop-blur-xl border border-slate-700/60 rounded-3xl p-8 md:p-10 shadow-2xl overflow-hidden text-center">
        <div class="absolute -top-24 left-1/2 -translate-x-1/2 w-64 h-64 bg-violet-500/10 rounded-full blur-3xl pointer-events-none"></div>
        <div class="absolute -bottom-24 left-1/2 -translate-x-1/2 w-64 h-64 bg-cyan-500/10 rounded-full blur-3xl pointer-events-none"></div>

        <div class="relative z-10">
          <div class="text-6xl mb-4 animate-bounce">{grade.emoji}</div>
          <h2 class="text-xl font-bold text-slate-300 mb-1">Quiz Selesai!</h2>
          <p class="text-slate-500 text-sm mb-4">{activeQuiz?.title}</p>
          <p class="text-5xl md:text-6xl font-extrabold mb-1 bg-clip-text text-transparent bg-gradient-to-r from-cyan-400 to-violet-400">
            {totalScore} <span class="text-3xl text-slate-500">/ {maxScore}</span>
          </p>
          <p class="text-2xl font-bold {grade.color} mb-6">{grade.label}</p>

          <div class="grid grid-cols-3 gap-4 mb-8">
            {#each [
              { label: 'Benar', value: correctCount, color: 'text-emerald-400', bg: 'bg-emerald-500/10 border-emerald-500/20' },
              { label: 'Salah', value: questions.length - correctCount, color: 'text-red-400', bg: 'bg-red-500/10 border-red-500/20' },
              { label: 'Akurasi', value: Math.round((correctCount / questions.length) * 100) + '%', color: 'text-cyan-400', bg: 'bg-cyan-500/10 border-cyan-500/20' }
            ] as stat}
              <div class="p-4 rounded-2xl border {stat.bg}">
                <p class="text-2xl font-bold {stat.color}">{stat.value}</p>
                <p class="text-xs text-slate-500 mt-1">{stat.label}</p>
              </div>
            {/each}
          </div>

          <!-- Save status indicator -->
          {#if submitStatus === 'saving'}
            <div class="flex items-center justify-center gap-2 mb-4 text-slate-400 text-sm">
              <svg class="animate-spin h-4 w-4 text-cyan-400" fill="none" viewBox="0 0 24 24">
                <circle class="opacity-25" cx="12" cy="12" r="10" stroke="currentColor" stroke-width="4"/>
                <path class="opacity-75" fill="currentColor" d="M4 12a8 8 0 018-8V0C5.373 0 0 5.373 0 12h4z"/>
              </svg>
              Menyimpan hasil...
            </div>
          {:else if submitStatus === 'saved'}
            <div class="flex items-center justify-center gap-2 mb-4 text-emerald-400 text-sm font-medium">
              <svg class="w-4 h-4" fill="none" stroke="currentColor" viewBox="0 0 24 24">
                <path stroke-linecap="round" stroke-linejoin="round" stroke-width="2.5" d="M5 13l4 4L19 7"/>
              </svg>
              Hasil tersimpan
            </div>
          {:else if submitStatus === 'error'}
            <div class="flex items-center justify-center gap-2 mb-4 text-amber-400 text-sm">
              <svg class="w-4 h-4" fill="none" stroke="currentColor" viewBox="0 0 24 24">
                <path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M12 9v2m0 4h.01M10.29 3.86L1.82 18a2 2 0 001.71 3h16.94a2 2 0 001.71-3L13.71 3.86a2 2 0 00-3.42 0z"/>
              </svg>
              Gagal menyimpan (login untuk menyimpan riwayat)
            </div>
          {/if}

          <div class="flex flex-col sm:flex-row gap-3 justify-center">
            <button
              id="restart-quiz-btn"
              onclick={retryQuiz}
              class="px-8 py-3.5 bg-gradient-to-r from-cyan-500 to-violet-500 hover:from-cyan-400 hover:to-violet-400 text-white font-bold rounded-2xl shadow-lg shadow-cyan-500/25 transition-all hover:-translate-y-0.5 active:scale-95"
            >
              🔄 Ulangi Quiz
            </button>
            <button
              onclick={backToList}
              class="px-8 py-3.5 bg-slate-800 hover:bg-slate-700 text-slate-300 hover:text-white font-semibold rounded-2xl border border-slate-700 hover:border-slate-600 transition-all"
            >
              📋 Pilih Quiz Lain
            </button>
          </div>
        </div>
      </div>

      <!-- Review -->
      <div>
        <h3 class="text-xl font-bold mb-4 flex items-center gap-2">
          <span class="w-8 h-8 bg-violet-500/20 text-violet-400 rounded-lg flex items-center justify-center text-sm">📋</span>
          Review Jawaban
        </h3>

        <div class="flex flex-col gap-3">
          {#each questions as q, i}
            {@const answer = userAnswers[i]}
            {@const isCorrect = answer?.isCorrect ?? false}
            {@const timedOut = answer?.selectedLetter === null && !isCorrect}

            <div class="relative bg-slate-900/60 backdrop-blur border rounded-2xl px-5 py-4 overflow-hidden
              {isCorrect ? 'border-emerald-500/25' : 'border-red-500/25'}">
              <div class="absolute left-0 top-0 bottom-0 w-1 rounded-l-2xl {isCorrect ? 'bg-emerald-500' : 'bg-red-500'}"></div>

              <div class="pl-2">
                <!-- Header row -->
                <div class="flex items-center justify-between gap-3 mb-3">
                  <div class="flex items-center gap-2">
                    <span class="shrink-0 w-7 h-7 rounded-lg flex items-center justify-center text-xs font-bold
                      {isCorrect ? 'bg-emerald-500/20 text-emerald-400' : 'bg-red-500/20 text-red-400'}">
                      {isCorrect ? '✓' : '✗'}
                    </span>
                    <span class="text-xs text-slate-500 font-medium">Soal {i + 1}</span>
                  </div>
                  {#if timedOut}
                    <span class="text-xs px-2 py-0.5 bg-amber-500/15 text-amber-400 rounded-lg border border-amber-500/20 font-medium">⏰ Waktu Habis</span>
                  {:else}
                    <span class="text-xs px-2 py-0.5 rounded-lg border font-medium
                      {isCorrect ? 'bg-emerald-500/15 text-emerald-400 border-emerald-500/20' : 'bg-red-500/15 text-red-400 border-red-500/20'}">
                      {isCorrect ? '+10 poin' : '0 poin'}
                    </span>
                  {/if}
                </div>

                <p class="text-slate-200 font-semibold text-sm mb-3 leading-snug">{q.question_text}</p>

                <div class="flex flex-col gap-1.5">
                  <!-- Correct answer -->
                  <div class="flex items-center gap-2 px-3 py-2 rounded-xl bg-emerald-500/10 border border-emerald-500/25">
                    <span class="shrink-0 text-xs font-bold text-emerald-400 w-20">Jawaban Benar</span>
                    <span class="w-px h-4 bg-emerald-500/30"></span>
                    <span class="text-emerald-300 text-sm">{q.correct_option}. {getOptionText(q, q.correct_option)}</span>
                  </div>

                  <!-- User answer -->
                  {#if timedOut}
                    <div class="flex items-center gap-2 px-3 py-2 rounded-xl bg-amber-500/10 border border-amber-500/25">
                      <span class="shrink-0 text-xs font-bold text-amber-400 w-20">Jawabanmu</span>
                      <span class="w-px h-4 bg-amber-500/30"></span>
                      <span class="text-amber-400 text-sm italic">Tidak menjawab</span>
                    </div>
                  {:else if answer?.selectedLetter}
                    <div class="flex items-center gap-2 px-3 py-2 rounded-xl
                      {isCorrect ? 'bg-emerald-500/8 border border-emerald-500/20' : 'bg-red-500/10 border border-red-500/25'}">
                      <span class="shrink-0 text-xs font-bold w-20 {isCorrect ? 'text-emerald-400' : 'text-red-400'}">Jawabanmu</span>
                      <span class="w-px h-4 {isCorrect ? 'bg-emerald-500/30' : 'bg-red-500/30'}"></span>
                      <span class="text-sm {isCorrect ? 'text-emerald-300' : 'text-red-300'}">
                        {answer.selectedLetter}. {getOptionText(q, answer.selectedLetter)}
                      </span>
                    </div>
                  {/if}
                </div>
              </div>
            </div>
          {/each}
        </div>

        <div class="mt-8 flex justify-center gap-3">
          <button
            onclick={retryQuiz}
            class="px-10 py-4 bg-gradient-to-r from-cyan-500 to-violet-500 hover:from-cyan-400 hover:to-violet-400 text-white font-bold rounded-2xl shadow-lg shadow-cyan-500/25 transition-all hover:-translate-y-1 active:scale-95"
          >
            🔄 Main Lagi
          </button>
          <button
            onclick={backToList}
            class="px-10 py-4 bg-slate-800 hover:bg-slate-700 text-slate-300 hover:text-white font-bold rounded-2xl border border-slate-700 transition-all"
          >
            📋 Pilih Quiz Lain
          </button>
        </div>
      </div>

    </div>
  {/if}

</div>
