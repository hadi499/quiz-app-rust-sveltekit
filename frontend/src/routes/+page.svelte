<script lang="ts">
  import { onMount } from 'svelte';
  import { goto } from '$app/navigation';

  interface QuizResult {
    id: number;
    quiz_id: number;
    quiz_title: string;
    score: number;
    correct_count: number;
    total_questions: number;
    completed_at: string;
  }

  let user: any = $state(null);
  let isLoading = $state(true);
  let errorMsg = $state('');
  let isLoggingOut = $state(false);

  // Quiz results state
  let results = $state<QuizResult[]>([]);
  let resultsLoading = $state(false);

  // Computed stats
  let totalPlayed = $derived(results.length);
  let avgAccuracy = $derived(
    results.length === 0
      ? 0
      : Math.round(results.reduce((acc, r) => acc + (r.correct_count / r.total_questions) * 100, 0) / results.length)
  );
  let rank = $derived(
    totalPlayed === 0 ? 'Pemula'
    : avgAccuracy >= 90 ? 'Master'
    : avgAccuracy >= 75 ? 'Expert'
    : avgAccuracy >= 60 ? 'Intermediate'
    : avgAccuracy >= 40 ? 'Beginner'
    : 'Pemula'
  );

  // Avatar modal state
  let showAvatarModal = $state(false);
  let avatarFile: File | null = $state(null);
  let avatarPreview: string | null = $state(null);
  let isUploadingAvatar = $state(false);
  let avatarError = $state('');
  let avatarSuccess = $state('');
  let isDeletingAvatar = $state(false);
  let dragOver = $state(false);

  onMount(async () => {
    try {
      const res = await fetch('/api/me', {
        method: 'GET',
        credentials: 'include'
      });

      if (res.ok) {
        user = await res.json();
        loadResults();
      } else {
        goto('/login');
      }
    } catch (err) {
      errorMsg = 'Failed to connect to the server.';
    } finally {
      isLoading = false;
    }
  });

  async function loadResults() {
    resultsLoading = true;
    try {
      const res = await fetch('/api/results/me', { credentials: 'include' });
      if (res.ok) results = await res.json();
    } finally {
      resultsLoading = false;
    }
  }

  async function handleLogout() {
    isLoggingOut = true;
    try {
      await fetch('/api/logout', {
        method: 'POST',
        credentials: 'include'
      });
      goto('/login');
    } catch (err) {
      alert('Failed to logout properly.');
      isLoggingOut = false;
    }
  }

  // Avatar helpers
  function getInitials(username: string): string {
    return username ? username.slice(0, 2).toUpperCase() : '??';
  }

  function openAvatarModal() {
    showAvatarModal = true;
    avatarFile = null;
    avatarPreview = null;
    avatarError = '';
    avatarSuccess = '';
  }

  function closeAvatarModal() {
    showAvatarModal = false;
    avatarFile = null;
    avatarPreview = null;
    avatarError = '';
    avatarSuccess = '';
  }

  function handleFileSelect(event: Event) {
    const input = event.target as HTMLInputElement;
    if (input.files && input.files[0]) {
      processFile(input.files[0]);
    }
  }

  function handleDrop(event: DragEvent) {
    event.preventDefault();
    dragOver = false;
    const file = event.dataTransfer?.files[0];
    if (file) processFile(file);
  }

  function handleDragOver(event: DragEvent) {
    event.preventDefault();
    dragOver = true;
  }

  function handleDragLeave() {
    dragOver = false;
  }

  function processFile(file: File) {
    avatarError = '';
    const validTypes = ['image/jpeg', 'image/jpg', 'image/png', 'image/webp', 'image/gif'];
    if (!validTypes.includes(file.type)) {
      avatarError = 'Invalid file type. Please use JPEG, PNG, WebP, or GIF.';
      return;
    }
    if (file.size > 5 * 1024 * 1024) {
      avatarError = 'File is too large. Maximum size is 5 MB.';
      return;
    }
    avatarFile = file;
    const reader = new FileReader();
    reader.onload = (e) => {
      avatarPreview = e.target?.result as string;
    };
    reader.readAsDataURL(file);
  }

  async function handleAvatarUpload() {
    if (!avatarFile) return;

    isUploadingAvatar = true;
    avatarError = '';
    avatarSuccess = '';

    try {
      const formData = new FormData();
      formData.append('avatar', avatarFile);

      const res = await fetch('/api/avatar', {
        method: 'POST',
        body: formData,
        credentials: 'include'
      });

      if (res.ok) {
        const updatedUser = await res.json();
        user = updatedUser;
        avatarSuccess = 'Avatar updated successfully!';
        // Close modal after short delay
        setTimeout(() => {
          closeAvatarModal();
        }, 1200);
      } else {
        const text = await res.text();
        avatarError = text || 'Failed to upload avatar.';
      }
    } catch (err) {
      avatarError = 'Failed to connect to the server.';
    } finally {
      isUploadingAvatar = false;
    }
  }

  async function handleDeleteAvatar() {
    isDeletingAvatar = true;
    avatarError = '';
    try {
      const res = await fetch('/api/avatar', {
        method: 'DELETE',
        credentials: 'include'
      });
      if (res.ok) {
        const updatedUser = await res.json();
        user = updatedUser;
        closeAvatarModal();
      } else {
        const text = await res.text();
        avatarError = text || 'Failed to delete avatar.';
      }
    } catch (err) {
      avatarError = 'Failed to connect to the server.';
    } finally {
      isDeletingAvatar = false;
    }
  }
