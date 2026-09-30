<script>
  import '../app.css';
</script>

<div class="min-h-screen bg-slate-950 text-slate-100 font-sans selection:bg-cyan-500/30">
  <div class="absolute inset-0 bg-[radial-gradient(ellipse_at_top_right,_var(--tw-gradient-stops))] from-violet-900/20 via-slate-950 to-slate-950 pointer-events-none"></div>
  <main class="relative z-10">
    <slot />
  </main>
</div>
