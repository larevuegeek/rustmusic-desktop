<script lang="ts">
  import Icon from "@iconify/svelte";
  import { t, currentLocale } from "#lib/i18n";
  import { fade, scale } from "svelte/transition";
  import ToggleSwitch from "#lib/components/ui/input/ToggleSwitch.svelte";
  import PopinFooter from "#lib/components/ui/popin/PopinFooter.svelte";

  /**
   * Ce qu'un import ferait, montré avant qu'il ne le fasse.
   *
   * Un import touche les réglages et les playlists. Le lancer à l'aveugle, c'est
   * découvrir après coup que la moitié des morceaux n'a pas été retrouvée — sans
   * savoir si c'est le fichier qui est mauvais ou la bibliothèque qui a bougé.
   * L'aperçu répond avant.
   */
  export type ImportReport = {
    app_version: string;
    exported_at: string;
    settings: number;
    settings_skipped: number;
    profils_created: number;
    playlists_created: number;
    playlists_replaced: number;
    playlists_skipped: number;
    tracks_matched_by_path: number;
    tracks_matched_by_tags: number;
    tracks_missing: number;
    liked: number;
    missing_samples: string[];
  };

  // La visibilité est décidée par le parent, qui affiche ce composant tant
  // qu'il tient un rapport. Un `open` lié ferait deux sources de vérité pour
  // la même chose ; un rappel de fermeture n'en fait qu'une.
  let {
    report,
    busy = false,
    replaceExisting = $bindable(false),
    onconfirm,
    onclose,
  }: {
    report: ImportReport;
    busy?: boolean;
    replaceExisting: boolean;
    onconfirm: () => void;
    onclose: () => void;
  } = $props();

  const retrouvees = $derived(
    report.tracks_matched_by_path + report.tracks_matched_by_tags
  );
  const total = $derived(retrouvees + report.tracks_missing);

  function readableDate(iso: string): string {
    const d = new Date(iso);
    return Number.isNaN(d.getTime()) ? iso : d.toLocaleString($currentLocale);
  }
</script>

<!-- svelte-ignore a11y_no_static_element_interactions -->
<div
  class="fixed inset-0 z-50 flex items-center justify-center"
  onkeydown={(e) => { if (e.key === 'Escape' && !busy) onclose(); }}
  transition:fade={{ duration: 150 }}
