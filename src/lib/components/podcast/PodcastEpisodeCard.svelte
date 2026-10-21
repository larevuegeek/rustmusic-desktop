<script lang="ts">
  // Carte d'un épisode à écouter : pochette de l'émission, lecture au survol, avancement.
  import Icon from "@iconify/svelte";
  import { t, currentLocale } from "#lib/i18n";
  import { episodeDate, episodeDuration } from "#lib/helper/tools/podcastFormat";
  import PodcastCover from "./PodcastCover.svelte";
  import type { LatestEpisode } from "#lib/types/ui/podcast/Podcast";

  let { episode, enLecture = false, onplay }: {
    episode: LatestEpisode;
    enLecture?: boolean;
    onplay: () => void;
  } = $props();

  const commence = $derived(episode.position > 0 && episode.position < (episode.duration ?? 0));
  const avancement = $derived(commence ? Math.min(100, (episode.position / (episode.duration ?? 1)) * 100) : 0);
  const detail = $derived(
    commence
      ? $t("podcasts.remaining").replace("{t}", episodeDuration((episode.duration ?? 0) - episode.position))
      : episodeDuration(episode.duration),
  );
</script>

<div class="group min-w-0 flex flex-col gap-2.5">
  <button type="button" onclick={onplay} title={$t("podcasts.play")} aria-label="{$t('podcasts.play')} : {episode.title}"
          class="relative block aspect-square w-full overflow-hidden rounded-2xl cursor-pointer
                 shadow-[0_10px_26px_rgba(0,0,0,0.16)] dark:shadow-[0_10px_26px_rgba(0,0,0,0.5)] transition-transform duration-200 group-hover:-translate-y-0.5">
    <PodcastCover path={episode.cover_path} icone={40} class="w-full h-full" />
    <span class="absolute inset-x-0 bottom-0 h-1/2 bg-linear-to-t from-black/55 to-transparent pointer-events-none"></span>
    <span class="absolute right-2.5 bottom-2.5 w-11 h-11 rounded-full flex items-center justify-center shadow-lg transition
                 {enLecture ? 'bg-(--rg-g) text-(--rg-on-g) opacity-100' : 'bg-white text-black opacity-0 translate-y-1 group-hover:opacity-100 group-hover:translate-y-0'}">
      <Icon icon={enLecture ? "material-symbols:graphic-eq-rounded" : "material-symbols:play-arrow-rounded"} width="26" />
    </span>
    {#if episode.new_count > 0}
      <span class="absolute left-2 top-2 h-6 px-2 inline-flex items-center rounded-full text-[11px] font-bold bg-(--rg-g) text-(--rg-on-g) shadow-sm">
        {episode.new_count > 1 ? `${episode.new_count} ${$t("podcasts.new_short_n")}` : $t("podcasts.new_badge")}
      </span>
    {/if}
    {#if commence}
      <span class="absolute left-2.5 right-16 bottom-4.5 h-1 rounded-full bg-white/30 overflow-hidden" aria-hidden="true">
        <span class="block h-full rounded-full bg-white" style="width: {avancement}%"></span>
      </span>
    {/if}
  </button>
  <span class="min-w-0 px-0.5">
    <a href="/podcasts/{episode.podcast_id}" class="block truncate text-[12px] font-semibold uppercase tracking-wide text-(--rg-mu) hover:text-(--rg-tx) hover:underline underline-offset-2">{episode.podcast_title}</a>
    <span class="mt-0.5 text-[14.5px] font-bold leading-snug line-clamp-2 {enLecture ? 'text-(--rg-gtx)' : 'text-(--rg-tx)'}" title={episode.title}>{episode.title}</span>
    <span class="mt-0.5 block truncate text-[12.5px] text-(--rg-mu)">{[episodeDate(episode.published_at, $currentLocale), detail].filter(Boolean).join(" · ")}</span>
  </span>
</div>
