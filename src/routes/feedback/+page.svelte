<script lang="ts">
  import Icon from "@iconify/svelte";
  import { t } from "$lib/i18n";
</script>

<div class="py-6 px-4 md:px-10 scrollbar-app overflow-y-auto" style="height: calc(100vh - 290px);">

  <!-- Header -->
  <div class="flex items-center gap-3 mb-10">
    <button
      class="w-8 h-8 flex items-center justify-center rounded-lg cursor-pointer
             text-neutral-500 dark:text-neutral-400 hover:text-neutral-700 dark:hover:text-neutral-200
             hover:bg-neutral-100 dark:hover:bg-neutral-800
             transition-all"
      onclick={() => history.back()}
    >
      <Icon icon="lucide:arrow-left" width="16" />
    </button>
    <div>
      <h1 class="text-2xl font-bold tracking-tight text-neutral-900 dark:text-neutral-100">
        {$t('feedback.title')}
      </h1>
      <div class="mt-1 h-0.5 w-10 rounded-full bg-green-500"></div>
    </div>
  </div>

  <div class="space-y-10">

    <!-- Hero card -->
    <div class="relative rounded-3xl overflow-hidden
                border border-neutral-200/70 dark:border-white/6">
      <!-- Gradient background -->
      <div class="absolute inset-0 bg-linear-to-br from-green-500/8 via-neutral-950 to-neutral-950"></div>
      <div class="absolute top-0 right-0 w-96 h-96 rounded-full blur-[120px] opacity-15 pointer-events-none bg-green-500"></div>
      <div class="absolute bottom-0 left-0 w-64 h-64 rounded-full blur-[100px] opacity-8 pointer-events-none bg-emerald-400"></div>

      <div class="relative px-8 py-10 md:px-12 md:py-14">
        <div class="flex items-start gap-5">
          <div class="w-14 h-14 rounded-2xl bg-green-500/10 border border-green-500/15
                      flex items-center justify-center shrink-0">
            <Icon icon="lucide:message-square-heart" width="24" class="text-green-400" />
          </div>
          <div class="space-y-3">
            <h2 class="text-2xl font-bold text-neutral-900 dark:text-white tracking-tight">
              {$t('feedback.intro_title')}
            </h2>
            <p class="text-sm text-neutral-500 dark:text-neutral-400 dark:text-white/40 leading-relaxed max-w-2xl">
              {$t('feedback.intro_desc')}
            </p>
          </div>
        </div>
      </div>
    </div>

    <!-- Contact section -->
    <div class="space-y-5">
      <h3 class="text-[11px] font-semibold uppercase tracking-widest text-neutral-500 dark:text-neutral-400 dark:text-neutral-500">
        {$t('feedback.contact')}
      </h3>

      <a
        href="mailto:contact@rustmusic.dev"
        class="group relative flex items-center gap-5 p-5 rounded-2xl
               bg-neutral-50 dark:bg-white/2 border border-neutral-200/70 dark:border-white/6
               hover:border-green-500/20 hover:bg-green-500/3
               transition-all duration-300"
      >
        <!-- Glow on hover -->
        <div class="absolute inset-0 rounded-2xl opacity-0 group-hover:opacity-100
                    transition-opacity duration-500 pointer-events-none"
             style="background: radial-gradient(ellipse at 30% 50%, rgba(34,197,94,0.06) 0%, transparent 70%);">
        </div>

        <div class="relative w-12 h-12 rounded-xl bg-green-500/8 border border-green-500/10
                    flex items-center justify-center shrink-0
                    group-hover:bg-green-500/12 group-hover:border-green-500/20
                    transition-all duration-300">
          <Icon icon="lucide:mail" width="20" class="text-green-500" />
        </div>

        <div class="relative min-w-0 flex-1">
          <p class="text-[15px] font-semibold text-neutral-800 dark:text-neutral-100
                    group-hover:text-green-400 transition-colors duration-200">
            contact@rustmusic.dev
          </p>
          <p class="text-xs text-neutral-500 dark:text-neutral-400 dark:text-neutral-500 mt-1">
            {$t('feedback.email_desc')}
          </p>
        </div>

        <div class="relative w-9 h-9 rounded-full flex items-center justify-center
                    bg-neutral-50 dark:bg-white/3 border border-neutral-200/70 dark:border-white/5
                    group-hover:bg-green-500/10 group-hover:border-green-500/15
                    transition-all duration-300">
          <Icon icon="lucide:arrow-right" width="15"
                class="text-neutral-500 dark:text-neutral-400 group-hover:text-green-400
                       group-hover:translate-x-0.5 transition-all duration-200" />
        </div>
      </a>
    </div>

  </div>
</div>
