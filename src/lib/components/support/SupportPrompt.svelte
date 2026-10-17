<script lang="ts">
  import Icon from "@iconify/svelte";
  import { onMount } from "svelte";
  import { fly } from "svelte/transition";
  import { t, currentLocale } from "#lib/i18n";
  import { updaterState } from "#lib/stores/updater/updater.store";
  import { supportPageUrl } from "#lib/config/support";
  import { messageSoutienAttendu, messageSoutienVu, messageSoutienJamais } from "#lib/helper/support/supportPrompt";

  // ─── Merci après une mise à jour ─────────────────────────────────────────
  // Carte discrète en bas à droite, comme la bannière de mise à jour : jamais
  // en même temps qu'elle, et pas tout de suite au lancement.
  const version = __APP_VERSION__;
  let attendu = $state(false);

  onMount(() => {
    if (!messageSoutienAttendu(version)) return;
    const minuterie = setTimeout(() => (attendu = true), 8000);
    return () => clearTimeout(minuterie);
  });

  const visible = $derived(attendu && ($updaterState.kind === "idle" || $updaterState.kind === "checking"));

  function fermer(jamais = false) {
    attendu = false;
    if (jamais) messageSoutienJamais(version);
    else messageSoutienVu(version);
  }
</script>

{#if visible}
  <div
    class="fixed bottom-38 right-4 z-50 max-w-sm w-[calc(100%-2rem)]
           rounded-xl shadow-2xl border backdrop-blur-xl
           bg-neutral-900/90 text-neutral-100 border-white/10 p-4"
    role="status"
    aria-live="polite"
    transition:fly={{ y: 16, duration: 220 }}
  >
    <div class="flex items-start gap-3">
      <span class="shrink-0 mt-0.5 flex items-center justify-center w-9 h-9 rounded-lg bg-amber-500/15 border border-amber-500/25">
        <Icon icon="lucide:heart" width={18} class="text-amber-400" />
      </span>
      <div class="flex-1 min-w-0">
        <p class="text-sm font-semibold text-white">{$t("support.prompt_title").replace("{version}", version)}</p>
        <p class="text-xs text-neutral-400 mt-1 leading-relaxed">{$t("support.prompt_text")}</p>
        <div class="flex flex-wrap items-center gap-2 mt-3">
          <a
            href={supportPageUrl($currentLocale)}
            target="_blank"
            rel="noopener noreferrer"
            onclick={() => fermer()}
            class="inline-flex items-center gap-1.5 px-3 py-1.5 rounded-lg bg-amber-500 hover:bg-amber-400 text-neutral-950 text-xs font-semibold transition-colors"
          >
            <Icon icon="lucide:heart-handshake" width={14} />
            {$t("support.prompt_cta")}
          </a>
          <button
            type="button"
            onclick={() => fermer()}
            class="px-3 py-1.5 rounded-lg text-xs font-medium text-neutral-300 hover:text-white hover:bg-white/8 transition-colors"
          >
            {$t("support.prompt_later")}
          </button>
        </div>
        <button
          type="button"
          onclick={() => fermer(true)}
          class="mt-2 text-[11px] text-neutral-500 hover:text-neutral-300 underline underline-offset-2 transition-colors"
        >
          {$t("support.prompt_never")}
        </button>
      </div>
      <button
        type="button"
        onclick={() => fermer()}
        aria-label={$t("support.prompt_later")}
        class="shrink-0 p-1 rounded-md text-neutral-500 hover:text-white hover:bg-white/8 transition-colors"
      >
        <Icon icon="lucide:x" width={14} />
      </button>
    </div>
  </div>
{/if}
