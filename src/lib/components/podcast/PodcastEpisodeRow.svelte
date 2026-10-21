<script lang="ts">
  // Un épisode : date et durée, titre, résumé ; à droite téléchargement, menu et lecture.
  // Dépliée, sa description complète.
  import Icon from "@iconify/svelte";
  import { invoke } from "@tauri-apps/api/core";
  import { t, currentLocale } from "#lib/i18n";
  import SafeHtml from "#lib/components/ui/text/SafeHtml.svelte";
  import Menu from "#lib/components/ui/menu/Menu.svelte";
  import MenuItem from "#lib/components/ui/menu/MenuItem.svelte";
  import { episodeDate, episodeDuration } from "#lib/helper/tools/podcastFormat";
  import type { EpisodeView } from "#lib/types/ui/podcast/Podcast";

  let { episode, onplay, ontoggleplayed, onnext, onenqueue, ondownload, ondelete, lecture = null, telechargement = false }: {
    episode: EpisodeView;
    onplay: () => void;
    ontoggleplayed: () => void;
    onnext: () => void;
    onenqueue: () => void;
    ondownload: () => void;
    ondelete: () => void;
    /** L'épisode est celui du lecteur : en cours ou en pause. */
    lecture?: "playing" | "paused" | null;
    /** Téléchargement en cours. */
    telechargement?: boolean;
  } = $props();

  const enCours = $derived(!episode.played && episode.position > 0 && episode.position < (episode.duration ?? 0));
  const avancement = $derived(enCours ? Math.min(100, (episode.position / (episode.duration ?? 1)) * 100) : 0);

  let menu = $state(false);
  const closeMenuThen = (f: () => void) => () => { menu = false; f(); };

  // La description complète n'est demandée qu'au dépliage : elle pèse sur les longues listes.
  let ouvert = $state(false);
  let description = $state<string | null>(null);
  async function toggle() {
    ouvert = !ouvert;
    if (ouvert && description === null) {
      const e = await invoke<EpisodeView | null>("get_podcast_episode", { episodeId: episode.id }).catch(() => null);
      description = e?.description ?? episode.summary ?? "";
    }
  }
</script>

