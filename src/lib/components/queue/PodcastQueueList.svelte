<script lang="ts">
  // Épisodes à suivre, pendant qu'un podcast joue : sa file à part, la musique attend dessous.
  import Icon from "@iconify/svelte";
  import { t, currentLocale } from "#lib/i18n";
  import CoverImg from "#lib/components/ui/image/CoverImg.svelte";
  import { podcastQueue } from "#lib/stores/podcast/podcastQueue.store";
  import { playerService } from "#lib/services/player/player.service";

  const upcoming = $derived($podcastQueue.active ? $podcastQueue.tracks.slice($podcastQueue.index + 1) : []);

  function play(queueId: string) {
    const episode = podcastQueue.jump(queueId);
    if (episode) void playerService.playFile(episode, true);
  }
</script>

{#if $podcastQueue.active}
  <div class="mb-6">
    <div class="flex items-center justify-between px-2 py-3">
      <div class="flex items-center gap-2.5">
        <div class="flex items-center justify-center h-6 w-6 rounded-lg bg-violet-500/12 text-violet-600 dark:text-violet-300">
          <Icon icon="material-symbols:podcasts-rounded" width="14" />
        </div>
        <h3 class="text-[11px] font-bold uppercase tracking-[0.15em] text-violet-700 dark:text-violet-300">{$t("queue_panel.episodes_next")}</h3>
      </div>
      <span class="text-[11px] font-semibold tabular-nums text-neutral-400 dark:text-neutral-500">
        {upcoming.length.toLocaleString($currentLocale)}
      </span>
    </div>

    {#if upcoming.length}
      <div class="flex flex-col gap-0.5">
        {#each upcoming as ep (ep.queueId)}
          <button type="button" onclick={() => play(ep.queueId)}
                  class="group w-full flex items-center gap-3 px-2 py-1.5 rounded-xl text-left cursor-pointer transition-colors hover:bg-neutral-100 dark:hover:bg-white/5">
            <span class="h-11 w-11 shrink-0 overflow-hidden rounded-sm bg-neutral-100 dark:bg-neutral-800 ring-1 ring-black/4 dark:ring-white/6">
              <CoverImg path={ep.cover} alt="" class="h-full w-full object-cover" />
            </span>
            <span class="min-w-0 flex-1 flex flex-col gap-0.5">
              <span class="truncate text-[13px] font-semibold text-neutral-800 dark:text-neutral-200" title={ep.title}>{ep.title}</span>
              <span class="truncate text-[11px] font-medium text-neutral-400 dark:text-neutral-500">{ep.artist}</span>
            </span>
            <Icon icon="material-symbols:play-arrow-rounded" width="20" class="shrink-0 opacity-0 group-hover:opacity-100 text-violet-600 dark:text-violet-300" />
          </button>
        {/each}
      </div>
    {:else}
      <p class="px-2 text-[12.5px] text-neutral-400 dark:text-neutral-500">{$t("queue_panel.episodes_none")}</p>
    {/if}
  </div>
{/if}
