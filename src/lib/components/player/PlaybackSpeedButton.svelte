<script lang="ts">
  // Vitesse de lecture d'un épisode : retenue pour son émission.
  import Icon from "@iconify/svelte";
  import { t, currentLocale } from "#lib/i18n";
  import { playbackSpeed, SPEEDS } from "#lib/stores/player/playbackSpeed.store";
  import { player } from "#lib/stores/player/player.store";
  import { playerService } from "#lib/services/player/player.service";

  const MIN = 0.5, MAX = 3, STEP = 0.05;

  let ouvert = $state(false);
  let menuEl: HTMLElement | null = $state(null);

  const label = (v: number) => `${v.toLocaleString($currentLocale, { maximumFractionDigits: 2 })}×`;
  const egal = (a: number, b: number) => Math.abs(a - b) < 0.01;
  const normale = $derived(egal($playbackSpeed, 1));
  const index = $derived(SPEEDS.findIndex((v) => egal(v, $playbackSpeed)));

  // Ce qu'il reste à écouter à cette vitesse, et le temps gagné (ou perdu).
  const reste = $derived(Math.max(0, ($player?.duration ?? 0) - ($player?.jsPosition ?? 0)));
  const resteVitesse = $derived(reste / $playbackSpeed);
  const gagne = $derived(reste - resteVitesse);

  function duree(s: number): string {
    const m = Math.round(s / 60);
    if (m < 60) return `${Math.max(1, m)} min`;
    return `${Math.floor(m / 60)} h ${String(m % 60).padStart(2, "0")}`;
  }

  function choisir(v: number) {
    void playerService.setSpeed(Math.round(Math.min(MAX, Math.max(MIN, v)) * 100) / 100);
  }

  // Fermeture au clic extérieur.
  $effect(() => {
    if (!ouvert) return;
    const handleOutside = (e: MouseEvent) => {
      if (menuEl && !menuEl.contains(e.target as Node)) ouvert = false;
    };
    const timer = setTimeout(() => document.addEventListener("click", handleOutside), 50);
    return () => {
      clearTimeout(timer);
      document.removeEventListener("click", handleOutside);
    };
  });
</script>

{#snippet pas(icone: string, delta: number, titre: string)}
  {@const bloque = delta < 0 ? $playbackSpeed <= MIN + 0.001 : $playbackSpeed >= MAX - 0.001}
  <button
    type="button"
    class="w-9 h-9 shrink-0 flex items-center justify-center rounded-full border border-(--lc-menu-bd) cursor-pointer transition-colors
           text-(--lc-tx) hover:bg-(--lc-survol) disabled:opacity-35 disabled:cursor-default disabled:hover:bg-transparent"
    title={titre}
    aria-label={titre}
    disabled={bloque}
    onclick={() => choisir($playbackSpeed + delta)}
  >
    <Icon icon={icone} width="18" />
  </button>
{/snippet}

<div class="relative" bind:this={menuEl}>
  <button
    type="button"
    class="h-9.5 min-w-9.5 px-2 shrink-0 flex items-center justify-center rounded-full cursor-pointer transition-colors
           font-mono text-[12.5px] font-bold tabular-nums hover:bg-(--lc-survol)
           {normale ? 'text-(--lc-ic) hover:text-(--lc-ic-fort)' : 'text-(--lc-acc)'} {ouvert ? 'bg-(--lc-survol)' : ''}"
    title={$t("podcasts.speed")}
    aria-label="{$t('podcasts.speed')} : {label($playbackSpeed)}"
    aria-expanded={ouvert}
    onclick={() => (ouvert = !ouvert)}
  >
    {label($playbackSpeed)}
  </button>

  {#if ouvert}
    <div
      class="absolute bottom-full right-0 mb-2 z-50 w-76 overflow-hidden rounded-[18px] border
             bg-(--lc-menu) border-(--lc-menu-bd) shadow-[0_18px_40px_rgba(0,0,0,0.35)] text-(--lc-tx)"
      role="dialog"
      aria-label={$t("podcasts.speed")}
    >
      <!-- En-tête, teinté de la couleur du lecteur -->
      <div class="flex items-center gap-2.5 px-4 pt-3.5 pb-3 border-b border-(--lc-menu-bd)
                  bg-linear-to-b from-(--lc-acc)/10 to-transparent">
        <span class="w-8 h-8 shrink-0 flex items-center justify-center rounded-[10px] bg-(--lc-acc)/15 text-(--lc-acc)">
          <Icon icon="material-symbols-light:speed-outline-rounded" width="20" />
        </span>
        <div class="min-w-0 leading-[1.2]">
          <p class="text-[13px] font-bold">{$t("podcasts.speed")}</p>
          <p class="text-[11px] text-(--lc-mu)">{$t("podcasts.speed_hint")}</p>
        </div>
      </div>

      <div class="px-3.5 pt-4 pb-3.5">
        <!-- Valeur et réglage fin -->
        <div class="flex items-center justify-between gap-3">
          {@render pas("material-symbols-light:remove-rounded", -STEP, $t("podcasts.speed_slower"))}
          <div class="min-w-0 text-center">
            <p class="font-mono text-[34px] leading-none font-bold tracking-[-0.03em] tabular-nums {normale ? '' : 'text-(--lc-acc)'}">
              {label($playbackSpeed)}
            </p>
            {#if reste > 0}
              <p class="mt-1.5 text-[11.5px] text-(--lc-mu) tabular-nums">
                {$t("podcasts.speed_left").replace("{time}", duree(resteVitesse))}
                {#if gagne >= 60}
                  · <span class="font-semibold text-(--lc-acc)">{$t("podcasts.speed_saved").replace("{time}", duree(gagne))}</span>
                {/if}
              </p>
            {/if}
          </div>
          {@render pas("material-symbols-light:add-rounded", STEP, $t("podcasts.speed_faster"))}
        </div>

        <!-- Préréglages, pastille glissante sous la vitesse choisie -->
        <div class="relative mt-4 grid grid-cols-6 p-1 rounded-[12px] bg-(--lc-survol)" role="radiogroup" aria-label={$t("podcasts.speed")}>
          {#if index >= 0}
            <span
              class="absolute top-1 bottom-1 left-1 rounded-[9px] bg-(--lc-menu) border border-(--lc-acc)/45 shadow-[0_2px_8px_rgba(0,0,0,0.18)]
                     transition-transform duration-200 ease-out"
              style="width: calc((100% - 0.5rem) / 6); transform: translateX({index * 100}%)"
            ></span>
          {/if}
          {#each SPEEDS as v, i}
            <button
              type="button"
              role="radio"
              aria-checked={i === index}
              class="relative h-8 rounded-[9px] cursor-pointer font-mono text-[11.5px] font-bold tabular-nums transition-colors
                     {i === index ? 'text-(--lc-acc)' : 'text-(--lc-mu) hover:text-(--lc-tx)'}"
              onclick={() => choisir(v)}
            >
              {v.toLocaleString($currentLocale)}
            </button>
          {/each}
        </div>
      </div>
    </div>
  {/if}
</div>
