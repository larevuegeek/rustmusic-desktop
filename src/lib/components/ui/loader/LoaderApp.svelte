<script lang="ts">
  import LogoRustMusic from "$lib/components/ui/logo/LogoRustMusic.svelte";
</script>

<div class="fixed inset-0 flex items-center justify-center
            bg-linear-to-br from-zinc-100 to-zinc-200
            dark:from-zinc-950 dark:to-black
            overflow-hidden">

  <!-- Halo background subtle -->
  <div class="absolute w-150 h-150
              bg-linear-to-r from-emerald-500/15 via-green-500/10 to-emerald-500/15
              blur-3xl rounded-full animate-pulse">
  </div>

  <!-- Glass card -->
  <div class="relative z-10
              flex flex-col items-center gap-6
              px-10 py-8 rounded-2xl
              backdrop-blur-xl
              bg-white/60 dark:bg-zinc-900/60
              border border-black/10 dark:border-white/10
              shadow-[0_20px_60px_-20px_rgba(0,0,0,0.6)]">

    <!-- Logo -->
    <LogoRustMusic width={123} />

    <!-- Animated loader -->
    <div class="relative w-12 h-12">
      <div class="absolute inset-0 rounded-full border-3
                  border-emerald-500/20"></div>
      <div class="absolute inset-0 rounded-full border-3
                  border-t-emerald-500 border-r-transparent border-b-transparent border-l-transparent
                  animate-spin"></div>
    </div>

    <!-- Loading text -->
    <span class="text-sm text-neutral-500 dark:text-neutral-400">
      Chargement…
    </span>
  </div>
</div>