</script>

<svelte:head>
  <title>Dashboard - Quiz App</title>
</svelte:head>

<!-- Avatar Change Modal -->
{#if showAvatarModal}
  <!-- svelte-ignore a11y_click_events_have_key_events -->
  <!-- svelte-ignore a11y_no_static_element_interactions -->
  <div
    class="fixed inset-0 z-50 flex items-center justify-center p-4 bg-slate-950/80 backdrop-blur-sm"
    on:click|self={closeAvatarModal}
  >
    <div class="relative w-full max-w-md bg-slate-900 border border-slate-700 rounded-3xl shadow-2xl overflow-hidden">
      <!-- Modal glow accents -->
      <div class="absolute -top-16 -right-16 w-40 h-40 bg-cyan-500/15 rounded-full blur-3xl pointer-events-none"></div>
      <div class="absolute -bottom-16 -left-16 w-40 h-40 bg-violet-500/15 rounded-full blur-3xl pointer-events-none"></div>

      <div class="relative z-10 p-8">
        <!-- Header -->
        <div class="flex items-center justify-between mb-6">
          <h2 class="text-xl font-bold text-slate-100">Change Avatar</h2>
          <button
            on:click={closeAvatarModal}
            class="w-8 h-8 flex items-center justify-center rounded-full bg-slate-800 hover:bg-slate-700 text-slate-400 hover:text-slate-100 transition-colors"
          >
            <svg class="w-4 h-4" fill="none" stroke="currentColor" viewBox="0 0 24 24">
              <path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M6 18L18 6M6 6l12 12" />
            </svg>
          </button>
        </div>

        <!-- Current Avatar Preview -->
        <div class="flex justify-center mb-6">
          <div class="relative group">
            {#if avatarPreview}
              <img
                src={avatarPreview}
                alt="Avatar preview"
                class="w-24 h-24 rounded-2xl object-cover ring-2 ring-cyan-500/50 shadow-lg shadow-cyan-500/20"
              />
            {:else if user?.avatar_url}
              <img
                src={user.avatar_url}
                alt="Current avatar"
                class="w-24 h-24 rounded-2xl object-cover ring-2 ring-violet-500/50 shadow-lg"
              />
            {:else}
              <div class="w-24 h-24 rounded-2xl bg-gradient-to-br from-cyan-500 to-violet-500 flex items-center justify-center shadow-lg shadow-cyan-500/20">
                <span class="text-3xl font-bold text-white">{getInitials(user?.username ?? '')}</span>
              </div>
            {/if}
            {#if avatarPreview}
              <div class="absolute -top-2 -right-2 w-6 h-6 bg-cyan-500 rounded-full flex items-center justify-center shadow-lg">
                <svg class="w-3 h-3 text-white" fill="none" stroke="currentColor" viewBox="0 0 24 24">
                  <path stroke-linecap="round" stroke-linejoin="round" stroke-width="3" d="M5 13l4 4L19 7" />
                </svg>
              </div>
            {/if}
          </div>
        </div>

        <!-- Drop Zone -->
        <!-- svelte-ignore a11y_click_events_have_key_events -->
        <!-- svelte-ignore a11y_no_static_element_interactions -->
        <label
          class="block w-full cursor-pointer"
          on:drop={handleDrop}
          on:dragover={handleDragOver}
          on:dragleave={handleDragLeave}
        >
          <div
            class="w-full border-2 border-dashed rounded-2xl p-6 text-center transition-all duration-200
              {dragOver
                ? 'border-cyan-500 bg-cyan-500/10 scale-[1.01]'
                : 'border-slate-700 hover:border-slate-600 hover:bg-slate-800/50'}"
          >
            <div class="flex flex-col items-center gap-3">
              <div class="w-12 h-12 rounded-xl bg-slate-800 flex items-center justify-center">
                <svg class="w-6 h-6 text-slate-400" fill="none" stroke="currentColor" viewBox="0 0 24 24">
                  <path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M4 16l4.586-4.586a2 2 0 012.828 0L16 16m-2-2l1.586-1.586a2 2 0 012.828 0L20 14m-6-6h.01M6 20h12a2 2 0 002-2V6a2 2 0 00-2-2H6a2 2 0 00-2 2v12a2 2 0 002 2z" />
                </svg>
              </div>
              {#if avatarFile}
                <p class="text-sm font-medium text-cyan-400">{avatarFile.name}</p>
                <p class="text-xs text-slate-500">{(avatarFile.size / 1024).toFixed(1)} KB · Click to change</p>
              {:else}
                <p class="text-sm font-medium text-slate-300">Drop image here or <span class="text-cyan-400">browse</span></p>
                <p class="text-xs text-slate-500">JPEG, PNG, WebP, GIF · Max 5 MB</p>
              {/if}
            </div>
            <input
              id="avatar-file-input"
              type="file"
              accept="image/jpeg,image/jpg,image/png,image/webp,image/gif"
              class="hidden"
              on:change={handleFileSelect}
            />
          </div>
        </label>

        <!-- Error / Success messages -->
        {#if avatarError}
          <div class="mt-4 p-3 rounded-xl bg-red-500/10 border border-red-500/20 text-red-400 text-sm">
            {avatarError}
          </div>
        {/if}
        {#if avatarSuccess}
          <div class="mt-4 p-3 rounded-xl bg-emerald-500/10 border border-emerald-500/20 text-emerald-400 text-sm flex items-center gap-2">
            <svg class="w-4 h-4 shrink-0" fill="none" stroke="currentColor" viewBox="0 0 24 24">
              <path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M5 13l4 4L19 7" />
            </svg>
            {avatarSuccess}
          </div>
        {/if}

        <!-- Actions -->
        <div class="mt-6 flex flex-col gap-3">
          <button
            on:click={handleAvatarUpload}
            disabled={!avatarFile || isUploadingAvatar}
            class="w-full py-3 px-4 bg-gradient-to-r from-cyan-500 to-violet-500 hover:from-cyan-400 hover:to-violet-400 text-white font-semibold rounded-xl shadow-lg shadow-cyan-500/25 transition-all transform hover:-translate-y-0.5 disabled:opacity-50 disabled:cursor-not-allowed disabled:transform-none flex justify-center items-center gap-2"
          >
            {#if isUploadingAvatar}
              <svg class="animate-spin h-4 w-4" xmlns="http://www.w3.org/2000/svg" fill="none" viewBox="0 0 24 24">
                <circle class="opacity-25" cx="12" cy="12" r="10" stroke="currentColor" stroke-width="4"></circle>
                <path class="opacity-75" fill="currentColor" d="M4 12a8 8 0 018-8V0C5.373 0 0 5.373 0 12h4zm2 5.291A7.962 7.962 0 014 12H0c0 3.042 1.135 5.824 3 7.938l3-2.647z"></path>
              </svg>
              Uploading...
            {:else}
              <svg class="w-4 h-4" fill="none" stroke="currentColor" viewBox="0 0 24 24">
                <path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M4 16v1a3 3 0 003 3h10a3 3 0 003-3v-1m-4-8l-4-4m0 0L8 8m4-4v12" />
              </svg>
              Upload Avatar
            {/if}
          </button>

          {#if user?.avatar_url}
            <button
              on:click={handleDeleteAvatar}
              disabled={isDeletingAvatar}
              class="w-full py-2.5 px-4 bg-red-500/10 hover:bg-red-500/20 text-red-400 hover:text-red-300 font-medium rounded-xl border border-red-500/20 hover:border-red-500/40 transition-all disabled:opacity-50 disabled:cursor-not-allowed flex justify-center items-center gap-2"
            >
              {#if isDeletingAvatar}
                <svg class="animate-spin h-4 w-4" xmlns="http://www.w3.org/2000/svg" fill="none" viewBox="0 0 24 24">
                  <circle class="opacity-25" cx="12" cy="12" r="10" stroke="currentColor" stroke-width="4"></circle>
                  <path class="opacity-75" fill="currentColor" d="M4 12a8 8 0 018-8V0C5.373 0 0 5.373 0 12h4zm2 5.291A7.962 7.962 0 014 12H0c0 3.042 1.135 5.824 3 7.938l3-2.647z"></path>
                </svg>
                Removing...
              {:else}
                <svg class="w-4 h-4" fill="none" stroke="currentColor" viewBox="0 0 24 24">
                  <path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M19 7l-.867 12.142A2 2 0 0116.138 21H7.862a2 2 0 01-1.995-1.858L5 7m5 4v6m4-6v6m1-10V4a1 1 0 00-1-1h-4a1 1 0 00-1 1v3M4 7h16" />
                </svg>
                Remove Avatar
              {/if}
            </button>
          {/if}
        </div>
      </div>
    </div>
  </div>
{/if}

<div class="min-h-screen p-6 md:p-12">
  {#if isLoading}
    <div class="flex h-[80vh] items-center justify-center">
      <div class="flex flex-col items-center gap-4">
        <svg class="animate-spin h-10 w-10 text-cyan-500" xmlns="http://www.w3.org/2000/svg" fill="none" viewBox="0 0 24 24">
          <circle class="opacity-25" cx="12" cy="12" r="10" stroke="currentColor" stroke-width="4"></circle>
          <path class="opacity-75" fill="currentColor" d="M4 12a8 8 0 018-8V0C5.373 0 0 5.373 0 12h4zm2 5.291A7.962 7.962 0 014 12H0c0 3.042 1.135 5.824 3 7.938l3-2.647z"></path>
        </svg>
        <p class="text-slate-400 font-medium animate-pulse">Loading your dashboard...</p>
      </div>
    </div>
  {:else if errorMsg}
    <div class="max-w-2xl mx-auto mt-20 p-6 bg-red-500/10 border border-red-500/20 rounded-2xl text-center">
      <p class="text-red-400 mb-4">{errorMsg}</p>
      <button on:click={() => window.location.reload()} class="px-6 py-2 bg-red-500/20 hover:bg-red-500/30 text-red-300 rounded-lg transition-colors">
        Try Again
      </button>
    </div>
  {:else if user}
    <div class="max-w-5xl mx-auto">
      
      <!-- Navbar -->
      <nav class="flex items-center justify-between mb-12 pb-6 border-b border-slate-800">
        <div class="flex items-center gap-3">
          <div class="w-10 h-10 rounded-xl bg-gradient-to-br from-cyan-500 to-violet-500 flex items-center justify-center shadow-lg shadow-cyan-500/20">
            <span class="font-bold text-white text-xl">Q</span>
          </div>
          <h1 class="text-xl font-bold tracking-tight">QuizApp</h1>
        </div>

        <div class="flex items-center gap-3">
          <!-- Avatar button in navbar -->
          <button
            on:click={openAvatarModal}
            id="navbar-avatar-btn"
            title="Change avatar"
            class="relative group focus:outline-none"
          >
            {#if user.avatar_url}
              <img
                src={user.avatar_url}
                alt="Your avatar"
                class="w-10 h-10 rounded-xl object-cover ring-2 ring-transparent group-hover:ring-cyan-500/70 transition-all shadow-md"
              />
            {:else}
              <div class="w-10 h-10 rounded-xl bg-gradient-to-br from-cyan-500/70 to-violet-500/70 flex items-center justify-center group-hover:from-cyan-500 group-hover:to-violet-500 transition-all shadow-md">
                <span class="font-bold text-white text-sm">{getInitials(user.username)}</span>
              </div>
            {/if}
            <!-- Edit badge -->
            <div class="absolute -bottom-1 -right-1 w-4 h-4 bg-slate-800 rounded-full border border-slate-700 flex items-center justify-center opacity-0 group-hover:opacity-100 transition-opacity">
              <svg class="w-2.5 h-2.5 text-cyan-400" fill="none" stroke="currentColor" viewBox="0 0 24 24">
                <path stroke-linecap="round" stroke-linejoin="round" stroke-width="2.5" d="M15.232 5.232l3.536 3.536m-2.036-5.036a2.5 2.5 0 113.536 3.536L6.5 21.036H3v-3.572L16.732 3.732z" />
              </svg>
            </div>
          </button>

          <button 
            on:click={handleLogout} 
            disabled={isLoggingOut}
            class="px-5 py-2.5 bg-slate-800 hover:bg-slate-700 text-slate-300 hover:text-white rounded-xl transition-all font-medium border border-slate-700 hover:border-slate-600 disabled:opacity-50 flex items-center gap-2"
          >
            {#if isLoggingOut}
              <svg class="animate-spin h-4 w-4" xmlns="http://www.w3.org/2000/svg" fill="none" viewBox="0 0 24 24">
                <circle class="opacity-25" cx="12" cy="12" r="10" stroke="currentColor" stroke-width="4"></circle>
                <path class="opacity-75" fill="currentColor" d="M4 12a8 8 0 018-8V0C5.373 0 0 5.373 0 12h4zm2 5.291A7.962 7.962 0 014 12H0c0 3.042 1.135 5.824 3 7.938l3-2.647z"></path>
              </svg>
              Logging out...
            {:else}
              Sign Out
            {/if}
          </button>
        </div>
      </nav>

      <!-- Welcome Banner -->
      <div class="relative bg-slate-900/50 backdrop-blur-xl border border-slate-800 rounded-3xl p-8 md:p-12 overflow-hidden shadow-2xl mb-8">
        <div class="absolute top-0 right-0 w-64 h-64 bg-cyan-500/10 rounded-full blur-3xl -translate-y-1/2 translate-x-1/3"></div>
        <div class="absolute bottom-0 left-0 w-64 h-64 bg-violet-500/10 rounded-full blur-3xl translate-y-1/2 -translate-x-1/3"></div>
        
        <div class="relative z-10 flex flex-col md:flex-row gap-8 items-center justify-between">
          <div class="flex items-center gap-6">
            <!-- Large Avatar in banner -->
            <button
              on:click={openAvatarModal}
              id="banner-avatar-btn"
              title="Change avatar"
              class="relative group shrink-0 focus:outline-none"
            >
              {#if user.avatar_url}
                <img
                  src={user.avatar_url}
                  alt="Your avatar"
                  class="w-20 h-20 rounded-2xl object-cover ring-2 ring-white/10 group-hover:ring-cyan-500/60 transition-all shadow-xl"
                />
              {:else}
                <div class="w-20 h-20 rounded-2xl bg-gradient-to-br from-cyan-500 to-violet-500 flex items-center justify-center shadow-xl shadow-cyan-500/20 group-hover:scale-105 transition-transform">
                  <span class="text-3xl font-bold text-white">{getInitials(user.username)}</span>
                </div>
              {/if}
              <!-- Camera overlay on hover -->
              <div class="absolute inset-0 rounded-2xl bg-slate-900/70 flex items-center justify-center opacity-0 group-hover:opacity-100 transition-opacity">
                <svg class="w-7 h-7 text-white" fill="none" stroke="currentColor" viewBox="0 0 24 24">
                  <path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M3 9a2 2 0 012-2h.93a2 2 0 001.664-.89l.812-1.22A2 2 0 0110.07 4h3.86a2 2 0 011.664.89l.812 1.22A2 2 0 0018.07 7H19a2 2 0 012 2v9a2 2 0 01-2 2H5a2 2 0 01-2-2V9z" />
                  <path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M15 13a3 3 0 11-6 0 3 3 0 016 0z" />
                </svg>
              </div>
            </button>

            <div>
              <h2 class="text-4xl md:text-5xl font-bold mb-4">
                Welcome back, <span class="bg-clip-text text-transparent bg-gradient-to-r from-cyan-400 to-violet-400">{user.username}</span>! 👋
              </h2>
              <p class="text-slate-400 text-lg mb-2">
                Ready to challenge yourself today?
              </p>
              <p class="text-slate-500 text-sm">
                Member since: {new Date(user.created_at).toLocaleDateString('en-US', { year: 'numeric', month: 'long', day: 'numeric' })}
              </p>
            </div>
          </div>
          
          <div class="shrink-0 flex gap-3">
            <a
              href="/quiz"
              class="px-8 py-4 bg-gradient-to-r from-cyan-500 to-violet-500 hover:from-cyan-400 hover:to-violet-400 text-white font-bold rounded-2xl shadow-lg shadow-cyan-500/25 transition-all transform hover:-translate-y-1"
            >
              Play a Quiz 🚀
            </a>
            {#if user.role === 'admin'}
              <a
                href="/manage"
                class="px-8 py-4 bg-slate-800 hover:bg-slate-700 text-slate-300 hover:text-white font-bold rounded-2xl border border-slate-700 hover:border-slate-600 transition-all transform hover:-translate-y-1"
              >
                ⚙️ Kelola Quiz
              </a>
            {/if}
          </div>
        </div>
      </div>

      <!-- Stats Grid -->
      <div class="grid grid-cols-1 md:grid-cols-3 gap-6 mb-10">
        <div class="bg-slate-900/40 border border-slate-800/80 rounded-2xl p-6 backdrop-blur-sm hover:bg-slate-800/50 transition-colors">
          <div class="w-12 h-12 bg-cyan-500/20 text-cyan-400 rounded-xl flex items-center justify-center mb-4">
            <svg class="w-6 h-6" fill="none" stroke="currentColor" viewBox="0 0 24 24"><path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M13 10V3L4 14h7v7l9-11h-7z"></path></svg>
          </div>
          <h3 class="text-slate-400 font-medium mb-1">Quiz Dimainkan</h3>
          <p class="text-3xl font-bold text-slate-100">{totalPlayed}</p>
        </div>

        <div class="bg-slate-900/40 border border-slate-800/80 rounded-2xl p-6 backdrop-blur-sm hover:bg-slate-800/50 transition-colors">
          <div class="w-12 h-12 bg-violet-500/20 text-violet-400 rounded-xl flex items-center justify-center mb-4">
            <svg class="w-6 h-6" fill="none" stroke="currentColor" viewBox="0 0 24 24"><path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M9 12l2 2 4-4m6 2a9 9 0 11-18 0 9 9 0 0118 0z"></path></svg>
          </div>
          <h3 class="text-slate-400 font-medium mb-1">Rata-rata Akurasi</h3>
          <p class="text-3xl font-bold text-slate-100">{avgAccuracy}%</p>
        </div>

        <div class="bg-slate-900/40 border border-slate-800/80 rounded-2xl p-6 backdrop-blur-sm hover:bg-slate-800/50 transition-colors">
          <div class="w-12 h-12 bg-pink-500/20 text-pink-400 rounded-xl flex items-center justify-center mb-4">
            <svg class="w-6 h-6" fill="none" stroke="currentColor" viewBox="0 0 24 24"><path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M5 3v4M3 5h4M6 17v4m-2-2h4m5-16l2.286 6.857L21 12l-5.714 2.143L13 21l-2.286-6.857L5 12l5.714-2.143L13 3z"></path></svg>
          </div>
          <h3 class="text-slate-400 font-medium mb-1">Rank</h3>
          <p class="text-3xl font-bold text-slate-100">{rank}</p>
        </div>
      </div>

      <!-- Quiz History -->
      <div class="bg-slate-900/40 border border-slate-800/80 rounded-2xl overflow-hidden backdrop-blur-sm">
        <div class="px-6 py-4 border-b border-slate-800 flex items-center justify-between">
          <div class="flex items-center gap-3">
            <div class="w-8 h-8 bg-violet-500/20 text-violet-400 rounded-lg flex items-center justify-center">
              <svg class="w-4 h-4" fill="none" stroke="currentColor" viewBox="0 0 24 24">
                <path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M9 5H7a2 2 0 00-2 2v12a2 2 0 002 2h10a2 2 0 002-2V7a2 2 0 00-2-2h-2M9 5a2 2 0 002 2h2a2 2 0 002-2M9 5a2 2 0 012-2h2a2 2 0 012 2"/>
              </svg>
            </div>
            <h3 class="font-bold text-slate-100">Riwayat Quiz</h3>
          </div>
          {#if resultsLoading}
            <svg class="animate-spin h-4 w-4 text-cyan-500" fill="none" viewBox="0 0 24 24">
              <circle class="opacity-25" cx="12" cy="12" r="10" stroke="currentColor" stroke-width="4"/>
              <path class="opacity-75" fill="currentColor" d="M4 12a8 8 0 018-8V0C5.373 0 0 5.373 0 12h4z"/>
            </svg>
          {/if}
        </div>

        {#if !resultsLoading && results.length === 0}
          <div class="flex flex-col items-center justify-center py-16 text-center">
            <div class="w-14 h-14 bg-slate-800 rounded-2xl flex items-center justify-center mb-4">
              <svg class="w-7 h-7 text-slate-600" fill="none" stroke="currentColor" viewBox="0 0 24 24">
                <path stroke-linecap="round" stroke-linejoin="round" stroke-width="1.5" d="M9 5H7a2 2 0 00-2 2v12a2 2 0 002 2h10a2 2 0 002-2V7a2 2 0 00-2-2h-2M9 5a2 2 0 002 2h2a2 2 0 002-2M9 5a2 2 0 012-2h2a2 2 0 012 2"/>
              </svg>
            </div>
            <p class="text-slate-500 font-medium">Belum ada riwayat quiz</p>
            <p class="text-slate-600 text-sm mt-1">Main quiz pertamamu sekarang!</p>
            <a href="/quiz" class="mt-4 px-5 py-2.5 bg-gradient-to-r from-cyan-500 to-violet-500 text-white text-sm font-bold rounded-xl shadow-lg shadow-cyan-500/20 hover:-translate-y-0.5 transition-all">
              🚀 Main Quiz
            </a>
          </div>
        {:else if results.length > 0}
          <!-- Table header -->
          <div class="hidden md:grid grid-cols-[1fr_80px_80px_100px_110px] gap-4 px-6 py-3 bg-slate-950/30 border-b border-slate-800/60 text-xs font-semibold text-slate-500 uppercase tracking-wide">
            <span>Quiz</span>
            <span class="text-center">Skor</span>
            <span class="text-center">Benar</span>
            <span class="text-center">Akurasi</span>
            <span class="text-right">Tanggal</span>
          </div>

          <div class="divide-y divide-slate-800/60">
            {#each results as r (r.id)}
              {@const accuracy = Math.round((r.correct_count / r.total_questions) * 100)}
              {@const maxScore = r.total_questions * 10}
              <div class="grid grid-cols-1 md:grid-cols-[1fr_80px_80px_100px_110px] gap-2 md:gap-4 px-6 py-4 hover:bg-slate-800/30 transition-colors items-center">
                <!-- Quiz title -->
                <div class="flex items-center gap-3">
                  <div class="shrink-0 w-9 h-9 rounded-xl flex items-center justify-center
                    {accuracy >= 80 ? 'bg-emerald-500/20 text-emerald-400'
                    : accuracy >= 60 ? 'bg-cyan-500/20 text-cyan-400'
                    : accuracy >= 40 ? 'bg-amber-500/20 text-amber-400'
                    : 'bg-red-500/20 text-red-400'}">
                    <svg class="w-4 h-4" fill="none" stroke="currentColor" viewBox="0 0 24 24">
                      <path stroke-linecap="round" stroke-linejoin="round" stroke-width="2"
                        d="{accuracy >= 80 ? 'M9 12l2 2 4-4m6 2a9 9 0 11-18 0 9 9 0 0118 0z'
                           : accuracy >= 60 ? 'M13 10V3L4 14h7v7l9-11h-7z'
                           : 'M12 9v2m0 4h.01M10.29 3.86L1.82 18a2 2 0 001.71 3h16.94a2 2 0 001.71-3L13.71 3.86a2 2 0 00-3.42 0z'}"/>
                    </svg>
                  </div>
                  <div class="min-w-0">
                    <p class="font-semibold text-slate-200 text-sm truncate">{r.quiz_title}</p>
                    <p class="text-xs text-slate-500 md:hidden">{new Date(r.completed_at).toLocaleDateString('id-ID', { day: 'numeric', month: 'short', year: 'numeric' })}</p>
                  </div>
                </div>

                <!-- Score -->
                <div class="text-center">
                  <span class="text-sm font-bold
                    {accuracy >= 80 ? 'text-emerald-400'
                    : accuracy >= 60 ? 'text-cyan-400'
                    : accuracy >= 40 ? 'text-amber-400'
                    : 'text-red-400'}">{r.score}</span>
                  <span class="text-xs text-slate-600">/{maxScore}</span>
                </div>

                <!-- Correct count -->
                <div class="text-center">
                  <span class="text-sm font-semibold text-slate-300">{r.correct_count}</span>
                  <span class="text-xs text-slate-600">/{r.total_questions}</span>
                </div>

                <!-- Accuracy bar -->
                <div class="flex items-center gap-2">
                  <div class="flex-1 h-1.5 bg-slate-800 rounded-full overflow-hidden">
                    <div class="h-full rounded-full transition-all
                      {accuracy >= 80 ? 'bg-emerald-400'
                      : accuracy >= 60 ? 'bg-cyan-400'
                      : accuracy >= 40 ? 'bg-amber-400'
                      : 'bg-red-400'}"
                      style="width: {accuracy}%"
                    ></div>
                  </div>
                  <span class="text-xs font-semibold tabular-nums
                    {accuracy >= 80 ? 'text-emerald-400'
                    : accuracy >= 60 ? 'text-cyan-400'
                    : accuracy >= 40 ? 'text-amber-400'
                    : 'text-red-400'}">{accuracy}%</span>
                </div>

                <!-- Date -->
                <div class="text-right hidden md:block">
                  <span class="text-xs text-slate-500">
                    {new Date(r.completed_at).toLocaleDateString('id-ID', { day: 'numeric', month: 'short', year: 'numeric' })}
                  </span>
                  <p class="text-xs text-slate-600">
                    {new Date(r.completed_at).toLocaleTimeString('id-ID', { hour: '2-digit', minute: '2-digit' })}
                  </p>
                </div>
              </div>
            {/each}
          </div>
        {/if}
      </div>

    </div>
  {/if}
</div>
