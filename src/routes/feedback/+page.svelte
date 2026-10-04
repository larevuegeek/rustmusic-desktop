<script lang="ts">
  import Icon from "@iconify/svelte";
  import { t } from "$lib/i18n";
</script>

<div class="h-full overflow-y-auto scrollbar-app py-6 px-4 md:px-10">
  <div class="max-w-4xl space-y-10">

    <div class="flex items-center gap-3">
      <button type="button" class="w-9 h-9 flex items-center justify-center rounded-[10px] cursor-pointer transition-colors text-(--rg-mu) hover:text-(--rg-tx) hover:bg-(--rg-s2)"
              aria-label={$t("settings.back")} title={$t("settings.back")} onclick={() => history.back()}>
        <Icon icon="material-symbols:arrow-back-rounded" width="20" />
      </button>
      <h1 class="text-[28px] font-extrabold tracking-tight text-(--rg-tx)">{$t("feedback.title")}</h1>
    </div>

    <!-- Bandeau teinté de vert, lisible dans les deux thèmes. -->
    <div class="relative overflow-hidden rounded-3xl border px-8 py-10 md:px-12 md:py-12 bg-(--rg-gbg) border-(--rg-gbd)">
      <div class="absolute -top-24 -right-16 w-96 h-96 rounded-full blur-[120px] opacity-20 pointer-events-none bg-green-500"></div>
      <div class="relative flex items-start gap-5">
        <div class="w-14 h-14 rounded-2xl shrink-0 flex items-center justify-center border bg-(--rg-carte) border-(--rg-gbd) text-(--rg-g)">
          <Icon icon="lucide:message-square-heart" width="24" />
        </div>
        <div class="space-y-3">
          <h2 class="text-2xl font-bold tracking-tight text-(--rg-tx)">{$t("feedback.intro_title")}</h2>
          <p class="text-sm leading-relaxed max-w-2xl text-(--rg-tx2)">{$t("feedback.intro_desc")}</p>
        </div>
      </div>
    </div>

    <div class="space-y-4">
      <h3 class="text-[11px] font-bold uppercase tracking-[0.12em] text-(--rg-mu)">{$t("feedback.contact")}</h3>

      <!-- Ouvert dans la messagerie du système (voir liensExternes). -->
      <a href="mailto:contact@rustmusic.dev"
         class="group flex items-center gap-5 p-5 rounded-2xl border transition-colors bg-(--rg-carte) border-(--rg-bd) hover:border-(--rg-gbd) hover:bg-(--rg-hover)">
        <div class="w-12 h-12 rounded-xl shrink-0 flex items-center justify-center border bg-(--rg-gbg) border-(--rg-gbd) text-(--rg-g)">
          <Icon icon="lucide:mail" width="20" />
        </div>
        <div class="min-w-0 flex-1">
          <p class="text-[15px] font-semibold transition-colors text-(--rg-tx) group-hover:text-(--rg-gtx)">contact@rustmusic.dev</p>
          <p class="text-xs mt-1 text-(--rg-mu)">{$t("feedback.email_desc")}</p>
        </div>
        <div class="w-9 h-9 rounded-full shrink-0 flex items-center justify-center border transition-colors border-(--rg-bd) text-(--rg-mu) group-hover:border-(--rg-gbd) group-hover:text-(--rg-gtx)">
          <Icon icon="lucide:arrow-right" width="15" class="transition-transform group-hover:translate-x-0.5" />
        </div>
      </a>
    </div>

  </div>
</div>
