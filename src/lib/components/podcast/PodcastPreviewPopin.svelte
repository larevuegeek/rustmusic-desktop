<script lang="ts">
  // Aperçu d'une émission de l'annuaire, sans s'y abonner : description et derniers épisodes.
  import Icon from "@iconify/svelte";
  import { goto } from "$app/navigation";
  import { t, currentLocale } from "#lib/i18n";
  import { popinStore } from "#lib/stores/ui/popin.store";
  import { podcastStore } from "#lib/stores/podcast/podcast.store";
  import { toasts } from "#lib/stores/ui/toast.store";
  import { episodeDate, episodeDuration } from "#lib/helper/tools/podcastFormat";
  import { isWebUrl } from "#lib/helper/tools/safeHtml";
  import SafeHtml from "#lib/components/ui/text/SafeHtml.svelte";
  import PodcastCover from "./PodcastCover.svelte";
  import type { DirectoryResult, PodcastPreview } from "#lib/types/ui/podcast/Podcast";

  let { podcast }: { podcast: DirectoryResult } = $props();

  let apercu = $state<PodcastPreview | null>(null);
  let erreur = $state(false);
  $effect(() => {
    const url = podcast.feed_url;
    apercu = null;
    erreur = false;
    podcastStore.preview(url).then(
      (p) => { if (url === podcast.feed_url) apercu = p; },
      () => { if (url === podcast.feed_url) erreur = true; },
    );
  });

  const suivi = $derived($podcastStore.podcasts.find((p) => p.feed_url === podcast.feed_url) ?? null);
  // Abonné : la pochette locale ; l'image distante n'est permise que de quelques hôtes (CSP).
  const locale = $derived(suivi?.cover_path ?? null);
  const image = $derived(locale ? null : (podcast.image_url ?? apercu?.image_url ?? null));
  const formatNumber = (n: number) => n.toLocaleString($currentLocale);

  let ajout = $state(false);
  async function follow() {
    ajout = true;
    try {
      const p = await podcastStore.follow(podcast.feed_url);
      toasts.push({ type: "success", title: $t("podcasts.followed").replace("{title}", p.title) });
    } catch (err) {
      toasts.push({ type: "error", title: $t("podcasts.error_follow"), message: String(err) });
    } finally {
      ajout = false;
    }
  }

  function open() {
    if (!suivi) return;
    popinStore.close();
    goto(`/podcasts/${suivi.id}`);
  }
</script>

