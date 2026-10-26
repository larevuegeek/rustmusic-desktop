<script lang="ts">
  // Chapitres de l'épisode : liste cliquable, chapitre en cours suivi, précédent / suivant.
  import Icon from "@iconify/svelte";
  import { tick } from "svelte";
  import { t } from "#lib/i18n";
  import { chapters, chapterAt } from "#lib/stores/podcast/chapters.store";
  import { player } from "#lib/stores/player/player.store";
  import { playerService } from "#lib/services/player/player.service";
  import { minutesSeconds } from "#lib/helper/tools/dateTools";

  let menuEl: HTMLElement | null = $state(null);
  let listeEl: HTMLElement | null = $state(null);

  const liste = $derived($chapters.list);
  const position = $derived($player?.jsPosition ?? 0);
  const duree = $derived($player?.duration ?? 0);
  const courant = $derived(chapterAt(liste, position));
  const titre = (i: number) => liste[i]?.title || $t("podcasts.chapter_untitled").replace("{n}", String(i + 1));

  // Avancement dans le chapitre en cours (0 → 1).
  const avance = $derived.by(() => {
    if (courant < 0) return 0;
    const fin = liste[courant + 1]?.start ?? duree;
    const debut = liste[courant].start;
    return fin > debut ? Math.min(1, Math.max(0, (position - debut) / (fin - debut))) : 0;
  });

  function aller(i: number) {
    const c = liste[Math.max(0, Math.min(liste.length - 1, i))];
    if (c) void playerService.seekTo(c.start);
  }

  // « Précédent » : début du chapitre, ou le précédent si on vient d'y entrer.
  function precedent() {
    const debut = liste[courant]?.start ?? 0;
    aller(position - debut > 3 ? courant : courant - 1);
  }

  // Ouverture : le chapitre en cours en vue.
  $effect(() => {
    if (!$chapters.open) return;
    void tick().then(() => listeEl?.querySelector("[aria-current='true']")?.scrollIntoView({ block: "center" }));
  });

  // Fermeture au clic extérieur.
  $effect(() => {
    if (!$chapters.open) return;
    const handleOutside = (e: MouseEvent) => {
      if (menuEl && !menuEl.contains(e.target as Node)) chapters.setOpen(false);
    };
    const timer = setTimeout(() => document.addEventListener("click", handleOutside), 50);
    return () => {
      clearTimeout(timer);
      document.removeEventListener("click", handleOutside);
    };
  });
</script>

{#snippet rond(icone: string, label: string, disabled: boolean, onclick: () => void)}
  <button
    type="button"
    class="w-8 h-8 shrink-0 flex items-center justify-center rounded-full border border-(--lc-menu-bd) cursor-pointer transition-colors
           text-(--lc-tx) hover:bg-(--lc-survol) disabled:opacity-35 disabled:cursor-default disabled:hover:bg-transparent"
    title={label}
    aria-label={label}
    {disabled}
    {onclick}
  >
    <Icon icon={icone} width="20" />
  </button>
{/snippet}

<div class="relative" bind:this={menuEl}>
  <button
    type="button"
    class="relative w-9.5 h-9.5 shrink-0 flex items-center justify-center rounded-full cursor-pointer transition-colors
           text-(--lc-ic) hover:text-(--lc-ic-fort) hover:bg-(--lc-survol) {$chapters.open ? 'bg-(--lc-survol) text-(--lc-ic-fort)!' : ''}"
    title={$t("podcasts.chapters")}
    aria-label={$t("podcasts.chapters")}
    aria-expanded={$chapters.open}
    onclick={() => chapters.setOpen(!$chapters.open)}
  >
    <Icon icon="material-symbols-light:toc-rounded" width="24" />
  </button>

  {#if $chapters.open}
    <div
      class="absolute bottom-full right-0 mb-2 z-50 w-88 overflow-hidden rounded-[18px] border
             bg-(--lc-menu) border-(--lc-menu-bd) shadow-[0_18px_40px_rgba(0,0,0,0.35)] text-(--lc-tx)"
      role="dialog"
      aria-label={$t("podcasts.chapters")}
    >
      <!-- En-tête : chapitre en cours, précédent / suivant -->
      <div class="flex items-center gap-2.5 px-4 pt-3.5 pb-3 border-b border-(--lc-menu-bd) bg-linear-to-b from-(--lc-acc)/10 to-transparent">
        <span class="w-8 h-8 shrink-0 flex items-center justify-center rounded-[10px] bg-(--lc-acc)/15 text-(--lc-acc)">
          <Icon icon="material-symbols-light:toc-rounded" width="20" />
        </span>
        <div class="flex-1 min-w-0 leading-[1.2]">
          <p class="text-[13px] font-bold">{$t("podcasts.chapters")}</p>
          <p class="text-[11px] text-(--lc-mu) tabular-nums">
            {$t("podcasts.chapter_of").replace("{n}", String(Math.max(1, courant + 1))).replace("{total}", String(liste.length))}
          </p>
        </div>
        {@render rond("material-symbols-light:skip-previous-rounded", $t("podcasts.chapter_prev"), courant <= 0 && position < 3, precedent)}
        {@render rond("material-symbols-light:skip-next-rounded", $t("podcasts.chapter_next"), courant >= liste.length - 1, () => aller(courant + 1))}
      </div>

      <ol class="max-h-80 overflow-y-auto p-1.5" bind:this={listeEl}>
        {#each liste as c, i (i)}
          {@const actif = i === courant}
          <li class="relative">
            <button
              type="button"
              aria-current={actif}
              class="w-full flex items-center gap-3 pl-3 py-2.25 rounded-[11px] text-left cursor-pointer transition-colors
                     {c.url ? 'pr-11' : 'pr-2.5'} {actif ? 'bg-(--lc-acc)/12' : 'hover:bg-(--lc-survol)'}"
              onclick={() => aller(i)}
            >
              <span class="w-12 shrink-0 font-mono text-[11.5px] tabular-nums {actif ? 'text-(--lc-acc) font-semibold' : 'text-(--lc-mu)'}">
                {minutesSeconds(c.start)}
              </span>
              <span class="flex-1 min-w-0 truncate text-[13px] {actif ? 'font-semibold text-(--lc-tx)' : 'text-(--lc-tx2)'}" title={titre(i)}>
                {titre(i)}
              </span>
              {#if actif}
                <Icon icon="material-symbols-light:graphic-eq-rounded" width="16" class="shrink-0 text-(--lc-acc)" />
              {/if}
            </button>
            {#if c.url}
              <a href={c.url} target="_blank" rel="noreferrer" title={$t("podcasts.chapter_link")} aria-label={$t("podcasts.chapter_link")}
                 class="absolute right-2 top-1/2 -translate-y-1/2 w-7 h-7 flex items-center justify-center rounded-full text-(--lc-mu) hover:text-(--lc-tx) hover:bg-(--lc-survol)">
                <Icon icon="material-symbols-light:open-in-new-rounded" width="15" />
              </a>
            {/if}
            {#if actif}
              <span class="absolute left-3 right-3 bottom-0.5 h-0.5 rounded-full bg-(--lc-acc)/15 overflow-hidden">
                <span class="block h-full bg-(--lc-acc)" style:width="{avance * 100}%"></span>
              </span>
            {/if}
          </li>
        {/each}
      </ol>
    </div>
  {/if}
</div>