>
  <button
    type="button"
    class="absolute inset-0 bg-black/60 backdrop-blur-sm cursor-default"
    onclick={() => { if (!busy) onclose(); }}
    aria-label={$t("common.close")}
  ></button>

  <div
    role="dialog"
    aria-modal="true"
    aria-label={$t("settings.import_preview_title")}
    class="relative w-full max-w-lg mx-4 max-h-[80vh] flex flex-col
           rounded-2xl overflow-hidden text-(--rg-tx)
           bg-(--rg-carte) border border-(--rg-bd) dark:border-[#2a312d]
           shadow-[0_24px_60px_-12px_rgba(0,0,0,0.25)] dark:shadow-[0_30px_80px_-20px_rgba(0,0,0,0.9)]"
    transition:scale={{ duration: 180, start: 0.96 }}
  >

    <div class="shrink-0 flex items-center gap-2.5 px-6 pt-5 pb-4 border-b border-(--rg-bd)
                bg-linear-135 from-(--rg-g)/8 to-transparent to-60%">
      <span class="shrink-0 w-8 h-8 rounded-[9px] flex items-center justify-center
                   bg-(--rg-gbg) text-(--rg-g) border border-(--rg-gbd)">
        <Icon icon="lucide:hard-drive-upload" width="15" />
      </span>
      <div class="min-w-0">
        <h2 class="text-[17px] font-extrabold tracking-[-0.01em] text-(--rg-tx) truncate">
          {$t("settings.import_preview_title")}
        </h2>
        <p class="text-xs text-(--rg-mu) truncate">
          {$t("settings.import_preview_source").replace("{version}", report.app_version).replace("{date}", readableDate(report.exported_at))}
        </p>
      </div>
    </div>

    <div class="flex-1 overflow-y-auto scrollbar-app px-6 pt-5 pb-6 space-y-4">

      <!-- Ce qui sera écrit -->
      <div class="px-4 py-3 rounded-2xl space-y-2 text-sm border border-(--rg-bd) bg-(--rg-creux) text-(--rg-tx2)">
        <div class="flex justify-between gap-4">
          <span>{$t("settings.import_preview_settings")}</span>
          <span class="tabular-nums font-semibold text-(--rg-tx)">{report.settings}</span>
        </div>
        {#if report.settings_skipped > 0}
          <!-- Nommer ce qui est écarté évite qu'on le prenne pour un oubli. -->
          <div class="flex justify-between gap-4 text-xs text-(--rg-mu)">
            <span>{$t("settings.import_preview_settings_skipped")}</span>
            <span class="tabular-nums">{report.settings_skipped}</span>
          </div>
        {/if}
        {#if report.profils_created > 0}
          <div class="flex justify-between gap-4">
            <span>{$t("settings.import_preview_profils")}</span>
            <span class="tabular-nums font-semibold text-(--rg-tx)">{report.profils_created}</span>
          </div>
        {/if}
        <div class="flex justify-between gap-4">
          <span>{$t("settings.import_preview_playlists_created")}</span>
          <span class="tabular-nums font-semibold text-(--rg-tx)">{report.playlists_created}</span>
        </div>
        {#if report.playlists_replaced > 0}
          <div class="flex justify-between gap-4 font-semibold text-(--rg-am)">
            <span>{$t("settings.import_preview_playlists_replaced")}</span>
            <span class="tabular-nums">{report.playlists_replaced}</span>
          </div>
        {/if}
        {#if report.playlists_skipped > 0}
          <div class="flex justify-between gap-4 text-xs text-(--rg-mu)">
            <span>{$t("settings.import_preview_playlists_skipped")}</span>
            <span class="tabular-nums">{report.playlists_skipped}</span>
          </div>
        {/if}
        <div class="flex justify-between gap-4">
          <span>{$t("settings.import_preview_liked")}</span>
          <span class="tabular-nums font-semibold text-(--rg-tx)">{report.liked}</span>
        </div>
      </div>

      <!-- L'appariement des morceaux : le vrai enjeu -->
      {#if total > 0}
        <div class="p-4 rounded-2xl border border-(--rg-gbd) bg-linear-135 from-(--rg-gbg) to-transparent">
          <div class="flex items-baseline justify-between gap-4">
            <span class="text-sm font-semibold text-(--rg-tx)">{$t("settings.import_preview_matched")}</span>
            <span class="tabular-nums text-(--rg-tx)">
              <span class="text-[22px] font-extrabold leading-none">{retrouvees}</span>
              <span class="text-sm font-semibold text-(--rg-mu)">/ {total}</span>
            </span>
          </div>
          <!-- Jauge : la part retrouvée se lit avant les chiffres -->
          <div class="mt-3 h-1.5 rounded-full overflow-hidden bg-(--rg-off)">
            <div class="h-full rounded-full bg-(--rg-g)" style="width: {(retrouvees / total) * 100}%"></div>
          </div>
          <div class="mt-3 space-y-1 text-xs text-(--rg-mu)">
            <div class="flex justify-between gap-4">
              <span>{$t("settings.import_preview_by_path")}</span>
              <span class="tabular-nums font-semibold text-(--rg-tx2)">{report.tracks_matched_by_path}</span>
            </div>
            {#if report.tracks_matched_by_tags > 0}
              <div class="flex justify-between gap-4">
                <span>{$t("settings.import_preview_by_tags")}</span>
                <span class="tabular-nums font-semibold text-(--rg-tx2)">{report.tracks_matched_by_tags}</span>
              </div>
            {/if}
          </div>
        </div>
      {/if}

      {#if report.tracks_missing > 0}
        <div class="p-4 rounded-2xl space-y-2 bg-(--rg-ambg) border border-(--rg-ambd)">
          <p class="flex items-start gap-2 text-sm font-semibold text-(--rg-amtx)">
            <Icon icon="material-symbols:warning-outline-rounded" width="17" class="shrink-0 mt-px text-(--rg-am)" />
            {$t(report.tracks_missing === 1 ? "settings.import_preview_missing_one" : "settings.import_preview_missing_n").replace("{n}", report.tracks_missing.toLocaleString($currentLocale))}
          </p>
          <ul class="space-y-0.5 pl-6">
            {#each report.missing_samples as chemin}
              <li class="text-[11px] font-mono truncate text-(--rg-amtx) opacity-75"
                  title={chemin}>
                {chemin}
              </li>
            {/each}
          </ul>
          <p class="pl-6 text-xs text-(--rg-amtx) opacity-80">
            {$t("settings.import_preview_missing_hint")}
          </p>
        </div>
      {/if}

      <!-- Le remplacement se demande, il n'est jamais le défaut -->
      <label class="flex items-center gap-4 p-4 rounded-2xl cursor-pointer transition-colors border
                    {replaceExisting ? 'border-(--rg-ambd) bg-(--rg-ambg)' : 'border-(--rg-bd) hover:border-(--rg-bd2)'}">
        <span class="flex-1 min-w-0">
          <span class="block text-sm font-semibold text-(--rg-tx)">{$t("settings.import_preview_replace")}</span>
          <span class="block mt-0.5 text-xs text-(--rg-mu)">{$t("settings.import_preview_replace_hint")}</span>
        </span>
        <ToggleSwitch
          checked={replaceExisting}
          onclick={() => (replaceExisting = !replaceExisting)}
          label={$t("settings.import_preview_replace")}
          color="amber"
        />
      </label>
    </div>

    <PopinFooter
      cancelLabel={$t("common.cancel")}
      oncancel={onclose}
      cancelDisabled={busy}
      submitLabel={$t("settings.import_btn")}
      submitIcon="lucide:hard-drive-upload"
      onsubmit={onconfirm}
      {busy}
    />
  </div>
</div>