<div class="relative flex flex-col max-h-[88vh] min-h-0">
  <!-- ─── En-tête : pochette floutée en fond, fiche, actions ─── -->
  <div class="relative shrink-0 overflow-hidden">
    {#if locale}
      <PodcastCover path={locale} size="1x" class="absolute! inset-0 w-full h-full scale-125 blur-3xl opacity-40 dark:opacity-30" />
    {:else if image}
      <img src={image} alt="" aria-hidden="true" class="absolute inset-0 w-full h-full object-cover scale-125 blur-3xl opacity-40 dark:opacity-30" />
    {/if}
    <div class="absolute inset-0 bg-linear-to-b from-transparent to-(--rg-carte)"></div>
    <button type="button" onclick={() => popinStore.close()} aria-label={$t("common.close")} title={$t("common.close")}
            class="absolute right-3 top-3 z-10 w-8.5 h-8.5 rounded-full flex items-center justify-center cursor-pointer transition-colors
                   bg-black/25 text-white hover:bg-black/45 backdrop-blur-sm">
      <Icon icon="material-symbols:close-rounded" width="20" />
    </button>

    <div class="relative flex gap-5 p-6 pb-5">
      <span class="w-32 h-32 shrink-0 overflow-hidden rounded-2xl bg-(--rg-s2) shadow-[0_14px_34px_rgba(0,0,0,0.35)]">
        {#if image}
          <img src={image} alt="" class="w-full h-full object-cover" />
        {:else}
          <PodcastCover path={locale} icone={40} class="w-full h-full" />
        {/if}
      </span>
      <div class="min-w-0 flex-1 flex flex-col justify-end gap-1.5 pr-8">
        <p class="text-[11.5px] font-bold uppercase tracking-widest text-violet-600 dark:text-violet-300">
          {[$t("podcasts.kicker_podcast"), podcast.genre].filter(Boolean).join(" · ")}
        </p>
        <h2 class="text-[26px] font-extrabold leading-tight tracking-[-0.02em] text-(--rg-tx) line-clamp-2">{podcast.title}</h2>
        {#if podcast.author}<p class="text-[14.5px] font-semibold text-(--rg-tx2) truncate">{podcast.author}</p>{/if}
        <div class="mt-2 flex flex-wrap items-center gap-2">
          {#if suivi}
            <button type="button" onclick={open}
                    class="h-10 px-4.5 inline-flex items-center gap-2 rounded-full cursor-pointer text-[13.5px] font-bold transition bg-violet-600 text-white hover:bg-violet-500">
              <Icon icon="material-symbols:arrow-forward-rounded" width="19" />{$t("podcasts.open")}
            </button>
            <span class="inline-flex items-center gap-1 text-[13px] font-semibold text-(--rg-gtx)">
              <Icon icon="material-symbols:check-rounded" width="17" />{$t("podcasts.following")}
            </span>
          {:else}
            <button type="button" onclick={follow} disabled={ajout}
                    class="h-10 px-4.5 inline-flex items-center gap-2 rounded-full cursor-pointer text-[13.5px] font-bold transition
                           bg-violet-600 text-white hover:bg-violet-500 disabled:opacity-60 disabled:cursor-wait">
              <Icon icon={ajout ? "material-symbols:progress-activity" : "material-symbols:add-rounded"} width="19" class={ajout ? "animate-spin" : ""} />
              {$t("podcasts.follow")}
            </button>
          {/if}
          {#if isWebUrl(apercu?.link)}
            <a href={apercu?.link} title={$t("podcasts.website")} aria-label={$t("podcasts.website")}
               class="w-10 h-10 rounded-full flex items-center justify-center transition-colors border border-(--rg-bd2) text-(--rg-tx2) hover:bg-(--rg-s2) hover:text-(--rg-tx)">
              <Icon icon="material-symbols:language" width="19" />
            </a>
          {/if}
        </div>
      </div>
    </div>
  </div>

  <!-- ─── Description et derniers épisodes ─── -->
  <div class="flex-1 min-h-0 overflow-y-auto scrollbar-app px-6 pb-6">
    {#if erreur}
      <p class="py-6 text-sm text-(--rg-mu)">{$t("podcasts.preview_error")}</p>
    {:else if !apercu}
      <div class="flex flex-col gap-3 py-2" aria-busy="true">
        {#each { length: 4 } as _, i (i)}
          <span class="h-3.5 rounded bg-(--rg-s2) animate-pulse" style="width: {90 - i * 15}%"></span>
        {/each}
      </div>
    {:else}
      {#if apercu.description}
        <SafeHtml html={apercu.description} class="text-[14px] leading-relaxed text-(--rg-tx2) line-clamp-4" />
      {/if}
      <div class="mt-5 mb-1.5 flex items-baseline gap-2">
        <h3 class="text-[12px] font-bold uppercase tracking-widest text-(--rg-mu)">{$t("podcasts.preview_latest")}</h3>
        <span class="text-[12px] text-(--rg-mu)">· {formatNumber(apercu.episode_count)} {$t(apercu.episode_count === 1 ? "podcasts.episodes_one" : "podcasts.episodes_n")}</span>
      </div>
      <ul class="flex flex-col">
        {#each apercu.episodes as e, i (i)}
          <li class="py-2.5 border-b border-(--rg-line) last:border-b-0">
            <p class="text-[12px] text-(--rg-mu)">
              {[episodeDate(e.published_at, $currentLocale), episodeDuration(e.duration)].filter(Boolean).join(" · ")}
            </p>
            <p class="mt-0.5 text-[14.5px] font-semibold leading-snug text-(--rg-tx) line-clamp-2">{e.title}</p>
            {#if e.summary}<p class="mt-0.5 text-[12.5px] leading-relaxed text-(--rg-mu) line-clamp-1">{e.summary}</p>{/if}
          </li>
        {/each}
      </ul>
    {/if}
  </div>
</div>
