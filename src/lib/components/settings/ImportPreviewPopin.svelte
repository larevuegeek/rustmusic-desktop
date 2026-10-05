<script lang="ts">
  import Icon from "@iconify/svelte";
  import { t, currentLocale } from "#lib/i18n";

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

  function dateLisible(iso: string): string {
    const d = new Date(iso);
    return Number.isNaN(d.getTime()) ? iso : d.toLocaleString($currentLocale);
  }
</script>

<!-- svelte-ignore a11y_no_static_element_interactions -->
<div
  class="fixed inset-0 z-50 flex items-center justify-center"
  onkeydown={(e) => { if (e.key === 'Escape' && !busy) onclose(); }}
>
  <button
    type="button"
    class="absolute inset-0 bg-black/60 backdrop-blur-sm cursor-default"
    onclick={() => { if (!busy) onclose(); }}
    aria-label={$t("common.close")}
  ></button>

  <div class="relative w-full max-w-lg mx-4 max-h-[80vh] flex flex-col
              bg-neutral-50 dark:bg-neutral-900
              border border-neutral-200/60 dark:border-white/8
              rounded-2xl shadow-2xl shadow-black/20 overflow-hidden">

    <div class="flex items-center gap-3 px-6 py-4
                border-b border-neutral-200/60 dark:border-white/6">
      <div class="w-8 h-8 rounded-lg flex items-center justify-center
                  bg-green-500/10 border border-green-500/20">
        <Icon icon="lucide:hard-drive-upload" width="16" class="text-green-500" />
      </div>
      <div>
        <h2 class="text-base font-semibold text-neutral-800 dark:text-neutral-100">
          {$t("settings.import_preview_title")}
        </h2>
        <p class="text-[11px] text-neutral-400 dark:text-neutral-500">
          {$t("settings.import_preview_source").replace("{version}", report.app_version).replace("{date}", dateLisible(report.exported_at))}
        </p>
      </div>
    </div>

    <div class="flex-1 overflow-y-auto scrollbar-app px-6 py-5 space-y-4">

      <!-- Ce qui sera écrit -->
      <div class="space-y-1.5 text-sm text-neutral-700 dark:text-neutral-300">
        <div class="flex justify-between gap-4">
          <span>{$t("settings.import_preview_settings")}</span>
          <span class="tabular-nums">{report.settings}</span>
        </div>
        {#if report.settings_skipped > 0}
          <!-- Nommer ce qui est écarté évite qu'on le prenne pour un oubli. -->
          <div class="flex justify-between gap-4 text-[11px] text-neutral-400">
            <span>{$t("settings.import_preview_settings_skipped")}</span>
            <span class="tabular-nums">{report.settings_skipped}</span>
          </div>
        {/if}
        {#if report.profils_created > 0}
          <div class="flex justify-between gap-4">
            <span>{$t("settings.import_preview_profils")}</span>
            <span class="tabular-nums">{report.profils_created}</span>
          </div>
        {/if}
        <div class="flex justify-between gap-4">
          <span>{$t("settings.import_preview_playlists_created")}</span>
          <span class="tabular-nums">{report.playlists_created}</span>
        </div>
        {#if report.playlists_replaced > 0}
          <div class="flex justify-between gap-4 text-amber-600 dark:text-amber-400">
            <span>{$t("settings.import_preview_playlists_replaced")}</span>
            <span class="tabular-nums">{report.playlists_replaced}</span>
          </div>
        {/if}
        {#if report.playlists_skipped > 0}
          <div class="flex justify-between gap-4 text-[11px] text-neutral-400">
            <span>{$t("settings.import_preview_playlists_skipped")}</span>
            <span class="tabular-nums">{report.playlists_skipped}</span>
          </div>
        {/if}
        <div class="flex justify-between gap-4">
          <span>{$t("settings.import_preview_liked")}</span>
          <span class="tabular-nums">{report.liked}</span>
        </div>
      </div>

      <!-- L'appariement des morceaux : le vrai enjeu -->
      {#if total > 0}
        <div class="rounded-lg p-3 space-y-1.5
                    bg-neutral-100 dark:bg-white/5
                    border border-neutral-200/60 dark:border-white/6">
          <div class="flex justify-between gap-4 text-sm text-neutral-800 dark:text-neutral-200">
            <span class="font-medium">{$t("settings.import_preview_matched")}</span>
            <span class="tabular-nums">{retrouvees} / {total}</span>
          </div>
          <div class="flex justify-between gap-4 text-[11px] text-neutral-500 dark:text-neutral-400">
            <span>{$t("settings.import_preview_by_path")}</span>
            <span class="tabular-nums">{report.tracks_matched_by_path}</span>
          </div>
          {#if report.tracks_matched_by_tags > 0}
            <div class="flex justify-between gap-4 text-[11px] text-neutral-500 dark:text-neutral-400">
              <span>{$t("settings.import_preview_by_tags")}</span>
              <span class="tabular-nums">{report.tracks_matched_by_tags}</span>
            </div>
          {/if}
        </div>
      {/if}

      {#if report.tracks_missing > 0}
        <div class="rounded-lg p-3 space-y-2
                    bg-amber-500/10 border border-amber-500/20">
          <p class="text-sm text-amber-700 dark:text-amber-300">
            {$t(report.tracks_missing === 1 ? "settings.import_preview_missing_one" : "settings.import_preview_missing_n").replace("{n}", report.tracks_missing.toLocaleString($currentLocale))}
          </p>
          <ul class="space-y-0.5">
            {#each report.missing_samples as chemin}
              <li class="text-[10px] font-mono truncate text-amber-700/70 dark:text-amber-300/60"
                  title={chemin}>
                {chemin}
              </li>
            {/each}
          </ul>
          <p class="text-[11px] text-amber-700/70 dark:text-amber-300/60">
            {$t("settings.import_preview_missing_hint")}
          </p>
        </div>
      {/if}

      <!-- Le remplacement se demande, il n'est jamais le défaut -->
      <label class="flex items-start gap-2.5 p-3 rounded-lg cursor-pointer
                    hover:bg-neutral-100 dark:hover:bg-white/5">
        <input
          type="checkbox"
          bind:checked={replaceExisting}
          class="accent-amber-500 cursor-pointer mt-0.5"
        />
        <span class="text-xs text-neutral-600 dark:text-neutral-400">
          {$t("settings.import_preview_replace")}
          <span class="block text-[11px] text-neutral-400 dark:text-neutral-500">
            {$t("settings.import_preview_replace_hint")}
          </span>
        </span>
      </label>
    </div>

    <div class="flex items-center justify-end gap-2 px-6 py-3
                border-t border-neutral-200/60 dark:border-white/6">
      <button
        type="button"
        disabled={busy}
        onclick={onclose}
        class="text-xs px-3 py-1.5 rounded-lg cursor-pointer
               text-neutral-600 dark:text-neutral-300
               hover:bg-neutral-200/60 dark:hover:bg-white/5
               disabled:opacity-40 disabled:cursor-default"
      >
        {$t("common.cancel")}
      </button>
      <button
        type="button"
        disabled={busy}
        onclick={onconfirm}
        class="text-xs px-3 py-1.5 rounded-lg cursor-pointer font-medium text-white
               bg-emerald-500 hover:bg-emerald-400
               disabled:opacity-40 disabled:cursor-default
               flex items-center gap-1.5"
      >
        {#if busy}
          <Icon icon="lucide:loader-2" width="13" class="animate-spin" />
        {/if}
        {$t("settings.import_btn")}
      </button>
    </div>
  </div>
</div>