<li class="group relative rounded-2xl px-4 py-3.5 -mx-4 transition-colors
           {lecture ? 'bg-violet-500/8 ring-1 ring-inset ring-violet-500/25' : 'hover:bg-(--rg-hover)'}">
  <div class="flex items-start gap-4">
    <div class="flex-1 min-w-0">
      <!-- Date, numéro, durée ou avancement, état -->
      <p class="flex flex-wrap items-center gap-x-2 gap-y-1 text-[12.5px] text-(--rg-mu)">
        {#if episode.is_new && !episode.played}
          <span class="h-5 px-1.5 inline-flex items-center rounded-md text-[10.5px] font-bold uppercase tracking-wide bg-(--rg-g) text-(--rg-on-g)">{$t("podcasts.new_badge")}</span>
        {/if}
        <span class="font-semibold text-(--rg-tx2)">{episodeDate(episode.published_at, $currentLocale)}</span>
        {#if episode.season != null && episode.episode != null}
          <span>·</span><span>{$t("podcasts.season_episode").replace("{s}", String(episode.season)).replace("{e}", String(episode.episode))}</span>
        {/if}
        {#if enCours}
          <span>·</span>
          <span class="inline-flex items-center gap-2">
            <span class="w-16 h-1 rounded-full bg-(--rg-s2) overflow-hidden" aria-hidden="true">
              <span class="block h-full rounded-full bg-violet-500" style="width: {avancement}%"></span>
            </span>
            <span class="font-semibold text-violet-600 dark:text-violet-300">{$t("podcasts.remaining").replace("{t}", episodeDuration((episode.duration ?? 0) - episode.position))}</span>
          </span>
        {:else if episode.duration}
          <span>·</span><span>{episodeDuration(episode.duration)}</span>
        {/if}
        {#if episode.played}
          <span>·</span>
          <span class="inline-flex items-center gap-1 font-semibold text-(--rg-gtx)"><Icon icon="material-symbols:check-circle-rounded" width="15" />{$t("podcasts.played")}</span>
        {/if}
        {#if episode.file_path}
          <span class="inline-flex items-center" title={$t("podcasts.downloaded")} aria-label={$t("podcasts.downloaded")}>
            <Icon icon="material-symbols:download-done-rounded" width="16" class="text-(--rg-mu)" />
          </span>
        {/if}
      </p>

      <button type="button" onclick={toggle} aria-expanded={ouvert} class="mt-1 block w-full text-left cursor-pointer">
        <span class="text-[16px] font-bold leading-snug line-clamp-2
                     {lecture ? 'text-violet-600 dark:text-violet-300' : episode.played ? 'text-(--rg-mu)' : 'text-(--rg-tx)'}">{episode.title}</span>
        {#if !ouvert && episode.summary}
          <span class="mt-1 text-[13.5px] leading-relaxed text-(--rg-mu) line-clamp-2">{episode.summary}</span>
        {/if}
      </button>

      {#if ouvert}
        {#if description === null}
          <Icon icon="material-symbols:progress-activity" width="18" class="mt-2 animate-spin text-(--rg-mu)" />
        {:else}
          <SafeHtml html={description} class="mt-2 max-w-170 text-[13.5px] leading-relaxed text-(--rg-tx2)" />
        {/if}
      {/if}
    </div>

    <!-- Actions : discrètes au repos, la lecture toujours visible -->
    <div class="shrink-0 self-center flex items-center gap-1">
      {#if telechargement}
        <span class="w-9 h-9 flex items-center justify-center text-(--rg-mu)" title={$t("podcasts.downloading")}>
          <Icon icon="material-symbols:progress-activity" width="18" class="animate-spin" />
        </span>
      {:else if !episode.file_path}
        <button type="button" onclick={ondownload} title={$t("podcasts.download")} aria-label={$t("podcasts.download")}
                class="w-9 h-9 rounded-full flex items-center justify-center cursor-pointer transition text-(--rg-mu)
                       opacity-0 group-hover:opacity-100 focus-visible:opacity-100 hover:bg-(--rg-s2) hover:text-(--rg-tx)">
          <Icon icon="material-symbols:download-rounded" width="20" />
        </button>
      {/if}
      <div class="relative">
        <button type="button" onclick={() => (menu = !menu)} title={$t("common.menu")} aria-label={$t("common.menu")} aria-expanded={menu}
                class="w-9 h-9 rounded-full flex items-center justify-center cursor-pointer transition text-(--rg-mu) hover:bg-(--rg-s2) hover:text-(--rg-tx)
                       {menu ? 'bg-(--rg-s2) text-(--rg-tx)' : 'opacity-0 group-hover:opacity-100 focus-visible:opacity-100'}">
          <Icon icon="material-symbols:more-horiz" width="20" />
        </button>
        <Menu bind:open={menu} class="w-64">
          <MenuItem icon="material-symbols:playlist-play-rounded" onclick={closeMenuThen(onnext)}>{$t("common.add_next")}</MenuItem>
          <MenuItem icon="material-symbols:playlist-add-rounded" onclick={closeMenuThen(onenqueue)}>{$t("podcasts.add_to_queue")}</MenuItem>
          <MenuItem icon={episode.played ? "material-symbols:remove-done-rounded" : "material-symbols:done-all-rounded"} onclick={closeMenuThen(ontoggleplayed)}>
            {episode.played ? $t("podcasts.mark_unplayed") : $t("podcasts.mark_played")}
          </MenuItem>
          {#if episode.file_path}
            <MenuItem icon="material-symbols:delete-outline-rounded" danger onclick={closeMenuThen(ondelete)}>{$t("podcasts.delete_download")}</MenuItem>
          {:else if !telechargement}
            <MenuItem icon="material-symbols:download-rounded" onclick={closeMenuThen(ondownload)}>{$t("podcasts.download")}</MenuItem>
          {/if}
        </Menu>
      </div>
      <button type="button" onclick={onplay}
              title={lecture === "playing" ? $t("podcasts.pause") : $t("podcasts.play")}
              aria-label={lecture === "playing" ? $t("podcasts.pause") : $t("podcasts.play")}
              class="ml-1 w-11 h-11 rounded-full flex items-center justify-center cursor-pointer transition
                     {lecture
                       ? 'bg-violet-600 text-white shadow-[0_6px_18px_rgba(139,92,246,0.4)]'
                       : 'border border-(--rg-bd2) text-(--rg-tx) group-hover:bg-violet-600 group-hover:border-violet-600 group-hover:text-white'}">
        <Icon icon={lecture === "playing" ? "material-symbols:pause-rounded" : "material-symbols:play-arrow-rounded"} width="24" />
      </button>
    </div>
  </div>
</li>
