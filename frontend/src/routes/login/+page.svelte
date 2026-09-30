<script lang="ts">
  import { goto } from '$app/navigation';

  let username = $state('');
  let password = $state('');
  let errorMsg = $state('');
  let isLoading = $state(false);

  async function handleLogin() {
    isLoading = true;
    errorMsg = '';
    
    try {
      const res = await fetch('/api/login', {
        method: 'POST',
        headers: {
          'Content-Type': 'application/json'
        },
        body: JSON.stringify({ username, password }),
        credentials: 'include' // Important for setting the HttpOnly cookie
      });

      if (res.ok) {
        goto('/'); // Redirect to dashboard
      } else {
        const text = await res.text();
        errorMsg = text || 'Login failed. Please check your credentials.';
      }
    } catch (err) {
      errorMsg = 'Failed to connect to the server.';
    } finally {
      isLoading = false;
    }
  }
</script>

<svelte:head>
  <title>Login - Quiz App</title>
</svelte:head>

<div class="min-h-screen flex items-center justify-center p-4">
  <div class="w-full max-w-md">
    <div class="relative bg-slate-900/50 backdrop-blur-xl border border-slate-800 rounded-2xl shadow-2xl p-8 overflow-hidden">
      
      <!-- Decorative glow -->
      <div class="absolute -top-24 -right-24 w-48 h-48 bg-cyan-500/20 rounded-full blur-3xl"></div>
      <div class="absolute -bottom-24 -left-24 w-48 h-48 bg-violet-500/20 rounded-full blur-3xl"></div>

      <div class="relative z-10">
        <div class="text-center mb-8">
          <h1 class="text-3xl font-bold bg-clip-text text-transparent bg-gradient-to-r from-cyan-400 to-violet-400">
            Welcome Back
          </h1>
          <p class="text-slate-400 mt-2">Sign in to continue to your account</p>
        </div>

        {#if errorMsg}
          <div class="mb-6 p-4 rounded-lg bg-red-500/10 border border-red-500/20 text-red-400 text-sm">
            {errorMsg}
          </div>
        {/if}

        <form on:submit|preventDefault={handleLogin} class="space-y-5">
          <div>
            <label for="username" class="block text-sm font-medium text-slate-300 mb-1">Username</label>
            <input 
              type="text" 
              id="username" 
              bind:value={username}
              required
              class="w-full px-4 py-3 bg-slate-950/50 border border-slate-700 rounded-xl focus:ring-2 focus:ring-cyan-500/50 focus:border-cyan-500 transition-all outline-none text-slate-100 placeholder-slate-600"
              placeholder="Enter your username"
            />
          </div>

          <div>
            <label for="password" class="block text-sm font-medium text-slate-300 mb-1">Password</label>
            <input 
              type="password" 
              id="password" 
              bind:value={password}
              required
              class="w-full px-4 py-3 bg-slate-950/50 border border-slate-700 rounded-xl focus:ring-2 focus:ring-cyan-500/50 focus:border-cyan-500 transition-all outline-none text-slate-100 placeholder-slate-600"
              placeholder="••••••••"
            />
          </div>

          <button 
            type="submit" 
            disabled={isLoading}
            class="w-full py-3 px-4 bg-gradient-to-r from-cyan-500 to-violet-500 hover:from-cyan-400 hover:to-violet-400 text-white font-semibold rounded-xl shadow-lg shadow-cyan-500/25 transition-all transform hover:-translate-y-0.5 active:translate-y-0 disabled:opacity-70 disabled:cursor-not-allowed disabled:transform-none flex justify-center items-center gap-2"
          >
            {#if isLoading}
              <svg class="animate-spin h-5 w-5 text-white" xmlns="http://www.w3.org/2000/svg" fill="none" viewBox="0 0 24 24">
                <circle class="opacity-25" cx="12" cy="12" r="10" stroke="currentColor" stroke-width="4"></circle>
                <path class="opacity-75" fill="currentColor" d="M4 12a8 8 0 018-8V0C5.373 0 0 5.373 0 12h4zm2 5.291A7.962 7.962 0 014 12H0c0 3.042 1.135 5.824 3 7.938l3-2.647z"></path>
              </svg>
              Signing in...
            {:else}
              Sign In
            {/if}
          </button>
        </form>

        <p class="mt-8 text-center text-sm text-slate-400">
          Don't have an account? 
          <a href="/register" class="text-cyan-400 hover:text-cyan-300 font-medium transition-colors">Create one</a>
        </p>
      </div>
    </div>
  </div>
</div>
